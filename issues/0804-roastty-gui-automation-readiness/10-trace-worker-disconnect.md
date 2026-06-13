# Experiment 10: Trace Worker Disconnect

## Description

Trace the terminal worker lifecycle around app launch and first keyboard input,
then fix the smallest proven cause of `CommandDisconnected`.

Experiment 9 showed that keyboard events reach Rust and encode correctly, but
`write_encoded_key_event` fails because `TermioWorker::queue_write` returns
`CommandDisconnected`. That means the `TermioWorker` command sender is still
stored on the surface while the worker thread's command receiver has exited.

This experiment instruments the lifecycle around that stale worker handle:

- `Surface::start_termio` command selection, spawn success, initial input, and
  worker assignment;
- `TermioWorker::spawn`;
- `run_termio_worker` exit reasons, including pump EOF, child exit, pump errors,
  disconnected event receiver, disconnected command sender, and shutdown;
- `Surface::drain_termio_events` and `Surface::apply_termio_event`;
- surface state before key writes when the worker is disconnected.

If the trace proves a simple bug, such as a terminal worker that exited but was
not cleared from the surface, this experiment may apply the smallest fix and
rerun the external keyboard marker oracle.

Per user instruction, this issue skips adversarial review.

## Changes

- `roastty/src/lib.rs`
  - Make the existing trace helper visible within the crate.
  - Trace `Surface::start_termio`, `drain_termio_events`, and
    `apply_termio_event`.
  - If the trace proves a stale worker-handle bug, clear or restart the stale
    worker in the smallest safe place.
- `roastty/src/termio.rs`
  - Trace `TermioWorker::spawn`, worker loop exit reasons, command drain events,
    pump EOF/child exit, and pump errors.
- `issues/0804-roastty-gui-automation-readiness/10-trace-worker-disconnect.md`
  - Record this experiment and result.
- `issues/0804-roastty-gui-automation-readiness/README.md`
  - Add Experiment 10 to the issue index.

Tracing must remain inert unless `ROASTTY_UI_KEY_TRACE_PATH` is set.

## Verification

Run from the repo root. Store transcripts in `logs/` with the `issue804-exp10-`
prefix.

### 1. Build

Commands:

```bash
cargo fmt --manifest-path roastty/Cargo.toml
scripts/roastty-app/build-roastty-kit.sh
cd roastty/macos
xcodebuild build \
  -project Roastty.xcodeproj \
  -scheme Roastty \
  -configuration Debug \
  -derivedDataPath build
cd ../..
```

Pass criteria:

- Rust formatting succeeds.
- The Rust kit rebuild succeeds.
- The local debug app at `roastty/macos/build/Build/Products/Debug/Roastty.app`
  rebuilds.

### 2. Launch and Observe Worker Lifecycle Before Typing

Commands:

```bash
mkdir -p logs
scripts/roastty-app/stop-app.sh \
  'roastty/macos/build/(Build/Products/Debug|Debug)/Roastty.app/Contents/MacOS/roastty' || true
TRACE="$PWD/logs/issue804-exp10-worker-trace.log"
rm -f "$TRACE"
launchctl setenv ROASTTY_UI_KEY_TRACE_PATH "$TRACE"
launchctl setenv DISABLE_AUTO_UPDATE true
ROASTTY_APP="$PWD/roastty/macos/build/Build/Products/Debug/Roastty.app" \
  ROASTTY_PID="$(ROASTTY_APP="$PWD/roastty/macos/build/Build/Products/Debug/Roastty.app" \
  scripts/roastty-app/start-app.sh)"
printf 'ROASTTY_PID=%s\nTRACE=%s\nROASTTY_APP=%s\n' \
  "$ROASTTY_PID" "$TRACE" \
  "$PWD/roastty/macos/build/Build/Products/Debug/Roastty.app" \
  > logs/issue804-exp10.env
sleep 2
swift scripts/roastty-app/list-windows.swift "$ROASTTY_PID"
scripts/roastty-app/screenshot.sh "$ROASTTY_PID" issue-804-exp10-before-keyboard
cat "$TRACE"
```

Pass criteria:

- Roastty launches and shows the terminal window.
- The trace explains whether the worker is still running before keyboard input.
- If the worker already exited, the trace identifies the reason before any
  typing attempt.

### 3. Rerun the Guarded Marker Attempt

Repeat the PID-guarded coordinate, click, and System Events typing flow from
Experiment 9 with `/tmp/termsurf-issue804-exp10-system-events`.

Pass criteria:

- The frontmost PID equals the Roastty PID before typing.
- If the marker file exists, the keyboard blocker is fixed.
- If the marker file does not exist, the trace now identifies why the worker
  disconnected and whether the surface recovered, cleared, or retained the stale
  worker.

### 4. Cleanup

Commands:

```bash
source logs/issue804-exp10.env || true
scripts/roastty-app/screenshot.sh "$ROASTTY_PID" issue-804-exp10-after-keyboard || true
scripts/roastty-app/stop-app.sh \
  'roastty/macos/build/(Build/Products/Debug|Debug)/Roastty.app/Contents/MacOS/roastty' || true
launchctl unsetenv ROASTTY_UI_KEY_TRACE_PATH || true
launchctl unsetenv DISABLE_AUTO_UPDATE || true
pgrep -fl 'Roastty.app/Contents/MacOS/roastty' || true
```

Pass criteria:

- Cleanup leaves no debug Roastty process running.
- Environment variables used for the app launch are removed from launchd.

Overall result:

- **Pass** if the worker disconnect cause is fixed and the marker file is
  created.
- **Partial** if the marker still fails but the worker disconnect cause is
  identified precisely.
- **Fail** if the worker lifecycle trace cannot be built or does not emit under
  `ROASTTY_UI_KEY_TRACE_PATH`.
