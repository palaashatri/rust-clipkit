@echo off
REM build.bat — Build and run ClipKit (Windows)
setlocal enabledelayedexpansion

cd /d "%~dp0.."

echo ========================================
echo  ClipKit ^| Build and Run
echo ========================================
echo.

REM ─── 1. Rust / Cargo ──────────────────────────────────────────────────────────

where cargo >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Rust toolchain not found.
    echo Please install Rust from https://rustup.rs/ and re-run this script.
    exit /b 1
)

for /f "tokens=*" %%i in ('cargo --version') do echo [OK]    %%i
for /f "tokens=*" %%i in ('rustc  --version') do echo [OK]    %%i
echo.

REM ─── 2. MSVC check ──────────────────────────────────────────────────────────

where cl >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo [INFO]  MSVC compiler not found on PATH. 
    echo         Attempting to locate and initialize...
    
    set "VSCMD="
    for /f "usebackq tokens=*" %%i in (`where /r "C:\Program Files\Microsoft Visual Studio" vcvars64.bat 2^>nul`) do (
        set "VSCMD=%%i"
        goto :found_vscmd
    )
    
    :found_vscmd
    if defined VSCMD (
        echo [INFO]  Found: !VSCMD!
        call "!VSCMD!" >nul
        echo [OK]    MSVC environment initialized.
    ) else (
        echo [ERROR] MSVC not found. Please install "Desktop development with C++" 
        echo         via the Visual Studio Installer.
        exit /b 1
    )
) else (
    echo [OK]    MSVC compiler found.
)
echo.

REM ─── 3. Optional runtime dependencies ────────────────────────────────────────

echo [INFO]  Checking optional runtime dependencies...

where yt-dlp >nul 2>&1
if %ERRORLEVEL% equ 0 (
    for /f "tokens=*" %%i in ('yt-dlp --version 2^>nul') do echo [OK]    Download engine %%i
) else (
    echo [--]    Download engine not found on PATH
)

where ffmpeg >nul 2>&1
if %ERRORLEVEL% equ 0 (
    for /f "tokens=*" %%i in ('ffmpeg -version 2^>nul ^| findstr "version"') do echo [OK]    %%i
) else (
    echo [--]    ffmpeg not found on PATH
)
echo.

REM ─── 4. Build ─────────────────────────────────────────────────────────────────

set "BINARY=target\release\clipkit.exe"

echo [INFO]  Building release binary (this may take a few minutes on first run)...
echo.

cargo build --release -p gui
if %ERRORLEVEL% neq 0 (
    echo.
    echo [ERROR] Build failed.
    exit /b 1
)

echo.
echo [OK]    Build complete: %BINARY%
echo.

REM ─── 5. Run ───────────────────────────────────────────────────────────────────

echo [INFO]  Launching ClipKit...
start "" "%BINARY%"
