# Experiment 179: Renderer Visual Residual Audit

## Description

`RUNTIME-008B2B2B2B2B` is now the only renderer-family CFG-223 gap, but its
remaining claim is intentionally vague: "broader GUI/pixel parity." Experiments
125, 133, 134, 144, 148, 151, 154, 163, 164, 177, and 178 already split out the
concrete renderer control, renderer option, cursor, padding, opacity, shader,
and focused live screenshot slices found in pinned Ghostty's renderer and macOS
host paths.

This experiment will audit the renderer residual bucket against pinned Ghostty
renderer, shader, config, surface, and macOS host sources and either:

- close the residual renderer row if every config-driven renderer-visible effect
  in the pinned Ghostty renderer/macOS-render-host paths is already represented
  by an oracle-complete inventory row or by a different still-open non-renderer
  row; or
- replace the vague residual row with one or more concrete follow-up rows for
  any renderer-visible config behavior still lacking proof.

The scope is renderer-visible output only. Broad font output parity remains in
`RUNTIME-007B2B2B2B2`, broader live macOS app walkthrough/titlebar/split
behavior remains in `RUNTIME-011B2B`, and native notification/link/bell GUI
effects remain in `RUNTIME-012B2B2B2B2B3`.

## Changes

- `issues/0805-roastty-ghostty-parity/renderer_visual_residual_audit.py`
  - Add a static guard that reads pinned Ghostty renderer, shader, surface,
    config, and macOS host sources.
  - Enumerate known config-driven renderer-visible effects and map each to an
    oracle-complete row or to one of the remaining non-renderer gap rows.
  - Assert the mapping covers renderer control and rebuild scheduling,
    renderer-sourced visual knobs, background opacity/cell opacity,
    window-padding layout and padding pixels, cursor render data and live cursor
    pixels, macOS glass and non-glass opacity host behavior, and custom/cursor
    shader pixel readback.
  - Assert font-renderer output, macOS walkthrough/titlebar/split workflows, and
    notification/link/bell UI effects are not counted as closure for the
    renderer residual row.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - If the audit finds no uncovered renderer-visible config behavior, mark
    `RUNTIME-008B2B2B2B2B` as `Oracle complete` with evidence from the new guard
    and explain that remaining CFG-223 gaps are font, macOS walkthrough, and
    notification/link/bell GUI gaps.
  - If the audit finds a real uncovered renderer-visible behavior, split the
    residual row into concrete rows instead of closing it.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-223 counts from the inventory script.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning recording whether the broad renderer residual row was closed
    or split and why.

No Roastty source code should change in this experiment. If the audit finds a
concrete renderer-visible parity bug, record it as a concrete remaining row and
leave implementation for the next experiment.

## Verification

Pass criteria:

- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/renderer_visual_residual_audit.py`
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
- Existing CFG-223 guard set:
  - `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/renderer_control_runtime_parity.py`
  - `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/window_padding_layout_runtime_parity.py`
  - `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/cursor_renderer_runtime_parity.py`
  - `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/cursor_priority_runtime_parity.py`
  - `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_glass_visual_runtime_parity.py`
  - `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/non_glass_opacity_runtime_parity.py`
  - `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/custom_shader_output_runtime_parity.py`
  - `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/metal_cursor_pixel_runtime_parity.py`
  - `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_window_padding_pixel_runtime.py`
  - `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_gui_cursor_pixel_runtime.py`
- `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py`
- `prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/179-renderer-visual-residual-audit.md issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
- `git diff --check`

The experiment passes only if the renderer residual row is no longer vague:
either the guard proves all pinned Ghostty config-driven renderer-visible fields
are covered by completed rows or intentionally owned by a different remaining
gap, or the inventory records the exact uncovered renderer-visible behavior that
remains. CFG-223 may still remain a gap because the font, macOS walkthrough, and
notification/link/bell GUI rows are outside this experiment.

## Design Review

Fresh-context adversarial design review initially returned **Changes required**:

- the verification listed
  `issues/0805-roastty-ghostty-parity/macos_non_glass_opacity_runtime_parity.py`,
  but the actual guard is
  `issues/0805-roastty-ghostty-parity/non_glass_opacity_runtime_parity.py`.

The verification command was corrected.

Re-review returned **Approved** with no new required findings.

## Result

**Result:** Pass

The vague renderer residual row is now narrowed to a concrete remaining renderer
gap. The new `renderer_visual_residual_audit.py` guard reads pinned Ghostty's
renderer, shader, surface, config, and macOS render-host sources and maps the
already-proven renderer-visible effects to existing oracle-complete rows. The
audit covers renderer control/rebuild behavior, renderer visual knobs,
background opacity/cell opacity, window-padding layout and live padding pixels,
cursor render data and live cursor pixels, macOS glass and non-glass opacity
host behavior, custom shader output, and Metal cursor shader pixel readback.

The completion review found real remaining renderer-visible behavior not covered
by completed rows: `custom-shader-animation` focus/always/false draw-timer
policy, background image rendering plus `background-image-opacity`,
`background-image-position`, `background-image-fit`, `background-image-repeat`,
`window-colorspace`, `alpha-blending`, and `scroll-to-bottom.output`. The
inventory now records those exact missing renderer behaviors instead of the
prior vague broader GUI/pixel bucket.

No Roastty source code changed. CFG-223 remains a gap overall with the same
number of runtime gaps, but one of those gaps is now concrete enough to drive
the next renderer experiment.

Verification completed:

- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/renderer_visual_residual_audit.py`
  — pass.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  — pass: `runtime_rows=84`, `oracle_complete=77`, `closed=80`,
  `audit_covered=0`, `incomplete=4`, `gap=4`, `cfg223=Gap`.
- Static CFG-223 renderer guards — pass: `renderer_control_runtime_parity.py`,
  `window_padding_layout_runtime_parity.py`,
  `cursor_renderer_runtime_parity.py`, `cursor_priority_runtime_parity.py`,
  `macos_glass_visual_runtime_parity.py`, `non_glass_opacity_runtime_parity.py`,
  `custom_shader_output_runtime_parity.py`, and
  `metal_cursor_pixel_runtime_parity.py`.
- Live CFG-223 renderer screenshot guards — pass:
  `macos_window_padding_pixel_runtime.py` and
  `macos_gui_cursor_pixel_runtime.py`.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py`
  — pass.
- `rg -n "Add renderer/runtime or GUI smoke rows for broader GUI/pixel parity" issues/0805-roastty-ghostty-parity/*.py issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
  — pass with no matches.

## Conclusion

The renderer residual was an under-specified audit bucket after the earlier
concrete renderer slices had already been split out. It was not empty:
`custom-shader-animation`, background image rendering/options,
`window-colorspace`, `alpha-blending`, and `scroll-to-bottom.output` still need
focused renderer proof. The next renderer experiment should start with those
specific behaviors instead of a broad GUI/pixel parity claim.

## Completion Review

Fresh-context adversarial completion review initially returned **Changes
required**:

- `custom-shader-animation` was detected in pinned Ghostty's renderer thread
  config, but the first result incorrectly mapped it to the completed custom
  shader output/readback row even though focus/always/false draw-timer behavior
  was not proven.
- The first result did not account for pinned Ghostty renderer-visible
  `background-image*`, `window-colorspace`, `alpha-blending`, and
  `scroll-to-bottom.output` fields in `renderer/generic.zig`.

The result was corrected to keep `RUNTIME-008B2B2B2B2B` as a `Gap` and narrow it
to those concrete remaining renderer-visible behaviors instead of closing the
row.

Re-review returned **Approved**. The reviewer confirmed both required findings
were resolved and found no new required findings.
