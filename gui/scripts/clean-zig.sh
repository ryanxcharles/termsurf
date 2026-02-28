#!/bin/bash
# Clean zig build artifacts without touching the Chromium cache.
# Usage: ./scripts/clean-zig.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GUI_DIR="$(dirname "$SCRIPT_DIR")"

echo "Cleaning zig build artifacts..."
rm -rf "$GUI_DIR/zig-out/"
rm -rf "$GUI_DIR/.zig-cache/"
rm -rf "$GUI_DIR/macos/build/"
rm -rf ~/Library/Developer/Xcode/DerivedData/Ghostty-*

echo "Done. Chromium cache at chromium/src/out/ is untouched."
