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

# Experiment 827: Build Snapshot Row Format Input

## Description

Continue wiring the prepared frame rebuild path toward live renderer use without
starting the renderer loop. Experiment 826 added `FrameTerminalSnapshot`, which
collects live terminal grid, row dirty flags, shaped viewport rows, cursor
viewport, dirty mode, and optional preedit state. The next missing bridge is row
formatting: `FrameRebuildPlan::format_rows` can already rebuild planned rows,
but callers still have to manually assemble a `FrameRowFormatInput` and remember
to source its `rows` field from the terminal snapshot.

This experiment adds a small renderer-side adapter that combines snapshot-owned
terminal rows with caller-supplied renderer formatting state such as highlights,
hovered-link ranges, selection colors, palette/default colors, alpha, font
thickening, and background opacity controls. It remains a prepared-input bridge
only. It does not mutate `Contents`, format rows by itself, update dirty flags,
draw overlays, update uniforms, present Metal frames, pace redraws, create the
renderer thread, or change the C ABI.

## Changes

- `roastty/src/renderer/frame_rebuild.rs`
  - Add `FrameSnapshotRowFormatInput<'a>` containing every renderer-supplied
    field currently needed by `FrameRowFormatInput<'a>` except `rows`.
  - Add:
    ```rust
    pub(crate) fn row_format_input<'a>(
        &'a self,
        input: FrameSnapshotRowFormatInput<'a>,
    ) -> FrameRowFormatInput<'a>
    ```
  - The method should return a `FrameRowFormatInput` whose `rows` field borrows
    `self.rows` and whose remaining fields are copied from the supplied
    formatting input.
  - Keep validation in the existing `FrameRebuildPlan::format_rows` path. The
    adapter should not duplicate row-width or missing-row validation.
  - Add tests proving:
    - snapshot row-format input borrows the collected snapshot rows by slice
      identity, not just by equal row contents,
    - highlights and link ranges are threaded through unchanged,
    - selection configuration and color/palette options are threaded through
      unchanged,
    - font rendering knobs (`alpha`, `faint_opacity`, `thicken`,
      `thicken_strength`) are threaded through unchanged,
    - background opacity controls are threaded through unchanged, and
    - the adapter can feed `FrameRebuildPlan::format_rows` to rebuild a dirty
      terminal row from a live snapshot.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - Add this experiment to the index as `Designed`.
  - After implementation, update the renderer tracker to say terminal snapshots
    can provide both rebuild planning input and row formatting input, while live
    renderer-loop orchestration remains open.

## Verification

- Inspect:
  - `roastty/src/renderer/frame_rebuild.rs` `FrameTerminalSnapshot`,
    `FrameRowFormatInput`, and `FrameRebuildPlan::format_rows`.
  - `roastty/src/renderer/cell.rs` row formatting/highlight/link input usage.
  - `roastty/src/terminal/terminal.rs` `shape_run_options`.
- Run Rust formatting:
  - `cargo fmt -p roastty`
- Run targeted tests:
  - `cargo test -p roastty renderer::frame_rebuild::tests::snapshot_row_format -- --nocapture`
  - `cargo test -p roastty renderer::frame_rebuild -- --nocapture`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/827-build-snapshot-row-format-input.md`
- Run:
  - `git diff --check`

The experiment passes if terminal snapshots can create a complete
`FrameRowFormatInput` for the existing row-formatting driver and the driver can
format planned rows using live snapshot rows. It is Partial if the adapter can
package the borrowed rows but a later experiment still needs to prove full
formatting with live terminal rows. It fails if the row-formatting input cannot
be assembled without starting the renderer thread or changing the C ABI.

## Design Review

Codex reviewed the design and initially found three plan issues: the
`row_format_input` signature needed an explicit shared lifetime, the README
provenance policy was stale relative to the current Codex-recorded experiments,
and the README experiment index line needed the provenance suffix.

The design was updated to use the explicit
`row_format_input<'a>(&'a self, input: FrameSnapshotRowFormatInput<'a>) -> FrameRowFormatInput<'a>`
signature, clarify that the row borrow test must prove slice identity, update
the provenance policy to make the experiment frontmatter/index tag the source of
truth for actual agents used, and tag Experiment 827 as
`Designed · Codex/Codex/Codex`.

Codex then re-reviewed the revised design and approved it for the plan commit
with no remaining blockers.

## Result

**Result:** Pass

Added `FrameSnapshotRowFormatInput` and
`FrameTerminalSnapshot::row_format_input`. The adapter borrows
`FrameTerminalSnapshot::rows` for the existing `FrameRowFormatInput::rows` field
and copies through all caller-supplied renderer formatting fields: highlights,
hovered-link ranges, selection config, default foreground/background colors,
palette, bold color behavior, alpha, faint opacity, font thickening, and
background opacity controls.

Implementation changes:

- `roastty/src/renderer/frame_rebuild.rs`
  - Added `FrameSnapshotRowFormatInput<'a>`.
  - Added
    `FrameTerminalSnapshot::row_format_input<'a>(&'a self, input: FrameSnapshotRowFormatInput<'a>) -> FrameRowFormatInput<'a>`.
  - Added tests proving row slice identity, renderer option threading, and live
    terminal snapshot rows feeding `FrameRebuildPlan::format_rows`.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - Marked Experiment 827 as `Pass`.
  - Updated the renderer tracker to say terminal frame snapshots can now feed
    both rebuild planning and row-formatting input while live renderer-loop
    orchestration remains open.

Verification:

- `cargo fmt -p roastty`
- `cargo test -p roastty renderer::frame_rebuild::tests::snapshot_row_format -- --nocapture`
  - 3 passed
- `cargo test -p roastty renderer::frame_rebuild -- --nocapture`
  - 99 passed
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/827-build-snapshot-row-format-input.md`
- `git diff --check`

## Conclusion

The prepared renderer path now has a complete terminal snapshot handoff for the
front of row rebuilding: a snapshot can build the rebuild plan and package the
same live terminal rows into the row-formatting driver. The existing
`FrameRebuildPlan::format_rows` validation remains the single place that checks
missing rows and row-width mismatches.

This still does not orchestrate a full live frame. The next useful experiment
can begin composing these prepared pieces into a single frame rebuild sequence,
or add a companion adapter for text overlay/cursor uniform inputs from snapshot
and renderer state.

## Completion Review

Codex reviewed the completed implementation and recorded result. The review
found no implementation correctness, regression, or lifetime-soundness blockers.
It confirmed that the adapter ties the snapshot row borrow and renderer-state
borrows through the intended shared lifetime, borrows `self.rows`, copies the
remaining fields directly, and leaves validation in
`FrameRebuildPlan::format_rows`.

The review found one workflow traceability issue: the result verification list
omitted the markdown `prettier` command and `git diff --check` even though both
had been run. The verification list was updated to include those commands.

After that doc-only fix, Codex approved the implementation and recorded result
for the result commit.
