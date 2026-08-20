/* global process, console */
import fs from 'node:fs'
import path from 'node:path'
import { parse as parseToml } from 'smol-toml'

// Keep the repository-wide version in sync with `version.toml`.
//
//   node tools/set-version.mjs           apply the central version everywhere
//   node tools/set-version.mjs --check   fail when any file is out of sync

const root = path.resolve(import.meta.dirname, '..')
const checkOnly = process.argv.includes('--check')

const targets = {
  'package.json': () => replaceJsonField('package.json', 1),
  'package-lock.json': () => replaceJsonField('package-lock.json', 2),
  'modules/upt-bootstrap/root/tauri.conf.json': () =>
    replaceJsonField('modules/upt-bootstrap/root/tauri.conf.json', 1),
  'modules/upt-bootstrap/root/Cargo.toml': () => {
    let text = readText('modules/upt-bootstrap/root/Cargo.toml')
    const marker = '\n[workspace]'
    const cut = text.indexOf(marker)
    const head = text.slice(0, cut === -1 ? text.length : cut)
    const tail = cut === -1 ? '' : text.slice(cut)
    if (!/version = "[^"]+"/.test(head)) throw new Error('package version not found in Cargo.toml')
    return `${head.replace(/version = "[^"]+"/, `version = "${version()}"`)}${tail}`
  },
  'modules/upt-bootstrap/root/Cargo.lock': () => {
    const text = readText('modules/upt-bootstrap/root/Cargo.lock')
    const pattern = /(name = "uniprog"\r?\nversion = ")[^"]+(")/
    if (!pattern.test(text)) throw new Error('uniprog package not found in Cargo.lock')
    return text.replace(pattern, `$1${version()}$2`)
  },
}

function version() {
  const document = parseToml(fs.readFileSync(path.join(root, 'version.toml'), 'utf8'))
  if (!document?.version || typeof document.version !== 'string') {
    throw new Error('version.toml must contain version = "..."')
  }
  return document.version
}

function replaceJsonField(relative, occurrences) {
  const text = readText(relative)
  const pattern = /("version"\s*:\s*)"[^"]*"/g
  let seen = 0
  const updated = text.replace(pattern, (match, prefix) => {
    seen += 1
    if (seen > occurrences) return match
    return `${prefix}"${version()}"`
  })
  if (seen < occurrences) throw new Error(`version field not found in ${relative}`)
  return updated
}

function readText(relative) {
  return fs.readFileSync(path.join(root, relative), 'utf8')
}

let dirty = 0
for (const [relative, render] of Object.entries(targets)) {
  const expected = render()
  const actual = readText(relative)
  if (actual === expected) continue
  dirty += 1
  console.log(`[set-version] ${checkOnly ? 'OUT OF SYNC' : 'update'} ${relative} -> ${version()}`)
  if (!checkOnly) fs.writeFileSync(path.join(root, relative), expected, 'utf8')
}

if (checkOnly && dirty > 0) {
  console.error(`[set-version] ${dirty} file(s) do not match version.toml`)
  process.exit(1)
}
console.log(`[set-version] ${checkOnly ? 'all files match' : 'version synchronized'}: ${version()}`)
