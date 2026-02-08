#!/bin/bash
# ts3/termsurf-xpc/scripts/build-test.sh
#
# Build the XPC test bundle for Experiment 1.

set -e
cd "$(dirname "$0")/.."

echo "=== Building XPC Test Bundle ==="

# Validate required files exist
echo "Checking prerequisites..."
if [ ! -f "xpc-service/Info.plist" ]; then
    echo "ERROR: xpc-service/Info.plist not found!"
    echo "Create this file with the XPC service configuration."
    exit 1
fi

# Validate Info.plist has required keys
if ! grep -q "com.termsurf.xpc-test" xpc-service/Info.plist; then
    echo "ERROR: Info.plist missing CFBundleIdentifier 'com.termsurf.xpc-test'"
    exit 1
fi
if ! grep -q "XPCService" xpc-service/Info.plist; then
    echo "ERROR: Info.plist missing XPCService dictionary"
    exit 1
fi
echo "Prerequisites OK"

# Build all Rust binaries
echo "Building Rust binaries..."
cargo build --release --example launcher
cargo build --release --example receiver
cargo build --release --example sender

# Find the target directory (could be ts3/termsurf-xpc/target or workspace target at ts3/target)
if [ -d "target/release/examples" ]; then
    TARGET_DIR="target/release/examples"
elif [ -d "../target/release/examples" ]; then
    TARGET_DIR="../target/release/examples"
elif [ -d "../../target/release/examples" ]; then
    TARGET_DIR="../../target/release/examples"
else
    echo "ERROR: Cannot find target directory"
    echo "Looked in:"
    echo "  - target/release/examples"
    echo "  - ../target/release/examples"
    echo "  - ../../target/release/examples"
    exit 1
fi
echo "Target directory: $TARGET_DIR"

# Verify binaries were created
for bin in launcher receiver sender; do
    if [ ! -f "$TARGET_DIR/$bin" ]; then
        echo "ERROR: Failed to build $bin"
        exit 1
    fi
done
echo "Binaries built successfully"

# Create test app bundle structure
APP="TestXPC.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/XPCServices/com.termsurf.xpc-test.xpc/Contents/MacOS"

# Copy binaries
cp "$TARGET_DIR/receiver" "$APP/Contents/MacOS/"
cp "$TARGET_DIR/sender" "$APP/Contents/MacOS/"
cp "$TARGET_DIR/launcher" \
   "$APP/Contents/XPCServices/com.termsurf.xpc-test.xpc/Contents/MacOS/"

# Copy XPC service Info.plist
cp xpc-service/Info.plist \
   "$APP/Contents/XPCServices/com.termsurf.xpc-test.xpc/Contents/"

# Create app Info.plist
cat > "$APP/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.termsurf.xpc-test</string>
    <key>CFBundleExecutable</key>
    <string>receiver</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
</dict>
</plist>
EOF

# Sign the XPC service (required for launchd to load it)
echo "Signing XPC service..."
codesign --force --sign - \
    "$APP/Contents/XPCServices/com.termsurf.xpc-test.xpc"

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
check_file "$APP/Contents/MacOS/receiver"
check_file "$APP/Contents/MacOS/sender"
check_file "$APP/Contents/XPCServices/com.termsurf.xpc-test.xpc/Contents/Info.plist"
check_file "$APP/Contents/XPCServices/com.termsurf.xpc-test.xpc/Contents/MacOS/launcher"

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
codesign --verify --verbose "$APP/Contents/XPCServices/com.termsurf.xpc-test.xpc" 2>&1 || {
    echo "ERROR: XPC service signature verification failed"
    exit 1
}

echo ""
echo "=== Build complete: $APP ==="
echo ""
echo "Bundle structure:"
find "$APP" -type f | sed 's/^/  /'
