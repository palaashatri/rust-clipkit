#!/bin/bash
# bundle_macos.sh - Create a macOS .app bundle for ClipKit
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

APP_NAME="ClipKit"
BIN_NAME="clipkit"
BUNDLE_ID="com.palaashatri.clipkit"
VERSION=$(grep "^version =" Cargo.toml | head -n1 | cut -d '"' -f2)

# 1. Build release binary
TARGET="${1:-}"
if [ -n "$TARGET" ]; then
    echo "[INFO] Building release binary for $TARGET..."
    cargo build --release -p gui --target "$TARGET"
    BIN_PATH="target/$TARGET/release/${BIN_NAME}"
else
    echo "[INFO] Building release binary..."
    cargo build --release -p gui
    BIN_PATH="target/release/${BIN_NAME}"
fi

# 2. Create folder structure
echo "[INFO] Creating bundle structure..."
mkdir -p "${APP_NAME}.app/Contents/MacOS"
mkdir -p "${APP_NAME}.app/Contents/Resources"

# 3. Copy binary
cp "$BIN_PATH" "${APP_NAME}.app/Contents/MacOS/${BIN_NAME}"

# 4. Create Info.plist
echo "[INFO] Creating Info.plist..."
cat > "${APP_NAME}.app/Contents/Info.plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>${BIN_NAME}</string>
    <key>CFBundleIdentifier</key>
    <string>${BUNDLE_ID}</string>
    <key>CFBundleName</key>
    <string>${APP_NAME}</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>${VERSION}</string>
    <key>CFBundleSignature</key>
    <string>????</string>
    <key>CFBundleVersion</key>
    <string>1</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.15</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
</dict>
</plist>
EOF

# 5. Handle Icon
if [ -f "assets/brand/icon.icns" ]; then
    echo "[INFO] Copying brand icon.icns..."
    mkdir -p "${APP_NAME}.app/Contents/Resources"
    cp "assets/brand/icon.icns" "${APP_NAME}.app/Contents/Resources/AppIcon.icns"
elif [ -f "assets/icon.icns" ]; then
    echo "[INFO] Copying icon.icns..."
    mkdir -p "${APP_NAME}.app/Contents/Resources"
    cp "assets/icon.icns" "${APP_NAME}.app/Contents/Resources/AppIcon.icns"
else
    echo "[WARN] No assets/brand/icon.icns found. App will use default icon."
fi

# 6. Finalize
touch "${APP_NAME}.app"
/usr/bin/python3 -c "import os; from Foundation import NSWorkspace; workspace = NSWorkspace.sharedWorkspace(); workspace.setIcon_forFile_options_(None, os.path.abspath('${APP_NAME}.app'), 0)" 2>/dev/null || true
echo "[TIP] You can now move ${APP_NAME}.app to your Applications folder."
