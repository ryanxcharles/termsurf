# Experiment 4: Reproduce Issue 802 Input Methods Against Roastty

## Description

Test every GUI automation method that Issue 802 proved useful against either
Ghostty or Roastty, but target only the current debug Roastty app in this VM.

Experiments 2 and 3 showed that two external keyboard routes return success but
do not create a terminal-side marker file in Roastty. Before designing a lower
level diagnostic, this experiment replays the full Issue 802 automation toolbox
against Roastty with independent oracles for each method:

1. System Events keyboard input, the successful Ghostty external-keyboard path.
2. CGEvent keyboard input, present in the generic helper and used by later
   harness code, but known to be focus-sensitive.
3. XCTest UI keyboard input, the successful Roastty native AppKit key route.
4. Launch-time bootstrap command delivery, the successful live A/B recipe path
   that avoids interactive keyboard injection.
5. CGEvent mouse input: move, click, drag, scroll, and context/right click.
6. Window screenshot capture and non-OCR oracles: marker files, accessibility
   output, pasteboard contents, and screenshots.

This experiment intentionally separates the methods. A failure in System Events
or CGEvent keyboard must not prevent testing XCTest, bootstrap, screenshots, or
mouse input.

Per user instruction, this issue skips adversarial review.

## Changes

Planned issue-doc changes:

- `issues/0804-roastty-gui-automation-readiness/04-reproduce-issue-802-methods-against-roastty.md`
  - Record the design, commands, result table, and conclusion.
- `issues/0804-roastty-gui-automation-readiness/README.md`
  - Add Experiment 4 to the issue index.

Allowed harness changes only if a reusable Issue 802 method is stale or cannot
be run on this VM for a harness reason:

- `scripts/roastty-app/*`
  - Add or fix Roastty wrappers for click, drag, scroll, screenshot,
    window-focus, or bootstrap execution.
- `scripts/ghostty-app/*`
  - Fix only generic helpers without breaking Ghostty workflows.

No Roastty product behavior should change in this experiment. If a product bug
is discovered, record it as a finding unless it directly blocks proving an
automation method.

## Verification

Run from the repo root. Write command transcripts to `logs/` with the prefix
`issue804-exp4-`. Write screenshots under the existing out-of-repo shot
directory used by the Roastty helpers.

### 1. Preflight and Launch

Commands:

```bash
git status --short
swift -e 'import ApplicationServices; print(AXIsProcessTrusted())'
osascript -e 'tell application "System Events" to count processes'
scripts/roastty-app/stop-app.sh || true
cd roastty && macos/build.nu --action build
cd ..
ROASTTY_PID="$(scripts/roastty-app/start-app.sh)"
export ROASTTY_PID
pgrep -fl 'Roastty.app/Contents/MacOS/roastty'
swift scripts/roastty-app/list-windows.swift "$ROASTTY_PID"
swift scripts/roastty-app/winid.swift "$ROASTTY_PID"
osascript -e 'tell application "System Events" to set frontmost of first process whose unix id is '"$ROASTTY_PID"' to true'
osascript -e 'tell application "System Events" to name of first process whose frontmost is true'
scripts/roastty-app/screenshot.sh "$ROASTTY_PID" issue-804-exp4-initial-window
```

Pass criteria:

- Accessibility is trusted.
- Apple Events to System Events work.
- Roastty builds and launches.
- The visible Roastty terminal window is discovered.
- Roastty is frontmost.
- A window screenshot captures the actual Roastty terminal.

### 2. System Events Keyboard to Roastty

Replay the Issue 802 successful Ghostty keyboard method against Roastty:
activate-first, warmup, bootstrap to bash, then type a marker-writing command.

Commands:

