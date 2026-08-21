# 构建 UniProgrammer

> English version: [BUILDING.md](BUILDING.md)

本文说明在一台全新的 Windows 机器上从零构建 UniProgrammer 的每一个步骤。
假设你会打开终端、复制文件、编辑文本，但不假设你有其他任何背景知识。

主要支持平台是 **Windows 10/11 x64**。Linux 和 macOS 可以编译部分代码，
但本文描述的发布包是 Windows 构建。

---

## 1. 构建会产出什么

一次完整发布构建会产出以下目录和文件：

```text
build/<profile>/src-tauri/   生成的 Rust/Tauri 工程（不要手动编辑）
dist/<profile>/
  installer/                 NSIS 安装包（.exe）
  portable/                  便携版 zip
  packages/                  可分发的插件包（.unipkg）
  manifest.json              构建元数据与 SHA-256 校验值
```

有两个 profile：

| Profile                | 后端                  | 使用对象        |
| ---------------------- | --------------------- | --------------- |
| `desktop-tauri-libusb` | libusb（WinUSB 驱动） | GitHub 公开发布 |
| `desktop-tauri-dll`    | 厂商 CH34X.DLL        | 本地/私密构建   |

DLL profile 需要闭源文件 `CH34X.DLL`。该文件永远不会提交进仓库，也永远
不会上传到 GitHub Releases。

---

## 2. 准备工作

### 2.1 安装 Git

1. 从 <https://git-scm.com/download/win> 下载 Git for Windows。
2. 运行安装程序，全部使用默认选项。
3. 打开 PowerShell，执行：

   ```powershell
   git --version
   ```

   输出应以 `git version` 开头。

### 2.2 安装 Node.js 24

1. 从 <https://nodejs.org/> 下载 LTS 安装包。本项目按 Node.js 24 测试。
2. 运行安装程序，全部使用默认选项。
3. 关闭并重新打开 PowerShell，执行：

   ```powershell
   node --version
   npm --version
   ```

   `node --version` 必须输出 `v24.x.x`，`npm --version` 必须输出版本号。

### 2.3 安装 Rust

1. 从 <https://rustup.rs/> 下载 `rustup-init.exe`。
2. 运行并按默认方式安装。
3. 关闭并重新打开 PowerShell，执行：

   ```powershell
   rustc --version
   cargo --version
   ```

4. 安装构建所需的组件：

   ```powershell
   rustup component add rustfmt clippy
   ```

### 2.4 安装 Microsoft C++ 构建工具

Rust 在 Windows 上需要 C/C++ 链接器和 Windows SDK。

1. 从 <https://visualstudio.microsoft.com/visual-cpp-build-tools/> 下载
   Visual Studio Build Tools 安装器。
2. 运行安装器。
3. 勾选工作负载 **“使用 C++ 的桌面开发”（Desktop development with C++）**。
4. 完成安装，如提示则重启电脑。

如果之后 `cargo build` 报错提到 `link.exe`，说明这一步没有完成好。

### 2.5 （可选）安装 NSIS

Tauri 打包器通常会自动下载它需要的 NSIS 版本。如果你的机器在构建时无法
下载，就手动从 <https://nsis.sourceforge.io/Download> 安装 NSIS，并确认
`makensis.exe` 在 `PATH` 中：

```powershell
makensis /VERSION
```

---

## 3. 克隆仓库

打开 PowerShell，执行：

```powershell
cd D:\WorkDIR
git clone https://github.com/SR-Laboratory/uniprog-tool.git
cd D:\WorkDIR\uniprog-tool
```

之后所有命令都假设当前目录是 `D:\WorkDIR\uniprog-tool`。

如果要显式使用 `main` 分支：

```powershell
git switch main
git pull
```

---

## 4. 安装 JavaScript 依赖

```powershell
npm ci
```

规则：

- 用 `npm ci`，不要用 `npm install`。`npm ci` 会严格按
  `package-lock.json` 安装。
- 这一步会创建 `node_modules/`。该目录被 git 忽略，不能提交。
- 如果失败，删除 `node_modules/` 后重新执行 `npm ci`；
  `package-lock.json` 应保持提交，不要手动还原到旧版本。

---

## 5. 先跑快速检查

以下检查不构建安装包。每次大构建前先跑它们，能尽早发现问题。

```powershell
npm run lint
npm run format:check
```

预期结果：

- `npm run lint` 打印命令后没有任何 error 行。
- `npm run format:check` 最后输出
  `All matched files use Prettier code style!`。

---

## 6. 理解构建流水线

项目源码在 `modules/`。一个 Node.js 脚本把模块组装成可编译的
Rust/Tauri 工程，输出到 `build/<profile>/src-tauri`。不要编辑 `build/`
下的文件：它们是重新生成的，并且被 git 忽略。

`tools/build.mjs` 按以下顺序执行完整流水线：

