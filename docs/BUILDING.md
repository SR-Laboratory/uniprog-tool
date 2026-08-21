# Building UniProgrammer

> 中文版：[BUILDING_CN.md](BUILDING_CN.md)

This document explains every step required to build UniProgrammer on a fresh
Windows machine. It assumes you know how to open a terminal, copy files, and
edit text files, but it does not assume any other background knowledge.

The supported primary build platform is **Windows 10/11 x64**. Linux and
macOS can compile parts of the project, but the release bundles described here
are Windows builds.

---

## 1. What the build produces

A full release build produces these directories and files:

```text
build/<profile>/src-tauri/   Generated Rust/Tauri workspace (do not edit)
dist/<profile>/
  installer/                 NSIS installer (.exe)
  portable/                  Portable zip
  packages/                  Distributable plugin packages (.unipkg)
  manifest.json              Build metadata and SHA-256 checksums
```

Two profiles are available:

| Profile                | Backend                | Who uses it            |
| ---------------------- | ---------------------- | ---------------------- |
| `desktop-tauri-libusb` | libusb (WinUSB driver) | Public GitHub releases |
| `desktop-tauri-dll`    | Vendor CH34X.DLL       | Local/private builds   |

The DLL profile requires a proprietary file (`CH34X.DLL`). That file is never
committed to the repository and never uploaded to GitHub Releases.

---

## 2. Prerequisites

### 2.1 Install Git

1. Download Git for Windows from <https://git-scm.com/download/win>.
2. Run the installer and accept the default options.
3. Open PowerShell and verify:

   ```powershell
   git --version
   ```

   Expected output starts with `git version`.

### 2.2 Install Node.js 24

1. Download the LTS installer from <https://nodejs.org/>.
   The project is tested with Node.js 24.
2. Run the installer and accept the default options.
3. Close and reopen PowerShell, then verify:

   ```powershell
   node --version
   npm --version
   ```

   `node --version` must print `v24.x.x`. `npm --version` must print a version
   number.

### 2.3 Install Rust

1. Download `rustup-init.exe` from <https://rustup.rs/>.
2. Run it and accept the default installation.
3. Close and reopen PowerShell, then verify:

   ```powershell
   rustc --version
   cargo --version
   ```

4. Add the components used by the build:

   ```powershell
   rustup component add rustfmt clippy
   ```

### 2.4 Install the Microsoft C++ build tools

The Rust compiler needs a C/C++ linker and the Windows SDK on Windows.

1. Download the Visual Studio Build Tools installer from
   <https://visualstudio.microsoft.com/visual-cpp-build-tools/>.
2. Run the installer.
3. In the installer, select the workload **Desktop development with C++**.
4. Complete the installation and restart the computer if prompted.

If `cargo build` later fails with a message that mentions `link.exe`, this
step was not completed correctly.

### 2.5 (Optional) Install NSIS

The Tauri bundler normally downloads the NSIS version it needs automatically.
If your machine cannot download it during the build, install NSIS manually
from <https://nsis.sourceforge.io/Download> and make sure `makensis.exe` is on
`PATH`:

```powershell
makensis /VERSION
```

---

## 3. Clone the repository

Open PowerShell and run:

```powershell
cd D:\WorkDIR
git clone https://github.com/SR-Laboratory/uniprog-tool.git
cd D:\WorkDIR\uniprog-tool
```

This creates `D:\WorkDIR\uniprog-tool`. All later commands assume this is the
current directory.

If you want to work with the `main` branch explicitly:

```powershell
git switch main
git pull
```

---

## 4. Install the JavaScript dependencies

```powershell
npm ci
```

Rules:

- Use `npm ci`, not `npm install`. `npm ci` installs exactly the versions in
  `package-lock.json`.
- This step creates `node_modules/`. That directory is ignored by git and must
  not be committed.
- If this step fails, delete `node_modules/` and `package-lock.json` changes
  are not needed; the lock file itself should stay committed. Re-run `npm ci`.

---

## 5. Run the fast checks first

These checks do not build the installer. Run them before every larger build so
errors are found early.

```powershell
npm run lint
npm run format:check
```

Expected output:

- `npm run lint` prints the command and finishes without any error lines.
- `npm run format:check` ends with `All matched files use Prettier code style!`.

---

## 6. Understand the build pipeline

The project source lives in `modules/`. A Node.js script assembles a
buildable Rust/Tauri workspace from those modules into
`build/<profile>/src-tauri`. Do not edit files under `build/`; they are
regenerated and ignored by git.

`tools/build.mjs` runs the full pipeline in this order:

