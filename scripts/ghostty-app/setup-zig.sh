#!/usr/bin/env bash
# Issue 802 / Exp 2 — pin the zig version Ghostty 1.3.2-dev requires (0.15.2).
# The system zig (0.16.0) cannot build ghostty (build.zig uses a 0.15.x API and
# minimum_zig_version is 0.15.2). Downloads into vendor/toolchains/ (gitignored).
# Prints the path to the pinned zig binary on stdout.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
VER=0.15.2
DEST="$ROOT/vendor/toolchains/zig-aarch64-macos-$VER"
ZIG="$DEST/zig"
if [ ! -x "$ZIG" ]; then
  mkdir -p "$ROOT/vendor/toolchains"
  url="https://ziglang.org/download/$VER/zig-aarch64-macos-$VER.tar.xz"
  echo "downloading $url ..." >&2
  curl -sSL -m 300 -o "$DEST.tar.xz" "$url"
  tar xf "$DEST.tar.xz" -C "$ROOT/vendor/toolchains"
  rm -f "$DEST.tar.xz"
fi
"$ZIG" version >&2
echo "$ZIG"
