+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "default"
reasoning = "medium"

[review.result]
agent = "codex"
model = "default"
reasoning = "medium"
+++

# Experiment 818: Drive Row Formatting

## Description

Connect the frame rebuild driver from Experiment 817 to Roastty's existing row
formatting helpers. `cell.rs` already has `rebuild_bg_row`, `rebuild_row`, and
`rebuild_viewport`, but `rebuild_viewport` formats every supplied row and does
not use the frame rebuild plan. The missing step is a tested wrapper that takes
a `FrameRebuildPlan`, per-row `RunOptions` inputs, and renderer formatting
state, then drives only the planned rows through
`clear -> mark clean -> rebuild row`.

This experiment still does not collect live terminal render state, compute
search/link highlights, draw cursor/preedit glyphs, upload GPU buffers, submit
Metal frames, pace redraws, or add the renderer thread. It makes prepared
row-formatting inputs executable through the frame rebuild plan.

## Changes

- `roastty/src/renderer/frame_rebuild.rs`
  - Add `FrameRowFormatInput<'a>` with:
    - `rows: &'a [RunOptions]`,
    - `highlights: &'a [Vec<Highlight>]`,
    - `link_ranges: &'a [Vec<[u16; 2]>]`,
    - selection/search color config,
    - default foreground/background colors,
    - palette and bold-color config,
    - alpha/faint opacity,
    - font thicken config, and
    - background opacity config.
  - Add `FrameRowFormatValidationError` for:
    - plan validation errors from `drive_row_rebuilds`,
    - missing row data for a planned row, and
    - a planned row whose `RunOptions.cells.len()` does not exactly match the
      effective grid column count.
  - Add `FrameRowRenderError` for row-local glyph/font render errors from the
    row-formatting callback.
  - Add
    `FrameRebuildPlan::format_rows(&self, contents: &mut Contents, grid: &mut SharedGrid, row_dirty: &mut [bool], input: FrameRowFormatInput<'_>) -> Result<FrameRowRebuildApplication<FrameRowRenderError>, FrameRowFormatValidationError>`.
  - Validate all prepared row data before mutation:
    - every `rows_to_rebuild` row must exist in `input.rows`, and
    - every planned row's cell count must exactly match
      `self.effective_grid.columns`, matching upstream's grid-width row slices.
  - Inside `format_rows`, call `drive_row_rebuilds` and use the callback to
    format each planned row:
    - select that row's highlights/link ranges,
    - map the plan's own `preedit_range` to `PreeditSkip` for only that row,
    - call `rebuild_bg_row`,
    - shape the row with `font::run::shape_row_cached`, and
    - call `rebuild_row`.
  - Treat row formatting failures the same way as upstream row errors through
    the driver: the callback error is recorded as a failed row and later rows
    continue. Validation errors still abort before mutation.
  - Add tests proving:
    - a partial plan formats only dirty/planned rows and preserves unplanned row
      contents,
    - full rebuild formats every row after the reset,
    - missing planned rows and wrong row widths abort before mutation,
    - highlights, links, and the plan-owned preedit mask are threaded into the
      row formatter for the planned row,
    - row dirty flags are marked clean for formatted and failed rows, matching
      upstream's mark-before-rebuild order, and
    - validation failures still avoid all mutation.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the renderer tracker to mention that prepared
    row-formatting inputs can rebuild planned `Contents` rows, while live
    terminal-state collection, cursor/preedit glyph emission, GPU upload/draw
    calls, pacing, and renderer-thread integration remain open.

## Verification

- Inspect:
  - `vendor/ghostty/src/renderer/generic.zig` `rebuildCells`
  - `roastty/src/renderer/frame_rebuild.rs`
  - `roastty/src/renderer/cell.rs`
  - `roastty/src/font/run.rs`
