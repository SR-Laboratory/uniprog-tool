# UniProgrammer 架构

> English version: [ARCHITECTURE.md](ARCHITECTURE.md)

本文说明当前源码树的组织方式，以及为什么这样组织。

---

## 1. 核心思想

一切皆插件。

- 可执行文件里只保留尽可能小的核心（L0）。
- 必需的应用部分（UI 壳、HAL、协议、芯片库、HexViewer）是可替换的
  包（L1）。
- 硬件适配器是冷启动插件（L2）。
- 运行时热加载插件计划在后续版本实现（L3）。

同一套插件规则也适用于官方代码：内置包就是带 `unipkg.toml` 清单的
普通文件夹。

---

## 2. 分层

| 层  | 名称   | 含义                                                 | 当前状态               |
| --- | ------ | ---------------------------------------------------- | ---------------------- |
| L0  | 核心   | 插件管理器、依赖解析、启动检查、设置、日志、协议解析 | 已实现；永远编译进 exe |
| L1  | 必需   | UI 壳、HAL、协议、芯片库、HexViewer                  | 已实现为内置包         |
| L2  | 冷启动 | CH34X 编程器适配器 sidecar（DLL 与 libusb 后端）     | 已实现                 |
| L3  | 热加载 | 运行中可加载/卸载的插件                              | 规划中，未实现         |

### L0 不变量

L0 永远不可替换，并且不依赖 Tauri。它提供：

- `upt-plugin` —— manifest 解析、依赖解析、能力白名单、插件状态。
- `upt-core` —— 插件安装、unipkg 协议、设置、日志、脚本插件运行时、
  运行时辅助函数。
- `HostApi` —— 由 UI 层实现的抽象，让核心不依赖具体 UI 框架。

### L1 必需集合

启动检查要求以下名称存在：

```text
upt.tauri
upt.tauri.hexview
upt.hal
upt.proto
upt.chipdb
```

如果任一必需包缺失或无效，程序会在 exe 旁边写入
`uniprog-boot-error.txt` 并退出。

### L2 适配器

存在两个 CH34X 适配器包：

```text
upt.hal.ch34x_libusb   基于 rusb/libusb 的 sidecar
upt.hal.ch34x_dll      加载厂商 CH34X.DLL 的 sidecar
```

它们是独立进程，使用 sidecar 协议通信；sidecar 崩溃不会让主程序崩溃。

---

## 3. 仓库布局

```text
version.toml                   版本号唯一事实来源
modules/                       手工维护的源码
  upt-bootstrap/               根 Cargo.toml、Tauri 配置、图标、测试
  upt-core/                    L0 核心模块
  upt-ui-tauri/                Tauri UI 层（HostApi 实现、命令）
  upt-app-ops/                 芯片状态与操作，供不同前端共用
  upt-plugin-runtime/          upt-plugin crate
  upt-hal-runtime/             upt-hal crate（HAL 路由、sidecar 客户端）
  upt-chipdb-runtime/          upt-chipdb crate 与 sidecar 二进制
  upt-proto-runtime/           upt-proto crate（NOR/NAND/EEPROM/45/SFDP/固件）
  upt-devices-runtime/         upt-devices crate 与 CH34X sidecar 二进制
  upt-shell-tauri/             upt.tauri UI 包
  upt-shell-tauri-hexview/     upt.tauri.hexview UI 包
  upt-hal-package/             upt.hal 内置包 manifest
  upt-proto-package/           upt.proto 内置包 manifest
  upt-chipdb-package/          upt.chipdb 内置包 manifest
  upt-adapter-ch34x-dll/       CH34X DLL 适配器包
  upt-adapter-ch34x-libusb/    CH34X libusb 适配器包

profiles/                      构建 profile
flashdb/                       按协议拆分的芯片 TOML 文件与 manifest
tools/                         Node.js 组装/构建/打包/验证脚本
scripts/                       开发与打包辅助脚本
build/                         生成的工程（gitignore）
dist/                          最终产物（gitignore）
docs/                          文档
```

每个模块都有 `module.toml`：

```toml
[module]
name = "upt-core"
source = "modules/upt-core/src"
target = "src/l0_core"
```

Profile 选择模块并声明哪些生成目标必须存在：

```toml
[build]
name = "desktop-tauri-libusb"
backend = "libusb"
modules = [ ... ]
required = [ "src/l0_core", "plugins/builtin/upt.tauri", ... ]
```

---

## 4. 源码组装

仓库本身不是所有路径都直接可解析的 Cargo workspace。
`tools/assemble.mjs` 把模块源码复制到一个干净的生成工程：

```text
build/<profile>/src-tauri/
```

组装器做的事：

1. 删除上一次生成的工程。
2. 把 profile 选择的每个模块复制到它声明的目标路径。
3. 对插件包只复制安装载荷：`unipkg.toml`、UI 包的 `dist/`、适配器包的
   sidecar/DLL 文件。
4. 把 `flashdb/protocols/` 下按协议拆分的 TOML 文件直接编译成生成工程里的
   运行时 `chiplib.bin`。
