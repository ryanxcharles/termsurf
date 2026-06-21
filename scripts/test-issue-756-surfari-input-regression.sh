#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="${TERMSURF_GHOSTBOARD_APP:-$ROOT/ghostboard/macos/build/Debug/TermSurf.app}"
APP_BIN="$APP/Contents/MacOS/termsurf"
WEB="${TERMSURF_WEB:-$ROOT/target/debug/web}"
SURFARI="${TERMSURF_SURFARI:-$ROOT/target/debug/surfari}"
WEBKIT_DEBUG="${TERMSURF_WEBKIT_DEBUG:-$ROOT/webkit/src/WebKitBuild/Debug}"
WEBKIT_DYLIB="$ROOT/surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib"

missing=0

require_executable() {
  if [ ! -x "$1" ]; then
    printf 'missing executable: %s\n' "$1" >&2
    missing=1
  fi
}

require_path() {
  if [ ! -e "$1" ]; then
    printf 'missing path: %s\n' "$1" >&2
    missing=1
  fi
}

require_executable "$APP_BIN"
require_executable "$WEB"
require_executable "$SURFARI"
require_path "$WEBKIT_DEBUG/WebKit.framework"
require_path "$WEBKIT_DYLIB"

if [ "$missing" -ne 0 ]; then
  cat >&2 <<EOF

Build the focused Surfari input guard prerequisites:

  surfari/libtermsurf_webkit/build.sh
  cargo build -p surfari
  cargo build -p webtui
  cd ghostboard && zig build

Then rerun:

  scripts/test-issue-756-surfari-input-regression.sh
EOF
  exit 1
fi

exec "$ROOT/scripts/test-issue-756-real-app-surfari-input-routing.sh"