1. `npm run build` —— 用 Vite 构建两个前端包。
2. `tools/assemble.mjs` —— 生成 `build/<profile>/src-tauri`。
3. `cargo build --release` —— 构建 Rust 主程序和 sidecar。
4. Tauri CLI `build` —— 生成 NSIS 安装包并复制资源。
5. `tools/package.mjs` —— 把安装包、便携 zip、插件包和
   `manifest.json` 收集到 `dist/<profile>/`。

---

## 7. 构建公开发布版（libusb）

这是唯一允许上传到 GitHub Releases 的构建。

```powershell
npm run dist:libusb
```

等价的直接命令：

```powershell
node tools/build.mjs --profile desktop-tauri-libusb
```

成功后的结果：

```text
dist/desktop-tauri-libusb/
  installer/UniProgrammer_<version>_x64-setup.exe
  portable/uniprog-<version>-win-x64.zip
  packages/upt.tauri-1.0.0.unipkg
  packages/upt.tauri.hexview-1.0.0.unipkg
  packages/upt.hal.ch34x_libusb-1.0.0.unipkg
  manifest.json
```

便携 zip 内包含：

```text
uniprog.exe
chiplib.bin
README.txt
plugins/
```

libusb 版便携 zip 里任何位置都不得出现 `CH34X.DLL`。

默认情况下，打包阶段会启动刚构建的程序 5 秒做冒烟检查，并做一次
sidecar 握手。程序窗口可能会弹出然后自动关闭。要跳过该检查（例如在
CI 中）：

```powershell
npm run dist:libusb -- --skip-smoke
```

---

## 8. 构建本地发布版（DLL）

这个构建用于本地、私密使用，永远不能上传到 GitHub Releases。

### 8.1 获取并放置厂商 DLL

厂商 DLL 是闭源文件。构建不会自动下载它，仓库也不会提交它。请从芯片
厂商官网获取：

1. 打开 WCH 官方 CH341/CH347 串口并口驱动包下载页：
   <https://www.wch.cn/downloads/CH341PAR_EXE.html>
2. 下载 `CH341PAR.EXE`。
3. `CH341PAR.EXE` 是自解压压缩包。用 7-Zip
   （<https://www.7-zip.org/>）解压，例如解压到 `C:\temp\ch341par`。
4. 打开解压后的这个路径：

   ```text
   CH341PAR\WIN 1X\CH347DLLA64.DLL
   ```

   压缩包里还有 `CH341DLLA64.DLL`。要选择 **CH347DLLA64.DLL**：我们的
   DLL 后端调用 CH347 API 导出函数（CH347OpenDevice、CH347SPI_Init 及
   相关函数）。

5. 把 `CH347DLLA64.DLL` 复制进仓库并改名：

   ```text
   <仓库根目录>\vendor\CH34X.DLL
   ```

   PowerShell 示例（把 `<仓库根目录>` 和 `<解压目录>` 换成实际路径）：

   ```powershell
   New-Item -ItemType Directory -Force -Path <仓库根目录>\vendor
   Copy-Item <解压目录>\CH341PAR\WIN 1X\CH347DLLA64.DLL `
             <仓库根目录>\vendor\CH34X.DLL
   ```

6. 可选校验：当前代码测试过的厂商 DLL 版本 SHA-256 为：

   ```text
   0A0B757F774A2C456D33985957C26BE41229DA84FA7882CF661073E96E215A54
   ```

   ```powershell
   Get-FileHash <仓库根目录>\vendor\CH34X.DLL -Algorithm SHA256
   ```

   更新版本的厂商 DLL 哈希会不同；构建不要求特定哈希，但 CH347 API
   导出函数必须存在。

如果 `vendor/CH34X.DLL` 缺失，组装器会停止并提示：

```text
vendor DLL not found: ...vendor\CH34X.DLL
```

### 8.2 执行构建

```powershell
npm run dist:dll
```

成功后的结果：

```text
dist/desktop-tauri-dll/
  installer/UniProgrammer_<version>_x64-setup.exe
  portable/uniprog-<version>-win-x64.zip
  packages/upt.tauri-1.0.0.unipkg
  packages/upt.tauri.hexview-1.0.0.unipkg
  packages/upt.hal.ch34x_dll-1.0.0.unipkg
  manifest.json
