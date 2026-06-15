# Experiment 175: macOS Split Layout Runtime

## Description

`RUNTIME-011B2B` still owns live macOS split visual/layout effects after earlier
experiments split out AppleScript split lifecycle, native menu behavior,
fullscreen, command-palette visibility, and Quick Terminal geometry. Experiment
170 proved that AppleScript can create, focus, resolve, and close split terminal
objects, but it did not prove that the live window visibly lays out split panes.

This experiment will split out one focused live GUI row: a right split in the
debug Roastty app must produce two visible terminal regions in the same
CoreGraphics window. The guard will make that measurable by running controlled
commands that paint the primary pane red and the split pane blue, capturing the
exact PID-owned window, and sampling the resulting screenshot.

This experiment will not claim titlebar visual parity, cursor/pointer pixels,
broad renderer screenshot parity, down/left/up split variants, resize behavior,
or the full GUI walkthrough.

## Changes

- `issues/0805-roastty-ghostty-parity/macos_split_layout_runtime.py`
  - Add a live debug-app guard using the established macOS launch discipline:
    absolute `roastty/macos/build/Debug/Roastty.app`, isolated config,
    `ROASTTY_CLEAR_USER_DEFAULTS=1`, a unique user-defaults suite, exact
    launched Unix PID targeting, scoped cleanup, and new-crash-report failure.
  - Configure deterministic app behavior with `macos-applescript = true`,
    `quit-after-last-window-closed = true`, cursor blink disabled, and a fixed
    window size large enough for two panes.
  - Create a primary terminal window through AppleScript with a command that
    paints a stable red ANSI background and sleeps long enough for screenshot
    capture.
  - Split the focused terminal `direction right` with a second surface
    configuration whose command paints a stable blue ANSI background and sleeps.
  - Require AppleScript object evidence that the selected tab contains two
    terminals after the split and that both terminal ids are non-empty.
  - Resolve the launched app's front PID-owned CoreGraphics layer-0 window id
    and capture that exact window id with `screencapture -x -o -l{window_id}`.
  - Add screenshot pixel sampling, implemented with a small local helper or
    inline Swift/AppKit reader, that samples grids inside stable visible
    left-pane and right-pane regions while avoiding the titlebar, divider,
    window edges, and renderer wrap gaps.
  - Treat split visual layout as proven only if:
    - the screenshot dimensions are nonzero and tied to the same exact
      CGWindowID observed after split creation;
    - at least 70% of sampled left-third pixels are red-dominant, using a
      tolerance that permits text antialiasing and compositor effects;
    - at least 70% of sampled right-third pixels are blue-dominant;
    - neither sampled region can pass as the other color;
    - the app process remains alive and no new `roastty-*.ips` crash report
      appears during the run.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Add a new Oracle-complete macOS app row for live right-split visual/layout
    proof if the guard passes.
  - Reduce `RUNTIME-011B2B` so it no longer owns the focused right-split visual
    layout evidence.
  - Keep `RUNTIME-011B2B` open for titlebar visuals, broader split variants and
    interactions, cursor/pointer pixels, and broader GUI walkthrough effects.
  - Update CFG-223 counts only if the new row is added and passing; CFG-223 must
    remain `Gap`.
- Existing CFG-223 guard scripts
  - Update only runtime-row, Oracle-complete, closed-row, and remaining-gap text
    that becomes stale after the split row is split out.
- Generated docs
  - Regenerate `config-runtime-inventory.md`, `config-matrix.md`, and
    `platform-runtime-classification.md`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Keep the Experiment 175 line at `Designed` until implementation and result
    review complete.
  - Add a learning only if the experiment discovers a reusable GUI automation or
    screenshot-sampling constraint.

## Verification

- Build the debug app:

```bash
(cd roastty && macos/build.nu --action build)
```

- Run the new live guard:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_split_layout_runtime.py
```

- Regenerate CFG-223 inventory and matrix:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
```

