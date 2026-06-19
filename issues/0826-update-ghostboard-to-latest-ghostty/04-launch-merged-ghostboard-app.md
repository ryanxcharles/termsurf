# Experiment 4: Launch the Merged Ghostboard App

## Description

Verify that the app bundle built in Experiment 3 launches on this macOS VM, or
capture the first launch/runtime failure that must be fixed before protocol and
browser-overlay parity work begins.

This experiment is a launch gate only. It should not test TermSurf protocol
behavior, `webtui`, Roamium, browser overlay geometry, input forwarding, or
ordinary browsing workflows. Those require later experiments after basic app
startup is proven.

## Changes

- `issues/0826-update-ghostboard-to-latest-ghostty/README.md`
  - Link this experiment with status `Designed`, then update the status after
    the result is known.
- `issues/0826-update-ghostboard-to-latest-ghostty/04-launch-merged-ghostboard-app.md`
  - Record the launch plan, verification commands, result, review, and
    conclusion.
- `ghostboard/`
  - Only modify launch-related code or bundle metadata if the launch failure is
    narrow, clearly caused by the upstream merge, and required to reach a
    running app process.

Do not modify `webtui/`, `roamium/`, or browser runtime/protocol behavior in
this experiment.

## Verification

Before implementation:

```bash
git status --short
test -d "ghostboard/macos/build/Debug/TermSurf Ghostboard.app"
plutil -extract CFBundleIdentifier raw \
  "ghostboard/macos/build/Debug/TermSurf Ghostboard.app/Contents/Info.plist"
plutil -extract CFBundleExecutable raw \
  "ghostboard/macos/build/Debug/TermSurf Ghostboard.app/Contents/Info.plist"
```

Launch command:

```bash
APP="$PWD/ghostboard/macos/build/Debug/TermSurf Ghostboard.app"
osascript -e "tell application \"$APP\" to activate" \
  > logs/issue-0826-exp04-launch.log 2>&1
```

Process checks:

```bash
APP="$PWD/ghostboard/macos/build/Debug/TermSurf Ghostboard.app"
sleep 5
ps -axo pid,comm,args \
  | rg "TermSurf Ghostboard.app/Contents/MacOS/ghostboard|$APP" \
  > logs/issue-0826-exp04-process.log
log show --last 5m --style compact \
  --predicate 'process == "ghostboard" || eventMessage CONTAINS[c] "TermSurf Ghostboard"' \
  > logs/issue-0826-exp04-system.log
```

If the app launches and stays alive long enough to inspect:

```bash
APP="$PWD/ghostboard/macos/build/Debug/TermSurf Ghostboard.app"
osascript -e "tell application \"$APP\" to quit" \
  > logs/issue-0826-exp04-quit.log 2>&1
```

Always target the app by absolute path, not by name or bundle ID, so the test
does not accidentally activate or quit another installed app.

Failure logging:

- Save command output and observations under repo-root `logs/`, using paths such
  as:
  - `logs/issue-0826-exp04-launch.log`
  - `logs/issue-0826-exp04-process.log`
  - `logs/issue-0826-exp04-system.log`
  - `logs/issue-0826-exp04-quit.log`
- If a crash report is produced, record its path and copy only the relevant
  diagnostic summary into the experiment result.
- If macOS shows a permission or first-launch dialog that blocks unattended
  launch, record the exact dialog text if observable and treat the result as
  `Partial` unless the dialog can be handled without expanding scope.

After any edits, return to the repository root and run the formatters relevant
to changed files:

```bash
git diff --name-only -- '*.zig' | xargs -r zig fmt
(cd ghostboard && swiftlint lint --strict --fix)
prettier --write --prose-wrap always --print-width 80 \
  issues/0826-update-ghostboard-to-latest-ghostty/README.md \
  issues/0826-update-ghostboard-to-latest-ghostty/04-launch-merged-ghostboard-app.md
git diff --check
```

If no Zig files change, `zig fmt` may be skipped and the result should say so.
If no Swift files change, SwiftLint may be skipped and the result should say so.

Result recording and review:

1. Append `## Result` and `## Conclusion` to this file.
2. Record the bundle ID, executable name, launch command, process check result,
   quit command, and log paths.
3. Update the issue README status for this experiment to `Pass`, `Partial`, or
   `Fail`.
4. Run Prettier on this experiment file and the issue README.
5. Request the required result review before committing the result.
6. Record the result review in this file.
7. Commit the reviewed experiment result before designing the next experiment.

Pass criteria:

- The built app bundle exists.
- The bundle ID and executable name can be read from `Info.plist`.
- Absolute-path AppleScript launches the app.
- A Ghostboard/TermSurf app process from the built app bundle is observable
  after a short delay.
- The app can be quit without force-killing it.
- No launch/runtime code outside the narrow launch path is changed.

Partial criteria:

- The app starts but immediately crashes, hangs behind a permission dialog, or
  cannot be quit cleanly, and the first actionable launch failure is documented
  with logs.

