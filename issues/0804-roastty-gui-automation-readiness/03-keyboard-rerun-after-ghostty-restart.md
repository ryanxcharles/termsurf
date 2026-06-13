# Experiment 3: Keyboard Rerun After Ghostty Restart

## Description

Retest external keyboard automation after fully restarting Ghostty, the host app
for this Codex session. Experiment 2 showed that Accessibility permission was
trusted but external keyboard events still did not reach the frontmost Roastty
window. This experiment checks whether restarting the permission-bearing host
process makes the granted Accessibility permission effective for event delivery.

Per user instruction, this issue skips adversarial review.

## Changes

- `issues/0804-roastty-gui-automation-readiness/03-keyboard-rerun-after-ghostty-restart.md`
  - Record the focused keyboard rerun and result.
- `issues/0804-roastty-gui-automation-readiness/README.md`
  - Add Experiment 3 to the issue index.

No product code or harness code changed in this experiment.

## Verification

Commands were run from the repo root. Transcripts were written to `logs/`.

### Preflight

```bash
git status --short
swift -e 'import ApplicationServices; print(AXIsProcessTrusted())'
osascript -e 'tell application "System Events" to count processes'
```

Expected:

- Worktree starts clean.
- Accessibility prints `true`.
- Apple Events to `System Events` returns a process count.
- Process ancestry confirms the session is running under the restarted
  `/Applications/Ghostty.app/Contents/MacOS/ghostty`.

### Launch Roastty

```bash
scripts/roastty-app/stop-app.sh || true
ROASTTY_PID="$(scripts/roastty-app/start-app.sh)"
pgrep -fl 'Roastty.app/Contents/MacOS/roastty'
swift scripts/roastty-app/list-windows.swift "$ROASTTY_PID"
swift scripts/roastty-app/winid.swift "$ROASTTY_PID"
osascript -e 'tell application "System Events" to set frontmost of first process whose unix id is '"$ROASTTY_PID"' to true'
osascript -e 'tell application "System Events" to name of first process whose frontmost is true'
scripts/roastty-app/screenshot.sh "$ROASTTY_PID" issue-804-exp3-before-keyboard
```

Expected:

- Debug Roastty launches.
- The visible `800x632` Roastty terminal window is selected.
- Roastty is frontmost.
- Screenshot captures the real window.

### CGEvent Keyboard Injection

```bash
swift scripts/ghostty-app/inject.swift key 49
swift scripts/ghostty-app/inject.swift key 51
printf 'exec bash --norc --noprofile' > /tmp/termsurf-issue804-exp3/type.txt
swift scripts/ghostty-app/inject.swift type /tmp/termsurf-issue804-exp3/type.txt
swift scripts/ghostty-app/inject.swift key 36
printf 'printf "ISSUE804_CGEVENT\n" > /tmp/termsurf-issue804-exp3/keyboard-cgevent.txt' > /tmp/termsurf-issue804-exp3/type.txt
swift scripts/ghostty-app/inject.swift type /tmp/termsurf-issue804-exp3/type.txt
swift scripts/ghostty-app/inject.swift key 36
cat /tmp/termsurf-issue804-exp3/keyboard-cgevent.txt
```

Expected:

- The marker file exists and contains `ISSUE804_CGEVENT`.

### System Events Keyboard Injection

```bash
osascript -e 'tell application "System Events" to key code 49'
osascript -e 'tell application "System Events" to key code 51'
printf 'exec bash --norc --noprofile' > /tmp/termsurf-issue804-exp3/system-events-type.txt
osascript -e 'tell application "System Events" to keystroke (read POSIX file "/tmp/termsurf-issue804-exp3/system-events-type.txt")'
osascript -e 'tell application "System Events" to key code 36'
printf 'printf "ISSUE804_SYSTEM_EVENTS\n" > /tmp/termsurf-issue804-exp3/keyboard-system-events.txt' > /tmp/termsurf-issue804-exp3/system-events-type.txt
osascript -e 'tell application "System Events" to keystroke (read POSIX file "/tmp/termsurf-issue804-exp3/system-events-type.txt")'
osascript -e 'tell application "System Events" to key code 36'
cat /tmp/termsurf-issue804-exp3/keyboard-system-events.txt
```