1. `npm run build` — builds the two frontend packages with Vite.
2. `tools/assemble.mjs` — generates `build/<profile>/src-tauri`.
3. `cargo build --release` — builds the Rust application and sidecars.
4. Tauri CLI `build` — creates the NSIS installer and copies resources.
5. `tools/package.mjs` — collects installer, portable zip, plugin packages,
   and `manifest.json` into `dist/<profile>/`.

---

## 7. Build the public (libusb) release

This is the only build that may be uploaded to GitHub Releases.

```powershell
npm run dist:libusb
```

Equivalent direct command:

```powershell
node tools/build.mjs --profile desktop-tauri-libusb
```

Expected results after success:

```text
dist/desktop-tauri-libusb/
  installer/UniProgrammer_<version>_x64-setup.exe
  portable/uniprog-<version>-win-x64.zip
  packages/upt.tauri-1.0.0.unipkg
  packages/upt.tauri.hexview-1.0.0.unipkg
  packages/upt.hal.ch34x_libusb-1.0.0.unipkg
  manifest.json
```

The portable zip contains:

```text
uniprog.exe
chiplib.bin
README.txt
plugins/
```

The libusb portable zip must NOT contain `CH34X.DLL` anywhere.

By default the packaging stage starts the freshly built program for 5 seconds
as a smoke check and performs a sidecar handshake. The program window may
appear and close automatically. To skip that check (for example in CI):

```powershell
npm run dist:libusb -- --skip-smoke
```

---

## 8. Build the local (DLL) release

This build is for private, local use. It must never be uploaded to GitHub
Releases.

### 8.1 Place the vendor DLL

The DLL is a proprietary vendor file. It is not downloaded by the build and
is never committed to the repository. Get it directly from the chip vendor:

1. Open the official WCH download page for the CH341/CH347 parallel and
   serial driver package:
   <https://www.wch.cn/downloads/CH341PAR_EXE.html>
