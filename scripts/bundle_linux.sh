#!/bin/bash
# bundle_linux.sh - Package ClipKit for Linux (tar.gz and AppImage)
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

APP_NAME="ClipKit"
BIN_NAME="clipkit"
VERSION=$(grep "^version =" Cargo.toml | head -n1 | cut -d '"' -f2)

# 1. Build release binary
TARGET="${1:-}"
ARCH="x64"
if [[ "$TARGET" == *"aarch64"* ]]; then
    ARCH="arm64"
fi

if [ -n "$TARGET" ]; then
    echo "[INFO] Building release binary for $TARGET..."
    cargo build --release -p gui --target "$TARGET"
    BIN_PATH="target/$TARGET/release/${BIN_NAME}"
else
    echo "[INFO] Building release binary..."
    cargo build --release -p gui
    BIN_PATH="target/release/${BIN_NAME}"
fi

# 2. Create tar.gz structure
echo "[INFO] Creating tar.gz structure..."
mkdir -p "dist/${APP_NAME}"
cp "$BIN_PATH" "dist/${APP_NAME}/${BIN_NAME}"
cp "LICENSE" "dist/${APP_NAME}/"
cp "README.md" "dist/${APP_NAME}/"

cd dist
tar -czf "${APP_NAME}-linux-${ARCH}.tar.gz" "${APP_NAME}"
cd ..

# 3. Create AppImage (requires appimagetool)
# In CI environments (like GitHub Actions), we might use an extracted appimagetool
APPIMAGE_TOOL=${APPIMAGE_TOOL:-$(command -v ./squashfs-root/AppRun || command -v appimagetool || command -v ./appimagetool-x86_64.AppImage || echo "")}

if [ -n "$APPIMAGE_TOOL" ]; then
    APPIMAGE_ARCH="x86_64"
    if [[ "$ARCH" == "arm64" ]]; then
        APPIMAGE_ARCH="aarch64"
    fi
    echo "[INFO] Creating AppImage for ${APPIMAGE_ARCH} using $APPIMAGE_TOOL..."
    mkdir -p "dist/${APP_NAME}.AppDir"
    cp "$BIN_PATH" "dist/${APP_NAME}.AppDir/AppRun"
    chmod +x "dist/${APP_NAME}.AppDir/AppRun"
    
    # Minimal desktop file
    cat > "dist/${APP_NAME}.AppDir/${APP_NAME}.desktop" <<EOF
[Desktop Entry]
Name=${APP_NAME}
Exec=AppRun
Icon=${APP_NAME}
Type=Application
Categories=Utility;
EOF

    # App icon
    if [ -f "assets/brand/icon-256.png" ]; then
        cp "assets/brand/icon-256.png" "dist/${APP_NAME}.AppDir/${APP_NAME}.png"
        cp "assets/brand/icon-256.png" "dist/${APP_NAME}.AppDir/.DirIcon"
    elif [ -f "assets/logo.svg" ]; then
        cp "assets/logo.svg" "dist/${APP_NAME}.AppDir/${APP_NAME}.svg"
    fi

    # Check if we need --appimage-extract-and-run (often needed in Docker/CI)
    if [[ "$APPIMAGE_TOOL" == *"appimagetool-x86_64.AppImage" ]]; then
        ARCH="$APPIMAGE_ARCH" ./appimagetool-x86_64.AppImage --appimage-extract-and-run "dist/${APP_NAME}.AppDir" "dist/${APP_NAME}-${APPIMAGE_ARCH}.AppImage"
    else
        ARCH="$APPIMAGE_ARCH" "$APPIMAGE_TOOL" "dist/${APP_NAME}.AppDir" "dist/${APP_NAME}-${APPIMAGE_ARCH}.AppImage"
    fi
else
    echo "[WARN] appimagetool not found. Skipping AppImage creation."
fi

echo "[OK] Linux bundles created in dist/"
