#!/bin/bash
# ts3/termsurf-xpc/scripts/run-test.sh
#
# Build and run the XPC test for Experiment 1.

set -e
cd "$(dirname "$0")/.."

# Build first
./scripts/build-test.sh

echo ""
echo "=== Registering XPC Service with launchd ==="
echo ""

# Get absolute path to launcher
LAUNCHER_PATH="$(cd TestXPC.app/Contents/XPCServices/com.termsurf.xpc-test.xpc/Contents/MacOS && pwd)/launcher"
echo "Launcher path: $LAUNCHER_PATH"

# Create launchd plist with actual path
PLIST_DIR="$HOME/Library/LaunchAgents"
PLIST_FILE="$PLIST_DIR/com.termsurf.xpc-test.plist"
mkdir -p "$PLIST_DIR"

cat > "$PLIST_FILE" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.termsurf.xpc-test</string>
    <key>Program</key>
    <string>$LAUNCHER_PATH</string>
    <key>MachServices</key>
    <dict>
        <key>com.termsurf.xpc-test</key>
        <true/>
    </dict>
</dict>
</plist>
EOF

echo "Created launchd plist: $PLIST_FILE"

# Unload if already loaded, then load
launchctl unload "$PLIST_FILE" 2>/dev/null || true
launchctl load "$PLIST_FILE"
echo "Loaded XPC service with launchd"

# Give launchd a moment
sleep 1

echo ""
echo "=== Running XPC Test ==="
echo ""

# Run receiver (it will ask launcher to spawn sender)
# The receiver handles its own timeout
./TestXPC.app/Contents/MacOS/receiver
EXIT_CODE=$?

# Cleanup: unload the service
echo ""
echo "Cleaning up..."
launchctl unload "$PLIST_FILE" 2>/dev/null || true
rm -f "$PLIST_FILE"

echo ""
if [ $EXIT_CODE -eq 0 ]; then
    echo "=== TEST PASSED ==="
else
    echo "=== TEST FAILED (exit code: $EXIT_CODE) ==="
    exit 1
fi
