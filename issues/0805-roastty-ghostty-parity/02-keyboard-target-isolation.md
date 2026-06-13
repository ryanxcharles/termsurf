# Experiment 2: Keyboard Target Isolation

## Description

Experiment 1 proved the clean build/render A/B baseline, but the keyboard-marker
step failed safely enough to diagnose the real problem: while Codex is running
inside installed Ghostty, System Events can resolve an activation attempt for
the debug Ghostty process to the installed Ghostty process instead. That sent
the typed marker command toward the Codex host window rather than the debug
Ghostty window under test.

This experiment finds and records a safe keyboard-targeting method for the
side-by-side Ghostty/Roastty baseline. It must prove target identity before
typing. If no method can safely target debug Ghostty while Codex is hosted in
installed Ghostty, the experiment should classify that as an environmental
constraint and define the next acceptable oracle for Ghostty keyboard reference
behavior.

## Changes

Planned issue-doc changes:

- `issues/0805-roastty-ghostty-parity/02-keyboard-target-isolation.md`
  - Record the plan, review, commands, result, and conclusion.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add Experiment 2 to the issue index with status `Designed`.
  - Add any reusable targeting learning to `## Learnings`.

Allowed harness changes:

- `scripts/roastty-app/*` or `scripts/ghostty-app/*`
  - Add a small PID-target activation/proof helper if needed.
  - Add a keyboard-target smoke script if it makes the proof repeatable.

Constraints:

- Do not change Ghostty source code.
- Do not change Roastty product behavior.
- Do not send keyboard text until a pre-type guard proves the intended target
  PID, not merely an app name, is frontmost.
- Do not rely on a marker file alone as target proof; marker creation must be
  paired with frontmost PID evidence immediately before typing.
- If any activation method resolves to installed Ghostty/Codex instead of debug
  Ghostty, stop that method without typing and record it as a failed targeting
  method.

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

Initial verdict: **Changes required**.

Required findings and fixes:

- The original preflight process check matched any
  `Ghostty.app/Contents/MacOS/ghostty`, which could include the installed
  Ghostty process hosting Codex. Fixed by recording installed Ghostty separately
  and restricting the stale debug process check to repo build paths.
- The original hygiene command checked only tracked modified harness files,
  missing newly added untracked helpers. Fixed by combining
  `git diff --name-only` with `git ls-files --others --exclude-standard` for the
  harness directories before parsing shell and Swift files.

Re-review verdict: **Approved**. The reviewer confirmed both required findings
were resolved and no new required findings were introduced.

## Verification

Run from the repo root. Write transcripts to `logs/` with the prefix
`issue805-exp2-`. Screenshots must stay outside the repo under
`${TERMSURF_SHOT_DIR:-$HOME/.cache/termsurf/shots}`.

### 1. Preflight

Commands:

```bash
git status --short
git -C vendor/ghostty status --short
git -C vendor/ghostty rev-parse HEAD
command -v zig
zig version
cmp -s ~/.config/ghostty/config ~/.config/roastty/config
pgrep -fl '/Applications/Ghostty.app/Contents/MacOS/ghostty' || true
pgrep -fl 'vendor/ghostty/macos/build/.*/Ghostty.app/Contents/MacOS/ghostty|roastty/macos/build/.*/Roastty.app/Contents/MacOS/roastty' || true
```

Pass criteria:

- `vendor/ghostty` is clean.
- Ghostty is pinned at `2c62d182cec246764ff725096a70b9ef44996f7f`.
- `zig` is Homebrew Zig `0.15.2` on `PATH`.
- Ghostty and Roastty config files match.
- Any installed Ghostty host process is recorded as expected context, not
  treated as a stale debug app.
- No stale debug Ghostty or Roastty process is running before the experiment
  starts; this check is restricted to the repo debug build paths.

### 2. Build and Launch Both Debug Apps

Commands:

```bash
cd vendor/ghostty
zig build -Demit-macos-app=false
nu macos/build.nu --configuration Debug
cd ../..

scripts/roastty-app/build-roastty-kit.sh
cd roastty/macos
xcodebuild build \
  -project Roastty.xcodeproj \
  -scheme Roastty \
  -configuration Debug \
  -derivedDataPath build
cd ../..

GHOSTTY_PID=$(scripts/ghostty-app/start-app.sh)
ROASTTY_PID=$(scripts/roastty-app/start-app.sh)
pgrep -fl 'Ghostty.app/Contents/MacOS/ghostty|Roastty.app/Contents/MacOS/roastty'
```

Pass criteria:

- Both apps build.
- Both apps launch from their debug build paths.
- The transcript records distinct debug PIDs.

### 3. Try Activation Methods Without Typing

Test activation methods one at a time. Each method must record:

- requested target app path;
- requested target PID;
- System Events process name and PID for the target if available;
- global frontmost process name and PID immediately after activation if
  available;
- CGWindow owner PID for the captured layer-0 target window;
- whether the method is safe to type.

Candidate methods:

1. System Events `set frontmost of first process whose unix id is $PID to true`.
2. `NSRunningApplication(processIdentifier: PID).activate(...)` from a Swift
   helper.
3. `open -a "$APP"` / `open -n "$APP"` followed by PID and frontmost proof.
4. Mouse click on the target window center followed by frontmost PID proof.

Pass criteria:

- At least one method can make Roastty frontmost and prove the frontmost PID is
  exactly `ROASTTY_PID`.
- At least one method can make debug Ghostty frontmost and prove the frontmost
  PID is exactly `GHOSTTY_PID`; or the experiment records a concrete environment
  limitation showing installed Ghostty/Codex prevents safe debug Ghostty
  keyboard targeting in this setup.
- No method types text after a failed PID guard.

### 4. Type Only After a Passing PID Guard

