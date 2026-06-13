# Experiment 5: Rerun After Input Monitoring Grant

## Description

Retest the two external keyboard paths after the user granted macOS Input
Monitoring permission and restarted Ghostty plus Codex.

Experiments 2, 3, and 4 proved that:

- Accessibility is trusted for the Ghostty-hosted agent process;
- Automation permission from Ghostty to System Events is enabled;
- Roastty can receive keyboard input through XCTest;
- launch-time bootstrap command delivery works;
- external System Events and CGEvent keyboard posting still did not reach the
  live Roastty terminal.

The user has now enabled Input Monitoring and restarted the responsible host
process. This experiment checks whether that permission changes external
keyboard delivery.

Per user instruction, this issue skips adversarial review.

## Changes

- `issues/0804-roastty-gui-automation-readiness/05-rerun-after-input-monitoring-grant.md`
  - Record the focused permission rerun and result.
- `issues/0804-roastty-gui-automation-readiness/README.md`
  - Add Experiment 5 to the issue index.

No product code or harness code should change in this experiment.

## Verification

Run from the repo root. Store transcripts in `logs/` with the `issue804-exp5-`
prefix.

### 1. Preflight

Commands:

```bash
git status --short
swift -e 'import ApplicationServices; print(AXIsProcessTrusted())'
osascript -e 'tell application "System Events" to count processes'
ps -o pid,ppid,comm -p $$ -p $(ps -o ppid= -p $$)
```

Pass criteria:

- Accessibility prints `true`.
- Apple Events to System Events works.
- Process ancestry confirms this is the restarted Ghostty-hosted session.

### 2. Launch and Focus Roastty

Commands:

```bash
scripts/roastty-app/stop-app.sh || true
ROASTTY_PID="$(scripts/roastty-app/start-app.sh)"
export ROASTTY_PID
pgrep -fl 'Roastty.app/Contents/MacOS/roastty'
swift scripts/roastty-app/list-windows.swift "$ROASTTY_PID"
swift scripts/roastty-app/winid.swift "$ROASTTY_PID"
osascript -e 'tell application "System Events" to set frontmost of first process whose unix id is '"$ROASTTY_PID"' to true'
osascript -e 'tell application "System Events" to name of first process whose frontmost is true'
scripts/roastty-app/screenshot.sh "$ROASTTY_PID" issue-804-exp5-before-keyboard
```

Pass criteria:

- Debug Roastty launches.
- The visible `800x632` terminal window is selected.
- Roastty is frontmost.
- The screenshot captures the actual Roastty terminal.

### 3. System Events Keyboard

Commands:

```bash
TS=/tmp/termsurf-issue804-exp5-system-events
mkdir -p "$TS"
rm -f "$TS/marker.txt"
osascript -e 'tell application "System Events" to set frontmost of first process whose unix id is '"$ROASTTY_PID"' to true'
osascript -e 'tell application "System Events" to key code 49'
printf 'exec bash --norc --noprofile' > "$TS/type.txt"
osascript -e 'tell application "System Events" to keystroke (read POSIX file "'"$TS"'/type.txt")'
osascript -e 'tell application "System Events" to key code 36'
printf 'printf "ISSUE804_EXP5_SYSTEM_EVENTS\n" > '"$TS"'/marker.txt' > "$TS/type.txt"
osascript -e 'tell application "System Events" to keystroke (read POSIX file "'"$TS"'/type.txt")'
osascript -e 'tell application "System Events" to key code 36'
cat "$TS/marker.txt"
```

Pass criteria:

- `marker.txt` exists and contains `ISSUE804_EXP5_SYSTEM_EVENTS`.

### 4. CGEvent Keyboard

Commands:

```bash
TS=/tmp/termsurf-issue804-exp5-cgevent
mkdir -p "$TS"
rm -f "$TS/marker.txt"
osascript -e 'tell application "System Events" to set frontmost of first process whose unix id is '"$ROASTTY_PID"' to true'
swift scripts/ghostty-app/inject.swift key 49
printf 'exec bash --norc --noprofile' > "$TS/type.txt"
swift scripts/ghostty-app/inject.swift type "$TS/type.txt"
swift scripts/ghostty-app/inject.swift key 36
printf 'printf "ISSUE804_EXP5_CGEVENT\n" > '"$TS"'/marker.txt' > "$TS/type.txt"
swift scripts/ghostty-app/inject.swift type "$TS/type.txt"
swift scripts/ghostty-app/inject.swift key 36
cat "$TS/marker.txt"
```

Pass criteria:

- `marker.txt` exists and contains `ISSUE804_EXP5_CGEVENT`.

### 5. Failure Investigation and Cleanup

If either route fails, capture the post-attempt window and focus state:

```bash
scripts/roastty-app/screenshot.sh "$ROASTTY_PID" issue-804-exp5-after-keyboard
osascript -e 'tell application "System Events" to name of first process whose frontmost is true'
osascript -e 'tell application "System Events" to tell (first process whose unix id is '"$ROASTTY_PID"') to get {name, frontmost, visible, enabled}'
ioreg -l -w 0 | rg -i 'SecureInput|SecureEventInput|kCGSSessionSecureInputPID|CGSSessionSecureInput' || true
log show --predicate 'subsystem == "com.apple.TCC" AND (eventMessage CONTAINS[c] "keyboard" OR eventMessage CONTAINS[c] "listen" OR eventMessage CONTAINS[c] "accessibility" OR eventMessage CONTAINS[c] "input")' --last 10m --style compact
scripts/roastty-app/stop-app.sh
pgrep -fl 'Roastty.app/Contents/MacOS/roastty' || true
```

