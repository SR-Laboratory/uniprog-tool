#!/usr/bin/env bash
# UniProgrammer 编译选单（Linux / macOS）
# 选择后端后运行完整构建管线（前端 + 组装 + Rust release + 打包）。
set -euo pipefail
cd "$(dirname "$0")"

echo
echo '  UniProgrammer build menu'
echo '  ------------------------'
echo '  [1] 自动（本平台默认：Windows=DLL，Linux=libusb）'
echo '  [2] 强制 libusb（rusb）'
echo '  [3] 强制 DLL（CH34X.DLL，仅 Windows 有意义）'
echo
read -r -p '  选择后端 (1/2/3): ' choice

case "$choice" in
  1)
    if [ "$(uname -s)" = "Linux" ]; then
      profile="desktop-tauri-libusb"
    else
      profile="desktop-tauri-dll"
    fi
    ;;
  2) profile="desktop-tauri-libusb" ;;
  3) profile="desktop-tauri-dll" ;;
  *) echo '无效选择' >&2; exit 1 ;;
esac

echo
echo ">>> node tools/build.mjs --profile $profile"
node tools/build.mjs --profile "$profile"

echo
echo '完成。产物目录：'
echo "  dist/$profile/installer"
echo "  dist/$profile/portable"
