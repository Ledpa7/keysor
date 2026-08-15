#!/bin/bash
set -e

PROJECT_DIR="/Volumes/scratch/14-Keysor"
DESKTOP_APP="/Users/jidu/Desktop/Keysor.app"
TARGET_APP="$PROJECT_DIR/target/release/Keysor.app"

echo "1. Building release binary with CARGO_INCREMENTAL=0..."
cd "$PROJECT_DIR"
CARGO_INCREMENTAL=0 cargo build --release

echo "2. Ensuring keysor.icns is up to date..."
swift "$PROJECT_DIR/scratch/generate_mac_icon.swift"

function package_app() {
    local APP_PATH="$1"
    echo "Packaging $APP_PATH..."
    
    mkdir -p "$APP_PATH/Contents/MacOS"
    mkdir -p "$APP_PATH/Contents/Resources"
    
    cp "$PROJECT_DIR/target/release/keysor" "$APP_PATH/Contents/MacOS/keysor"
    chmod +x "$APP_PATH/Contents/MacOS/keysor"
    
    cp "$PROJECT_DIR/keysor.icns" "$APP_PATH/Contents/Resources/keysor.icns"
    
    cat << 'EOF' > "$APP_PATH/Contents/Info.plist"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE PLIST PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>keysor</string>
    <key>CFBundleIconFile</key>
    <string>keysor.icns</string>
    <key>CFBundleIconName</key>
    <string>keysor</string>
    <key>CFBundleIdentifier</key>
    <string>app.keysor.keysor</string>
    <key>CFBundleName</key>
    <string>Keysor</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.1</string>
    <key>LSMinimumSystemVersion</key>
    <string>10.13</string>
    <key>NSHighResolutionCapable</key>
    <true/>
</dict>
</plist>
EOF
    
    touch "$APP_PATH"
}

package_app "$TARGET_APP"
if [ -d "$DESKTOP_APP" ]; then
    package_app "$DESKTOP_APP"
fi

echo "3. Refreshing macOS Finder Icon Cache..."
/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$TARGET_APP" || true
if [ -d "$DESKTOP_APP" ]; then
    /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister -f "$DESKTOP_APP" || true
fi
killall Finder || true

echo "SUCCESS! Keysor.app successfully bundled with keysor.icns and Finder cache refreshed!"
