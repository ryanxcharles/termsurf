# Experiment 171: AppleScript Keyboard and Mouse Events

## Description

`RUNTIME-011B2B` still includes deeper input navigation and walkthrough evidence
in the remaining live macOS GUI gap. Experiments 167 and 170 proved AppleScript
`input text` and split-terminal lifecycle behavior, but they did not prove the
lower-level AppleScript input commands that are specifically needed to automate
keyboard and mouse workflows:

- `send key`;
- `send mouse position`;
- `send mouse button`;
- `send mouse scroll`.

This experiment will split a narrow live input-command slice out of
`RUNTIME-011B2B` by extending the existing absolute-bundle live debug app guard
with controlled child processes:

- a keyboard terminal records bytes produced by scripted `send key` commands;
- a mouse terminal enables terminal mouse reporting and records bytes produced
  by scripted mouse position/button/scroll commands.

This experiment will not claim visual cursor shape, pointer pixels, native menu
display/validation, titlebar/fullscreen/quick-terminal visuals, screenshot/pixel
parity, broader command-palette GUI behavior, link hover preview display, or
full keyboard/mouse walkthrough parity.

## Changes

- `issues/0805-roastty-ghostty-parity/macos_applescript_workflow_runtime.py`
  - Extend the live debug-app workflow with a keyboard input terminal whose
    command records a fixed number of stdin bytes to a temp file.
  - Use AppleScript `send key` commands against that terminal and assert the
    child process records the expected bytes.
  - Extend the workflow with a mouse-report terminal whose command enables
    terminal mouse reporting, records raw stdin bytes to a temp file, and keeps
    the terminal alive long enough for scripted events.
  - Use AppleScript `send mouse position`, `send mouse button`, and
    `send mouse scroll` against that terminal.
  - Assert the mouse child process records terminal mouse-report bytes, not just
    that AppleScript returned without error.
  - Keep the existing isolated config, absolute app path, scoped cleanup,
    crash-report guard, and split-terminal lifecycle assertions.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split a new Oracle-complete row from `RUNTIME-011B2B` for live AppleScript
    keyboard and mouse command delivery with child-process side effects.
  - Reduce the remaining `RUNTIME-011B2B` gap so it no longer lists this
    lower-level AppleScript input-command delivery slice.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-223 summary. It must remain `Gap`.
- Existing CFG-223/runtime guards
  - Update expected counts from 76 runtime rows, 69 Oracle-complete rows, and 72
    closed rows to 77 runtime rows, 70 Oracle-complete rows, and 73 closed rows.
    Incomplete and gap counts remain 4.
  - Update references that describe the remaining macOS app GUI gap so they no
    longer require this child-process-proven AppleScript keyboard/mouse command
    slice.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add the experiment link and update Learnings after the result.

## Verification

Pass criteria:

- The built debug Roastty app launches from the absolute app bundle path.
- The guard uses an isolated config with `macos-applescript = true` and does not
  depend on the user's normal config.
- `send key` is issued to a controlled terminal and the child process records
  the exact expected bytes.
- `send mouse position`, `send mouse button`, and `send mouse scroll` are issued
  to a controlled terminal after mouse reporting is enabled.
- The mouse child process records terminal mouse-report bytes after the scripted
  mouse commands.
- The guard fails if the child marker files are missing, contain unexpected key
  bytes, or contain no mouse-report bytes.
- The live guard still fails if a new Roastty crash report appears during the
  workflow.
- The new runtime inventory row is `Oracle complete`.
- `RUNTIME-011B2B` remains `Gap` for native menu display/validation,
  titlebar/fullscreen/quick-terminal visuals, screenshot/pixel evidence, broader
  command-palette GUI behavior, split visual/layout parity, cursor/pointer
  pixels, and broader keyboard/mouse walkthroughs.
- CFG-223 remains `Gap`.

Commands:

