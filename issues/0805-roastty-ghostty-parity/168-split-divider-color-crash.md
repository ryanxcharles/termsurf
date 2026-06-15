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

## Result

**Result:** Pass

The split-divider crash path is fixed and guarded.

Implementation:

- `OSColor.darken(by:)` now converts AppKit colors to sRGB before calling
  `getHue`. If AppKit cannot convert the color, the helper returns the original
  color instead of raising through `getHue`.
- `OSColorExtensionTests` covers a dynamic AppKit color and a non-convertible
  pattern color, proving the helper does not terminate the test process on the
  crash class.
- `macos_applescript_workflow_runtime.py` now snapshots
  `~/Library/Logs/DiagnosticReports/roastty-*.ips` before launching the debug
  app and fails if a new crash report appears after the live window/tab/split/
  input workflow and cleanup.
- `RUNTIME-011B2C` records the split-divider color crash guard as
  `Oracle complete`. `RUNTIME-011B2B` remains the broader live macOS GUI gap.

Verification run:

```text
(cd roastty/macos && swiftlint)
Done linting! Found 0 violations, 0 serious in 196 files.

(cd roastty && macos/build.nu --action test)
Test run with 221 tests in 24 suites passed after 1.518 seconds.
** TEST SUCCEEDED **

(cd roastty && macos/build.nu --action build)
** BUILD SUCCEEDED **

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_applescript_workflow_runtime.py
macos_applescript_workflow_runtime=pass

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
runtime_rows=75
oracle_complete=68
closed=71
audit_covered=0
incomplete=4
gap=4
cfg223=Gap

for f in issues/0805-roastty-ghostty-parity/*_runtime_parity.py issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py issues/0805-roastty-ghostty-parity/link_hover_preview_dispatch_parity.py issues/0805-roastty-ghostty-parity/link_hover_modifier_refresh_parity.py issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py; do
  PYTHONDONTWRITEBYTECODE=1 python3 "$f"
done
parity_guards=pass

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/platform_runtime_classification.py --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/platform-runtime-classification.md
platform_options=32
gap=15
not_applicable=15
oracle_complete=2
```

The newest `~/Library/Logs/DiagnosticReports/roastty-*.ips` file after the live
guard remained the pre-fix `roastty-2026-06-15-062015.ips`; the strengthened
guard created no new crash report.

The macOS build/test run still emits existing Swift 6/Main Thread Checker,
pasteboard, terminfo, and linker deployment-version warnings, but the
`xcodebuild` actions reported success.

## Conclusion

Roastty no longer crashes in the observed split-divider color path, and the live
AppleScript workflow guard now treats new macOS crash reports as a first-class
failure. This prevents a repeat of Experiment 167's narrow success condition,
where terminal side effects could pass while the app still crashed during GUI
rendering.

The remaining `RUNTIME-011B2B` work is still broad live macOS GUI parity: native
menu display/validation, titlebar/fullscreen/quick-terminal visuals,
screenshot/pixel evidence, returned split-terminal object re-resolution and
focus/close commands, broader command-palette GUI behavior, and deeper input
navigation/pixel walkthroughs.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Initial verdict: **Changes required**.

- Required: `wait_for_crash_report_settle` could return after the first
  unchanged empty poll, creating a false negative for delayed crash reports.

Fix:

- Updated `wait_for_crash_report_settle` to poll for the full five-second window
  and accumulate any new `roastty-*.ips` reports before returning.

Re-review verdict: **Approved**. The reviewer confirmed the guard no longer
returns early on an unchanged empty poll and introduced no new required
findings.
