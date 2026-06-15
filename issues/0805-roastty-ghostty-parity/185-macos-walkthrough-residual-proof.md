# Experiment 185: macOS walkthrough residual proof

## Description

`RUNTIME-011B2B` is one of the two remaining CFG-223 runtime/UI gaps. Earlier
experiments already proved copied macOS workflow plumbing, live AppleScript
window/tab/split/input automation, split terminal object lifecycle, keyboard and
mouse side effects, native menu behavior, fullscreen screenshots, command
palette visibility, Quick Terminal geometry, right-split visual layout, hidden
titlebar traffic-light absence, window padding pixels, and GUI cursor pixels.

The remaining row still groups broader live macOS app concerns:

- broader titlebar behavior beyond hidden-titlebar traffic-light proof;
- broader split variants and interactions beyond right-split visual proof;
- screenshot/pixel evidence beyond the focused fullscreen, command-palette,
  Quick Terminal, right-split, hidden-titlebar, padding, and cursor guards;
- broader input navigation walkthrough evidence.

This experiment will turn `RUNTIME-011B2B` into a final macOS walkthrough
residual proof if the existing and added live guards are sufficient. If a
sub-area is still genuinely unproven, the implementation will split that
remaining sub-area into an explicit smaller macOS row instead of hiding it.

This experiment will not claim notification/link/bell GUI parity. That remains
owned by `RUNTIME-012B2B2B2B2B3`.

## Changes

- Audit the live macOS app evidence already present in:
  - `macos_applescript_workflow_runtime.py`;
  - `macos_split_layout_runtime.py`;
  - `macos_titlebar_runtime.py`;
  - `macos_gui_state_runtime.py`;
  - `macos_quick_terminal_runtime.py`;
  - `macos_native_menu_runtime.py`;
  - `macos_gui_cursor_pixel_runtime.py`;
  - `macos_window_padding_pixel_runtime.py`.
- Add a new focused guard,
  `issues/0805-roastty-ghostty-parity/macos_walkthrough_residual_parity.py`, to
  bind the final `RUNTIME-011B2B` decision to concrete live guards, source
  anchors, screenshot artifacts, side-effect markers, and CFG-223 counts.
- Add live coverage only where the audit finds a concrete missing macOS app
  walkthrough slice. Candidate missing slices are:
  - tabs titlebar/proxy-icon behavior;
  - non-right split directions or split focus/equalize/resize interactions;
  - keyboard navigation across window/tab/split UI;
  - exact-window screenshot evidence for any remaining titlebar/split state.
- Update `config_runtime_inventory.py`, generated `config-runtime-inventory.md`,
  `config-matrix.md`, and any stale macOS guard count assertions. The successful
  closure path should move CFG-223 from 2 gaps to 1 gap, leaving only
  `RUNTIME-012B2B2B2B2B3`.
- Update `README.md` Learnings and Experiments index.

## Verification

Pass criteria:

- `RUNTIME-011B2B` is either:
  - `Oracle complete`, with evidence for broader titlebar, split, screenshot,
    and input walkthrough behavior; or
  - split into an explicit smaller remaining macOS row with exact missing
    evidence if the audit proves a real residual remains.
- CFG-223 reports 87 runtime rows, 83 Oracle-complete rows, 86 closed rows, 1
  incomplete row, and 1 runtime gap on the successful closure path.
- The remaining CFG-223 gap is only `RUNTIME-012B2B2B2B2B3`.
- If the experiment proves only a smaller macOS slice and must keep a macOS
  residual open, the split path must:
  - add a new adjacent Oracle-complete macOS row, starting at `RUNTIME-011B2K`;
  - narrow `RUNTIME-011B2B` to the exact remaining macOS behavior instead of
    preserving broad wording;
  - report 88 runtime rows, 83 Oracle-complete rows, 86 closed rows, 2
    incomplete rows, and 2 runtime gaps;
  - list exactly two remaining gaps: the narrowed `RUNTIME-011B2B` and
    `RUNTIME-012B2B2B2B2B3`;
  - add guard commands for the new complete row and the narrowed remaining gap.
- Existing macOS guards no longer assert stale `82/85/2/2` or older CFG-223
  counts after the macOS row is closed or split.

Commands:

```bash
(cd roastty && macos/build.nu --action build)
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_walkthrough_residual_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_applescript_workflow_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_split_layout_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_titlebar_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_gui_state_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_quick_terminal_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_native_menu_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_gui_cursor_pixel_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_window_padding_pixel_runtime.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
python3 -m py_compile issues/0805-roastty-ghostty-parity/macos_walkthrough_residual_parity.py
prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/185-macos-walkthrough-residual-proof.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Fresh-context Codex adversarial reviewer `Kepler the 3rd` reviewed the initial
design and returned `VERDICT: CHANGES REQUIRED` with two required findings:

- the verification commands omitted `macos_gui_cursor_pixel_runtime.py` and
  `macos_window_padding_pixel_runtime.py`, even though the design relies on
  their stale CFG-223 count assertions being updated;
- the split fallback path did not define concrete CFG-223 count and remaining
  gap expectations.

The design was updated to include the omitted guards and to define the split
path as a new adjacent Oracle-complete macOS row starting at `RUNTIME-011B2K`,
with 88 runtime rows, 83 Oracle-complete rows, 86 closed rows, 2 incomplete
rows, and 2 runtime gaps. Fresh-context Codex re-reviewer `Carson the 3rd`
reviewed the fixes and returned `VERDICT: APPROVED` with no findings.
