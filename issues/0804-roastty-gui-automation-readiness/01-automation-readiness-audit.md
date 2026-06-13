# Experiment 1: Automation Readiness Audit

## Description

Prove that this macOS VM can automatically build, launch, drive, observe, and
shut down the actual Roastty GUI window. This experiment is an automation
readiness audit only: it should not change Roastty product behavior. Any code
changes are limited to test harness or script fixes needed to make the audit
repeatable.

The experiment should answer four questions:

1. Can the repo build and launch the current Roastty GUI in this VM?
2. Can automation make Roastty frontmost and capture its real window contents?
3. Can automation inject keyboard and mouse input into the Roastty window and
   prove the app received it?
4. If macOS blocks any capability, which permission is missing and which process
   must be granted access in System Settings?

The experiment stops at the first permission blocker that prevents a capability
from being tested. In that case, record the exact blocker and the permission the
user must grant, then rerun the same experiment after permission is granted.

## Changes

Expected files:

- `issues/0804-roastty-gui-automation-readiness/01-automation-readiness-audit.md`
  - Record commands, logs, permission findings, results, and conclusion.
- `issues/0804-roastty-gui-automation-readiness/README.md`
  - Track the experiment status in the issue index.

Allowed only if the audit exposes a harness blocker:

- `scripts/roastty-app/*`
  - Fix Roastty-specific automation helpers for launch, stop, screenshots,
    window lookup, click, drag, scroll, or typing.
- `scripts/ghostty-app/*`
  - Reuse or adapt existing Ghostty automation helpers only when a helper is
    generic and the change does not break Ghostty workflows.

No Roastty product code should change in this experiment unless the audit proves
that an automation-critical bug is in the app rather than the harness. If that
happens, record the finding and stop before implementing the product fix.

## Verification

Run the audit from the repo root. Store artifacts outside the repo unless a
small text log is intentionally added to the experiment result.

### 1. Environment and Permission Preflight

Commands:

```bash
git status --short
git config --global user.name
git config --global user.email
swift -e 'import ApplicationServices; print(AXIsProcessTrusted())'
osascript -e 'tell application "System Events" to count processes'
mkdir -p /tmp/termsurf-issue804-exp1
```

Pass criteria:

- Git identity is `Max Commits <maxcommits@ryanxcharles.com>`.
- Accessibility preflight prints `true`.
- If Accessibility prints `false`, stop and record that the app hosting this
  agent needs System Settings -> Privacy & Security -> Accessibility permission.
- The Apple Events preflight prints a process count. If it fails with `-1743` or
  a "not authorized to send Apple events" message, stop and record that the
  responsible host app needs System Settings -> Privacy & Security -> Automation
  permission to control `System Events`.
- Screen Recording does not have a reliable direct preflight here; the first
  window screenshot is the Screen Recording preflight.
- CGEvent posting and event-tap readiness are checked by the keyboard and mouse
  injection steps. If they fail despite Accessibility being trusted, classify
  the failure by the exact macOS error or visible TCC prompt.

### 2. Build and Launch Roastty

Commands:

```bash
scripts/roastty-app/stop-app.sh || true
cd roastty && macos/build.nu --action build
cd ..
ROASTTY_PID="$(scripts/roastty-app/start-app.sh)"
export ROASTTY_PID
```

Then identify the app PID and window:

```bash
pgrep -fl 'Roastty.app/Contents/MacOS/roastty'
swift scripts/roastty-app/list-windows.swift "$ROASTTY_PID"
```

Pass criteria:

- Build succeeds.
- Exactly one expected Roastty app process is running.
- At least one visible Roastty window is listed.

Partial criteria:

- Build succeeds but launch or window discovery fails because a harness path,
  bundle identifier, or app name is stale. Fix the harness if the correction is
  clearly Roastty-specific and rerun.

### 3. Foreground Activation and Screenshot

Activate Roastty, wait for focus, and capture a window screenshot:

```bash
osascript -e 'tell application "System Events" to set frontmost of first process whose unix id is '"$ROASTTY_PID"' to true'
sleep 1
scripts/roastty-app/screenshot.sh "$ROASTTY_PID" issue-804-exp1-window
```

Pass criteria:

- Roastty becomes the frontmost app.
- Screenshot command writes a non-empty image file.
- The image visibly contains the Roastty window, not the desktop or another app.

