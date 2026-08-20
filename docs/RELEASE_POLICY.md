# Release Policy

> 中文版：[RELEASE_POLICY_CN.md](RELEASE_POLICY_CN.md)

This document defines what may be published to GitHub Releases and how the
project handles the proprietary vendor DLL.

---

## 1. Public GitHub Release contents

Every public GitHub Release contains exactly two assets:

```text
UniProgrammer_<version>_x64-setup.exe
uniprog-<version>-win-x64.zip
```

Rules:

- The installer and the portable zip are built from the **libusb profile**
  (`desktop-tauri-libusb`).
- Neither asset may contain `CH34X.DLL` anywhere, including inside the
  `plugins/` tree.
- No other assets are uploaded: no raw exe, no DLL package, no `.unipkg`
  packages, no Linux bundle.
- Alpha/beta/rc releases are marked as pre-release.

## 2. The proprietary CH34X.DLL is never committed

The project itself is licensed under GPL-3.0-or-later. The CH34X DLL is a
proprietary binary supplied by Nanjing Qinheng Microelectronics (WCH) inside
the official `CH341PAR.EXE` driver package and is distributed under WCH's
own terms.

Because of that license situation, this project has a permanent policy:

- `CH34X.DLL` is **never committed** to the repository.
- The project will **not consider** committing the DLL or any build that
  embeds it, now or in the future.
- Public source snapshots and public release assets stay free of that
  proprietary file.
- The public release workflow only builds and uploads the libusb profile;
  it has no step that builds the DLL profile.

Users who want the DLL backend obtain the official DLL themselves and build
locally. See [BUILDING.md](BUILDING.md) for the exact download and placement
steps. The resulting DLL build is for private, local use only; it must not be
uploaded to a public download location.

## 3. CI enforcement

The workflow `.github/workflows/release.yml` runs only on `v*` tags and:

1. validates that `package.json`, `tauri.conf.json`, and `Cargo.toml` agree
   on the version;
2. runs lint and format checks;
3. runs the full Rust verification suite;
4. builds `npm run dist:libusb -- --skip-smoke`;
5. uploads exactly the two libusb assets.

## 4. Version scheme

- Current scheme: `0.<minor>.0-alpha.<n>`.
- Patch-level alpha bumps: `0.4.0-alpha.12` -> `0.4.0-alpha.13`.
- Structural changes in the pre-1.0 phase: minor bump, for example
  `0.4.0-alpha.12` -> `0.5.0-alpha.1`.
- `1.0.0` is reserved for a stable release that has passed the hardware
  validation checklist.

Before tagging, update all five version locations listed in
[BUILDING.md](BUILDING.md).
