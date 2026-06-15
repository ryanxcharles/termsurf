# Experiment 164: Metal Cursor Pixel Readback

## Description

`RUNTIME-008B2B2B2B2` still groups three renderer-visible gaps together: GUI
cursor pixels, broader GUI/pixel parity, and screenshot-level padding pixel
proof. Experiments 134 and 144 already proved the cursor render data and active
cursor-priority path, but they intentionally did not claim final pixel output.

Roastty already has focused Metal render-target readback tests for the text
shader cursor branch:

- a non-cursor glyph drawn at the cursor position is recolored from
  `uniforms.cursor_color`;
- a cursor glyph flagged with `IS_CURSOR_GLYPH` keeps its own vertex color;
- a wide block cursor recolors the second cell;
- a non-wide cursor does not recolor the second cell.

Pinned Ghostty uses the same Metal shader branch in
`vendor/ghostty/src/renderer/shaders/shaders.metal`, where a cell under
`cursor_pos` or `cursor_pos + 1` for wide cursors swaps non-cursor glyph color
to `uniforms.cursor_color` while preserving cursor glyph color.

This experiment will split the remaining renderer-visible row into:

- `RUNTIME-008B2B2B2B2A`: **Oracle complete** for deterministic Metal text
  shader cursor pixel readback.
- `RUNTIME-008B2B2B2B2B`: **Gap** for remaining renderer-visible GUI/pixel
  effects: actual app/GUI cursor screenshots, broader GUI/pixel parity, and
  screenshot-level padding pixel proof.

This experiment will not claim full app cursor screenshot parity. It only proves
that the copied Metal text shader cursor color branch produces the expected
target bytes in a deterministic render pass.

## Changes

