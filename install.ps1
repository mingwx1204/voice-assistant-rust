# install.ps1 — 安装脚本
# =======================
# 将 voice-assistant 安装到系统目录

param(
    [string]$InstallDir = "$env:LOCALAPPDATA\MiniAssistant"
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  Mini 语音助手 - 安装程序" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan

# 1. 检查构建文件
$exePath = "target\release\voice-assistant.exe"
if (-not (Test-Path $exePath)) {
    Write-Host "`n未找到 release 构建，正在编译..." -ForegroundColor Yellow
    .\build.ps1 -Release
    if ($LASTEXITCODE -ne 0) {
        Write-Host "编译失败!" -ForegroundColor Red
        exit 1
    }
}

# 2. 创建安装目录
Write-Host "`n[1/5] 创建安装目录..." -ForegroundColor Yellow
New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null

# 3. 复制文件
Write-Host "[2/5] 复制程序文件..." -ForegroundColor Yellow
Copy-Item $exePath "$InstallDir\voice-assistant.exe" -Force

# 复制模型目录
$modelSource = "models"
$modelDest = "$InstallDir\models"
if (Test-Path $modelSource) {
    if (-not (Test-Path $modelDest)) {
        New-Item -ItemType Directory -Path $modelDest -Force | Out-Null
    }
    Copy-Item "$modelSource\*" $modelDest -Recurse -Force
}

# 复制配置文件
if (Test-Path "config.json") {
    Copy-Item "config.json" $InstallDir -Force
}

# 4. 创建启动脚本
Write-Host "[3/5] 创建启动脚本..." -ForegroundColor Yellow
$startScript = @"
@echo off
cd /d "$InstallDir"
start "" "voice-assistant.exe"
"@
$startScript | Out-File -FilePath "$InstallDir\start.bat" -Encoding ASCII

# 5. 创建桌面快捷方式
Write-Host "[4/5] 创建桌面快捷方式..." -ForegroundColor Yellow
$desktop = [Environment]::GetFolderPath("Desktop")
$shortcutPath = "$desktop\Mini语音助手.lnk"
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = "$InstallDir\voice-assistant.exe"
$shortcut.WorkingDirectory = $InstallDir
$shortcut.Description = "Mini 语音助手 v0.3.0"
$shortcut.Save()

# 6. 添加到 PATH（可选）
Write-Host "[5/5] 配置环境变量..." -ForegroundColor Yellow
$currentPath = [Environment]::GetEnvironmentVariable("PATH", "User")
if ($currentPath -notlike "*$InstallDir*") {
    [Environment]::SetEnvironmentVariable("PATH", "$currentPath;$InstallDir", "User")
    Write-Host "  已添加到 PATH" -ForegroundColor Green
}

Write-Host "`n========================================" -ForegroundColor Green
Write-Host "  安装完成! 🎉" -ForegroundColor Green
Write-Host "  安装目录: $InstallDir" -ForegroundColor Green
Write-Host "  桌面快捷方式: $shortcutPath" -ForegroundColor Green
Write-Host "  启动方式:" -ForegroundColor Green
Write-Host "    - 双击桌面快捷方式" -ForegroundColor Green
Write-Host "    - 运行 $InstallDir\start.bat" -ForegroundColor Green
Write-Host "    - 命令行: voice-assistant" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Green
