#!/bin/bash
# Clean zig build artifacts without touching the Chromium cache.
# Usage: ./scripts/clean-zig.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(dirname "$SCRIPT_DIR")"
GUI_DIR="$REPO_ROOT/ghostboard"

echo "Cleaning zig build artifacts..."
rm -rf "$GUI_DIR/zig-out/"
rm -rf "$GUI_DIR/.zig-cache/"
rm -rf "$GUI_DIR/macos/build/"
rm -rf ~/Library/Developer/Xcode/DerivedData/Ghostty-*

echo "Done. Chromium cache at chromium/src/out/ is untouched."
