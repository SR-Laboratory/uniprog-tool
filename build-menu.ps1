# UniProgrammer 编译选单（Windows PowerShell）
# 选择后端后运行完整构建管线（前端 + 组装 + Rust release + 打包）。

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot

Write-Host ''
Write-Host '  UniProgrammer build menu'
Write-Host '  ------------------------'
Write-Host '  [1] 自动（本平台默认：Windows=DLL，Linux=libusb）'
Write-Host '  [2] 强制 libusb（rusb）'
Write-Host '  [3] 强制 DLL（CH34X.DLL，仅 Windows 有意义）'
Write-Host ''
$choice = Read-Host '  选择后端 (1/2/3)'
$profile = switch ($choice) {
  '1' { if ($IsWindows -or $env:OS -eq 'Windows_NT') { 'desktop-tauri-dll' } else { 'desktop-tauri-libusb' } }
  '2' { 'desktop-tauri-libusb' }
  '3' { 'desktop-tauri-dll' }
  default { Write-Host '无效选择'; exit 1 }
}

Write-Host ''
Write-Host ">>> node tools/build.mjs --profile $profile"
node tools/build.mjs --profile $profile
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ''
Write-Host '完成。产物目录：'
Write-Host "  dist\$profile\installer"
Write-Host "  dist\$profile\portable"