```

DLL 版便携 zip 的根目录和
`plugins/builtin/upt.hal.ch34x_dll/` 内都有 `CH34X.DLL`。这是预期的，
也是这个构建必须留在本地的原因。

---

## 9. 运行 Rust 验证套件

这一步在生成的工程里运行格式化检查、类型检查、lint 和测试：

```powershell
npm run verify:libusb
```

它依次执行：

```text
cargo fmt --all -- --check
cargo check --all-targets
cargo clippy --all-targets -- -D warnings
cargo test --workspace --all-targets
```

DLL profile 可用 `npm run verify:dll` 验证。和完整 DLL 构建一样，这个命令
也需要 `vendor/CH34X.DLL`，因为组装器会把该文件复制进生成的工程。

---

## 10. 开发模式

```powershell
npm run tauri -- dev --profile desktop-tauri-libusb
```

会发生什么：

1. 组装器生成 `build/desktop-tauri-libusb/src-tauri`。
2. `scripts/dev-all.cjs` 构建并复制 CH34X sidecar 二进制。
3. Vite 启动两个开发服务器：
   - 主界面：<http://localhost:1420>
   - HexViewer 界面：<http://localhost:1421>
4. Tauri 编译并启动 `uniprog.exe`。

按 `Ctrl+C` 停止。如果 1420 或 1421 端口被占用，先关闭占用端口的程序。

---

## 11. 只组装不编译

只生成 Rust/Tauri 工程：

```powershell
npm run assemble:libusb
npm run assemble:dll
```

生成位置：

```text
build/desktop-tauri-libusb/src-tauri/
build/desktop-tauri-dll/src-tauri/
```

这些目录每次组装都会被删除并重建。

---

## 12. 发布流程

### 12.1 提升版本号

所有版本号都从一个集中文件同步：`version.toml`。

1. 只编辑 `version.toml`：

   ```toml
   version = "<新版本号>"
   ```

2. 运行同步（或者直接开始任何构建；`build`/`verify` 会自动同步）：

   ```powershell
   npm run version:sync
   ```

   该命令会更新必须保持同步的五个位置（`package.json`、
   `package-lock.json`、`tauri.conf.json`、`Cargo.toml`、
   `Cargo.lock`）。

3. 检查结果：

   ```powershell
   npm run version:check
   ```

4. 提交：

   ```powershell
   git add version.toml package.json package-lock.json `
           modules/upt-bootstrap/root/tauri.conf.json `
           modules/upt-bootstrap/root/Cargo.toml `
           modules/upt-bootstrap/root/Cargo.lock
   git commit -m "chore(release): bump version to <version>"
   git push
   ```

`tools/build.mjs` 和 `tools/verify.mjs` 在开始构建前也会执行同样的同步，
所以只编辑 `version.toml` 就足够进行下一次构建。CI 使用
`npm run version:check`，文件不同步时会拒绝发布。

### 12.2 打 tag

```powershell
git tag -a v<version> -m "UniProgrammer <version>"
git push origin v<version>
```

推送 `v*` tag 会自动触发 GitHub Actions 发布流程。

Workflow 按顺序执行：

1. `npm ci`
2. 三个版本文件一致性检查
3. `npm run lint`
4. `npm run format:check`
5. `npm run verify:libusb`
6. `npm run dist:libusb -- --skip-smoke`
7. 上传恰好两个资产到 GitHub Release：
   - `UniProgrammer_<version>_x64-setup.exe`
   - `uniprog-<version>-win-x64.zip`

它永远不会构建或上传 DLL profile。

---

## 13. 常见问题和解决办法

### 找不到 `link.exe`

按第 2.4 节安装 Visual Studio C++ 构建工具。

### `npm ci` 失败

- 检查 `package-lock.json` 没有未提交的修改。
- 删除 `node_modules/` 后重新执行 `npm ci`。
- 检查 Node.js 是 24 版本。

### `vendor DLL not found`

选择了 DLL profile，但 `vendor/CH34X.DLL` 不存在。按第 8.1 节处理。

### 1420 或 1421 端口被占用

另一个 Vite 或 UniProgrammer 实例正在运行。关闭后重试。

### `cargo run could not determine which binary to run`

当前仓库已通过根 `Cargo.toml` 的 `default-run = "uniprog"` 修复。
如果再次出现，检查 `modules/upt-bootstrap/root/Cargo.toml` 是否仍有该行。

### 删除 `build/<profile>/src-tauri` 时出现 `EPERM`

有进程的工作目录或打开的文件在该文件夹内。关闭正在运行的
UniProgrammer/Vite/cargo 进程后重试。

### 找不到 NSIS 安装包

`tools/package.mjs` 要求在
`build/<profile>/src-tauri/target/release/bundle/nsis/` 中恰好有一个
`*-setup.exe`。请运行完整 `npm run dist:*` 命令，而不是单独运行
`tools/package.mjs`。

### Git 提示 `.gitignore` 将使用 CRLF

这只是换行符提示，不是失败，不用处理。

---

## 14. 目录速查

```text
version.toml                 版本号唯一事实来源
modules/                     手工维护的源码，每个模块一个目录
profiles/                    TOML 格式的构建 profile
flashdb/                     按协议拆分的明文芯片 TOML + manifest
tools/                       assemble / build / package / verify / tauri 包装脚本
scripts/                     构建和开发服务器使用的辅助脚本
build/                       生成的工程（gitignore）
dist/                        最终产物（gitignore）
vendor/                      用户自行放入的 CH34X.DLL（DLL profile 使用，gitignore）
docs/                        本文档
.github/workflows/           GitHub Actions
```