5. DLL profile 复制厂商 DLL。
6. 写入 `build-manifest.json`。
7. 在 profile 根目录生成 `package.json`，并改写生成的
   `tauri.conf.json` 钩子。

`build/` 和 `dist/` 被 gitignore，禁止手动编辑。

---

## 5. 工具链

| 工具                 | 用途                                                    |
| -------------------- | ------------------------------------------------------- |
| `tools/assemble.mjs` | 生成 `build/<profile>/src-tauri`                        |
| `tools/verify.mjs`   | 在生成工程里跑 fmt + check + clippy + test              |
| `tools/build.mjs`    | 前端构建 + 组装 + cargo release + Tauri 打包 + 产物收集 |
| `tools/tauri.mjs`    | `npm run tauri -- dev                                   | build` 包装，基于 profile |
| `tools/package.mjs`  | 收集 `dist/<profile>/` 产物、冒烟检查、失败隔离         |

发布流水线顺序：

```text
前端构建
  -> 组装
  -> cargo build --release
  -> Tauri 打包（NSIS + 资源）
  -> 打包进 dist/<profile>/
```

---

## 6. 插件包

### Manifest

插件包是一个根目录含 `unipkg.toml` 的文件夹：

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

可选 `[packaging]` 段：

```toml
[packaging]
include = ["dist"]            # 发布包里要包含的文件
distributable = false         # 生成 .unipkg 时跳过
```

### 解析顺序

启动时插件管理器扫描：

```text
<app root>/plugins/           第三方插件
<app root>/plugins/builtin/   随程序发布的内置插件
```

优先使用 `unipkg.toml`，其次才是旧格式 `manifest.toml`。第三方插件在
用户启用前保持禁用。

### 能力模型

默认拒绝。适配器只能使用同时满足以下条件的能力：

1. manifest 中显式声明；
2. 适配器运行时上报。

两者交集是有效能力集合。用户可以进一步禁用能力，但不能添加。主程序
绝不根据 VID/PID 或芯片型号推断能力。

### 安装格式

`.unipkg` 是普通 ZIP 归档，归档根目录必须有 manifest。安装时先解压到
staging 目录，再原子重命名到 `plugins/<name>/`。同一套代码也接受本地
文件夹和 Git 仓库。

---

## 7. UI 服务

UI 包是静态 Web 包。`unipkg://` 协议从包目录提供页面：

```text
unipkg://localhost/upt.tauri/          -> plugins/builtin/upt.tauri/dist/
unipkg://localhost/upt.tauri.hexview/  -> plugins/builtin/upt.tauri.hexview/dist/
```

已安装的第三方插件先于内置包解析，因此可以替换 UI 包。调试构建中，
两个内置 UI 包会重定向到 Vite 开发服务器。

---

## 8. Sidecar 适配器

见 [SIDECAR_PROTOCOL_V1_CN.md](SIDECAR_PROTOCOL_V1_CN.md)。HAL 路由器的
工作流程：

1. 找到已启用的适配器插件。
2. 启动每个 sidecar entry。
3. 执行握手。
4. 调用 `probe`。
5. 把 `open`/`execute`/`close` 请求路由到选中的会话。

Sidecar 进程独立于主程序，程序退出时会被关闭。

---

## 9. 脚本插件

脚本插件是在 QuickJS 沙箱里执行的 JavaScript 文件。默认禁止文件、
网络和进程权限，权限在 manifest 中声明。脚本获得受限的 `uni.*` host
API，主要用于协议规则和小工具。

---

## 10. 安全规则

- 第三方插件默认不自动加载。
- 首次启用必须用户确认，并展示声明的权限。
- 原生动态库视为完全可信代码；v1 不提供原生 SDK。
- 安全门禁（写保护、电压限制、危险操作确认）不能被插件或配置关闭。
- 无遥测。日志只在本机。诊断包只在用户主动导出时生成。

---

## 11. 当前实现状态

已实现：

- L0 插件管理器、manifest 解析、依赖解析、启动检查。
- `upt.log` 文本日志与轮转。
- L1 内置包与 `unipkg://` 服务。
- L2 CH34X sidecar（DLL 与 libusb）、probe/open/execute/close 会话。
- 基于 sidecar 的 NOR 操作与 SPI 总线抽象。
- JavaScript 插件运行时与示例脚本插件。
- 源码组装、生成式构建、CI 发布流程、`dist/` 流水线。
- 从文件夹、`.unipkg`、Git 仓库安装插件包。

v1 不实现：

- L3 热加载。
- 原生动态库插件 SDK。
- 任意第三方 UI 插件。
- 插件源/市场。
- 签名校验。
- Lua 运行时。

---

## 12. 版本兼容

| 标识                   | 含义                                 |
| ---------------------- | ------------------------------------ |
| `plugin_api`           | 插件与主程序之间的总接口版本         |
| `upt-base`             | 代表全部内置模块的虚拟依赖           |
| `UPT_BASE_API_VERSION` | 当前 `upt-base` 版本，目前是 `1.0.0` |

内置模块在 `upt_plugin::builtin_modules()` 中注册，版本固定为
`1.0.0`。
