# Experiment 166: macOS App Workflow Plumbing

## Description

`RUNTIME-011B` still groups the remaining macOS app/window/tab/split/menu,
titlebar, fullscreen, quick-terminal, and broader command-palette GUI effects.
Some of that row is truly live GUI behavior and needs real app walkthrough
evidence, but a narrower copied-app slice can be proven deterministically before
the full walkthrough:

- pinned Ghostty's copied macOS sources define window, tab, split, titlebar,
  fullscreen, and quick-terminal command plumbing in Swift;
- Roastty's copied macOS sources should match those command/action/config paths
  after expected Ghostty-to-Roastty renames;
- existing Swift tests already exercise split-tree and split drop-zone mechanics
  below the live app window surface.

This experiment will split `RUNTIME-011B` into:

- `RUNTIME-011B1`: **Oracle complete** for copied macOS workflow plumbing for
  window/tab/split/menu/titlebar/fullscreen/quick-terminal command and config
  source paths that can be compared after expected renames, plus focused split
  helper tests.
- `RUNTIME-011B2`: **Gap** for the remaining live app walkthrough evidence:
  actual window/tab/split/menu/titlebar/fullscreen/quick-terminal GUI behavior,
  native menu validation/display, screenshot/pixel/input navigation, and broad
  command-palette GUI behavior.

This experiment will not claim that the actual macOS app windows render
correctly, that System Events can drive every menu item, that fullscreen or
quick terminal visually behaves correctly, that screenshots match pinned
Ghostty, or that the broader command palette GUI walkthrough is complete.

## Changes

- `issues/0805-roastty-ghostty-parity/macos_app_workflow_plumbing_parity.py`
  - Add a static guard that normalized-compares copied Ghostty and Roastty macOS
    source blocks for window/tab/split/menu/titlebar/fullscreen and
    quick-terminal plumbing.
  - Check the specific source anchors that route actions through
    `TerminalController`, `BaseTerminalController`, `TerminalWindow`,
    `SplitTree`, `TerminalSplitTreeView`, `QuickTerminalController`,
    quick-terminal app intents, app delegate wiring, and the Roastty/Ghostty
    action/config bridge files.
  - Assert the inventory split and CFG-223 counts after the new complete row is
    added.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-011B` into complete copied workflow-plumbing evidence and the
    reduced remaining live GUI walkthrough gap.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-223 summary. It must remain `Gap`.
- Existing CFG-223/runtime guards
  - Update expected counts from 72 runtime rows, 65 Oracle-complete rows, and 68
    closed rows to 73 runtime rows, 66 Oracle-complete rows, and 69 closed rows.
    Incomplete and gap counts remain 4.
  - Update references from `RUNTIME-011B` to `RUNTIME-011B2` where they mean the
    remaining live macOS app GUI gap.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add the experiment link and update Learnings after the result.

## Verification

Pass criteria:

- The new guard proves the selected copied Ghostty macOS workflow source blocks
  match Roastty after expected renames and expected package/app identifier
  substitutions.
- The checked anchors cover at least:
  - new window/tab and close/move/goto-tab action plumbing;
  - split creation, focus, resize, equalize, zoom, and drop-zone helper paths;
  - titlebar style/window button/fullscreen command plumbing;
  - quick-terminal controller/app-intent/app-delegate plumbing;
  - menu or command selectors that dispatch these workflows.
- Existing Swift tests for split tree and split drop-zone behavior still pass.
- `RUNTIME-011B1` is `Oracle complete` and cites the new guard plus the focused
  Swift split tests.
- `RUNTIME-011B2` remains `Gap` for live GUI walkthrough evidence.
- CFG-223 remains `Gap`.

Commands:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_app_workflow_plumbing_parity.py
(cd roastty && macos/build.nu --action test)
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
for f in issues/0805-roastty-ghostty-parity/*_runtime_parity.py issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py issues/0805-roastty-ghostty-parity/link_hover_preview_dispatch_parity.py issues/0805-roastty-ghostty-parity/link_hover_modifier_refresh_parity.py issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py; do
  PYTHONDONTWRITEBYTECODE=1 python3 "$f"
done
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/166-macos-app-workflow-plumbing.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

Initial verdict: **Changes required**.

- Required: the verification command block used
  `cd roastty && macos/build.nu --action test`, which would leave the following
  repo-root commands resolving under `roastty/`.

Fix:

- Changed the macOS test command to run in a subshell:
  `(cd roastty && macos/build.nu --action test)`.

Re-review verdict: **Approved**. The reviewer confirmed the command-block
finding is resolved and reported no new required findings.

## Result

**Result:** Pass

`RUNTIME-011B` was split into:

- `RUNTIME-011B1`: **Oracle complete** for copied macOS workflow plumbing for
  windows, tabs, splits, menus, titlebar, fullscreen, and quick terminal.
- `RUNTIME-011B2`: **Gap** for remaining live macOS GUI behavior: actual
  window/tab/split/menu/titlebar/fullscreen/quick-terminal GUI behavior, native
  menu validation/display, screenshot/pixel/input navigation, and broader
  command-palette GUI behavior.

The new `macos_app_workflow_plumbing_parity.py` guard proves the selected
workflow source surface matches pinned Ghostty after expected renames for:

- `TerminalController.swift`
- `TerminalWindow.swift`
- `SplitTree.swift`
- `SplitView.swift`
- `SplitView.Divider.swift`
- `TerminalSplitTreeView.swift`
- `QuickTerminalController.swift`
- `QuickTerminalIntent.swift`
- `AppDelegate.swift`
- `Roastty.Config.swift`
- `Roastty.App.swift`
- `RoasttyPackage.swift`
- `FullscreenMode+Extension.swift`

It checks workflow anchors in `BaseTerminalController.swift` instead of
full-file identity because Experiment 152 already introduced a command-palette
testable extraction there.

Verification run:

```text
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
runtime_rows=73
oracle_complete=66
closed=69
audit_covered=0
incomplete=4
gap=4
cfg223=Gap

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_app_workflow_plumbing_parity.py
macos_app_workflow_plumbing_parity=pass

