/* global process, console */
import fs from 'node:fs'
import path from 'node:path'
import { parse as parseToml } from 'smol-toml'

// Merge the split chiplib fragments under `flashdb/protocols/` into the
// single custom XML document consumed by `upt-chipdb`.
//
//   node tools/merge-chipdb.mjs --output build/chipdb/chiplib.xml
//   node tools/merge-chipdb.mjs --check
//
// The merged file is generated output and must never be edited by hand.

const root = path.resolve(import.meta.dirname, '..')
const flashDbRoot = path.join(root, 'flashdb')

function fail(message) {
  console.error(`[chipdb] ${message}`)
  process.exit(1)
}

const args = process.argv.slice(2)
const outputIndex = args.indexOf('--output')
const output = outputIndex >= 0 ? args[outputIndex + 1] : null
const checkOnly = args.includes('--check')
if (!output) fail('missing --output <path>')

const manifestPath = path.join(flashDbRoot, 'manifest.toml')
if (!fs.existsSync(manifestPath)) fail(`manifest not found: ${manifestPath}`)
const document = parseToml(fs.readFileSync(manifestPath, 'utf8'))
const order = document.chipdb?.order
const sources = document.chipdb?.sources
if (!Array.isArray(order) || !sources || typeof sources !== 'object') {
  fail(`invalid chiplib manifest: ${manifestPath}`)
}

const fragments = []
for (const tag of order) {
  const relative = sources[tag]
  if (!relative) fail(`manifest has no source for protocol ${tag}`)
  const file = path.join(flashDbRoot, relative)
  if (!fs.existsSync(file)) fail(`chipdb fragment not found: ${file}`)
  const text = fs.readFileSync(file, 'utf8')
  if (!text.includes(`<${tag}>`) || !text.includes(`</${tag}>`)) {
    fail(`fragment ${relative} does not contain <${tag}>`)
  }
  fragments.push({ tag, text: text.replace(/^\uFEFF/, '') })
  console.log(`[chipdb] ${tag} <- ${relative}`)
}

const merged = [
  '<?xml version="1.0" encoding="utf-8"?>',
  '<chiplist>',
  ...fragments.map((fragment) => fragment.text.trimEnd()),
  '</chiplist>',
  '',
].join('\n')

const chipPattern = /<[A-Za-z0-9_().-]+\s+[^>]*\bid="([^"]*)"[^>]*\/>/g
const ids = new Map()
let chipCount = 0
let duplicateIds = 0
for (const match of merged.matchAll(chipPattern)) {
  const id = match[1]
  if (!id) continue
  chipCount += 1
  if (ids.has(id)) duplicateIds += 1
  ids.set(id, (ids.get(id) ?? 0) + 1)
}
console.log(
  `[chipdb] merged ${chipCount} chips, ${ids.size} unique non-empty ids, ${duplicateIds} duplicate id(s)`,
)
if (duplicateIds > 0) {
  console.log(
    '[chipdb] duplicate ids are preserved intentionally (multiple models can share a JEDEC id)',
  )
}

if (checkOnly) {
  console.log(`[chipdb] manifest and fragments are valid (${chipCount} chips)`)
} else {
  fs.mkdirSync(path.dirname(path.resolve(output)), { recursive: true })
  fs.writeFileSync(path.resolve(output), merged, 'utf8')
  console.log(`[chipdb] wrote ${path.resolve(output)}`)
}
