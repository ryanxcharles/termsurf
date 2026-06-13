# Experiment 9: Trace Key Forwarding Path

## Description

Instrument the post-AppKit keyboard path so the failed focus-owned keyboard run
can be classified precisely.

Experiment 8 proved that external System Events keyboard input reaches Roastty's
focused `AXTextArea` and flows through `SurfaceView_AppKit.keyDown`,
`insertText`, and `keyAction`, but the typed command does not appear in the
terminal and does not execute. The missing evidence is below Swift AppKit:

- whether Swift's `roastty_surface_key` call returns `true` or `false`;
- whether the Rust `roastty_surface_key` ABI receives the expected text/key
  event;
- whether `write_encoded_key_event` encodes non-empty bytes;
- whether the surface is read-only;
- whether `termio_worker` is present;
- whether queueing bytes to the PTY succeeds or fails.

This experiment adds trace-only logging guarded by the existing
`ROASTTY_UI_KEY_TRACE_PATH` environment variable, rebuilds Roastty, repeats the
focus-owned keyboard marker attempt, and then records the exact loss point.

Per user instruction, this issue skips adversarial review.

## Changes

- `roastty/macos/Sources/Roastty/Surface View/SurfaceView_AppKit.swift`
  - Extend the existing UI key trace to log the boolean result returned from
    `roastty_surface_key` for both text and raw key events.
- `roastty/src/lib.rs`
  - Add a trace helper that appends to `ROASTTY_UI_KEY_TRACE_PATH`.
  - Log `roastty_surface_key` inputs/results.
  - Log `write_encoded_key_event` encoded bytes, read-only state,
    `termio_worker` presence, and queue-write success or failure.
- `issues/0804-roastty-gui-automation-readiness/09-trace-key-forwarding-path.md`
  - Record this experiment and result.
- `issues/0804-roastty-gui-automation-readiness/README.md`
  - Add Experiment 9 to the issue index.

The trace must remain inert when `ROASTTY_UI_KEY_TRACE_PATH` is unset.

## Verification

Run from the repo root. Store transcripts in `logs/` with the `issue804-exp9-`
prefix. Store the Roastty trace in `logs/`.

### 1. Format and Build

Commands:

```bash
cargo fmt --manifest-path roastty/Cargo.toml
scripts/roastty-app/build-roastty-kit.sh
cd roastty/macos
xcodebuild build \
  -project Roastty.xcodeproj \
  -scheme Roastty \
  -configuration Debug
cd ../..
```

Pass criteria:

- Rust formatting succeeds.
- The Rust kit rebuild succeeds.
- The macOS debug app rebuild succeeds.

### 2. Rerun Focus-Owned Keyboard Marker

Use the same guarded launch/focus/typing flow from Experiment 8 with fresh
paths:

```bash
mkdir -p logs
scripts/roastty-app/stop-app.sh || true
TRACE="$PWD/logs/issue804-exp9-key-forwarding-trace.log"
rm -f "$TRACE"
launchctl setenv ROASTTY_UI_KEY_TRACE_PATH "$TRACE"
launchctl setenv DISABLE_AUTO_UPDATE true
ROASTTY_PID="$(scripts/roastty-app/start-app.sh)"
printf 'ROASTTY_PID=%s\nTRACE=%s\n' "$ROASTTY_PID" "$TRACE" \
  > logs/issue804-exp9.env
swift scripts/roastty-app/list-windows.swift "$ROASTTY_PID"
scripts/roastty-app/screenshot.sh "$ROASTTY_PID" issue-804-exp9-before-keyboard
```

Then compute coordinates, activate, click, and prove focus:

```bash
source logs/issue804-exp9.env
LINE="$(swift scripts/roastty-app/list-windows.swift "$ROASTTY_PID" |
  awk '/layer=0/ { print; exit }')"
test -n "$LINE"
read -r X Y W H < <(printf '%s\n' "$LINE" |
  sed -E 's/.*bounds=\(([0-9.-]+),([0-9.-]+) ([0-9.-]+)x([0-9.-]+)\).*/\1 \2 \3 \4/' |
  awk '{ printf "%d %d %d %d\n", $1, $2, $3, $4 }')
FOCUS_X=$((X + 40))
FOCUS_Y=$((Y + 72))
SAFE_X=$((X + 120))
SAFE_Y=$((Y + 140))
printf 'X=%s\nY=%s\nW=%s\nH=%s\nFOCUS_X=%s\nFOCUS_Y=%s\nSAFE_X=%s\nSAFE_Y=%s\n' \
  "$X" "$Y" "$W" "$H" "$FOCUS_X" "$FOCUS_Y" "$SAFE_X" "$SAFE_Y" \
  > logs/issue804-exp9-coords.env
osascript -e 'tell application "System Events" to set frontmost of first process whose unix id is '"$ROASTTY_PID"' to true'
swift scripts/ghostty-app/inject.swift click "$SAFE_X" "$SAFE_Y" left 1
swift scripts/ghostty-app/inject.swift click "$FOCUS_X" "$FOCUS_Y" left 1
osascript -e 'delay 1.0'
osascript -e 'tell application "System Events" to unix id of first process whose frontmost is true' \
  > logs/issue804-exp9-frontmost-pid.txt
test "$(cat logs/issue804-exp9-frontmost-pid.txt)" = "$ROASTTY_PID"
```

Finally type only after the PID guard passes:

```bash
source logs/issue804-exp9.env
source logs/issue804-exp9-coords.env
TS=/tmp/termsurf-issue804-exp9-system-events
mkdir -p "$TS"
rm -f "$TS/marker.txt"
: > "$TRACE"
printf '/bin/echo ISSUE804_EXP9_SYSTEM_EVENTS > %s/marker.txt' "$TS" \
  > "$TS/type.txt"
osascript -e 'tell application "System Events" to keystroke (read POSIX file "'"$TS"'/type.txt")'
osascript -e 'tell application "System Events" to key code 36'
osascript -e 'delay 1.0'
cat "$TRACE"
cat "$TS/marker.txt"
scripts/roastty-app/screenshot.sh "$ROASTTY_PID" issue-804-exp9-after-keyboard
```

Pass criteria:

- The immediate frontmost PID equals the Roastty PID before typing.
- The trace includes Swift `keyAction result=...` entries.
- The trace includes Rust `roastty_surface_key` and `write_encoded_key_event`
  entries.
- If `marker.txt` exists, the keyboard blocker is fixed by the rebuild or trace
  changes and the issue can move to final readiness validation.
- If `marker.txt` does not exist, the trace identifies the exact loss point.

### 3. Cleanup

Commands:

```bash
source logs/issue804-exp9.env || true
scripts/roastty-app/stop-app.sh || true
launchctl unsetenv ROASTTY_UI_KEY_TRACE_PATH || true
launchctl unsetenv DISABLE_AUTO_UPDATE || true
pgrep -fl 'Roastty.app/Contents/MacOS/roastty' || true
```

Pass criteria:

- Cleanup leaves no debug Roastty process running.
- Environment variables used for the app launch are removed from launchd.

Overall result:

- **Pass** if the marker file is created and the trace confirms successful
  forwarding.
- **Partial** if the marker still fails but the trace identifies the failing
  layer.
- **Fail** if the trace instrumentation cannot be built or does not emit under
  `ROASTTY_UI_KEY_TRACE_PATH`.