```bash
(cd roastty && macos/build.nu --action build)
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_applescript_workflow_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/platform_runtime_classification.py --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/platform-runtime-classification.md
for f in issues/0805-roastty-ghostty-parity/*_runtime_parity.py issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py issues/0805-roastty-ghostty-parity/link_hover_preview_dispatch_parity.py issues/0805-roastty-ghostty-parity/link_hover_modifier_refresh_parity.py issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py; do
  PYTHONDONTWRITEBYTECODE=1 python3 "$f"
done
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/171-applescript-keyboard-mouse-events.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

Fail criteria:

- The guard treats AppleScript command success as sufficient without checking
  child-process output files.
- The keyboard child output does not exactly match the expected bytes.
- Mouse commands are sent before mouse reporting is enabled.
- The mouse child output is empty or does not contain terminal mouse-report
  bytes.
- The guard depends on the user's normal config or leaves the debug app running.
- The inventory claims cursor pixels, pointer visuals, native menu,
  fullscreen/quick-terminal, screenshot, or broad keyboard/mouse GUI parity.
- CFG-223 is marked complete.

## Design Review

Adversarial review was performed by a fresh-context Codex subagent.

Verdict: Approved.

Findings: none.

The reviewer verified that the README links Experiment 171 as `Designed`, the
experiment has the required sections, the scope is narrow, the AppleScript
commands are present in `Roastty.sdef` and backed by current Swift handlers, the
verification requires child-process side effects for keyboard and mouse
delivery, the mouse check requires mouse reporting before scripted mouse
commands, hygiene checks are present, and the proposed runtime count delta is
consistent with CFG-223 remaining `Gap`.

## Result

**Result:** Pass

Experiment 171 implemented the live child-process proof for lower-level
AppleScript keyboard and mouse command delivery.

Changes:

- `roastty/macos/Sources/Features/AppleScript/ScriptKeyEventCommand.swift`
  - Fixed printable `send key` delivery by attaching `text` and
    `unshiftedCodepoint` for scriptable printable keys. Before this fix, the
    live keyboard child reached raw-mode readiness but `send key "a"` and
    `send key "b"` produced no PTY bytes.
- `issues/0805-roastty-ghostty-parity/macos_applescript_workflow_runtime.py`
  - Added raw-mode keyboard and mouse capture child programs in the existing
    isolated live debug-app workflow.
  - Added readiness markers so AppleScript commands are sent only after the
    children have configured raw input and, for mouse, terminal mouse reporting.
  - Proved `send key "a"` plus `send key "b"` by requiring exact raw `ab` bytes
    in the keyboard child output file.
  - Proved `send mouse position`, `send mouse button` press/release, and
    `send mouse scroll` by requiring the mouse child output file to grow after
    each scripted position, button press, drag-position, button release, and
    scroll phase once mouse reporting is enabled.
  - Kept the existing absolute app-bundle launch, isolated config, scoped
    cleanup, split-terminal lifecycle checks, and new-crash-report guard.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Added `RUNTIME-011B2E` as Oracle complete for live AppleScript keyboard and
    mouse command delivery.
  - Reduced the remaining `RUNTIME-011B2B` gap to broader GUI walkthrough,
    native menu/fullscreen/quick-terminal, visual/pixel, split visual/layout,
    broader command-palette GUI, and broader input walkthrough proof.
  - Updated CFG-223 count assertions to `77` runtime rows, `70` Oracle-complete
    rows, `73` closed rows, `4` incomplete rows, and `4` gap rows.
- Existing CFG-223 runtime guard scripts
  - Updated shared CFG-223 count expectations from `69` Oracle-complete / `72`
    closed rows to `70` Oracle-complete / `73` closed rows.
- Generated docs
  - Regenerated `config-runtime-inventory.md`, `config-matrix.md`, and
    `platform-runtime-classification.md`.

Verification:

```bash
(cd roastty && macos/build.nu --action build)
# passed; Xcode/SwiftLint accepted the AppleScript key-handler change.

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_applescript_workflow_runtime.py
# macos_applescript_workflow_runtime=pass

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
# runtime_rows=77
# oracle_complete=70
# closed=73
# audit_covered=0
# incomplete=4
# gap=4
# cfg223=Gap

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/platform_runtime_classification.py --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/platform-runtime-classification.md
# platform_options=32
# gap=15
# not_applicable=15
# oracle_complete=2

for f in issues/0805-roastty-ghostty-parity/*_runtime_parity.py issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py issues/0805-roastty-ghostty-parity/link_hover_preview_dispatch_parity.py issues/0805-roastty-ghostty-parity/link_hover_modifier_refresh_parity.py issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py; do
  PYTHONDONTWRITEBYTECODE=1 python3 "$f"
done
# all listed guards passed

PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_app_workflow_plumbing_parity.py
# macos_app_workflow_plumbing_parity=pass
```

## Conclusion

Lower-level live AppleScript keyboard and mouse command delivery is no longer
part of the remaining CFG-223 macOS app gap. The live guard now proves
side-effectful `send key`, `send mouse position`, `send mouse button`, and
`send mouse scroll` delivery against the actual debug Roastty app and controlled
PTY children.

CFG-223 remains `Gap` because unrelated GUI work still needs proof: native-menu
display/validation, titlebar/fullscreen/quick-terminal visuals, screenshot/pixel
evidence, broader command-palette GUI behavior, split visual/layout parity,
cursor/pointer pixels, and broader keyboard/mouse walkthrough parity.

## Completion Review

Adversarial completion review was performed by a fresh-context Codex subagent.

Initial verdict: Changes required.

Required finding:

- The first mouse guard could pass after any one terminal mouse report and
  therefore did not prove that `send mouse button` or `send mouse scroll`
  generated PTY-side bytes.

Fix:

- The mouse capture child now writes the output file after each read.
- The guard now sends mouse position, button press, drag-position, button
  release, and scroll in separate AppleScript phases.
- The guard now requires the output file length to grow after each phase.

Final verdict: Approved.

Final findings: none.