```bash
TS=/tmp/termsurf-issue804-exp4-system-events
mkdir -p "$TS"
rm -f "$TS/marker.txt"
osascript -e 'tell application "System Events" to set frontmost of first process whose unix id is '"$ROASTTY_PID"' to true'
osascript -e 'tell application "System Events" to key code 49'
printf 'exec bash --norc --noprofile' > "$TS/type.txt"
osascript -e 'tell application "System Events" to keystroke (read POSIX file "'"$TS"'/type.txt")'
osascript -e 'tell application "System Events" to key code 36'
printf 'printf "ISSUE804_EXP4_SYSTEM_EVENTS\n" > '"$TS"'/marker.txt' > "$TS/type.txt"
osascript -e 'tell application "System Events" to keystroke (read POSIX file "'"$TS"'/type.txt")'
osascript -e 'tell application "System Events" to key code 36'
cat "$TS/marker.txt"
```

Pass criteria:

- `marker.txt` exists and contains `ISSUE804_EXP4_SYSTEM_EVENTS`.

Record if the command returns success but no text appears or no marker file is
created. That distinguishes "posting returned" from "Roastty received input."

### 3. CGEvent Keyboard to Roastty

Replay the generic Issue 802 helper's keyboard subcommands against Roastty.

Commands:

```bash
TS=/tmp/termsurf-issue804-exp4-cgevent
mkdir -p "$TS"
rm -f "$TS/marker.txt"
osascript -e 'tell application "System Events" to set frontmost of first process whose unix id is '"$ROASTTY_PID"' to true'
swift scripts/ghostty-app/inject.swift key 49
printf 'exec bash --norc --noprofile' > "$TS/type.txt"
swift scripts/ghostty-app/inject.swift type "$TS/type.txt"
swift scripts/ghostty-app/inject.swift key 36
printf 'printf "ISSUE804_EXP4_CGEVENT\n" > '"$TS"'/marker.txt' > "$TS/type.txt"
swift scripts/ghostty-app/inject.swift type "$TS/type.txt"
swift scripts/ghostty-app/inject.swift key 36
cat "$TS/marker.txt"
```

Pass criteria:

- `marker.txt` exists and contains `ISSUE804_EXP4_CGEVENT`.

If this fails, capture a screenshot and record frontmost state immediately after
the failed attempt.

### 4. XCTest Keyboard and Accessibility Output

Run the UI automation route that Issue 802 proved can reach Roastty's native
AppKit key path and the terminal accessibility oracle.

Commands:

```bash
cd roastty/macos
xcodebuild test \
  -project Roastty.xcodeproj \
  -scheme Roastty \
  -testPlan Roastty \
  -destination 'platform=macOS' \
  -only-testing:RoasttyUITests/RoasttyTerminalOutputUITests/testTerminalOutputIsVisibleToUIAutomation
xcodebuild test \
  -project Roastty.xcodeproj \
  -scheme Roastty \
  -testPlan Roastty \
  -destination 'platform=macOS' \
  -only-testing:RoasttyUITests/RoasttyDeadKeyUITests/testDeadKeyCompositionCommitsText
cd ../..
```

Pass criteria:

- `RoasttyTerminalOutputUITests.testTerminalOutputIsVisibleToUIAutomation`
  executes and passes.
- `RoasttyDeadKeyUITests.testDeadKeyCompositionCommitsText` executes its body
  and either passes or records the known Issue 802 route-proof skip, with a
  trace showing `keyDown`, `setMarkedText`, `insertText`, or
  `committedPreeditText`.

This route is considered successful for keyboard delivery if XCTest reaches the
Roastty AppKit key path, even if it does not prove the external-agent keyboard
path.

### 5. Launch-Time Bootstrap Command Delivery

Replay the Issue 802 live A/B approach that avoids interactive keyboard input:
launch Roastty directly with temporary shell startup files that run a recipe.

Commands:

