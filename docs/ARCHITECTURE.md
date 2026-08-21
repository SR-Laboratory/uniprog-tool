# UniProgrammer Architecture

> 中文版：[ARCHITECTURE_CN.md](ARCHITECTURE_CN.md)

This document describes how the current source tree is organized and why it
is organized that way.

---

## 1. Core idea

Everything is a plugin.

- The executable contains only the smallest possible core (L0).
- Required application parts (UI shell, HAL, protocols, chip database,
  hex viewer) are replaceable packages (L1).
- Hardware adapters are cold-start plugins (L2).
- Runtime-loadable plugins are planned for a later version (L3).

The same plugin rules apply to first-party code: the built-in packages are
plain folders with a `unipkg.toml` manifest.

---

## 2. Layers

| Layer | Name     | Meaning                                                                                 | Current state                             |
| ----- | -------- | --------------------------------------------------------------------------------------- | ----------------------------------------- |
| L0    | Core     | Plugin manager, dependency resolution, boot check, settings, logging, protocol resolver | Implemented; always compiled into the exe |
| L1    | Required | UI shell, HAL, protocols, chip database, hex viewer                                     | Implemented as built-in packages          |
| L2    | Cold     | Programmer adapter sidecars (CH34X DLL and libusb backends)                             | Implemented                               |
| L3    | Hot      | Plugins that can load/unload at runtime                                                 | Planned; not implemented                  |

### L0 invariants

L0 is never replaceable and never Tauri-dependent. It provides:

- `upt-plugin` — manifest parsing, dependency resolution, capability
  whitelist, plugin state.
- `upt-core` — plugin installation, unipkg protocol, settings, logging,
  script plugin runtime, runtime helpers.
- `HostApi` — an abstraction that the UI layer implements so the core does
  not depend on a specific UI framework.

### L1 required set

The boot check requires these names:

```text
upt.tauri
upt.tauri.hexview
upt.hal
upt.proto
upt.chipdb
```

If any required package is missing or invalid, the program writes
`uniprog-boot-error.txt` next to the executable and exits.

### L2 adapters

Two CH34X adapter packages exist:

```text
upt.hal.ch34x_libusb   sidecar built against rusb/libusb
upt.hal.ch34x_dll      sidecar loading the vendor CH34X.DLL
```

They are separate processes that speak the sidecar protocol; a crash in a
sidecar cannot crash the main program.

---

## 3. Repository layout

```text
version.toml                   Single source of truth for the version
modules/                       Hand-maintained source
  upt-bootstrap/               Root Cargo.toml, tauri configs, icons, tests
  upt-core/                    L0 core modules
  upt-ui-tauri/                Tauri UI layer (HostApi implementation, commands)
  upt-app-ops/                 Chip state and operations shared by frontends
  upt-plugin-runtime/          upt-plugin crate
  upt-hal-runtime/             upt-hal crate (HAL router, sidecar client)
  upt-chipdb-runtime/          upt-chipdb crate and sidecar binary
  upt-proto-runtime/           upt-proto crate (NOR/NAND/EEPROM/45/SFDP/firmware)
  upt-devices-runtime/         upt-devices crate and CH34X sidecar binaries
  upt-shell-tauri/             upt.tauri UI package
  upt-shell-tauri-hexview/     upt.tauri.hexview UI package
  upt-hal-package/             upt.hal built-in package manifest
  upt-proto-package/           upt.proto built-in package manifest
  upt-chipdb-package/          upt.chipdb built-in package manifest
  upt-adapter-ch34x-dll/       CH34X DLL adapter package
  upt-adapter-ch34x-libusb/    CH34X libusb adapter package

profiles/                      Build profiles
flashdb/                       Per-protocol chip TOML files and manifest
tools/                         Node.js assembly/build/package/verify scripts
scripts/                       Helper scripts for dev and bundling
build/                         Generated workspace (gitignored)
dist/                          Final artifacts (gitignored)
docs/                          Documentation
```

Every module has a `module.toml`:

```toml
[module]
name = "upt-core"
source = "modules/upt-core/src"
target = "src/l0_core"
```

A profile selects modules and declares which generated targets are required:

```toml
[build]
name = "desktop-tauri-libusb"
backend = "libusb"
modules = [ ... ]
required = [ "src/l0_core", "plugins/builtin/upt.tauri", ... ]
```

---

## 4. Source assembly

The repository itself is not directly a Cargo workspace with all paths
resolved. `tools/assemble.mjs` copies module sources into a clean generated
workspace:

```text
build/<profile>/src-tauri/
```

What the assembler does:

1. Deletes the previous generated workspace.
2. Copies every module selected by the profile to its declared target path.
3. For plugin packages, copies only the install payload: `unipkg.toml`,
   `dist/` for UI packages, and sidecar/DLL files for adapter packages.
4. Compiles the TOML files under `flashdb/protocols/` directly into the
   generated runtime `chiplib.bin`.
5. Copies the vendor DLL for the DLL profile.
6. Writes `build-manifest.json`.
7. Generates `package.json` at the profile root and rewrites the generated
   `tauri.conf.json` hooks.

`build/` and `dist/` are gitignored and must never be edited manually.

---

## 5. Toolchain

