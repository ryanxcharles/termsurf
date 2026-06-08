#!/usr/bin/env bash
# Issue 802 / Exp 4 — capture a single app window (Space/occlusion-independent) via
# `screencapture -l<id>`, with the window id resolved by winid.swift.
#
#   screenshot.sh [--list] <owner-name|bundle-id|pid> [out-name]
#
# Screenshots are written OUTSIDE the repo (policy): $TERMSURF_SHOT_DIR, default
# ~/.cache/termsurf/shots. Prints the saved PNG path on stdout.
set -euo pipefail
DIR="$(cd "$(dirname "$0")" && pwd)"
SHOT_DIR="${TERMSURF_SHOT_DIR:-$HOME/.cache/termsurf/shots}"
SWIFT="$(command -v swift || echo /usr/bin/swift)"

if [ "${1:-}" = "--list" ]; then
  shift
  TS_LIST=1 "$SWIFT" "$DIR/winid.swift" "${1:-}"
  exit 0
fi

TARGET="${1:-}"
[ -n "$TARGET" ] || { echo "usage: screenshot.sh [--list] <owner-name|bundle-id|pid> [out-name]" >&2; exit 2; }
OUT="$(basename "${2:-shot}" .png)"   # basename: never let out-name escape SHOT_DIR
mkdir -p "$SHOT_DIR"
PNG="$SHOT_DIR/${OUT}-$(date +%Y%m%d-%H%M%S).png"

# Resolve the window id + point bounds.
if ! IFS=$'\t' read -r WID X Y W H < <("$SWIFT" "$DIR/winid.swift" "$TARGET"); then
  echo "no window found for: $TARGET" >&2; exit 1
fi
[ -n "${WID:-}" ] || { echo "no window found for: $TARGET" >&2; exit 1; }

# Capture just that window (no shadow, no sound), independent of Space/layering.
screencapture -x -o -l"$WID" "$PNG"

# Validate: pixel dims should be the window bounds × the display backing scale.
PW="$(sips -g pixelWidth  "$PNG" 2>/dev/null | awk '/pixelWidth/{print $2}')"
PH="$(sips -g pixelHeight "$PNG" 2>/dev/null | awk '/pixelHeight/{print $2}')"
echo "id=$WID bounds=${W}x${H}pt captured=${PW}x${PH}px -> $PNG" >&2
echo "$PNG"
