#!/usr/bin/env bash
# Issue 802 / Exp 14 — cleanly stop the debug Roastty app this harness spawned. Kills by
# PID, scoped to the build-output path, with SIGKILL so there is NO graceful-quit
# confirmation dialog. Touches nothing but the debug build under roastty/macos/build/ —
# never an installed/stable Roastty (there is none) or any other app.
set -uo pipefail
SCOPE="${1:-roastty/macos/build/.*Roastty.app/Contents/MacOS/roastty}"
PIDS=$(pgrep -f "$SCOPE" || true)
if [ -n "$PIDS" ]; then
  echo "killing debug Roastty PIDs: $PIDS"
  kill -9 $PIDS
else
  echo "no debug Roastty running"
fi