Permission blocker:

- If screenshot output is blank, missing, or replaced by a privacy placeholder,
  record that the responsible process needs System Settings -> Privacy &
  Security -> Screen & System Audio Recording permission.
- If the activation command fails with Apple Events error `-1743`, record that
  the responsible host app needs System Settings -> Privacy & Security ->
  Automation permission to control `System Events`.

### 4. Keyboard Injection Oracle

Use a deterministic file oracle rather than visual inspection alone. Bring
Roastty frontmost, bootstrap a predictable shell, type a command that writes a
marker file, press Return, and verify the marker file appears. Keyboard text
injection currently lives in the generic Issue 802 helper
`scripts/ghostty-app/inject.swift`, so use it directly unless this experiment
adds a Roastty wrapper.

Commands:

```bash
rm -f /tmp/termsurf-issue804-exp1/keyboard.txt
osascript -e 'tell application "System Events" to set frontmost of first process whose unix id is '"$ROASTTY_PID"' to true'

printf 'exec bash --norc --noprofile' > /tmp/termsurf-issue804-exp1/type.txt
swift scripts/ghostty-app/inject.swift type /tmp/termsurf-issue804-exp1/type.txt
swift scripts/ghostty-app/inject.swift key 36
sleep 1

printf 'printf "ISSUE804_KEYBOARD\\n" > /tmp/termsurf-issue804-exp1/keyboard.txt' > /tmp/termsurf-issue804-exp1/type.txt
swift scripts/ghostty-app/inject.swift type /tmp/termsurf-issue804-exp1/type.txt
swift scripts/ghostty-app/inject.swift key 36
sleep 1
cat /tmp/termsurf-issue804-exp1/keyboard.txt
```

Pass criteria:

- The file exists and contains exactly `ISSUE804_KEYBOARD`.

Partial criteria:

- If the typed command reaches the window but the default shell state makes the
  oracle unreliable, add or adapt a Roastty-specific helper that launches a
  known shell/test session and rerun the keyboard step.

Permission blocker:

- If macOS refuses synthetic keyboard events, record that the responsible
  process needs System Settings -> Privacy & Security -> Accessibility
  permission, and possibly Input Monitoring if the specific API path requires
  it.
- If the activation command fails with Apple Events error `-1743`, record that
  the responsible host app needs System Settings -> Privacy & Security ->
  Automation permission to control `System Events`.

### 5. Mouse Injection Oracle

Use window bounds from the helper scripts to target coordinates inside the
Roastty content area. Start `byteprobe.py` inside Roastty with mouse reporting
enabled, then exercise click, drag, shift-click, and scroll. The Roastty helper
scripts take global screen coordinates, not `--window` arguments.

Commands:

```bash
rm -f /tmp/termsurf-issue804-exp1/mouse-bytes.log
printf 'python3 /Users/astrohacker/dev/termsurf/scripts/ghostty-app/byteprobe.py /tmp/termsurf-issue804-exp1/mouse-bytes.log mouse' > /tmp/termsurf-issue804-exp1/type.txt
osascript -e 'tell application "System Events" to set frontmost of first process whose unix id is '"$ROASTTY_PID"' to true'
swift scripts/ghostty-app/inject.swift type /tmp/termsurf-issue804-exp1/type.txt
swift scripts/ghostty-app/inject.swift key 36
sleep 1

swift scripts/roastty-app/list-windows.swift "$ROASTTY_PID"
IFS=$'\t' read -r WID X Y W H < <(swift scripts/roastty-app/winid.swift "$ROASTTY_PID")
CX=$((X + W / 2))
CY=$((Y + H / 2))
swift scripts/roastty-app/click.swift "$CX" "$CY" 1
swift scripts/roastty-app/drag.swift "$((CX - 80))" "$CY" "$((CX + 80))" "$CY" 12
swift scripts/roastty-app/shiftclick.swift "$CX" "$CY" 1
swift scripts/roastty-app/scroll.swift "$CX" "$CY" -5
scripts/roastty-app/screenshot.sh "$ROASTTY_PID" issue-804-exp1-mouse
pkill -TERM -f '/Users/astrohacker/dev/termsurf/scripts/ghostty-app/byteprobe.py /tmp/termsurf-issue804-exp1/mouse-bytes.log' || true
cat /tmp/termsurf-issue804-exp1/mouse-bytes.log
```