- `issues/0805-roastty-ghostty-parity/metal_cursor_pixel_runtime_parity.py`
  - Add a static guard checking pinned Ghostty's Metal shader cursor markers:
    `cursor_pos`, `cursor_wide`, `IS_CURSOR_GLYPH`, and the
    `uniforms.cursor_color` replacement branch.
  - Check Roastty's matching Metal shader markers.
  - Check Roastty's existing readback tests:
    `cell_text_cursor_pos_overrides_non_cursor_glyph_color`,
    `cell_text_cursor_glyph_flag_preserves_vertex_color`,
    `cell_text_wide_cursor_overrides_second_cell`, and
    `cell_text_non_wide_cursor_does_not_override_second_cell`.
  - Check the inventory split, remaining gap wording, and CFG-223 counts.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-008B2B2B2B2` into the complete Metal cursor-pixel readback
    row and the reduced remaining renderer-visible GUI/pixel gap.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-223 summary. It must remain `Gap`.
- Existing CFG-223 guards that hard-code the remaining renderer visual gap row
  or counts
  - Update references from `RUNTIME-008B2B2B2B2` to `RUNTIME-008B2B2B2B2B` where
    they mean the remaining gap.
  - Update expected counts after the split.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add the experiment link and update Learnings after the result.

## Verification

Pass criteria:

- `RUNTIME-008B2B2B2B2A` is `Oracle complete` and cites deterministic Metal
  target-byte readback tests for cursor-position recolor, cursor-glyph color
  preservation, wide-cursor second-cell recolor, and non-wide second-cell
  non-recolor.
- The complete row explicitly limits itself to Metal text shader cursor pixel
  readback.
- `RUNTIME-008B2B2B2B2B` remains `Gap` and explicitly owns actual app/GUI cursor
  screenshots, broader GUI/pixel parity, and screenshot-level padding pixel
  proof.
- CFG-223 remains `Gap`.
- The focused Metal tests run on the VM and do not silently skip the cursor
  pixel proof.

Commands:

```bash
cargo test --manifest-path roastty/Cargo.toml cell_text_cursor -- --test-threads=1
cargo test --manifest-path roastty/Cargo.toml cell_text_wide_cursor_overrides_second_cell -- --test-threads=1
cargo test --manifest-path roastty/Cargo.toml cell_text_non_wide_cursor_does_not_override_second_cell -- --test-threads=1
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/metal_cursor_pixel_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
for f in issues/0805-roastty-ghostty-parity/*_runtime_parity.py issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py issues/0805-roastty-ghostty-parity/link_hover_preview_dispatch_parity.py issues/0805-roastty-ghostty-parity/link_hover_modifier_refresh_parity.py issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py; do PYTHONDONTWRITEBYTECODE=1 python3 "$f" >/tmp/$(basename "$f").out || { echo FAIL:$f; cat /tmp/$(basename "$f").out; exit 1; }; done; echo parity_guards=pass
cargo fmt --manifest-path roastty/Cargo.toml --check
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/164-metal-cursor-pixel-readback.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

Fail criteria:

- The experiment claims full GUI cursor screenshot parity, full renderer visual
  parity, or broad GUI/pixel parity from Metal unit readback tests.
- The remaining gap no longer owns actual app/GUI cursor screenshots, broader
  GUI/pixel parity, or screenshot-level padding pixel proof.
- The cursor pixel proof can be skipped on this VM without failing.
- CFG-223 is marked complete.

## Design Review

**Reviewer:** Codex adversarial subagent with fresh context.

**Initial verdict:** Changes required.

The reviewer found one required issue: the original verification command used
`cargo test --manifest-path roastty/Cargo.toml cell_text_cursor`, which only
matches the two tests whose names contain `cell_text_cursor`. It did not run
`cell_text_wide_cursor_overrides_second_cell` or
`cell_text_non_wide_cursor_does_not_override_second_cell`, even though the pass
criteria require the wide and non-wide second-cell proofs.

**Fix:** Updated Verification to run the wide and non-wide cursor tests
explicitly, in addition to the existing cursor-position and cursor-glyph tests.

**Re-review verdict:** Approved.

The reviewer confirmed the required verification-command issue was fixed and
approved the design for implementation.

## Result

**Result:** Pass

Implemented the narrow renderer-visible split for deterministic Metal cursor
pixel readback:

- Added `metal_cursor_pixel_runtime_parity.py` to guard pinned Ghostty and
  Roastty Metal shader cursor markers, the four Roastty target-byte readback
  tests, the new inventory rows, and CFG-223 counts.
- Split `RUNTIME-008B2B2B2B2` into:
  - `RUNTIME-008B2B2B2B2A`: **Oracle complete** for Metal text shader cursor
    pixel readback.
  - `RUNTIME-008B2B2B2B2B`: **Gap** for actual app/GUI cursor
    pixels/screenshots, broader GUI/pixel parity, and screenshot-level padding
    pixel proof.

Verification passed:

```bash
cargo test --manifest-path roastty/Cargo.toml cell_text_cursor -- --test-threads=1
cargo test --manifest-path roastty/Cargo.toml cell_text_wide_cursor_overrides_second_cell -- --test-threads=1
cargo test --manifest-path roastty/Cargo.toml cell_text_non_wide_cursor_does_not_override_second_cell -- --test-threads=1
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/metal_cursor_pixel_runtime_parity.py
for f in issues/0805-roastty-ghostty-parity/*_runtime_parity.py issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py issues/0805-roastty-ghostty-parity/link_hover_preview_dispatch_parity.py issues/0805-roastty-ghostty-parity/link_hover_modifier_refresh_parity.py issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py; do PYTHONDONTWRITEBYTECODE=1 python3 "$f" >/tmp/$(basename "$f").out || { echo FAIL:$f; cat /tmp/$(basename "$f").out; exit 1; }; done; echo parity_guards=pass
python3 -m py_compile issues/0805-roastty-ghostty-parity/metal_cursor_pixel_runtime_parity.py issues/0805-roastty-ghostty-parity/config_runtime_inventory.py
cargo fmt --manifest-path roastty/Cargo.toml --check
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/164-metal-cursor-pixel-readback.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

The regenerated inventory reported:

```text
runtime_rows=71
oracle_complete=64
closed=67
audit_covered=0
incomplete=4
gap=4
cfg223=Gap
```

## Conclusion

The cursor shader's deterministic Metal pixel output is now guarded without
claiming full GUI cursor parity. CFG-223 remains `Gap`; the reduced renderer
visual row now owns actual app/GUI cursor pixels/screenshots, broader GUI/pixel
parity, and screenshot-level padding pixel proof.

## Completion Review

**Reviewer:** Codex adversarial subagent with fresh context.

**Initial verdict:** Changes required.

The reviewer found one documentation issue: the Result verification block
omitted the already-run `cargo fmt --check`, Prettier, and `git diff --check`
hygiene commands even though those commands were part of the designed
verification.

**Fix:** Added the missing hygiene commands to the Result verification block.

**Re-review verdict:** Approved.

The reviewer confirmed the missing verification commands were recorded and
approved the completed experiment for the result commit.