2. Download `CH341PAR.EXE`.
3. `CH341PAR.EXE` is a self-extracting archive. Extract it with 7-Zip
   (<https://www.7-zip.org/>), for example to `C:\temp\ch341par`.
4. Open the extracted folder and then this path:

   ```text
   CH341PAR\WIN 1X\CH347DLLA64.DLL
   ```

   The archive also contains `CH341DLLA64.DLL`. Use **CH347DLLA64.DLL**:
   the DLL backend calls the CH347 API exports (CH347OpenDevice,
   CH347SPI_Init, and related functions).

5. Copy `CH347DLLA64.DLL` into the repository and rename it:

   ```text
   <repository-root>\vendor\CH34X.DLL
   ```

   PowerShell example (replace `<repository-root>` and `<extract-root>`):

   ```powershell
   New-Item -ItemType Directory -Force -Path <repository-root>\vendor
   Copy-Item <extract-root>\CH341PAR\WIN 1X\CH347DLLA64.DLL `
             <repository-root>\vendor\CH34X.DLL
   ```

6. Optional: verify the file. The vendor DLL version the current code was
   tested with has this SHA-256:

   ```text
   0A0B757F774A2C456D33985957C26BE41229DA84FA7882CF661073E96E215A54
   ```

   ```powershell
   Get-FileHash <repository-root>\vendor\CH34X.DLL -Algorithm SHA256
   ```

   A newer vendor version may have a different hash; the build does not
   require a specific hash, but the CH347 API exports must be present.

If `vendor/CH34X.DLL` is missing, the assembler stops with:

```text
vendor DLL not found: ...vendor\CH34X.DLL
```

### 8.2 Run the build

```powershell
npm run dist:dll
```

Expected results:

```text
dist/desktop-tauri-dll/
  installer/UniProgrammer_<version>_x64-setup.exe
  portable/uniprog-<version>-win-x64.zip
  packages/upt.tauri-1.0.0.unipkg
  packages/upt.tauri.hexview-1.0.0.unipkg
  packages/upt.hal.ch34x_dll-1.0.0.unipkg
  manifest.json
```

The DLL portable zip contains `CH34X.DLL` at its root and inside
`plugins/builtin/upt.hal.ch34x_dll/`. This is expected and is the reason this
build stays local.

---

## 9. Run the Rust verification suite

This step runs formatting, type checking, linting, and tests in a generated
workspace:

```powershell
npm run verify:libusb
```

It runs:

```text
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --workspace --all-targets
```

The DLL profile can be verified with `npm run verify:dll`. Like the full DLL
build, this command requires `vendor/CH34X.DLL` because the assembler copies
the file into the generated workspace.

---

## 10. Development mode

```powershell
npm run tauri -- dev --profile desktop-tauri-libusb
```

What happens:

1. The assembler generates `build/desktop-tauri-libusb/src-tauri`.
2. `scripts/dev-all.cjs` builds and copies the CH34X sidecar binaries.
3. Vite starts two dev servers:
   - main UI: <http://localhost:1420>
   - hex viewer UI: <http://localhost:1421>
4. Tauri compiles and launches `uniprog.exe`.

Stop it with `Ctrl+C`. If ports 1420 or 1421 are already in use, close the
program using them before starting again.

---

## 11. Assembly only

To generate the Rust/Tauri workspace without compiling:

```powershell
npm run assemble:libusb
npm run assemble:dll
```

The generated workspace is:

```text
build/desktop-tauri-libusb/src-tauri/
build/desktop-tauri-dll/src-tauri/
```

These directories are deleted and recreated on every assembly.

---

## 12. Creating a release

### 12.1 Bump the version

All version numbers are synchronized from one central file:
`version.toml`.

1. Edit only `version.toml`:

   ```toml
   version = "<new-version>"
   ```

2. Run the sync (or simply start any build; `build`/`verify` sync
   automatically):

   ```powershell
   npm run version:sync
   ```

   This updates the five generated locations that must stay in sync
   (`package.json`, `package-lock.json`, `tauri.conf.json`, `Cargo.toml`,
   `Cargo.lock`).

3. Check the result:

   ```powershell
   npm run version:check
   ```

4. Commit:

   ```powershell
   git add version.toml package.json package-lock.json `
           modules/upt-bootstrap/root/tauri.conf.json `
           modules/upt-bootstrap/root/Cargo.toml `
           modules/upt-bootstrap/root/Cargo.lock
   git commit -m "chore(release): bump version to <version>"
   git push
   ```

`tools/build.mjs` and `tools/verify.mjs` run the same sync before doing
anything, so editing `version.toml` is enough for the next build. CI uses
`npm run version:check` to refuse a release when files are out of sync.

### 12.2 Tag the release

```powershell
git tag -a v<version> -m "UniProgrammer <version>"
git push origin v<version>
```

Pushing a `v*` tag starts the GitHub Actions release workflow automatically.

The workflow runs, in order:

1. `npm ci`
2. version consistency check
3. `npm run lint`
4. `npm run format:check`
5. `npm run verify:libusb`
6. `npm run dist:libusb -- --skip-smoke`
7. uploads exactly two assets to the GitHub Release:
   - `UniProgrammer_<version>_x64-setup.exe`
   - `uniprog-<version>-win-x64.zip`

It never builds or uploads the DLL profile.

---

## 13. Common problems and fixes

### `link.exe` not found

Install the Visual Studio C++ build tools as described in section 2.4.

### `npm ci` fails

- Check that `package-lock.json` has no uncommitted edits.
- Delete `node_modules/` and run `npm ci` again.
- Check that Node.js is version 24.

### `vendor DLL not found`

The DLL profile was selected but `vendor/CH34X.DLL` is missing. Follow
section 8.1.

### Port 1420 or 1421 is already in use

Another instance of Vite or UniProgrammer is running. Close it and retry.

### `cargo run could not determine which binary to run`

This is fixed in the current repository by `default-run = "uniprog"` in the
root `Cargo.toml`. If it appears again, check that the file
`modules/upt-bootstrap/root/Cargo.toml` still contains that line.

### `EPERM` while deleting `build/<profile>/src-tauri`

A process has its working directory or an open file inside that folder. Close
the running UniProgrammer/Vite/cargo process and retry.

### No NSIS installer found

`tools/package.mjs` looks for exactly one `*-setup.exe` in
`build/<profile>/src-tauri/target/release/bundle/nsis/`. Run the full
`npm run dist:*` command rather than `tools/package.mjs` alone.

### Git warns that `.gitignore` will use CRLF

This is a line-ending notice, not a failure. Leave it as is.

---

## 14. Directory reference

```text
version.toml                 Single source of truth for the version
modules/                     Hand-maintained source, one directory per module
profiles/                    Build profiles in TOML
flashdb/                     Per-protocol plaintext chip XML + manifest
tools/                       assemble / build / package / verify / tauri wrappers
scripts/                     Helper scripts used by the build and dev servers
build/                       Generated workspace (gitignored)
dist/                        Final artifacts (gitignored)
vendor/                      User-supplied CH34X.DLL for the DLL profile (gitignored)
docs/                        This documentation
.github/workflows/           GitHub Actions
```