```bash
scripts/roastty-app/stop-app.sh || true
BOOT="$(mktemp -d /tmp/termsurf-exp4-bootstrap.XXXXXX)"
mkdir -p "$BOOT/nushell"
cat > "$BOOT/recipe.sh" <<'SH'
#!/usr/bin/env bash
clear
printf 'ISSUE804_EXP4_BOOTSTRAP_READY\n'
printf 'BOOTSTRAP_MARKER\n' > /tmp/termsurf-issue804-exp4-bootstrap-marker.txt
sleep 20
SH
chmod +x "$BOOT/recipe.sh"
printf 'bash %q\n' "$BOOT/recipe.sh" > "$BOOT/.zshrc"
printf 'bash "%s/recipe.sh"\n' "$BOOT" > "$BOOT/nushell/config.nu"
rm -f /tmp/termsurf-issue804-exp4-bootstrap-marker.txt
ZDOTDIR="$BOOT" XDG_CONFIG_HOME="$BOOT" SHELL=/bin/zsh \
  roastty/macos/build/Build/Products/Debug/Roastty.app/Contents/MacOS/roastty \
  > logs/issue804-exp4-bootstrap-stdout.log \
  2> logs/issue804-exp4-bootstrap-stderr.log &
ROASTTY_BOOT_PID="$!"
sleep 3
cat /tmp/termsurf-issue804-exp4-bootstrap-marker.txt
swift scripts/roastty-app/list-windows.swift "$ROASTTY_BOOT_PID"
scripts/roastty-app/screenshot.sh "$ROASTTY_BOOT_PID" issue-804-exp4-bootstrap-window
kill "$ROASTTY_BOOT_PID" || true
rm -rf "$BOOT"
```

Pass criteria:

- The marker file exists and contains `BOOTSTRAP_MARKER`.
- The screenshot visibly contains `ISSUE804_EXP4_BOOTSTRAP_READY`.

This proves command delivery to Roastty without relying on synthetic keyboard
input.

### 6. CGEvent Mouse Click and Right Click

Use the Issue 802 CGEvent mouse driver against the visible Roastty window. Since
mouse click receipt can be hard to prove without a byteprobe, use screenshot and
frontmost/focus state as the basic oracle, then use stronger oracles for drag
and scroll in later steps.

Commands:

```bash
ROASTTY_PID="$(pgrep -f 'Roastty.app/Contents/MacOS/roastty' | head -1)"
LINE="$(swift scripts/roastty-app/list-windows.swift "$ROASTTY_PID" | awk '/layer=0/ { print; exit }')"
read -r X Y W H < <(printf '%s\n' "$LINE" |
  sed -E 's/.*bounds=\(([0-9.-]+),([0-9.-]+) ([0-9.-]+)x([0-9.-]+)\).*/\1 \2 \3 \4/' |
  awk '{ printf "%d %d %d %d\n", $1, $2, $3, $4 }')
CX=$((X + W / 2))
CY=$((Y + H / 2))
swift scripts/ghostty-app/inject.swift move "$CX" "$CY"
swift scripts/ghostty-app/inject.swift click "$CX" "$CY" left 1
swift scripts/ghostty-app/inject.swift click "$CX" "$CY" right 1
scripts/roastty-app/screenshot.sh "$ROASTTY_PID" issue-804-exp4-mouse-clicks
```

Pass criteria:

- Commands return without error.
- Roastty remains frontmost and screenshots show the Roastty window after the
  events.
- If a context menu is visible after right click, record that as receipt
  evidence. If not, classify right-click receipt as weakly observed unless a
  stronger oracle is available.

### 7. CGEvent Mouse Scroll

Use the Roastty-specific scroll driver that Issue 802 proved live against
Roastty. Prefer bootstrap content with enough scrollback so this does not depend
on keyboard input.

Commands:

