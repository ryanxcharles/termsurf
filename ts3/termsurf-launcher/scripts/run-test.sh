#!/bin/bash
# ts3/termsurf-launcher/scripts/run-test.sh
#
# Run the Experiment 2 XPC test.
# This registers the XPC service with launchd and runs a test.

set -e
cd "$(dirname "$0")/.."

APP="LauncherTest.app"

if [ ! -d "$APP" ]; then
    echo "ERROR: $APP not found. Run build-test.sh first."
    exit 1
fi

# Get absolute path
APP_PATH="$(cd "$APP" && pwd)"
XPC_PATH="$APP_PATH/Contents/XPCServices/com.termsurf.launcher.xpc"
PLIST_PATH="/tmp/com.termsurf.launcher.plist"

echo "=== Running Experiment 2: XPC IOSurface Transfer ==="
echo ""
echo "App path: $APP_PATH"
echo "XPC path: $XPC_PATH"

# Create launchd plist for the XPC service
echo "Creating launchd plist..."
cat > "$PLIST_PATH" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.termsurf.launcher</string>
    <key>MachServices</key>
    <dict>
        <key>com.termsurf.launcher</key>
        <true/>
    </dict>
    <key>ProgramArguments</key>
    <array>
        <string>$XPC_PATH/Contents/MacOS/termsurf-launcher</string>
    </array>
    <key>EnvironmentVariables</key>
    <dict>
        <key>TERMSURF_APP_PATH</key>
        <string>$APP_PATH</string>
    </dict>
</dict>
</plist>
EOF

# Unload if already loaded, ignoring errors
echo "Unloading existing service (if any)..."
launchctl unload "$PLIST_PATH" 2>/dev/null || true

# Load the service
echo "Loading XPC service..."
launchctl load "$PLIST_PATH"

# Wait for service to start
sleep 1

# Verify service is registered
echo "Verifying service registration..."
if launchctl list | grep -q "com.termsurf.launcher"; then
    echo "  Service registered successfully"
else
    echo "  WARNING: Service may not be registered"
fi

echo ""
echo "=== XPC Service Ready ==="
echo ""
echo "The launcher XPC service is now running."
echo ""
echo "To test Experiment 2:"
echo "1. Build and run wezterm-gui:"
echo "   cd ts3 && cargo build -p wezterm-gui"
echo "   ./target/debug/wezterm-gui"
echo ""
echo "2. In another terminal, send a test command:"
echo "   SOCKET=\$(ls /tmp/termsurf-gui-*.sock 2>/dev/null | head -1)"
echo "   echo '{\"id\":\"1\",\"action\":\"test_xpc\",\"data\":{\"pane_id\":0}}' | nc -U \"\$SOCKET\""
echo ""
echo "3. Check if the surface was received:"
echo "   echo '{\"id\":\"2\",\"action\":\"check_xpc_surface\",\"data\":{\"pane_id\":0}}' | nc -U \"\$SOCKET\""
echo ""
echo "To stop the service:"
echo "   launchctl unload $PLIST_PATH"
