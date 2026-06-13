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

Not run yet.
