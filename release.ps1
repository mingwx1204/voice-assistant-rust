# release.ps1 — 发布脚本
# =======================
# 编译 release + 打包 + 创建 git tag

param(
    [string]$Version = "0.5.0"
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Mini 语音助手 v$Version - 发布" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# 1. 运行测试
Write-Host "`n[1/5] 运行测试..." -ForegroundColor Yellow
cargo test
if ($LASTEXITCODE -ne 0) { Write-Host "测试失败!" -ForegroundColor Red; exit 1 }

# 2. Clippy 检查
Write-Host "[2/5] Clippy 检查..." -ForegroundColor Yellow
cargo clippy -- -D warnings
if ($LASTEXITCODE -ne 0) { Write-Host "Clippy 失败!" -ForegroundColor Red; exit 1 }

# 3. 格式化检查
Write-Host "[3/5] 格式化检查..." -ForegroundColor Yellow
cargo fmt --check
if ($LASTEXITCODE -ne 0) { Write-Host "格式化失败!" -ForegroundColor Red; exit 1 }

# 4. 编译 release
Write-Host "[4/5] 编译 release..." -ForegroundColor Yellow
cargo build --release
if ($LASTEXITCODE -ne 0) { Write-Host "编译失败!" -ForegroundColor Red; exit 1 }

# 5. 打包
Write-Host "[5/5] 打包..." -ForegroundColor Yellow
$exePath = "target\release\voice-assistant.exe"
$zipPath = "voice-assistant-windows-x64.zip"

if (Test-Path $zipPath) { Remove-Item $zipPath }
Compress-Archive -Path $exePath -DestinationPath $zipPath
$size = [math]::Round((Get-Item $zipPath).Length / 1MB, 1)

# 6. Git tag
Write-Host "`n创建 git tag v$Version..." -ForegroundColor Yellow
git tag -a "v$Version" -m "Release v$Version"
git push origin "v$Version"

Write-Host "`n========================================" -ForegroundColor Green
Write-Host "  发布完成! v$Version" -ForegroundColor Green
Write-Host "  文件: $zipPath ($size MB)" -ForegroundColor Green
Write-Host "  GitHub Actions 将自动构建并发布" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