Pass criteria:

- Cleanup leaves no debug Roastty process running.
- Any failure is classified with frontmost state, visible state, Secure Input
  signal, and TCC log signal.

Overall result:

- **Pass** if either external keyboard route creates its marker file.
- **Partial** if external keyboard still fails but the experiment proves the
  Input Monitoring grant did not change the observed behavior.
- **Fail** if Roastty cannot be launched, focused, or observed.

## Result

**Result:** Partial.

Granting Input Monitoring to Ghostty and restarting Ghostty plus Codex did not
make either external keyboard path reach Roastty. Both System Events and CGEvent
keyboard commands returned successfully while Roastty was frontmost and visible,
but neither marker file was created.

Logs are in `logs/` with the `issue804-exp5-` prefix. Screenshots are in
`/Users/astrohacker/.cache/termsurf/shots/`.

### Preflight

`logs/issue804-exp5-preflight.log` shows:

- `AXIsProcessTrusted()` printed `true`.
- Apple Events to `System Events` returned process count `63`.
- The session is a restarted Ghostty-hosted Codex session:
  `/Applications/Ghostty.app/Contents/MacOS/ghostty -> login -> zsh -> codex -> zsh`.

`logs/issue804-exp5-tcc-rerun.log` also confirms the relevant Input Monitoring
TCC service was changed for Ghostty before this rerun:

```text
Publishing <TCCDEvent: type=Modify, service=kTCCServiceListenEvent,
identifier_type=Bundle ID, identifier=com.mitchellh.ghostty>
```

This confirms the new permission under test was present for the responsible host
app.

### Launch and Focus

`logs/issue804-exp5-launch.log` shows:

- Debug Roastty launched as PID `93603`.
- Window discovery found the visible terminal window:
  `id=396 layer=0 bounds=(489,161 800x632) name="👻"`.
- `winid.swift` selected that same visible window: `396 489 161 800 632`.
- System Events made Roastty frontmost; the frontmost process was `roastty`.
- Screenshot capture passed:
  `/Users/astrohacker/.cache/termsurf/shots/issue-804-exp5-before-keyboard-20260613-134953.png`.

### System Events Keyboard

`logs/issue804-exp5-keyboard-system-events.log` shows:

- The warmup key, `keystroke`, and Return commands returned without tool errors.
- The marker file was not created:
  `cat: /tmp/termsurf-issue804-exp5-system-events/marker.txt: No such file or directory`.

Result: **Fail** for System Events keyboard after Input Monitoring grant.

### CGEvent Keyboard

`logs/issue804-exp5-keyboard-cgevent.log` shows:

- `inject.swift key`, `inject.swift type`, and Return commands returned without
  tool errors.
- The marker file was not created:
  `cat: /tmp/termsurf-issue804-exp5-cgevent/marker.txt: No such file or directory`.

Result: **Fail** for CGEvent keyboard after Input Monitoring grant.

### Investigation and Cleanup

`logs/issue804-exp5-investigation-cleanup.log` shows:

- Post-attempt screenshot capture passed:
  `/Users/astrohacker/.cache/termsurf/shots/issue-804-exp5-after-keyboard-20260613-134955.png`.
- Roastty was still frontmost and visible after both failed keyboard attempts:
  `roastty, true, true, missing value`.
- The checked `ioreg` Secure Input query did not report a Secure Input marker.
- Cleanup killed debug Roastty PID `93603`.
- No debug Roastty process remained afterward.

The initial TCC diagnostic in this log used `log` without an absolute path, so
zsh interpreted it as its `log` builtin and printed
`zsh:log:61: too many arguments`. The diagnostic was rerun successfully with
`/usr/bin/log` in `logs/issue804-exp5-tcc-rerun.log`.

The TCC rerun shows many `kTCCServiceListenEvent` preflight queries, plus the
Ghostty Input Monitoring modify event noted above. It did not reveal an explicit
TCC denial for the keyboard-posting commands. It did log a separate Roastty
Accessibility preflight warning:

```text
identifier=com.mitchellh.roastty.debug ... attempted to call TCCAccessRequest
for kTCCServiceAccessibility without the recommended entitlement
```

That warning is from the Roastty process itself checking Accessibility and does
not explain the external keyboard failure, because the keyboard posting is being
attempted from the Ghostty-hosted agent path and Roastty stayed frontmost and
visible.

## Conclusion

Input Monitoring did not fix external keyboard delivery to Roastty in this VM.
At this point, the likely issue is not a missing Accessibility, Automation, or
Input Monitoring grant for Ghostty.

The next experiment should instrument Roastty's AppKit event entry points
(`keyDown`, `insertText`, marked text, and first-responder/focus callbacks) and
rerun the same two keyboard injections. That will tell us whether the synthetic
events fail before entering Roastty, enter AppKit but do not reach the terminal
view, or reach the terminal view and fail during terminal forwarding.