(cd roastty && macos/build.nu --action test)
Test run with 219 tests in 23 suites passed after 1.989 seconds.
** TEST SUCCEEDED **

for f in issues/0805-roastty-ghostty-parity/*_runtime_parity.py issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py issues/0805-roastty-ghostty-parity/link_hover_preview_dispatch_parity.py issues/0805-roastty-ghostty-parity/link_hover_modifier_refresh_parity.py issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py; do
  PYTHONDONTWRITEBYTECODE=1 python3 "$f"
done
```

The full runtime guard loop passed. The macOS test run emitted existing Main
Thread Checker and pasteboard background-thread warnings, but `xcodebuild`
reported `TEST SUCCEEDED`.

## Conclusion

Copied macOS app workflow command/action/config plumbing now has a durable
source-parity guard and focused split helper tests. This does not close the live
app walkthrough gap: actual GUI rendering, native menu display/validation,
screenshots/pixels, input navigation, fullscreen visuals, quick-terminal
visuals, and broader command-palette GUI behavior remain in `RUNTIME-011B2`.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.

The reviewer independently verified that:

- Experiment 166 records `Pass`, has Result and Conclusion sections, and the
  issue README links it as `Pass`.
- `RUNTIME-011B1` is `Oracle complete`.
- `RUNTIME-011B2` remains `Gap` for live GUI parity.
- CFG-223 remains `Gap` with `runtime_rows=73`, `oracle_complete=66`,
  `closed=69`, `incomplete=4`, and `gap=4`.
- The new guard is non-vacuous and passed.
- The full runtime parity guard loop passed.
- `prettier --check` and `git diff --check` passed.
- The result commit had not been made before review.

The reviewer did not rerun the generated-inventory write command because it
mutates markdown during a read-only review.
