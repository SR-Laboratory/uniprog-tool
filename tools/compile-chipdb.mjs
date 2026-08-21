/* global process, console, Buffer */
import fs from 'node:fs'
import path from 'node:path'
import { parse as parseToml } from 'smol-toml'

// Compile the split TOML flash database directly into the runtime
// `chiplib.bin` format. There is no XML intermediate step.
//
//   node tools/compile-chipdb.mjs --output build/chipdb/chiplib.bin
//   node tools/compile-chipdb.mjs --check

const root = path.resolve(import.meta.dirname, '..')
const flashDbRoot = path.join(root, 'flashdb')

const MAGIC = 0x50494843
const VERSION = 20250628
const MAX_ID_LEN = 16
const INDEX_ENTRY_SIZE = 24
const HEADER_SIZE = 20

const PROTOCOL_IDS = {
  SPI_EC: 0,
  SPI_DATA_45: 1,
  SPI_NAND: 2,
  SPI_NOR: 3,
  SPI_EEPROM: 4,
  'SPI_F-RAM': 5,
  I2C: 6,
  'I2C_F-RAM': 7,
  I2C_SPD: 8,
  Microwire: 9,
  AVR: 10,
  MCU: 11,
  PARALLEL_NAND: 12,
}

function fail(message) {
  console.error(`[chipdb] ${message}`)
  process.exit(1)
}

function parseArgs(args) {
  const outputIndex = args.indexOf('--output')
  const output = outputIndex >= 0 ? args[outputIndex + 1] : null
  if (!output) fail('missing --output <path>')
  return { output: path.resolve(output), checkOnly: args.includes('--check') }
}

function readManifest() {
  const file = path.join(flashDbRoot, 'manifest.toml')
  if (!fs.existsSync(file)) fail(`manifest not found: ${file}`)
  const document = parseToml(fs.readFileSync(file, 'utf8'))
  const order = document.chipdb?.order
  const sources = document.chipdb?.sources
  if (!Array.isArray(order) || !sources || typeof sources !== 'object') {
    fail(`invalid chiplib manifest: ${file}`)
  }
  return { order, sources }
}

function stringifyTomlValue(value) {
  if (typeof value === 'string') return value
  if (typeof value === 'number') return String(value)
  if (typeof value === 'boolean') return value ? 'true' : 'false'
  return JSON.stringify(value)
}

function collectEntries() {
  const { order, sources } = readManifest()
  const entries = []
  const dataChunks = []
  let chips = 0
  const idCounts = new Map()

  for (const tag of order) {
    const protocolId = PROTOCOL_IDS[tag]
    if (protocolId === undefined) fail(`unknown protocol in manifest: ${tag}`)
    const relative = sources[tag]
    if (!relative) fail(`manifest has no source for protocol ${tag}`)
    const file = path.join(flashDbRoot, relative)
    if (!fs.existsSync(file)) fail(`flash database source not found: ${file}`)
    const document = parseToml(fs.readFileSync(file, 'utf8'))
    if (document.protocol !== tag) {
      fail(`${relative}: expected protocol "${tag}", got "${document.protocol}"`)
    }
    const vendors = document.vendors
    if (!vendors || typeof vendors !== 'object') fail(`${relative}: missing [vendors]`)

    for (const vendor of Object.keys(vendors).sort()) {
      const vendorTable = vendors[vendor]
      if (!Array.isArray(vendorTable?.chips)) fail(`${relative}: ${vendor}.chips must be an array`)
      for (const chip of vendorTable.chips) {
        if (!chip || typeof chip !== 'object') fail(`${relative}: invalid chip entry`)
        const model = chip.model
        if (typeof model !== 'string' || model.length === 0) {
          fail(`${relative}: chip entry missing model string`)
        }
        const id = typeof chip.id === 'string' ? chip.id : String(chip.id ?? '')
        const idBuffer = Buffer.alloc(MAX_ID_LEN)
        idBuffer.write(id.slice(0, MAX_ID_LEN), 0, 'utf8')

        const attrKeys = Object.keys(chip)
          .filter((key) => key !== 'model')
          .sort()
        const payload = [`vendor=${vendor}`, `model=${model}`, `protocol=${tag}`]
        for (const key of attrKeys) {
          payload.push(`${key}=${stringifyTomlValue(chip[key])}`)
        }
        const blob = Buffer.from(`${payload.join('\0')}\0`, 'utf8')
        if (blob.length > 0xffff) fail(`${relative}: ${model} attribute blob too large`)

        const dataOffset = dataChunks.reduce((sum, chunk) => sum + chunk.length, 0)
        dataChunks.push(blob)
        entries.push({ id: idBuffer, dataOffset, dataLen: blob.length, protocol: protocolId })
        chips += 1
        if (id) idCounts.set(id, (idCounts.get(id) ?? 0) + 1)
      }
    }
    console.log(`[chipdb] ${tag} <- ${relative}`)
  }

  let duplicateIds = 0
  for (const count of idCounts.values()) if (count > 1) duplicateIds += 1
  console.log(
    `[chipdb] collected ${chips} chips, ${idCounts.size} unique ids, ${duplicateIds} ids with multiple models`,
  )
  return { entries, dataChunks, chips }
}

function encodeBin({ entries, dataChunks }) {
  entries.sort((a, b) => Buffer.compare(a.id, b.id))
  const data = Buffer.concat(dataChunks)
  const dataOffset = HEADER_SIZE + entries.length * INDEX_ENTRY_SIZE
  const plain = Buffer.alloc(dataOffset + data.length)

  plain.writeUInt32LE(MAGIC, 0)
  plain.writeUInt32LE(VERSION, 4)
  plain.writeUInt32LE(entries.length, 8)
  plain.writeUInt32LE(dataOffset, 12)
  plain.writeUInt16LE(INDEX_ENTRY_SIZE, 16)
  plain.writeUInt16LE(0, 18)

  entries.forEach((entry, index) => {
    const offset = HEADER_SIZE + index * INDEX_ENTRY_SIZE
    entry.id.copy(plain, offset, 0, MAX_ID_LEN)
    plain.writeUInt32LE(entry.dataOffset, offset + MAX_ID_LEN)
    plain.writeUInt16LE(entry.dataLen, offset + MAX_ID_LEN + 4)
    plain.writeUInt16LE(entry.protocol, offset + MAX_ID_LEN + 6)
  })
  data.copy(plain, dataOffset)
  return plain
}

function obfuscate(data) {
  const output = Buffer.alloc(data.length)
  for (let i = 0; i < data.length; i += 1) {
    const mask = ((1 << (i & 3)) ^ (1 << (i % 7)) ^ (1 << ((i % 13) + 4))) & 0xff
    const value = (data[i] ^ mask) & 0xff
    const rotate = i % 8
    output[i] = rotate === 0 ? value : ((value << rotate) | (value >> (8 - rotate))) & 0xff
  }
  return output
}

const { output, checkOnly } = parseArgs(process.argv.slice(2))
const collected = collectEntries()
const plain = encodeBin(collected)
const sealed = obfuscate(plain)

if (checkOnly) {
  console.log(`[chipdb] format check passed for ${collected.chips} chips`)
} else {
  fs.mkdirSync(path.dirname(output), { recursive: true })
  fs.writeFileSync(output, sealed)
  console.log(`[chipdb] wrote ${output} (${sealed.length} bytes)`)
}
