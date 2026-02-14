#!/bin/bash
# Build and bundle CefTest.app — the full cef-test harness
#
# Usage:
#   cd ts3 && ./cef-test-scripts/build.sh [--clean] [--open] [--release]
#
# Produces: ts3/CefTest.app
# Run with: ./CefTest.app/Contents/MacOS/cef-test-gui

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TS3_DIR="$(dirname "$SCRIPT_DIR")"
CEF_RS_DIR="$(dirname "$TS3_DIR")/vendor/cef-rs"

# Parse flags
CLEAN=false
OPEN=false
RELEASE=false
for arg in "$@"; do
    case $arg in
        --clean) CLEAN=true ;;
        --open) OPEN=true ;;
        --release) RELEASE=true ;;
    esac
done

if [ "$RELEASE" = true ]; then
    CARGO_FLAGS="--release"
    PROFILE="release"
else
    CARGO_FLAGS=""
    PROFILE="debug"
fi

if [ "$CLEAN" = true ]; then
    echo "=== Cleaning ==="
    rm -rf "$TS3_DIR/CefTest.app"
fi

# Clean stale XPC service registration
launchctl bootout "gui/$(id -u)/com.cef-test.launcher" 2>/dev/null || true
rm -f /private/tmp/com.cef-test.launcher.plist

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

# 2. Build all three cef-test binaries
echo "=== Building cef-test binaries ==="
cd "$TS3_DIR"
cargo build $CARGO_FLAGS -p cef-test-gui -p cef-test-profile -p cef-test-launcher

# 3. Create app bundle
APP="$TS3_DIR/CefTest.app"
echo "=== Creating app bundle at $APP ==="

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/Frameworks"

# 4. Copy main binary
cp "$TS3_DIR/target/$PROFILE/cef-test-gui" "$APP/Contents/MacOS/"

# 5. Copy profile server binary
cp "$TS3_DIR/target/$PROFILE/cef-test-profile" "$APP/Contents/Frameworks/"

# 6. Copy CEF framework
echo "Copying CEF framework..."
cp -R "$CEF_RS_DIR/cef-osr.app/Contents/Frameworks/Chromium Embedded Framework.framework" \
      "$APP/Contents/Frameworks/"

# 7. Copy and rename CEF helper apps
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

# 8. Copy XPC launcher service
echo "Copying XPC launcher service..."
mkdir -p "$APP/Contents/XPCServices/com.cef-test.launcher.xpc/Contents/MacOS"
cp "$TS3_DIR/target/$PROFILE/cef-test-launcher" \
   "$APP/Contents/XPCServices/com.cef-test.launcher.xpc/Contents/MacOS/"
cp "$TS3_DIR/cef-test-launcher/xpc-service/Info.plist" \
   "$APP/Contents/XPCServices/com.cef-test.launcher.xpc/Contents/"

# 9. Create app Info.plist
cat > "$APP/Contents/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>cef-test-gui</string>
    <key>CFBundleIdentifier</key>
    <string>com.cef-test.gui</string>
    <key>CFBundleName</key>
    <string>CefTest</string>
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

# 10. Sign the bundle
echo "Signing bundle..."
codesign --sign - --force --deep "$APP"

# 11. Register XPC launcher as launchd Mach service
LAUNCHER_BIN="$APP/Contents/XPCServices/com.cef-test.launcher.xpc/Contents/MacOS/cef-test-launcher"
PLIST_PATH="/private/tmp/com.cef-test.launcher.plist"

cat > "$PLIST_PATH" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.cef-test.launcher</string>
    <key>MachServices</key>
    <dict>
        <key>com.cef-test.launcher</key>
        <true/>
    </dict>
    <key>ProgramArguments</key>
    <array>
        <string>$LAUNCHER_BIN</string>
    </array>
</dict>
</plist>
EOF

launchctl bootstrap "gui/$(id -u)" "$PLIST_PATH"
echo "Registered com.cef-test.launcher with launchd"

echo ""
echo "=== Build Complete ==="
echo "App: $APP"
echo "  Contents/MacOS/cef-test-gui                                    (window + renderer)"
echo "  Contents/Frameworks/cef-test-profile                           (headless CEF)"
echo "  Contents/Frameworks/Chromium Embedded Framework.framework/     (CEF)"
echo "  Contents/XPCServices/com.cef-test.launcher.xpc/               (XPC bootstrap)"
echo ""

if [ "$OPEN" = true ]; then
    echo "Running CefTest..."
    open --stdout /tmp/cef-test-gui.log --stderr /tmp/cef-test-gui.log "$APP"
    echo "Logs: /tmp/cef-test-gui.log"
fi