Pass criteria:

- Commands complete without macOS automation errors.
- Roastty remains frontmost.
- A post-mouse screenshot captures the window after the actions.
- The byte log starts with `# byteprobe start modes='mouse'` and contains SGR
  mouse-report bytes for each event class:
  - click: left button press and release, such as `ESC [ < 0;... M` and
    `ESC [ < 0;... m`;
  - drag: motion reports while the left button is held, such as
    `ESC [ < 32;... M`;
  - shift-click: shift-modified left button press/release, such as SGR button
    values with the shift modifier bit set;
  - scroll: wheel reports, such as `ESC [ < 64;... M` or `ESC [ < 65;... M`.

Partial criteria:

- If helper coordinate assumptions are wrong, fix the harness to derive
  coordinates from the live Roastty window bounds and rerun.
- If the byteprobe starts but one class of mouse event is missing, classify the
  missing event type precisely and fix only the relevant helper or permission
  blocker.
- If the activation command fails with Apple Events error `-1743`, record that
  the responsible host app needs System Settings -> Privacy & Security ->
  Automation permission to control `System Events`.

### 6. Live A/B Harness Smoke Test

Run one existing Roastty/Ghostty visual comparison smoke test if the baseline
Ghostty debug app is available. This checks whether the higher-level automation
used by previous GUI work is still operational on the new VM.

Command:

```bash
scripts/roastty-app/live-ab-smoke.sh --recipe smoke --comparison-region content --max-mismatch-ratio 1 --max-mean-channel-delta 255
```

Pass criteria:

- The harness launches both apps, captures comparable screenshots, and exits
  successfully.

Partial criteria:

- If Ghostty is not built or not present, record that the baseline app is
  missing. Do not treat that as a Roastty automation failure.

### 7. XCTest UI Automation Availability

Run a minimal Roastty XCTest path to verify Xcode's UI automation plumbing is
usable in this VM. Prefer the narrowest UI test target or test case available
after inspecting `roastty/macos/RoasttyUITests/`.

Commands:

```bash
find roastty/macos/RoasttyUITests -maxdepth 2 -type f | sort
cd roastty/macos
xcodebuild test \
  -project Roastty.xcodeproj \
  -scheme Roastty \
  -testPlan Roastty \
  -destination 'platform=macOS' \
  -only-testing:RoasttyUITests/RoasttyTerminalOutputUITests/testTerminalOutputIsVisibleToUIAutomation
cd ../..
```

Pass criteria:

- Xcode starts the UI automation session without a host permission error.
- At least one Roastty UI test runs to completion.

Partial criteria:

- If UI tests are absent, stale, or too broad for this readiness audit, record
  that XCTest can be compiled but still needs a focused smoke test.

Permission blocker:

- If macOS blocks UI automation, record the exact System Settings permission and
  responsible process shown by the failure.

### 8. Cleanup

Commands:

```bash
scripts/roastty-app/stop-app.sh
pgrep -fl 'Roastty.app/Contents/MacOS/roastty' || true
```

Pass criteria:

- No test Roastty process remains running.

## Design Review

Codex reviewer `Schrodinger` reviewed the first draft and returned
`REQUEST_CHANGES`.

Findings addressed in this design:

- Swift helpers must be invoked with `swift`.
- Missing helper names were replaced with existing activation and
  `scripts/ghostty-app/inject.swift` commands.
- `list-windows.swift` now receives the Roastty PID captured from
  `start-app.sh`.
- Mouse helper calls now use positional global coordinates derived from
  `winid.swift`.
- Mouse verification now uses `byteprobe.py` in mouse mode as a deterministic
  receipt oracle.
- Shift-click and focused XCTest UI automation checks are included.
- Screen Recording and event-posting readiness are classified explicitly.
- Process checks are scoped to the debug Roastty app path.

Codex reviewer `Archimedes` reviewed the revised draft and returned
`REQUEST_CHANGES`.

Findings addressed in this design:

- Apple Events / Automation preflight and `-1743` blocker handling are included
  for the `osascript` / `System Events` path.
- Mouse pass criteria now require separate byte-level evidence for click, drag,
  shift-click, and scroll event classes.

Codex reviewer `Feynman` reviewed the twice-revised draft and returned `APPROVE`
with no blocking findings.

## Result

Not run yet.

## Conclusion

Pending.