Fail criteria:

- The app bundle from Experiment 3 is missing or structurally invalid.
- The launch command cannot be invoked.
- The experiment expands into protocol, browser, overlay, or browsing behavior
  before basic app startup is proven.

## Design Review

An adversarial Codex subagent reviewed the initial design with fresh context.

**Verdict:** Changes required.

Required findings and fixes:

- Launch and quit targeting used relative `open -n` and bundle ID. Fixed by
  following `ghostboard/macos/AGENTS.md`: use absolute-path AppleScript to
  activate and quit the built app.
- Failure logging was not concrete enough for early GUI startup failures. Fixed
  by adding a bounded unified-log capture to `logs/issue-0826-exp04-system.log`.

The optional process-detection finding was also addressed by replacing broad
`pgrep` with a `ps`/`rg` check that looks for the built app bundle path or its
`Contents/MacOS/ghostboard` executable path.

The re-review approved the design with no required findings. Its optional
finding was adopted by repeating the absolute `APP` assignment in the process
check and quit command blocks, so each block can run in a fresh shell.

## Result

**Result:** Pass

The merged app bundle from Experiment 3 exists and can be launched by absolute
path on this macOS VM.

Preflight observations:

- `git status --short` was clean before the launch run.
- `CFBundleIdentifier` is `com.termsurf.ghostboard.debug`.
- `CFBundleExecutable` is `ghostboard`.
- `CFBundleName` and `CFBundleDisplayName` are both `TermSurf Ghostboard`.

Launch command:

```bash
APP="$PWD/ghostboard/macos/build/Debug/TermSurf Ghostboard.app"
osascript -e "tell application \"$APP\" to activate" \
  > logs/issue-0826-exp04-launch.log 2>&1
```

The launch command exited `0` and produced an empty launch log.

Process check:

```text
21266 /Users/astrohack /Users/astrohacker/dev/termsurf/ghostboard/macos/build/Debug/TermSurf Ghostboard.app/Contents/MacOS/ghostboard
```

The running process came from the built app bundle path, so the test did not
accidentally target another installed application.

The unified log was captured with `/usr/bin/log` after the shell builtin name
collision made plain `log show ...` fail with `zsh:log:5: too many arguments`.
The corrected log capture is in `logs/issue-0826-exp04-system.log`.

Important unified-log observations:

- LaunchServices registered `TermSurf Ghostboard`.
- RunningBoard checked launch for the exact built executable path.
- macOS reported the executable, debug dylib, DockTile plugin, and Sparkle
  framework as adhoc signed. This is expected for the local Debug build and did
  not block launch.
- The app checked in, stayed alive long enough for inspection, and later logged
  normal AppKit termination after the quit command.
- Sentry wrote a crash envelope at
  `/Users/astrohacker/.local/state/ghostty/crash/5e76317e-6ced-4afa-b2e7-53378676c2ba.ghosttycrash`.
  The envelope was created during this launch, but its embedded event timestamp
  was `2026-06-19T14:10:34.051349Z`, earlier than this launch window, and the
  process remained alive until the explicit quit command. That makes it look
  like a pending/stale fatal report was flushed during startup rather than this
  launch crashing.

Quit command:

```bash
APP="$PWD/ghostboard/macos/build/Debug/TermSurf Ghostboard.app"
osascript -e "tell application \"$APP\" to quit" \
  > logs/issue-0826-exp04-quit.log 2>&1
```

The quit command exited `0` and produced an empty quit log. A post-quit process
check found no remaining built-app process.

Verification artifacts:

- `logs/issue-0826-exp04-launch.log`: empty.
- `logs/issue-0826-exp04-process.log`: one matching built-app process.
- `logs/issue-0826-exp04-system.log`: 499 unified-log lines.
- `logs/issue-0826-exp04-quit.log`: empty.
- `logs/issue-0826-exp04-post-quit-process.log`: empty.

No source files were changed, so `zig fmt` and SwiftLint were skipped. Markdown
formatting was run for this experiment file and the issue README.

## Conclusion

Basic macOS launch is proven for the merged Ghostboard tree: the Debug app
bundle launches by absolute path, creates a running process from the built
bundle, and quits cleanly without force-kill.

This experiment also confirmed that identity is not yet at the issue target: the
built bundle still presents as `TermSurf Ghostboard`, the bundle ID is
`com.termsurf.ghostboard.debug`, and the executable is `ghostboard`. A later
experiment must verify and fix app identity, CLI naming, and config path before
the issue can be closed.

## Result Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

**Verdict:** Approved.

The reviewer reported no required, optional, or nit findings. It independently
checked that only the two issue docs changed, the README status is `Pass`, the
experiment contains `Result` and `Conclusion`, the recorded plist values match
the built app, the logs support launch and clean quit from the built app path,
and the Sentry envelope timestamp predates the launch window, making the
stale-flush explanation justified. The reviewer also confirmed the result commit
had not already been made.