```bash
scripts/roastty-app/stop-app.sh || true
BOOT="$(mktemp -d /tmp/termsurf-exp4-scroll.XXXXXX)"
mkdir -p "$BOOT/nushell"
cat > "$BOOT/recipe.sh" <<'SH'
#!/usr/bin/env bash
clear
seq 1 200
sleep 20
SH
chmod +x "$BOOT/recipe.sh"
printf 'bash %q\n' "$BOOT/recipe.sh" > "$BOOT/.zshrc"
printf 'bash "%s/recipe.sh"\n' "$BOOT" > "$BOOT/nushell/config.nu"
ZDOTDIR="$BOOT" XDG_CONFIG_HOME="$BOOT" SHELL=/bin/zsh \
  roastty/macos/build/Build/Products/Debug/Roastty.app/Contents/MacOS/roastty &
ROASTTY_SCROLL_PID="$!"
sleep 3
LINE="$(swift scripts/roastty-app/list-windows.swift "$ROASTTY_SCROLL_PID" | awk '/layer=0/ { print; exit }')"
read -r X Y W H < <(printf '%s\n' "$LINE" |
  sed -E 's/.*bounds=\(([0-9.-]+),([0-9.-]+) ([0-9.-]+)x([0-9.-]+)\).*/\1 \2 \3 \4/' |
  awk '{ printf "%d %d %d %d\n", $1, $2, $3, $4 }')
CX=$((X + W / 2))
CY=$((Y + H / 2))
scripts/roastty-app/screenshot.sh "$ROASTTY_SCROLL_PID" issue-804-exp4-scroll-before
swift scripts/roastty-app/scroll.swift "$CX" "$CY" 20
sleep 1
scripts/roastty-app/screenshot.sh "$ROASTTY_SCROLL_PID" issue-804-exp4-scroll-after-up
swift scripts/roastty-app/scroll.swift "$CX" "$CY" -20
sleep 1
scripts/roastty-app/screenshot.sh "$ROASTTY_SCROLL_PID" issue-804-exp4-scroll-after-down
kill "$ROASTTY_SCROLL_PID" || true
rm -rf "$BOOT"
```

Pass criteria:

- The before screenshot shows the tail of `seq 1 200`.
- The scroll-up screenshot shows earlier history lines.
- The scroll-down screenshot returns toward the tail.

### 8. CGEvent Drag Selection and Pasteboard

Use the Roastty-specific drag driver that Issue 802 proved live, then invoke the
copy action through the menu as Issue 802 did when CGEvent Command-C was
unreliable.

Commands:

```bash
scripts/roastty-app/stop-app.sh || true
BOOT="$(mktemp -d /tmp/termsurf-exp4-drag.XXXXXX)"
mkdir -p "$BOOT/nushell"
cat > "$BOOT/recipe.sh" <<'SH'
#!/usr/bin/env bash
clear
printf 'DRAGSELECTME_TARGET_HERE\n'
sleep 20
SH
chmod +x "$BOOT/recipe.sh"
printf 'bash %q\n' "$BOOT/recipe.sh" > "$BOOT/.zshrc"
printf 'bash "%s/recipe.sh"\n' "$BOOT" > "$BOOT/nushell/config.nu"
printf 'CLIPBOARD_PROBE_STALE' | pbcopy
ZDOTDIR="$BOOT" XDG_CONFIG_HOME="$BOOT" SHELL=/bin/zsh \
  roastty/macos/build/Build/Products/Debug/Roastty.app/Contents/MacOS/roastty &
ROASTTY_DRAG_PID="$!"
sleep 3
LINE="$(swift scripts/roastty-app/list-windows.swift "$ROASTTY_DRAG_PID" | awk '/layer=0/ { print; exit }')"
read -r X Y W H < <(printf '%s\n' "$LINE" |
  sed -E 's/.*bounds=\(([0-9.-]+),([0-9.-]+) ([0-9.-]+)x([0-9.-]+)\).*/\1 \2 \3 \4/' |
  awk '{ printf "%d %d %d %d\n", $1, $2, $3, $4 }')
swift scripts/roastty-app/drag.swift "$((X + 80))" "$((Y + 95))" "$((X + 310))" "$((Y + 95))" 18
scripts/roastty-app/screenshot.sh "$ROASTTY_DRAG_PID" issue-804-exp4-drag-selection
osascript <<OSA
tell application "System Events"
  tell first process whose unix id is $ROASTTY_DRAG_PID
    click menu item "Copy" of menu "Edit" of menu bar 1
  end tell
end tell
OSA
pbpaste
kill "$ROASTTY_DRAG_PID" || true
rm -rf "$BOOT"
```

