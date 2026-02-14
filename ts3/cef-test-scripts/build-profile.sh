#!/bin/bash
# Build and bundle cef-test-profile as a standalone macOS app
#
# Usage:
#   ./cef-test-scripts/build-profile.sh
#
# Produces: ts3/cef-test-profile.app
# Run with: ./cef-test-profile.app/Contents/MacOS/cef-test-profile --url https://google.com

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TS3_DIR="$(dirname "$SCRIPT_DIR")"
CEF_RS_DIR="$(dirname "$TS3_DIR")/vendor/cef-rs"

# 1. Ensure cef-osr.app exists (provides CEF framework and helper processes)
if [ ! -d "$CEF_RS_DIR/cef-osr.app" ]; then
    echo "=== Building cef-osr for CEF framework ==="
    cd "$CEF_RS_DIR"
    cargo build -p cef-osr
    cargo run -p cef --bin bundle-cef-app -- cef-osr -o cef-osr.app
fi

if [ ! -d "$CEF_RS_DIR/cef-osr.app" ]; then
    echo "ERROR: cef-osr.app not found at $CEF_RS_DIR/cef-osr.app"
    exit 1
fi

# 2. Build cef-test-profile
echo "=== Building cef-test-profile ==="
cd "$TS3_DIR"
cargo build -p cef-test-profile

# 3. Create app bundle
APP="$TS3_DIR/cef-test-profile.app"
echo "=== Creating app bundle at $APP ==="

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Frameworks"

# 4. Copy binary
cp "$TS3_DIR/target/debug/cef-test-profile" "$APP/Contents/MacOS/"

# 5. Copy CEF framework
echo "Copying CEF framework..."
cp -R "$CEF_RS_DIR/cef-osr.app/Contents/Frameworks/Chromium Embedded Framework.framework" \
      "$APP/Contents/Frameworks/"

# 6. Copy and rename CEF helper apps
echo "Copying CEF helper apps..."
for suffix in "" " (GPU)" " (Renderer)" " (Plugin)" " (Alerts)"; do
    src="$CEF_RS_DIR/cef-osr.app/Contents/Frameworks/cef-osr Helper${suffix}.app"
    dst="$APP/Contents/Frameworks/cef-test-profile Helper${suffix}.app"
    if [ -d "$src" ]; then
        cp -R "$src" "$dst"
        sed -i '' 's/cef-osr/cef-test-profile/g' "$dst/Contents/Info.plist"
        mv "$dst/Contents/MacOS/cef-osr Helper${suffix}" \
           "$dst/Contents/MacOS/cef-test-profile Helper${suffix}"
    else
        echo "WARNING: Helper not found: $src"
    fi
done

# 7. Create Info.plist
cat > "$APP/Contents/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>cef-test-profile</string>
    <key>CFBundleIdentifier</key>
    <string>com.cef-test.profile</string>
    <key>CFBundleName</key>
    <string>cef-test-profile</string>
    <key>CFBundleVersion</key>
    <string>1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSEnvironment</key>
    <dict>
        <key>MallocNanoZone</key>
        <string>0</string>
    </dict>
</dict>
</plist>
PLIST

# 8. Sign the bundle
echo "Signing bundle..."
codesign --sign - --force --deep "$APP"

echo ""
echo "=== Build Complete ==="
echo "App: $APP"
echo ""
echo "Run:"
echo "  $APP/Contents/MacOS/cef-test-profile --url https://google.com"
