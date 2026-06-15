#!/usr/bin/env bash
# build.sh — Build and run ClipKit (macOS / Linux)
set -euo pipefail

BOLD=$'\e[1m'; RESET=$'\e[0m'
GREEN=$'\e[32m'; YELLOW=$'\e[33m'; RED=$'\e[31m'

info()  { echo "${BOLD}[INFO]${RESET}  $*"; }
ok()    { echo "${GREEN}[OK]${RESET}    $*"; }
note()  { echo "${YELLOW}[--]${RESET}    $*"; }
error() { echo "${RED}[ERROR]${RESET} $*" >&2; }

echo "${BOLD}========================================"
echo " ClipKit | Build and Run"
echo "========================================${RESET}"
echo

# ─── 1. Rust / Cargo ──────────────────────────────────────────────────────────

# Source the env file written by rustup so cargo is on PATH even in a fresh shell
if [[ -f "$HOME/.cargo/env" ]]; then
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
fi

if ! command -v cargo &>/dev/null; then
    info "Rust toolchain not found — installing via rustup..."
    if ! command -v curl &>/dev/null; then
        error "curl is required to download rustup. Install curl and re-run."
        exit 1
    fi
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
        | sh -s -- -y --no-modify-path
    source "$HOME/.cargo/env"
fi

if ! command -v cargo &>/dev/null; then
    error "cargo not found after install. Open a new shell and re-run this script."
    exit 1
fi

ok "$(cargo --version)"
ok "$(rustc  --version)"
echo

# ─── 2. C linker check ────────────────────────────────────────────────────────

OS="$(uname -s)"

if [[ "$OS" == "Darwin" ]]; then
    if ! xcode-select -p &>/dev/null; then
        info "Xcode Command Line Tools not found — installing..."
        xcode-select --install 2>/dev/null || true
        echo
        echo "  Re-run this script after the Xcode CLT installer finishes."
        exit 0
    fi
    ok "Xcode CLT: $(xcode-select -p)"
else
    # Linux: need cc / gcc
    if ! command -v cc &>/dev/null && ! command -v gcc &>/dev/null; then
        note "No C compiler found."
        note "Install build tools, e.g.:"
        note "  Ubuntu/Debian : sudo apt install build-essential"
        note "  Fedora/RHEL   : sudo dnf groupinstall 'Development Tools'"
        note "  Arch          : sudo pacman -S base-devel"
        echo
    else
        ok "C compiler: $(cc --version 2>&1 | head -1)"
    fi
fi
echo

# ─── 3. Optional runtime dependencies ────────────────────────────────────────

info "Checking optional runtime dependencies..."

if command -v yt-dlp &>/dev/null; then
    ok "Download engine $(yt-dlp --version 2>/dev/null || echo '(version unknown)')"
else
    note "Download engine not on PATH  (use the Engine Management tab inside the app to download it)"
fi

if command -v ffmpeg &>/dev/null; then
    ok "$(ffmpeg -version 2>&1 | head -1)"
else
    note "ffmpeg not on PATH  (required only for the Transcoder pane)"
fi
echo

# ─── 4. Build ─────────────────────────────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$SCRIPT_DIR"

BINARY="$SCRIPT_DIR/target/release/clipkit"

info "Building release binary (this may take a few minutes on first run)..."
echo
cargo build --release -p gui

# Ensure the binary is executable (it should be, but make certain)
chmod +x "$BINARY"

echo
ok "Build complete: $BINARY"
echo

# ─── 5. Run ───────────────────────────────────────────────────────────────────

info "Launching ClipKit..."

if [[ "$OS" == "Darwin" ]]; then
    # If the bundle exists, launch it. Otherwise launch the binary detached.
    APP_BUNDLE="$SCRIPT_DIR/ClipKit.app"
    if [ -d "$APP_BUNDLE" ]; then
        open "$APP_BUNDLE"
        ok "Launched App Bundle: $APP_BUNDLE"
    else
        # Launch detached from the terminal so closing the shell doesn't kill the app
        nohup "$BINARY" >/dev/null 2>&1 &
        disown
        ok "Launched binary detached (PID $!)"
        note "For a full native experience (icon, menu bar), run: ./scripts/bundle_macos.sh"
    fi
else
    # Launch detached from the terminal so closing the shell doesn't kill the app
    nohup "$BINARY" >/dev/null 2>&1 &
    disown
    ok "Launched (PID $!)"
fi
