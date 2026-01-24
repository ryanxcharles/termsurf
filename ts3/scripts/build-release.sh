#!/bin/bash
# Build TermSurf 3.0 in Release mode
#
# Usage:
#   ./scripts/build-release.sh [--clean] [--open]
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
    rm -rf "$REPO_DIR/target/release"
    echo "Cleared target/release"
fi

echo "=== Building TermSurf 3.0 (Release) ==="

cd "$REPO_DIR"

# Build binaries
echo "Building web coordinator..."
cargo build -p termsurf-web --release

# TODO: Build browser subprocess when implemented
# echo "Building browser subprocess..."
# cargo build -p termsurf-browser --release

# TODO: Build GUI when implemented
# echo "Building GUI..."
# cargo build -p termsurf-gui --features cef --release

# TODO: CEF framework copying (when browser subprocess is ready)
# TODO: App bundling (when GUI is ready)

echo ""
echo "=== Release Build Complete ==="
echo "Binaries:"
echo "  $REPO_DIR/target/release/web"
echo ""

# Open if requested
if [ "$OPEN" = true ]; then
    echo "Running web binary..."
    "$REPO_DIR/target/release/web"
fi
