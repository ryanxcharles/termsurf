#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"
CHROMIUM_OUT="$REPO_DIR/chromium/src/out/Default"
ROAMIUM_SRC="$REPO_DIR/roamium/target/release/roamium"
INSTALL_DIR="/usr/local/lib/roamium"

# Verify release build exists.
if [ ! -f "$ROAMIUM_SRC" ]; then
  echo "Error: Release build not found at $ROAMIUM_SRC"
  echo "Run: scripts/build-roamium.sh --release"
  exit 1
fi

echo "==> Installing Roamium to $INSTALL_DIR..."
sudo mkdir -p "$INSTALL_DIR"

# Copy roamium binary.
sudo cp "$ROAMIUM_SRC" "$INSTALL_DIR/roamium"

# Copy dylibs.
echo "==> Copying dylibs..."
sudo cp "$CHROMIUM_OUT"/*.dylib "$INSTALL_DIR/"

# Copy resources.
echo "==> Copying resources..."
sudo cp "$CHROMIUM_OUT"/*.pak "$INSTALL_DIR/"
sudo cp "$CHROMIUM_OUT/icudtl.dat" "$INSTALL_DIR/"
sudo cp "$CHROMIUM_OUT"/v8_context_snapshot*.bin "$INSTALL_DIR/"

# Symlink to /usr/local/bin.
echo "==> Symlinking /usr/local/bin/roamium..."
sudo ln -sf "$INSTALL_DIR/roamium" /usr/local/bin/roamium

echo ""
echo "Done."
echo "  Dir:  $INSTALL_DIR"
echo "  Bin:  /usr/local/bin/roamium"