Pass criteria:

- Screenshot shows a highlighted selection.
- `pbpaste` changes from `CLIPBOARD_PROBE_STALE` to a substring of
  `DRAGSELECTME_TARGET_HERE`.

### 9. Classification Table

Record a table in the result with one row per method:

| Method                    | Prior Issue 802 target   | Roastty result    | Oracle               | Notes |
| ------------------------- | ------------------------ | ----------------- | -------------------- | ----- |
| System Events keyboard    | Ghostty                  | Pass/Partial/Fail | marker file          |       |
| CGEvent keyboard          | helper/focus-sensitive   | Pass/Partial/Fail | marker file          |       |
| XCTest keyboard           | Roastty                  | Pass/Partial/Fail | xcodebuild trace     |       |
| Launch bootstrap          | Ghostty/Roastty live A/B | Pass/Partial/Fail | marker + screenshot  |       |
| CGEvent click/right-click | Ghostty                  | Pass/Partial/Fail | screenshot/menu      |       |
| CGEvent scroll            | Roastty                  | Pass/Partial/Fail | screenshots          |       |
| CGEvent drag selection    | Roastty                  | Pass/Partial/Fail | screenshot + pbpaste |       |
| Window screenshot         | Ghostty/Roastty          | Pass/Partial/Fail | PNG artifact         |       |

Overall result:

- **Pass** if every previously successful Issue 802 method either works against
  Roastty or has a stronger Roastty-specific replacement that works and is
  documented.
- **Partial** if one or more methods still fail but the experiment proves the
  other independent methods and classifies the failure.
- **Fail** if Roastty cannot be launched, observed, or interacted with at all.

## Result

**Result:** Partial.

The current VM can build, launch, focus, screenshot, bootstrap, XCTest-drive,
scroll, and drag-select the real Roastty GUI. The two external keyboard routes
still fail against Roastty exactly as they did in Experiments 2 and 3: the
posting commands return successfully while Roastty is frontmost and visible, but
no marker file is created.

Logs are in `logs/` with the `issue804-exp4-` prefix. Screenshots are in
`/Users/astrohacker/.cache/termsurf/shots/`.

### Summary Table

| Method                    | Prior Issue 802 target   | Roastty result | Oracle               | Notes                                                                                                                              |
| ------------------------- | ------------------------ | -------------- | -------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| System Events keyboard    | Ghostty                  | **Fail**       | marker file          | Commands returned, Roastty stayed frontmost/visible, but `/tmp/termsurf-issue804-exp4-system-events/marker.txt` did not exist.     |
| CGEvent keyboard          | helper/focus-sensitive   | **Fail**       | marker file          | Commands returned, Roastty stayed frontmost/visible, but `/tmp/termsurf-issue804-exp4-cgevent/marker.txt` did not exist.           |
| XCTest keyboard           | Roastty                  | **Pass**       | xcodebuild trace     | Terminal-output UI test passed; dead-key UI test passed outright.                                                                  |
| Launch bootstrap          | Ghostty/Roastty live A/B | **Pass**       | marker + screenshot  | Direct debug app launch with `ZDOTDIR`/`XDG_CONFIG_HOME` created `BOOTSTRAP_MARKER` and displayed `ISSUE804_EXP4_BOOTSTRAP_READY`. |
| CGEvent click/right-click | Ghostty                  | **Partial**    | screenshot/frontmost | Move, left click, and right click returned; Roastty stayed frontmost. No context menu or byteprobe oracle proved receipt.          |
| CGEvent scroll            | Roastty                  | **Pass**       | screenshots          | `seq 1 200` viewport moved from tail `178..200` to top `1..24`, then returned to tail.                                             |
| CGEvent drag selection    | Roastty                  | **Pass**       | screenshot + pbpaste | First drag missed the text row; rerun at `windowY + 72pt` succeeded and `pbpaste` returned `DRAGSELECTME_TARGET_HERE`.             |
| Window screenshot         | Ghostty/Roastty          | **Pass**       | PNG artifact         | Full-window screenshots captured the actual Roastty terminal window.                                                               |