For each app with a passing activation method:

```bash
MARKER=/tmp/termsurf-issue805-exp2-<app>-marker
rm -f "$MARKER"
printf 'touch %s' "$MARKER" > "logs/issue805-exp2-<app>-command.txt"

# Run the chosen activation method.
# Assert the immediate pre-type frontmost PID equals the target PID.
# Only then type the command and Return.

test -f "$MARKER"
stat -f 'MARKER_OK path=%N size=%z mtime=%Sm' "$MARKER"
```

Pass criteria:

- For each typed app, the immediate pre-type frontmost PID equals the intended
  debug PID.
- Marker creation succeeds only after that guard.
- If Ghostty cannot be safely targeted, no Ghostty marker is attempted, and the
  result records the next proposed Ghostty keyboard oracle.

### 5. Cleanup

Commands:

```bash
scripts/ghostty-app/stop-app.sh || true
scripts/roastty-app/stop-app.sh || true
rm -f /tmp/termsurf-issue805-exp2-ghostty-marker
rm -f /tmp/termsurf-issue805-exp2-roastty-marker
pgrep -fl 'vendor/ghostty/macos/build/.*/Ghostty.app/Contents/MacOS/ghostty|roastty/macos/build/.*/Roastty.app/Contents/MacOS/roastty' || true
```

Pass criteria:

- No debug Ghostty or Roastty process launched by the experiment remains.
- Temporary marker files are removed.

### 6. Hygiene

Commands:

```bash
git status --short
git diff --check

{
  git diff --name-only -- scripts/ghostty-app scripts/roastty-app
  git ls-files --others --exclude-standard -- scripts/ghostty-app scripts/roastty-app
} |
  awk '/\.(sh|swift)$/ { print }' |
  sort -u |
  xargs -r -n1 sh -c 'case "$1" in *.sh) bash -n "$1";; *.swift) swiftc -parse "$1";; esac' sh
```

Pass criteria:

- Only planned issue docs and intentional harness files are modified.
- `git diff --check` passes.
- Any changed shell or Swift helper parses.

Overall result:

- **Pass** if the experiment finds a safe method to target and type into both
  debug Ghostty and debug Roastty, with immediate pre-type PID proof and marker
  evidence for both.
- **Partial** if Roastty can be safely typed into but debug Ghostty cannot be
  safely targeted while Codex is hosted in installed Ghostty, and the result
  defines a concrete alternate Ghostty keyboard oracle for the next experiment.
- **Fail** if no app can be safely targeted or cleanup cannot be bounded to the
  launched debug processes.

## Result

**Result:** Pass

The experiment found a safe keyboard-targeting method for both debug apps:

1. activate the target by exact Unix PID with System Events;
2. immediately verify the global frontmost PID equals the target debug PID;
3. for Roastty, click the terminal window center to establish first-responder
   focus and verify the frontmost PID again;
4. type only after the final pre-type PID guard passes.

Evidence:

- Full build and first activation transcript: `logs/issue805-exp2-run.log`.
- Combined successful keyboard transcript:
  `logs/issue805-exp2-combined-keyboard-pass.log`.
- Pinned Ghostty was built from clean source before the activation probes:
  `zig build -Demit-macos-app=false` and
  `nu macos/build.nu --configuration Debug`.
- Current Roastty was built before the activation probes:
  `scripts/roastty-app/build-roastty-kit.sh` and
  `xcodebuild build -project Roastty.xcodeproj -scheme Roastty -configuration Debug -derivedDataPath build`.
- Side-by-side debug app PIDs in the successful combined run:
  - Ghostty PID `21183`:
    `vendor/ghostty/macos/build/Debug/Ghostty.app/Contents/MacOS/ghostty`
  - Roastty PID `21206`:
    `roastty/macos/build/Build/Products/Debug/Roastty.app/Contents/MacOS/roastty`
- Ghostty target proof and marker:
  - activation proof:
    `FRONTMOST_AFTER_ACTIVATE target=ghostty name=ghostty pid=21183`
  - pre-type guard:
    `PRETYPE_GUARD target=ghostty target_pid=21183 frontmost_pid=21183`
  - marker proof:
    `MARKER_OK ... path=/tmp/termsurf-issue805-exp2-ghostty-marker ...`
- Roastty target proof and marker:
  - activation proof:
    `FRONTMOST_AFTER_ACTIVATE target=roastty name=roastty pid=21206`
  - first-responder focus proof:
    `ROASTTY_WINDOW target_pid=21206 id=206 x=959 y=112 w=731 h=632` followed by
    `FRONTMOST_AFTER_ROASTTY_CLICK name=roastty pid=21206`
  - pre-type guard:
    `PRETYPE_GUARD target=roastty target_pid=21206 frontmost_pid=21206`
  - marker proof:
    `MARKER_OK ... path=/tmp/termsurf-issue805-exp2-roastty-marker ...`
- Cleanup proof:
  - `killing debug Ghostty PIDs: 21183`
  - `killing debug Roastty PIDs: 21206`
  - `CLEANUP_NO_DEBUG_PROCESSES=yes`

No Ghostty source code, Roastty product code, or harness files were changed.

## Conclusion

Keyboard automation is safe in this VM when every typed command is guarded by
the exact target PID immediately before typing. The installed Ghostty host
collision from Experiment 1 is avoided by treating app names as insufficient and
using Unix PID equality as the typing gate.

The reusable keyboard recipe for future A/B app tests is:

- Ghostty: System Events activate by debug PID, verify frontmost PID, type.
- Roastty: System Events activate by debug PID, click the terminal window
  center, verify frontmost PID again, type.

Future experiments can now use keyboard input for real app walkthrough steps, as
long as they preserve the pre-type PID guard and Roastty focus click.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.