Expected:

- The marker file exists and contains `ISSUE804_SYSTEM_EVENTS`.

### Investigation and Cleanup

```bash
scripts/roastty-app/screenshot.sh "$ROASTTY_PID" issue-804-exp3-after-keyboard
osascript -e 'tell application "System Events" to name of first process whose frontmost is true'
osascript -e 'tell application "System Events" to tell (first process whose unix id is '"$ROASTTY_PID"') to get {name, frontmost, visible, enabled}'
ioreg -l -w 0 | rg -i 'SecureInput|SecureEventInput|kCGSSessionSecureInputPID|CGSSessionSecureInput' || true
log show --predicate 'subsystem == "com.apple.TCC" AND (eventMessage CONTAINS[c] "keyboard" OR eventMessage CONTAINS[c] "listen" OR eventMessage CONTAINS[c] "accessibility" OR eventMessage CONTAINS[c] "input")' --last 10m --style compact
scripts/roastty-app/stop-app.sh
pgrep -fl 'Roastty.app/Contents/MacOS/roastty' || true
```

Expected:

- If keyboard injection still fails, the investigation records frontmost state,
  visible window state, and any Secure Input or TCC signal.
- Cleanup leaves no debug Roastty process running.

## Result

**Result:** Partial

Preflight passed:

- `logs/issue804-exp3-preflight.log`
  - `AXIsProcessTrusted()` printed `true`.
  - Apple Events to `System Events` returned process count `61`.
  - Process ancestry confirmed a restarted Ghostty-hosted session:
    `/Applications/Ghostty.app/Contents/MacOS/ghostty -> login -> zsh -> codex -> zsh`.

Launch and window capture passed:

- `logs/issue804-exp3-launch.log`
  - Debug Roastty launched as PID `91470`.
  - The visible Roastty window was listed as
    `id=212 layer=0 bounds=(489,161 800x632) name="👻"`.
  - `winid.swift` selected window `212`.
  - Roastty was frontmost.
  - Screenshot capture wrote
    `/Users/astrohacker/.cache/termsurf/shots/issue-804-exp3-before-keyboard-20260613-132300.png`.

CGEvent keyboard injection still failed:

- `logs/issue804-exp3-keyboard-cgevent.log`
  - `inject.swift` warmup, text, and Return commands returned without a tool
    error.
  - `/tmp/termsurf-issue804-exp3/keyboard-cgevent.txt` was not created.

System Events keyboard injection still failed:

- `logs/issue804-exp3-keyboard-system-events.log`
  - `System Events` warmup, `keystroke`, and Return commands returned without a
    tool error.
  - `/tmp/termsurf-issue804-exp3/keyboard-system-events.txt` was not created.

Investigation found no obvious host-state explanation:

- `logs/issue804-exp3-keyboard-investigation.log`
  - Post-attempt screenshot was captured:
    `/Users/astrohacker/.cache/termsurf/shots/issue-804-exp3-after-keyboard-20260613-132343.png`.
  - Roastty remained frontmost.
  - System Events reported Roastty as `frontmost=true` and `visible=true`.
- `logs/issue804-exp3-input-blocker-investigation.log`
  - No Secure Input marker was reported by the checked `ioreg` query.
  - No relevant TCC denial appeared in the checked log query.

Cleanup passed:

- `logs/issue804-exp3-cleanup.log`
  - Stopped debug Roastty PID `91470`.
  - No debug Roastty process remained.

## Conclusion

Restarting Ghostty after granting Accessibility did not make external keyboard
events reach Roastty. At this point, both tested external keyboard paths fail in
the same way: they return successfully, Roastty is frontmost and visible, but no
text reaches the terminal and no marker-file oracle is created.

The issue is now narrower: the VM can run the app, capture its window, and run
XCTest UI automation, but cannot yet drive the live Roastty terminal by external
keyboard synthesis from the agent host. The next useful experiment should avoid
retesting the same permission hypothesis and instead instrument or probe where
keyboard events are lost: host event posting, macOS delivery to the app,
Roastty/AppKit first responder handling, or Roastty terminal input handling.
