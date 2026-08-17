# build.ps1 — 构建脚本
# =====================
# 使用方法: .\build.ps1 [-Release] [-Clean]

param(
    [switch]$Release,
    [switch]$Clean,
    [switch]$Test
)

$ErrorActionPreference = "Stop"

# 设置 VS Build Tools 环境
$vsPath = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools"
if (Test-Path $vsPath) {
    $cmakePath = "$vsPath\Common7\IDE\CommonExtensions\Microsoft\CMake\CMake\bin"
    if (Test-Path $cmakePath) {
        $env:PATH = "$cmakePath;$env:PATH"
    }
}

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Mini 语音助手 - 构建脚本" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

if ($Clean) {
    Write-Host "`n[1/4] 清理旧构建..." -ForegroundColor Yellow
    cargo clean
}

$profile = if ($Release) { "--release" } else { "" }

Write-Host "`n[2/4] 检查代码..." -ForegroundColor Yellow
cargo check $profile

if ($LASTEXITCODE -ne 0) {
    Write-Host "检查失败!" -ForegroundColor Red
    exit 1
}

if ($Test) {
    Write-Host "`n[3/4] 运行测试..." -ForegroundColor Yellow
    cargo test $profile
    
    if ($LASTEXITCODE -ne 0) {
        Write-Host "测试失败!" -ForegroundColor Red
        exit 1
    }
}

Write-Host "`n[$(if($Test){'4'}else{'3'})/$(if($Test){'4'}else{'3'})] 编译中..." -ForegroundColor Yellow
cargo build $profile

if ($LASTEXITCODE -ne 0) {
    Write-Host "编译失败!" -ForegroundColor Red
    exit 1
}

$exePath = if ($Release) { "target\release\voice-assistant.exe" } else { "target\debug\voice-assistant.exe" }

if (Test-Path $exePath) {
    $size = [math]::Round((Get-Item $exePath).Length / 1MB, 1)
    Write-Host "`n========================================" -ForegroundColor Green
    Write-Host "  构建成功! 🎉" -ForegroundColor Green
    Write-Host "  文件: $exePath" -ForegroundColor Green
    Write-Host "  大小: $size MB" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
} else {
    Write-Host "构建文件未找到" -ForegroundColor Red
    exit 1
}
