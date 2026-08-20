# 发布政策

> English version: [RELEASE_POLICY.md](RELEASE_POLICY.md)

本文规定什么可以发到 GitHub Releases，以及项目如何处理闭源厂商 DLL。

---

## 1. 公开发布内容

每个公开 GitHub Release 只包含恰好两个资产：

```text
UniProgrammer_<version>_x64-setup.exe
uniprog-<version>-win-x64.zip
```

规则：

- 安装包和便携 zip 都来自 **libusb profile**
  （`desktop-tauri-libusb`）。
- 两个资产内的任何位置都不得包含 `CH34X.DLL`，包括 `plugins/` 目录树内。
- 不上传其他任何资产：不上传裸 exe、不上传 DLL 包、不上传 `.unipkg`
  包、不上传 Linux 包。
- alpha/beta/rc 版本标记为 pre-release。

## 2. 闭源 CH34X.DLL 永不提交

项目本身使用 GPL-3.0-or-later 许可证。CH34X DLL 是南京沁恒
微电子（WCH）在官方 `CH341PAR.EXE` 驱动包中提供的闭源二进制文件，
按 WCH 自己的条款分发。

由于这一许可证情况，本项目采用永久性政策：

- `CH34X.DLL` **永不提交**进仓库。
- 项目**不会考虑**提交该 DLL，也不会提交任何内嵌它的构建，现在和将来
  都如此。
- 公开源码快照和公开发布资产都不包含该闭源文件。
- 公开 release workflow 只构建并上传 libusb profile；它没有任何步骤
  构建 DLL profile。

需要 DLL 后端的用户自行获取官方 DLL 并在本地构建，具体下载和放置步骤
见 [BUILDING_CN.md](BUILDING_CN.md)。这样得到的 DLL 构建只能私密、本地
使用，不得上传到公开下载位置。

## 3. CI 强制

`.github/workflows/release.yml` 只在 `v*` tag 上运行，并且：

1. 校验 `package.json`、`tauri.conf.json`、`Cargo.toml` 三处版本一致；
2. 运行 lint 和格式检查；
3. 运行完整 Rust 验证套件；
4. 构建 `npm run dist:libusb -- --skip-smoke`；
5. 上传恰好两个 libusb 资产。

## 4. 版本规则

- 当前方案：`0.<minor>.0-alpha.<n>`。
- patch 级 alpha 递增：`0.4.0-alpha.12` -> `0.4.0-alpha.13`。
- 1.0 之前的结构性变化：minor 递增，例如
  `0.4.0-alpha.12` -> `0.5.0-alpha.1`。
- `1.0.0` 保留给通过硬件验证清单的稳定版本。

打 tag 前，更新 [BUILDING_CN.md](BUILDING_CN.md) 列出的全部五个版本
位置。
