# Experiment 168: Split Divider Color Crash

## Description

The user reported a visible macOS crash dialog after Roastty launches. The
newest diagnostic reports in `~/Library/Logs/DiagnosticReports` show the same
crash shape:

- exception: `EXC_BREAKPOINT` / `SIGTRAP`
- AppKit exception path: `-[NSColor getHue:saturation:brightness:alpha:]`
- Roastty stack:
  - `OSColor.darken(by:)`
  - `Roastty.Config.splitDividerColor`
  - `TerminalSplitSubtreeView.body`

Experiment 167 created a split and proved terminal-side side effects, but its
guard did not check whether macOS wrote a crash report or displayed a crash
dialog after split rendering. This experiment will fix the crash path and add
explicit regression evidence so future live GUI automation cannot silently pass
while Roastty crashes.

This experiment will not claim broader split UI, native menu, fullscreen,
quick-terminal, screenshot, or pixel parity. It only closes the split-divider
color crash slice inside `RUNTIME-011B2B`.

## Changes

- `roastty/macos/Sources/Helpers/Extensions/OSColor+Extension.swift`
  - Make `darken(by:)` convert to a concrete RGB color space before asking
    AppKit for HSB components.
  - Preserve the current darkening formula and alpha behavior for normal RGB
    colors.
  - Add a safe fallback path if AppKit cannot convert the color.
- macOS tests
  - Add a focused unit test for `OSColor.darken(by:)` using a color shape that
    previously triggers the AppKit `getHue` exception path when used directly.
  - Assert the helper returns a valid color instead of throwing an Objective-C
    exception.
- `issues/0805-roastty-ghostty-parity/macos_applescript_workflow_runtime.py`
  - Strengthen the live guard to snapshot existing Roastty diagnostic reports
    before launch and fail if a new `roastty-*.ips` crash report appears before
    cleanup completes.
  - Keep the existing side-effect checks for split creation and `input text`.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split the crash slice out of `RUNTIME-011B2B` into a new `RUNTIME-011B2C`
    row marked `Oracle complete` after the result, or update the existing
    `RUNTIME-011B2B` evidence if the review finds the row split would be too
    narrow.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md` and
  `config-matrix.md`
  - Regenerate if the runtime inventory changes.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Record the learning that GUI guards must assert absence of new macOS crash
    reports, not only command side effects.

## Verification

Pass criteria:

- The focused macOS test proves `OSColor.darken(by:)` does not call `getHue` on
  a non-converted AppKit color that can raise.
- `(cd roastty && macos/build.nu --action test)` passes.
- `(cd roastty && macos/build.nu --action build)` passes.
- The live AppleScript workflow guard still passes.
- The live guard fails if a new Roastty diagnostic report appears during the
  guarded launch/split/input/cleanup window.
- Running the live guard against the fixed app creates no new
  `~/Library/Logs/DiagnosticReports/roastty-*.ips` crash report.
- `RUNTIME-011B2B` no longer hides the split-divider color crash as an
  unresolved generic app walkthrough gap.
- CFG-223 remains `Gap` unless all other runtime/UI gaps are also closed.

Commands:

```bash
(cd roastty && macos/build.nu --action test)
(cd roastty && macos/build.nu --action build)
(cd roastty/macos && swiftlint)
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_applescript_workflow_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/168-split-divider-color-crash.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

Fail criteria:

- The fix merely avoids creating splits or avoids reading `splitDividerColor`.
- The test uses only simple RGB colors and does not cover the crash class.
- The live guard can pass while a new Roastty crash report is written.
- The experiment claims broad split visual parity, menu parity, fullscreen
  parity, quick-terminal parity, or screenshot/pixel parity from this fix.

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

Initial verdict: **Changes required**.

- Required: the verification commands changed Swift files but did not include an
  explicit SwiftLint hygiene gate required by `roastty/macos/AGENTS.md`.

Fix:

- Added `(cd roastty/macos && swiftlint)` to the verification commands.

Re-review verdict: **Approved**. The reviewer confirmed the SwiftLint command is
present and introduced no new required findings.
