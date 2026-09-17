/* global process, console */
import fs from 'node:fs'
import path from 'node:path'
import { spawnSync } from 'node:child_process'

// Assembles a profile and runs the complete Rust verification suite inside
// the generated workspace:
//   fmt --check, check --all-targets, clippy -D warnings, test --all-targets.
//
// Usage:
//   node tools/verify.mjs --profile desktop-tauri-libusb

const root = path.resolve(import.meta.dirname, '..')

function run(command, args, cwd) {
  console.log(`[verify] $ ${command} ${args.join(' ')}`)
  const result = spawnSync(command, args, { cwd, stdio: 'inherit' })
  return result.status ?? 1
}

function fail(message) {
  console.error(`[verify] ${message}`)
  process.exit(1)
}

const args = process.argv.slice(2)
const profileIndex = args.indexOf('--profile')
const profile = profileIndex >= 0 ? args[profileIndex + 1] : null
if (!profile) fail('missing --profile <name>')

const profileFile = path.join(root, 'profiles', `${profile}.toml`)
if (!fs.existsSync(profileFile)) fail(`profile not found: ${profileFile}`)
const profileText = fs.readFileSync(profileFile, 'utf8')
const ui = /^ui = "([^"]+)"/m.exec(profileText)?.[1] ?? 'tauri'
if (!['tauri', 'slint'].includes(ui)) fail(`profile ui must be "tauri" or "slint": ${profileFile}`)
const uiFeatureArgs = ui === 'slint' ? ['--features', 'ui-slint'] : []

const versionSync = run(process.execPath, [path.join(root, 'tools', 'set-version.mjs')], root)
if (versionSync !== 0) fail('version sync failed')

const assemble = run(
  process.execPath,
  [path.join(root, 'tools', 'assemble.mjs'), '--profile', profile],
  root,
)
if (assemble !== 0) fail('assembler failed')

const srcTauri = path.join(root, 'build', profile, 'src-tauri')
if (!fs.existsSync(path.join(srcTauri, 'Cargo.toml'))) fail('generated Cargo.toml missing')

const steps = [
  ['cargo', ['fmt', '--all', '--', '--check']],
  ['cargo', ['check', '--all-targets', ...uiFeatureArgs]],
  ['cargo', ['clippy', '--all-targets', ...uiFeatureArgs, '--', '-D', 'warnings']],
  ['cargo', ['test', '--workspace', '--all-targets', ...uiFeatureArgs]],
]

for (const [command, commandArgs] of steps) {
  if (run(command, commandArgs, srcTauri) !== 0) fail(`${command} ${commandArgs.join(' ')} failed`)
}

console.log(`[verify] profile ${profile} (ui=${ui}) passed all checks`)
