#!/bin/bash
# ts3/termsurf-launcher/scripts/build-test.sh
#
# Build the launcher XPC service and test-sender for Experiment 2.
# This creates a test app bundle with the XPC service embedded.

set -e
cd "$(dirname "$0")/.."

echo "=== Building Launcher XPC Test Bundle ==="

# Validate required files exist
echo "Checking prerequisites..."
if [ ! -f "xpc-service/Info.plist" ]; then
    echo "ERROR: xpc-service/Info.plist not found!"
    exit 1
fi

# Validate Info.plist has required keys
if ! grep -q "com.termsurf.launcher" xpc-service/Info.plist; then
    echo "ERROR: Info.plist missing CFBundleIdentifier 'com.termsurf.launcher'"
    exit 1
fi
echo "Prerequisites OK"

# Build all Rust binaries
echo "Building Rust binaries..."
cargo build --release -p termsurf-launcher
cargo build --release -p termsurf-test-sender

# Find the target directory
if [ -d "../target/release" ]; then
    TARGET_DIR="../target/release"
elif [ -d "../../target/release" ]; then
    TARGET_DIR="../../target/release"
else
    echo "ERROR: Cannot find target directory"
    exit 1
fi
echo "Target directory: $TARGET_DIR"

# Verify binaries were created
for bin in termsurf-launcher termsurf-test-sender; do
    if [ ! -f "$TARGET_DIR/$bin" ]; then
        echo "ERROR: Failed to build $bin"
        exit 1
    fi
done
echo "Binaries built successfully"

# Create test app bundle structure
# Note: This is a test bundle. For real usage, these would be integrated
# into the wezterm-gui app bundle.
APP="LauncherTest.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/XPCServices/com.termsurf.launcher.xpc/Contents/MacOS"

# Copy binaries
# The test-sender goes in MacOS (it will be spawned by the launcher)
cp "$TARGET_DIR/termsurf-test-sender" "$APP/Contents/MacOS/"

# The launcher goes in the XPC service bundle
cp "$TARGET_DIR/termsurf-launcher" \
   "$APP/Contents/XPCServices/com.termsurf.launcher.xpc/Contents/MacOS/"

# Copy XPC service Info.plist
cp xpc-service/Info.plist \
   "$APP/Contents/XPCServices/com.termsurf.launcher.xpc/Contents/"

# Create app Info.plist
cat > "$APP/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.termsurf.launcher-test</string>
    <key>CFBundleExecutable</key>
    <string>termsurf-test-sender</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleName</key>
    <string>Launcher Test</string>
</dict>
</plist>
EOF

# Sign the XPC service (required for launchd to load it)
echo "Signing XPC service..."
codesign --force --sign - \
    "$APP/Contents/XPCServices/com.termsurf.launcher.xpc"

# Sign the app
echo "Signing app bundle..."
codesign --force --sign - "$APP"

# Validate bundle structure
echo "Validating bundle structure..."
ERRORS=0

check_file() {
    if [ ! -f "$1" ]; then
        echo "  MISSING: $1"
        ERRORS=$((ERRORS + 1))
    else
        echo "  OK: $1"
    fi
}

check_file "$APP/Contents/Info.plist"
check_file "$APP/Contents/MacOS/termsurf-test-sender"
check_file "$APP/Contents/XPCServices/com.termsurf.launcher.xpc/Contents/Info.plist"
check_file "$APP/Contents/XPCServices/com.termsurf.launcher.xpc/Contents/MacOS/termsurf-launcher"

if [ $ERRORS -gt 0 ]; then
    echo "ERROR: Bundle validation failed with $ERRORS errors"
    exit 1
fi

# Verify code signing
echo "Verifying code signatures..."
codesign --verify --verbose "$APP" 2>&1 || {
    echo "ERROR: App signature verification failed"
    exit 1
}
codesign --verify --verbose "$APP/Contents/XPCServices/com.termsurf.launcher.xpc" 2>&1 || {
    echo "ERROR: XPC service signature verification failed"
    exit 1
}

echo ""
echo "=== Build complete: $APP ==="
echo ""
echo "Bundle structure:"
find "$APP" -type f | sed 's/^/  /'
echo ""
echo "Next: Run ./scripts/run-test.sh to test the XPC service"
