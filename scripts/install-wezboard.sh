#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
BINARY="$REPO_DIR/wezboard/target/release/wezboard-gui"
TEMPLATE="$REPO_DIR/wezboard/assets/macos/Wezboard.app"
APP="/Applications/Wezboard.app"

# Verify release build exists.
if [ ! -f "$BINARY" ]; then
  echo "Error: Release build not found at $BINARY"
  echo "Run: cd wezboard && cargo build --release -p wezboard-gui"
  exit 1
fi

# Remove old install.
echo "==> Installing to $APP..."
sudo rm -rf "$APP"

# Copy app bundle template.
sudo cp -R "$TEMPLATE" "$APP"

# Create MacOS dir and copy binary.
sudo mkdir -p "$APP/Contents/MacOS"
sudo cp "$BINARY" "$APP/Contents/MacOS/wezboard-gui"

# Ad-hoc codesign.
echo "==> Codesigning..."
sudo codesign --force --deep --sign - "$APP"

echo ""
echo "Done."
echo "  App: $APP"