### Preflight, Build, Launch, and Screenshot

`logs/issue804-exp4-preflight-launch.log` shows:

- `AXIsProcessTrusted()` printed `true`.
- Apple Events to `System Events` returned process count `61`.
- `roastty/macos/build.nu --action build` completed with
  `** BUILD SUCCEEDED **`.
- Debug Roastty launched as PID `92483`.
- System Events made Roastty frontmost; the frontmost process name was
  `roastty`.
- The screenshot helper captured the visible terminal window:
  `/Users/astrohacker/.cache/termsurf/shots/issue-804-exp4-initial-window-20260613-133522.png`.

One initial `list-windows.swift` call printed no window rows immediately after
launch, and `winid.swift` briefly printed an empty-size candidate (`227 0 0 0`).
The screenshot helper then selected the real visible window
`id=230 bounds=800x632pt` and captured a valid `1600x1264px` image. Later
window-list calls consistently returned the visible `800x632` window after the
same settle period.

### External Keyboard

System Events keyboard failed:

- `logs/issue804-exp4-keyboard-system-events.log`
  - Warmup key, text keystroke, and Return commands returned without tool
    errors.
  - `cat /tmp/termsurf-issue804-exp4-system-events/marker.txt` failed with
    `No such file or directory`.
  - Post-attempt screenshot:
    `/Users/astrohacker/.cache/termsurf/shots/issue-804-exp4-system-events-after-20260613-133542.png`.
  - Roastty remained frontmost and visible:
    `roastty, true, true, missing value`.

CGEvent keyboard failed:

- `logs/issue804-exp4-keyboard-cgevent.log`
  - `inject.swift key`, `inject.swift type`, and Return commands returned
    without tool errors.
  - `cat /tmp/termsurf-issue804-exp4-cgevent/marker.txt` failed with
    `No such file or directory`.
  - Post-attempt screenshot:
    `/Users/astrohacker/.cache/termsurf/shots/issue-804-exp4-cgevent-after-20260613-133544.png`.
  - Roastty remained frontmost and visible:
    `roastty, true, true, missing value`.

This confirms the external-keyboard failure is not caused by the app being in
the background.

### XCTest Keyboard

Both focused XCTest routes passed:

- `logs/issue804-exp4-xctest-terminal-output.log`
  - `RoasttyTerminalOutputUITests.testTerminalOutputIsVisibleToUIAutomation`
    passed in `3.512` seconds.
  - XCTest found the `TERMSURF_READY_158` text view.
  - `Executed 1 test, with 0 failures`.
  - `** TEST SUCCEEDED **`.
- `logs/issue804-exp4-xctest-dead-key.log`
  - `RoasttyDeadKeyUITests.testDeadKeyCompositionCommitsText` passed in `6.414`
    seconds.
  - `Executed 1 test, with 0 failures`.
  - `** TEST SUCCEEDED **`.

This is stronger than the Issue 802 route-proof skip: in this VM, XCTest can
drive the terminal element and observe the committed `é` output.

### Launch-Time Bootstrap

Bootstrap command delivery passed:

- `logs/issue804-exp4-bootstrap.log`
  - Direct debug app launch created
    `/tmp/termsurf-issue804-exp4-bootstrap-marker.txt`.
  - The file contained `BOOTSTRAP_MARKER`.
  - The visible window was `id=268 layer=0 bounds=(489,161 800x632)`.
  - Screenshot:
    `/Users/astrohacker/.cache/termsurf/shots/issue-804-exp4-bootstrap-window-20260613-133655.png`.

