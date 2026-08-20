# UniProgrammer

> 一款跨平台 NAND/NOR SPI Flash 编程器，具备可插拔的硬件抽象层（HAL）。

[![License: GPL v3+](https://img.shields.io/badge/License-GPLv3+-blue.svg)](LICENSE)
[![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)

[English](README.md)

> **⚠️ 项目尚未完成实机验证，请谨慎使用 / Most features have NOT been
> validated on real hardware; use with caution.**
>
> 2026-08-16：CH341A + SPI NOR 基础操作已在单台测试环境通过；
> 其他编程器/芯片组合仍需按验证清单逐项测试。

> **Alpha 版本警告** — 不要用于内容无法承受丢失的芯片。

## 目录

- [背景](#背景)
- [功能特性](#功能特性)
- [安装](#安装)
- [使用方法](#使用方法)
- [文档](#文档)
- [硬件后端](#硬件后端)
- [芯片数据库](#芯片数据库)
- [开发](#开发)
- [维护者](#维护者)
- [参与贡献](#参与贡献)
- [许可证](#许可证)

## 背景

UniProgrammer 是经典 CH341/CH347 编程工具的现代重写。协议层参考
[flashrom](https://www.flashrom.org/) 和
[IMSProg](https://github.com/bigbigmdm/IMSProg)，并在芯片命令与
USB/串口传输之间做了清晰分层。

## 功能特性

- 编程器支持
  - CH341A、CH347T、CH347F
  - serprog 串口
  - HIDProg（预留占位）
- 协议：SPI NOR、SPI NAND、I2C EEPROM、Microwire EEPROM、
  SPI EEPROM、DataFlash AT45
- 读 / 写 / 擦除 / 校验，带实时进度
- 查空与可配置自动流程（读 / 擦除 / 查空 / 写入 / 校验）
- 带 JEDEC 自动识别的芯片数据库，也支持手动选择
- 深色 / 浅色 / 跟随系统主题
- 设置保存到 `Setting.set`，并支持从浏览器存储迁移
- 带安全确认的电压控制面板
- 关于对话框显示动态版本与芯片库统计
- SPI NAND 坏块模式（Skip / Bypass / Ignore）、BBM LUT 读写、
  on-die ECC 控制、OTP 与参数页读取、按芯片配置 dummy/plane/die
- 十六进制编辑器：编辑、撤销、搜索、跳转、填充、校验和
- Windows 原生文件对话框；Linux 支持进行中
- 插件系统：脚本插件、sidecar 适配器、能力白名单

## 安装

### 预构建产物

Alpha 构建从 `v0.5.0-alpha.1` 开始发布在 GitHub Releases。

- Windows：CI 产出 NSIS 安装包 + 便携 zip（libusb 后端）。
  由于许可证原因，官方 `CH34X.DLL` 不随源码分发；Windows DLL 后端需要
  本地放置厂商 DLL 后构建。
- Linux：在 Ubuntu 24.04 + WebKitGTK 上构建的 `uniprog`。

### 从源码构建

完整分步说明见 [docs/BUILDING_CN.md](docs/BUILDING_CN.md)。

Windows 快速开始：

```powershell
npm ci
npm run lint
npm run format:check
npm run verify:libusb
npm run dist:libusb
```

本地 DLL 构建（需要厂商 DLL，见构建文档）：

```powershell
npm run dist:dll
```

## 使用方法

1. 连接编程器，在左侧面板选择类型。
2. 点击 **连接**，再点击 **检测**。JEDEC ID 会在芯片数据库中查询。
3. 加载二进制文件，或把芯片读入十六进制编辑器。
4. 按需使用 **读 / 写 / 擦除 / 校验 / 查空**。

没有 JEDEC ID 的芯片（I2C、Microwire）请手动选择
类型 → 厂商 → 型号。

## 文档

- [构建](docs/BUILDING_CN.md) — 完整的分步构建说明。
- [架构](docs/ARCHITECTURE_CN.md) — 插件分层、源码组装、模块布局。
- [发布政策](docs/RELEASE_POLICY_CN.md) — 什么可以公开发布。
- [Sidecar 协议 v1](docs/SIDECAR_PROTOCOL_V1_CN.md) — 适配器通信协议。

每份文档都有对应的英文版本（不带 `_CN` 后缀）。

## 硬件后端

`modules/upt-devices-runtime/crate/src/ch34x.rs` 中的 HAL trait 是芯片
协议与硬件传输之间的边界。

- `hal-dll`：官方 CH34X.DLL 后端（Windows 默认）
- `hal-libusb`：rusb/libusb 后端（Linux 默认，Windows 可选）

后端选择是编译期 Cargo feature。构建命令见
[docs/BUILDING_CN.md](docs/BUILDING_CN.md)。

## 芯片数据库

`chiplib.bin` 是运行时数据库，在磁盘上轻度混淆（FFW 风格逐字节掩码 +
旋转）。可维护的明文芯片清单是 `flashdb/chiplib.xml`；构建时组装器会把它
复制进生成工程。仓库里不再存放混淆后的 XML，也不会在工作目录留下明文
数据库文件。

维护工具（另见 `cargo run --example chipdb_tool -- help`）：

```bash
# 从明文 XML 源重新生成 chiplib.bin
cargo run --example chipdb_tool -- xml2bin \
  flashdb/chiplib.xml modules/upt-bootstrap/root/chiplib.bin

# 合并 TSV 芯片表（插入缺失项、补全已有属性）
cargo run --example chipdb_tool -- merge modules/upt-bootstrap/root/chiplib.bin chips.tsv

# 按 JEDEC ID 添加或替换一个芯片
cargo run --example chipdb_tool -- add modules/upt-bootstrap/root/chiplib.bin 5E3213 \
  Zbit ZB25D40B SPI_NOR page=256 size=524288 sector=4096 block=65536

# 从 IMSProg.Dat 字段补全（只补缺失值）
cargo run --example chipdb_tool -- \
  modules/upt-bootstrap/root/chiplib.bin IMSProg.Dat --backup
```

## 开发

- Rust + Node.js 24
- `npm run verify:libusb`：在生成工程里运行 fmt、check、clippy 和测试
- 代码质量：`npm run lint` / `npm run format:check`
- 发布前 CI 强制执行同样的检查
- 宣布稳定版本前必须完成硬件验证

## 维护者

- [M0rt1s0114](https://github.com/M0rt1s0114)

## 参与贡献

见 [CONTRIBUTING_CN.md](CONTRIBUTING_CN.md)（中文）/
[CONTRIBUTING.md](CONTRIBUTING.md)（英文）。

## 许可证

[GPL-3.0-or-later](LICENSE)
