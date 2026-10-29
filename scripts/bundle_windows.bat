@echo off
REM bundle_windows.bat - Package ClipKit for Windows

cd /d "%~dp0.."

set APP_NAME=ClipKit
set BIN_NAME=clipkit.exe
set DIST_DIR=dist
set TARGET=%1

if "%TARGET%"=="" (
    echo [INFO] Building release binary...
    cargo build --release -p gui
    if errorlevel 1 exit /b %errorlevel%
    set BIN_PATH=target\release\%BIN_NAME%
) else (
    echo [INFO] Building release binary for %TARGET%...
    cargo build --release -p gui --target %TARGET%
    if errorlevel 1 exit /b %errorlevel%
    set BIN_PATH=target\%TARGET%\release\%BIN_NAME%
)

if not exist %DIST_DIR% mkdir %DIST_DIR%

echo [INFO] Copying binary to dist...
copy %BIN_PATH% %DIST_DIR%\%APP_NAME%.exe
if exist assets\brand\icon.ico copy assets\brand\icon.ico %DIST_DIR%\%APP_NAME%.ico

echo [OK] Windows bundle created in %DIST_DIR%\%APP_NAME%.exe
