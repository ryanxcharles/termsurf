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