The experiment design used the stale path
`roastty/macos/build/Build/Products/Debug/Roastty.app`. The actual build output
is `roastty/macos/build/Debug/Roastty.app`, matching `start-app.sh`; the run
used the actual debug app path.

### Mouse

Click/right-click CGEvents were only partially proven:

- `logs/issue804-exp4-mouse-clicks.log`
  - Window bounds were `id=282 layer=0 bounds=(489,161 800x632)`.
  - The driver posted move, left click, and right click at `(889,477)`.
  - Screenshot:
    `/Users/astrohacker/.cache/termsurf/shots/issue-804-exp4-mouse-clicks-20260613-133713.png`.
  - Roastty remained frontmost.

The screenshot did not show an obvious context menu, and no byteprobe was
running, so this proves event posting and focus state but not terminal receipt
for click/right-click.

Scroll passed:

- `logs/issue804-exp4-scroll.log`
  - Bootstrap content printed `seq 1 200`.
  - Window bounds were `id=289 layer=0 bounds=(489,161 800x632)`.
  - `scroll.swift 889 477 20` returned `scrolled 20 ticks at (889,477)`.
  - `scroll.swift 889 477 -20` returned `scrolled -20 ticks at (889,477)`.
  - Before screenshot:
    `/Users/astrohacker/.cache/termsurf/shots/issue-804-exp4-scroll-before-20260613-133735.png`
    showed tail lines `178..200`.
  - Scroll-up screenshot:
    `/Users/astrohacker/.cache/termsurf/shots/issue-804-exp4-scroll-after-up-20260613-133738.png`
    showed top/history lines `1..24`.
  - Scroll-down screenshot:
    `/Users/astrohacker/.cache/termsurf/shots/issue-804-exp4-scroll-after-down-20260613-133740.png`
    returned to tail lines `178..200`.

Drag selection passed after correcting the vertical coordinate:

- `logs/issue804-exp4-drag.log`
  - First drag used `(X + 80, Y + 95)` to `(X + 310, Y + 95)`.
  - Screenshot showed no highlight.
  - `pbpaste` stayed `CLIPBOARD_PROBE_STALE`.
- `logs/issue804-exp4-drag-rerun.log`
  - Rerun used `(X + 8, Y + 72)` to `(X + 390, Y + 72)`, aligned with the text
    row visible in the screenshot.
  - `drag.swift` returned `dragged (497,233) -> (879,233)`.
  - Screenshot:
    `/Users/astrohacker/.cache/termsurf/shots/issue-804-exp4-drag-selection-rerun-20260613-133840.png`.
  - Menu-driven Copy returned a System Events menu-item reference without error.
  - `pbpaste` returned `DRAGSELECTME_TARGET_HERE`, replacing the stale sentinel.

### Cleanup

After the final run:

```bash
pgrep -fl 'Roastty.app/Contents/MacOS/roastty' || true
```

printed no debug Roastty process.

## Conclusion

The reproducible Roastty automation set for this VM is:

- XCTest keyboard input and accessibility output observation;
- launch-time bootstrap command delivery;
- window screenshots;
- CGEvent scroll;
- CGEvent drag selection with `pbpaste` verification.

The external keyboard paths are still not usable against Roastty from this agent
host, even though they were the successful Ghostty keyboard path in Issue 802
and even though Roastty remains frontmost and visible during the attempts.

Click/right-click CGEvents can be posted, but this experiment did not prove a
strong receipt oracle for them against Roastty. A future experiment should not
spend more time on permission restarts; it should either instrument Roastty's
AppKit event entry points or use a bootstrap-started in-terminal byteprobe/mouse
reporting program so keyboard-independent mouse click/right-click receipt can be
observed deterministically.