| Tool                 | Purpose                                                              |
| -------------------- | -------------------------------------------------------------------- |
| `tools/assemble.mjs` | Generate `build/<profile>/src-tauri`                                 |
| `tools/verify.mjs`   | fmt + check + clippy + test in the generated workspace               |
| `tools/build.mjs`    | Frontend build + assemble + cargo release + Tauri bundle + package   |
| `tools/tauri.mjs`    | `npm run tauri -- dev                                                | build` wrapper against a profile |
| `tools/package.mjs`  | Collect `dist/<profile>/` artifacts, smoke checks, failure isolation |

Release pipeline order:

```text
frontend build
  -> assemble
  -> cargo build --release
  -> Tauri bundle (NSIS + resources)
  -> package into dist/<profile>/
```

---

## 6. Plugin packages

### Manifest

A plugin package is a directory containing `unipkg.toml` at its root:

```toml
[package]
name = "upt.hal.ch34x_libusb"
version = "1.0.0"
plugin_api = 1
kind = "adapter"        # adapter | protocol | chipdb | ui
layer = "cold"          # required | cold | hot
entry = "upt_ch34x_sidecar_libusb"
provider = "builtin"

[dependencies]
"upt.hal" = "^1"

[permissions]
usb = true

[capabilities.spi]
enabled = true
pins = { cs = "CS0", sck = "SCK", mosi = "MOSI", miso = "MISO" }
max_frame = 4092
max_freq_khz = 60000
```

Optional `[packaging]` section:

```toml
[packaging]
include = ["dist"]            # files copied into the release package
distributable = false         # skip when generating .unipkg files
```

### Resolution order

At startup the plugin manager scans:

```text
<app root>/plugins/           third-party plugins
<app root>/plugins/builtin/   shipped built-in plugins
```

`unipkg.toml` is preferred over the legacy `manifest.toml`. Installed
third-party plugins are disabled until the user enables them.

### Capability model

Default is deny. An adapter may only use capabilities that are:

1. declared in its manifest, and
2. reported by the adapter at runtime.

The intersection is the effective capability set. Users can further disable
capabilities but can never add them. The main program never infers
capabilities from VID/PID or chip model.

### Install format

`.unipkg` is an ordinary ZIP archive containing the manifest at the archive
root. Installation extracts to a staging directory and atomically renames it
into `plugins/<name>/`. The same code accepts a local folder or a Git
repository.

---

## 7. UI serving

UI packages are static web packages. The `unipkg://` protocol serves them
from their package directory:

```text
unipkg://localhost/upt.tauri/            -> plugins/builtin/upt.tauri/dist/
unipkg://localhost/upt.tauri.hexview/    -> plugins/builtin/upt.tauri.hexview/dist/
```

Installed third-party plugins are resolved before built-in packages, which is
how a UI package can be replaced. In debug builds the two built-in UI
packages are redirected to their Vite dev servers.

---

## 8. Sidecar adapters

See [SIDECAR_PROTOCOL_V1.md](SIDECAR_PROTOCOL_V1.md). The HAL router:

1. Finds enabled adapter plugins.
2. Spawns each sidecar entry.
3. Performs a handshake.
4. Calls `probe`.
5. Routes `open`/`execute`/`close` requests to the selected session.

Sidecar processes run independently of the main program and are killed on
shutdown.

---

## 9. Script plugins

Script plugins are JavaScript files executed inside a QuickJS sandbox.
Defaults deny files, network, and processes. Permissions are declared in the
manifest. Scripts receive a restricted `uni.*` host API and are intended for
protocol rules and small tools.

---

## 10. Security rules

- Third-party plugins do not load automatically.
- First enable requires user confirmation and shows declared permissions.
- Native dynamic libraries are considered fully trusted code; v1 does not
  ship a native SDK.
- Safety gates (write protection, voltage limits, destructive operation
  confirmation) cannot be disabled by plugins or configuration.
- No telemetry. Logs are local. Diagnostic packages are created only when the
  user explicitly exports them.

---

## 11. Current implementation status

Implemented:

- L0 plugin manager, manifest parser, dependency resolution, boot check.
- `upt.log` text logging with rotation.
- L1 built-in packages and `unipkg://` serving.
- L2 CH34X sidecars (DLL and libusb), probe/open/execute/close sessions.
- Sidecar-backed NOR operations and SPI bus abstraction.
- JavaScript plugin runtime and example script plugins.
- Source assembly, generated builds, CI release workflow, `dist/` pipeline.
- Plugin package install from folder, `.unipkg`, and Git repository.

Not implemented in v1:

- L3 hot loading.
- Native dynamic library plugin SDK.
- Arbitrary third-party UI plugins.
- Plugin feed/marketplace.
- Signature verification.
- Lua runtime.

---

## 12. Version compatibility

| Identifier             | Meaning                                              |
| ---------------------- | ---------------------------------------------------- |
| `plugin_api`           | Total interface version between plugins and host     |
| `upt-base`             | Virtual dependency representing all built-in modules |
| `UPT_BASE_API_VERSION` | Current `upt-base` version, currently `1.0.0`        |

Built-in modules are registered in `upt_plugin::builtin_modules()` and have
fixed version `1.0.0`.