- Regenerate platform runtime classification:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/platform_runtime_classification.py --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/platform-runtime-classification.md
```

- Run the existing CFG-223 guard set:

```bash
for f in issues/0805-roastty-ghostty-parity/*_runtime_parity.py issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py issues/0805-roastty-ghostty-parity/link_hover_preview_dispatch_parity.py issues/0805-roastty-ghostty-parity/link_hover_modifier_refresh_parity.py issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py; do
  PYTHONDONTWRITEBYTECODE=1 python3 "$f"
done
```

- Run the macOS app guards that reference the macOS app gap:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_app_workflow_plumbing_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_applescript_workflow_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_native_menu_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_gui_state_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_quick_terminal_runtime.py
```

- Format and hygiene:

```bash
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/175-macos-split-layout-runtime.md \
  issues/0805-roastty-ghostty-parity/config-runtime-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md \
  issues/0805-roastty-ghostty-parity/platform-runtime-classification.md
git diff --check
```

Pass criteria:

- The new guard passes only after proving exact-PID targeting, scoped cleanup,
  no new crash report, two AppleScript terminal objects in the selected tab,
  exact-CGWindowID screenshot capture, and red/blue sampled regions in opposite
  halves of the live window.
- The sampled-color oracle cannot pass if the screenshot is blank, captures the
  wrong window, captures only one pane, or swaps both halves to the same color.
- Generated CFG-223 counts are internally consistent.
- CFG-223 remains `Gap`.
- `RUNTIME-011B2B` remains open and still lists titlebar visuals, broader split
  interactions/variants, cursor/pointer pixels, and broader GUI walkthrough
  effects.

Fail criteria:

- The guard can pass using only AppleScript object counts without screenshot
  evidence.
- The guard can pass without sampling both sides of the exact captured window.
- The guard relies on an installed app, ambiguous process name, or PID-only
  screenshot selection instead of the launched debug app and exact CGWindowID.
- CFG-223 is marked complete.
- The experiment claims full split parity, titlebar parity, pointer/cursor
  parity, broad renderer pixel parity, or full GUI walkthrough parity without
  directly proving those behaviors.

## Design Review

Fresh-context adversarial reviewer `Peirce the 3rd` reviewed the design and
returned `APPROVED` with no Required, Optional, or Nit findings.

Evidence checked by the reviewer:

- The issue README links Experiment 175 with status `Designed`.
- The experiment has Description, Changes, and Verification sections.
- The scope is focused on the remaining live right-split visual/layout slice and
  explicitly avoids titlebar, cursor/pointer, broad renderer, other split
  directions, resize, and full walkthrough claims.
- The plan follows the established macOS helper discipline: absolute debug app,
  isolated config/defaults, launched PID targeting, exact CGWindowID capture,
  scoped cleanup, and crash-report failure.
- Verification has concrete pass/fail criteria for the same-window right split:
  two AppleScript terminals, exact window capture, nonzero screenshot, left
  red/right blue pixel dominance, cross-color rejection, app alive, and no new
  crash report.
- Hygiene checks are present: regeneration commands, the existing guard suite,
  macOS guard reruns, prettier formatting, and `git diff --check`.

Final design verdict: **Approved**.

## Result

**Result:** Pass

Experiment 175 implemented and verified a focused live right-split visual layout
guard.

Changes:

- `issues/0805-roastty-ghostty-parity/macos_split_layout_runtime.py`
  - Added a live debug-app guard using the absolute `Roastty.app` bundle,
    isolated config, exact launched Unix PID targeting, scoped cleanup, and
    new-crash-report detection.
  - Creates a primary terminal whose controlled child process repeatedly paints
    a red truecolor ANSI background, then creates a `direction right` split
    whose controlled child process repeatedly paints a blue truecolor ANSI
    background.
  - Waits for marker files proving both child painter processes started.
  - Requires the selected tab to contain exactly two non-empty terminal IDs.
  - Captures the exact PID-owned layer-0 CoreGraphics window with
    `screencapture -x -o -l{window_id}`.
  - Samples the captured PNG with a temporary Swift/AppKit helper and requires
    red-dominant samples in the left pane, blue-dominant samples in the right
    pane, and cross-color rejection so both sides cannot pass as the same color.
  - Saves the latest debug screenshot and metrics to
    `/tmp/termsurf-issue805-exp175-split-layout.png` and
    `/tmp/termsurf-issue805-exp175-split-layout.json` for failure diagnosis.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Added `RUNTIME-011B2I` for focused live right-split visual layout proof.
  - Narrowed `RUNTIME-011B2B` so the remaining split gap is broader split
    variants and interactions, not the focused right-split visual proof.
- Generated docs
  - Regenerated `config-runtime-inventory.md`, `config-matrix.md`, and
    `platform-runtime-classification.md`.
- Existing CFG-223 guard scripts
  - Updated CFG-223 expected counts from 73/76 to 74/77 and replaced stale broad
    split-gap assertions with the remaining broader split gap.

Verification run:

```bash
(cd roastty && macos/build.nu --action build)
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_split_layout_runtime.py
```

The live guard passed:

```text
macos_split_layout_runtime=pass left_terminal=7BDBB864-F693-4560-91BC-361BFA3E48AF right_terminal=C9EA3360-DAF2-4F51-835A-454C4FDA148F
```

The regenerated CFG-223 counts are:

```text
runtime_rows=81
oracle_complete=74
closed=77
incomplete=4
gap=4
cfg223=Gap
```

## Conclusion

Focused right-split visual layout is now proven in the live macOS app: two
AppleScript terminal objects are created in one tab, the same PID-owned window
is captured by exact CoreGraphics window id, and the captured image contains
distinct red and blue regions in the expected split panes. `RUNTIME-011B2B`
remains open for titlebar visuals, broader split variants/interactions,
cursor/pointer pixels, broader screenshot/pixel parity, and broader input
walkthrough effects.

## Result Review

Fresh-context adversarial reviewer `Lovelace the 3rd` reviewed the completed
experiment and returned `APPROVED` with no findings.

Evidence checked by the reviewer:

- The working tree contains the result edits and the result commit had not been
  made before review.
- The issue README marks Experiment 175 as `Pass`.
- The experiment file has `## Result` and `## Conclusion`.
- CFG-223 remains `Gap` with `81` runtime rows, `74` Oracle-complete rows, `77`
  closed rows, `4` incomplete rows, and `4` gap rows.
- `RUNTIME-011B2B` remains open for broader GUI/titlebar/split/input/pixel work,
  while `RUNTIME-011B2I` is scoped to focused right-split visual layout.
- The guard requires two AppleScript terminal IDs, exact PID-owned CGWindowID
  capture, and red/blue screenshot sampling, so it cannot pass on object counts
  alone.

Read-only checks run by the reviewer:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py
git diff --check -- issues/0805-roastty-ghostty-parity
```

Final result verdict: **Approved**.
