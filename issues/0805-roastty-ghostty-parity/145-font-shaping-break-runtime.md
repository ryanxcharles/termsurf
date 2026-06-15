# Experiment 145: Font Shaping Break Runtime

## Description

`RUNTIME-007B2` still groups remaining font renderer output effects. One
deterministic slice in that gap is `font-shaping-break`: pinned Ghostty stores
the config in renderer `DerivedConfig`, computes a row-local `cursor_x` from the
cursor viewport, and then calls `run_iter_opts.applyBreakConfig` before shaping
that row. The only pinned flag is `cursor`; when disabled, the cursor no longer
splits shaping runs.

Roastty already has the value-level pieces: `FontShapingBreak`, parser/formatter
oracles, `RunOptions::apply_break_config`, and run-iterator tests showing that
`cursor_x` splits runs. The remaining runtime gap is that active frame row
formatting currently consumes `terminal.shape_run_options()` without applying
the config-derived `font_shaping_break`, so the live renderer path cannot prove
that `font-shaping-break = no-cursor` affects shaping input.

This experiment will split `RUNTIME-007B2`:

- `RUNTIME-007B2A`: **Oracle complete** for deterministic `font-shaping-break`
  cursor-run break behavior through active frame row formatting.
- `RUNTIME-007B2B`: **Gap** for remaining font renderer output effects: OpenType
  feature/variation effects, thicken/thicken-strength rendering, metric
  adjustment, fallback/shaping visual output, glyph metrics as seen by the
  renderer, and broader font pixel parity.

This experiment will not claim OpenType feature or variation parity, font
thickening pixels, metric adjustment, fallback visual output, glyph metric pixel
parity, or broader GUI font rendering parity.

## Changes

- `roastty/src/renderer/frame_rebuild.rs`
  - Add `font_shaping_break: FontShapingBreak` to the row-format input that
    feeds `FrameRebuildPlan::format_rows`.
  - Apply `RunOptions::apply_break_config(input.font_shaping_break)` to the
    row-local shaping options immediately before rebuilding each row, matching
    pinned Ghostty's renderer-side application after cursor viewport derivation.
  - Preserve default behavior by using `FontShapingBreak::default()` in existing
    helper/test inputs.
  - Add focused frame-rebuild tests proving `cursor` keeps the cursor split and
    `no-cursor` removes it before row shaping.
- `roastty/src/renderer/frame_renderer.rs`
  - Thread `config.font_shaping_break` from `FrameRenderKnobs::from_config` into
    the row-format input used by active `render_frame` and presenting render
    paths.
  - Keep existing frame-render tests passing with default cursor-break behavior.
  - Add a focused active-frame test proving `font-shaping-break = no-cursor`
    reaches the frame rebuild input and removes the cursor run break.
- `issues/0805-roastty-ghostty-parity/font_shaping_break_runtime_parity.py`
  - Add a guard checking pinned Ghostty's `DerivedConfig.font_shaping_break`,
    `run_iter_opts.applyBreakConfig`, Roastty's row-format input wiring,
    `FrameRenderKnobs::from_config`, focused tests, and the inventory split.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-007B2` into `RUNTIME-007B2A` and `RUNTIME-007B2B`.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-223 summary. It must remain `Gap`.
- Existing CFG-223 static guards that hard-code current runtime row counts or
  the remaining font gap row
  - Update expected counts after the split.
  - Update references from `RUNTIME-007B2` to `RUNTIME-007B2B` where they mean
    the remaining font renderer gap.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add the experiment link and update Learnings after the result.

## Verification

Pass criteria:

- Pinned Ghostty evidence shows `font-shaping-break` is renderer-derived config
  and is applied to row run options after cursor viewport-derived `cursor_x`.
- Roastty active row formatting applies `FontShapingBreak` in the renderer row
  formatting path, not by mutating terminal state.
- Default `font-shaping-break = cursor` preserves existing cursor-run splitting.
- `font-shaping-break = no-cursor` removes cursor-run splitting before shaping.
- Active frame render input sources `font_shaping_break` from `Config`.
- `RUNTIME-007B2A` is Oracle complete and cites frame-rebuild, active-frame, and
  static guard evidence.
- `RUNTIME-007B2B` remains `Gap` for the remaining font renderer output and font
  pixel parity effects.
- `CFG-223` remains `Gap`.

Commands:

```bash
cargo test --manifest-path roastty/Cargo.toml font_shaping_break_runtime
cargo test --manifest-path roastty/Cargo.toml apply_break_config_clears_cursor_x_when_off
cargo test --manifest-path roastty/Cargo.toml next_breaks_on_cursor
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/font_shaping_break_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
for f in issues/0805-roastty-ghostty-parity/*_runtime_parity.py; do PYTHONDONTWRITEBYTECODE=1 python3 "$f" >/tmp/$(basename "$f").out || { echo FAIL:$f; cat /tmp/$(basename "$f").out; exit 1; }; done; echo all_runtime_parity_guards=pass
cargo fmt --manifest-path roastty/Cargo.toml
cargo fmt --manifest-path roastty/Cargo.toml --check
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/145-font-shaping-break-runtime.md
git diff --check
```

Fail criteria:

- The implementation changes terminal-owned `shape_run_options()` semantics
  instead of applying the break config at renderer row-format time.
- `font-shaping-break = no-cursor` does not remove the cursor run break on the
  active frame row-format path.
- Default cursor-break behavior regresses.
- The experiment promotes OpenType feature/variation effects, thickening pixels,
  metric adjustment, fallback visual output, glyph metric pixel parity, or
  broader GUI font parity from the remaining gap.
- CFG-223 is marked complete.

## Design Review

**Reviewer:** Codex adversarial subagent with fresh context.

**Verdict:** Approved.

The reviewer found no findings. The reviewer verified the design and referenced
code paths read-only, and did not run mutating format commands.
