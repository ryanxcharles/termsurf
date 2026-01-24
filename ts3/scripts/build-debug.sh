#!/bin/bash
# Build TermSurf 3.0 in Debug mode
#
# Usage:
#   ./scripts/build-debug.sh [--clean] [--open]
#
# Flags:
#   --clean  Clear build caches and do a fresh build
#   --open   Run the web binary after building

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_DIR="$(dirname "$SCRIPT_DIR")"

# Parse flags
CLEAN=false
OPEN=false
for arg in "$@"; do
    case $arg in
        --clean) CLEAN=true ;;
        --open) OPEN=true ;;
    esac
done

# Clean if requested
if [ "$CLEAN" = true ]; then
    echo "=== Cleaning build caches ==="
    rm -rf "$REPO_DIR/target/debug"
    echo "Cleared target/debug"
fi

echo "=== Building TermSurf 3.0 (Debug) ==="

cd "$REPO_DIR"

# Build everything
echo "Building workspace..."
cargo build

echo ""
echo "=== Debug Build Complete ==="
echo "Binaries:"
echo "  $REPO_DIR/target/debug/wezterm-gui  (terminal)"
echo "  $REPO_DIR/target/debug/wezterm      (CLI)"
echo "  $REPO_DIR/target/debug/web          (web coordinator)"
echo ""

# Open if requested
if [ "$OPEN" = true ]; then
    echo "Running terminal..."
    "$REPO_DIR/target/debug/wezterm-gui"
fi