- Run:
  - `cargo fmt -p roastty`
  - `cargo test -p roastty renderer::frame_rebuild -- --nocapture`
  - `cargo test -p roastty renderer::cell::tests::rebuild_viewport -- --nocapture`
  - `cargo test -p roastty renderer::cell::tests::rebuild_row -- --nocapture`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/818-drive-row-formatting.md`
- Run:
  - `git diff --check`

The experiment passes if prepared row-formatting inputs can be applied through a
`FrameRebuildPlan`, producing the same row-local `Contents` output as the
existing row-formatting helpers for only the planned rows, while preserving
upstream validation and row-error recovery semantics. It is Partial if the
wrapper lands but needs a follow-up for a missed row-formatting input. It fails
if plan-driven row formatting cannot be separated cleanly from live terminal
state collection.

## Design Review

Codex reviewed the initial design and found three blockers before
implementation. First, missing prepared row data was incorrectly treated as a
recoverable row-format failure; upstream's loop only iterates rows that have
already been sliced, so missing prepared rows are invalid caller input and must
abort before mutation. Second, row width was not validated before mutation even
though `rebuild_bg_row` writes one background cell per `RunOptions.cells` entry;
malformed widths could panic or partially mutate. Third, preedit had two sources
(`FrameRowFormatInput::preedit_skip` and the plan's `preedit_range`), creating
ambiguous precedence. Codex also noted that validation/setup errors and
row-local render errors should be separate types, and that `shape_row_cached`
lives in `font::run`, not `cell.rs`.

The design was amended to pre-validate all planned row data and exact row widths
before mutation, use only the plan-owned preedit range, split validation errors
from row-local render errors, and clarify that shaping is done through
`font::run::shape_row_cached`.

Codex re-reviewed the amended design and approved it with no blocking findings.
The follow-up review confirmed that the validation, preedit-source, and error
type blockers were resolved. It noted one implementation risk: if deterministic
row render-error injection is not practical with the real formatting helpers,
the test plan should explicitly rely on Experiment 817's callback-failure
coverage and keep Experiment 818 focused on validation plus successful real row
formatting.

## Result

**Result:** Pass

Roastty can now format prepared row inputs through the frame rebuild plan:

- `roastty/src/renderer/frame_rebuild.rs` adds `FrameRowFormatInput`,
  `FrameRowFormatValidationError`, and `FrameRowRenderError`.
- `FrameRebuildPlan::format_rows` pre-validates planned row data before any
  mutation, rejecting missing rows and rows whose `RunOptions.cells.len()` does
  not exactly match the effective grid columns.
- The wrapper uses `drive_row_rebuilds` so successful row formatting follows the
  upstream row-loop order: clear planned partial rows, mark rows clean, then
  rebuild each planned row.
- The row callback formats through the existing row helpers: `rebuild_bg_row`,
  `font::run::shape_row_cached`, and `rebuild_row`.
- The plan-owned preedit range is the only preedit mask source for this wrapper.
- Tests cover partial planned-row formatting, full rebuild formatting, missing
  row validation, row-width validation, plan-owned preedit masking, search
  highlight/link threading, and driver validation propagation.
- Deterministic row render-error injection is not added here; Experiment 817
  already covers callback failure recovery. This experiment verifies validation
  and successful real row formatting.

Verification:

- Inspected `vendor/ghostty/src/renderer/generic.zig` `rebuildCells`.
- Inspected `roastty/src/renderer/frame_rebuild.rs`.
- Inspected `roastty/src/renderer/cell.rs`.
- Inspected `roastty/src/font/run.rs`.
- `cargo fmt -p roastty` — passed.
- `cargo test -p roastty renderer::frame_rebuild -- --nocapture` — passed, 38
  tests.
- `cargo test -p roastty renderer::cell::tests::rebuild_viewport -- --nocapture`
  — passed, 5 tests.
- `cargo test -p roastty renderer::cell::tests::rebuild_row -- --nocapture` —
  passed, 11 tests.
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/818-drive-row-formatting.md`
  — passed.
- `git diff --check` — passed.

## Conclusion

Experiment 818 connects the frame rebuild driver to real row formatting for
prepared inputs. The renderer can now plan rows, apply upstream row sequencing,
and rebuild only planned `Contents` rows through existing background,
foreground, shaping, highlight, link, and preedit-mask logic. The remaining live
render-loop work is still substantial: collecting terminal render state,
threading real search/link inputs, drawing cursor/preedit glyphs, syncing GPU
buffers, submitting Metal frames, pacing redraws, and integrating the renderer
thread remain open.

## Completion Review

Codex reviewed the completed implementation and found no blocking correctness
issues. The review confirmed that `format_rows` pre-validates planned row
presence and width before mutation, delegates plan/content validation and row
sequencing to `drive_row_rebuilds`, uses the same background, shaping, and
foreground helpers as `rebuild_viewport`, and uses only `self.preedit_range` for
the preedit mask. It also confirmed that relying on Experiment 817 for
deterministic callback failure recovery is consistent with the amended design.

The only finding was that the result verification record initially omitted the
successful Prettier and `git diff --check` commands. Those bullets were added
before the result commit.
