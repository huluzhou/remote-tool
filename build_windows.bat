@echo off
chcp 65001 >nul 2>&1
REM Windows 64位打包脚本
REM 使用 PyInstaller 将 query_tool 打包为 Windows 可执行文件

echo ========================================
echo Query Tool Windows 64位打包脚本
echo ========================================
echo.

REM 检查 Python 是否安装
python --version >nul 2>&1
if errorlevel 1 (
    echo Error: Python not found, please install Python 3.8+
    pause
    exit /b 1
)

echo [1/4] Checking dependencies...
python -m pip install --upgrade pip
python -m pip install -r requirements.txt
python -m pip install pyinstaller

if errorlevel 1 (
    echo Error: Failed to install dependencies
    pause
    exit /b 1
)

echo.
echo [2/4] Cleaning old build files...
if exist build rmdir /s /q build
if exist dist rmdir /s /q dist
if exist query_tool.spec del /q query_tool.spec

echo.
echo [3/4] Building package...
cd /d %~dp0
pyinstaller build_windows.spec

if errorlevel 1 (
    echo Error: Build failed
    pause
    exit /b 1
)

echo.
echo [4/4] Copying config file...
copy /Y "csv_export_config.toml" "dist\csv_export_config.toml" >nul 2>&1
if errorlevel 1 (
    echo Warning: Failed to copy config file
) else (
    echo Config file copied to dist directory
)

echo.
echo Build complete!
echo.
echo Executable location: dist\query_tool.exe
echo Config file location: dist\csv_export_config.toml
echo.
echo Tips:
echo   - Copy both query_tool.exe and csv_export_config.toml together
echo   - Place them in the same directory
echo   - First run may take a few seconds to start
echo   - If you encounter issues, try running from command line to see error messages
echo.

pause
