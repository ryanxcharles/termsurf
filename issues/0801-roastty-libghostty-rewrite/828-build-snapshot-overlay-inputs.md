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

# Experiment 828: Build Snapshot Overlay Inputs

## Description

Continue the prepared frame rebuild bridge from live terminal snapshots to the
remaining per-frame renderer inputs. Experiments 826 and 827 let
`FrameTerminalSnapshot` feed rebuild planning and row formatting. Text overlays
and cursor uniforms are still manually assembled from separate pieces:
`FrameTextOverlayInput` needs snapshot-owned preedit plus cursor overlay data,
and `FrameCursorUniformInput` needs preedit activity plus an optional block
cursor uniform at the current cursor viewport position.

This experiment adds snapshot adapter methods for those overlay inputs. The
snapshot should provide the parts it owns: optional preedit, cursor viewport,
and preedit-active state. The caller still supplies renderer-owned cursor
styling/color/wide information, screen foreground, and alpha. The experiment
remains a prepared-input bridge only. It does not draw overlays by itself,
update uniforms by itself, format rows, present Metal frames, pace redraws,
create the renderer thread, or change the C ABI.

## Changes

- `roastty/src/renderer/frame_rebuild.rs`
  - Add a caller-supplied cursor overlay payload that omits grid position, for
    example `FrameSnapshotCursorOverlayInput { style, wide, color }`.
  - Add `FrameSnapshotTextOverlayInput` containing:
    - `cursor: Option<FrameSnapshotCursorOverlayInput>`,
    - `screen_fg: Rgb`, and
    - `alpha: u8`.
  - Add
    `FrameTerminalSnapshot::text_overlay_input(&self, input: FrameSnapshotTextOverlayInput) -> FrameTextOverlayInput<'_>`.
    The returned input should borrow `self.preedit`, use `self.cursor_viewport`
    as the cursor grid position when both snapshot cursor and caller cursor
    payload are present, and copy through `screen_fg` and `alpha`.
  - Add a caller-supplied block-cursor uniform payload that omits grid position,
    for example `FrameSnapshotBlockCursorUniformInput { wide, color }`.
  - Add `FrameSnapshotCursorUniformInput` containing
    `block_cursor: Option<FrameSnapshotBlockCursorUniformInput>`.
  - Add
    `FrameTerminalSnapshot::cursor_uniform_input(&self, input: FrameSnapshotCursorUniformInput) -> FrameCursorUniformInput`.
    The returned input should set `preedit_active` from `self.preedit.is_some()`
    and use `self.cursor_viewport` as the block cursor grid position only when
    both snapshot cursor and caller block-cursor payload are present.
  - Keep validation in the existing `FrameRebuildPlan::draw_text_overlays` and
    `FrameRebuildPlan::apply_cursor_uniforms` paths. The adapters should not
    duplicate bounds or wide-cursor validation.
  - Add tests proving:
    - text overlay input borrows snapshot preedit and copies screen foreground
      and alpha,
    - text overlay cursor grid position comes from snapshot cursor viewport and
      renderer cursor style/wide/color come from the caller payload,
    - text overlay cursor is `None` when the snapshot cursor is `None`,
    - cursor uniform input derives `preedit_active` from snapshot preedit,
    - block cursor uniform grid position comes from snapshot cursor viewport and
      wide/color come from the caller payload,
    - block cursor uniform is `None` when the snapshot cursor is `None`, and
    - the produced inputs can feed the existing overlay/uniform drivers for a
      live snapshot without adding renderer-loop orchestration.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - Add this experiment to the index as `Designed`.
  - After implementation, update the renderer tracker to say terminal snapshots
    can provide rebuild planning, row formatting, text overlay, and cursor
    uniform inputs while live renderer-loop orchestration remains open.

## Verification

- Inspect:
  - `roastty/src/renderer/frame_rebuild.rs` `FrameTerminalSnapshot`,
    `FrameTextOverlayInput`, `FrameCursorUniformInput`,
    `FrameRebuildPlan::draw_text_overlays`, and
    `FrameRebuildPlan::apply_cursor_uniforms`.
  - `roastty/src/renderer/cursor.rs` cursor style values.
  - `roastty/src/terminal/terminal.rs` `cursor_position`.
- Run Rust formatting:
  - `cargo fmt -p roastty`
- Run targeted tests:
  - `cargo test -p roastty renderer::frame_rebuild::tests::snapshot_overlay -- --nocapture`
  - `cargo test -p roastty renderer::frame_rebuild -- --nocapture`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/828-build-snapshot-overlay-inputs.md`
- Run:
  - `git diff --check`

The experiment passes if a terminal snapshot can create complete text overlay
and cursor uniform inputs for the existing prepared drivers. It is Partial if
the adapters can package the values but a later experiment is still needed to
prove they can drive the existing overlay/uniform methods. It fails if the
inputs cannot be assembled without starting the renderer thread or changing the
C ABI.

## Design Review

Codex reviewed the design and approved it for implementation with no blocking
findings. The review confirmed that the adapters are correctly scoped to package
snapshot-owned preedit, cursor viewport, and preedit-active state with
caller-owned renderer styling, color, and alpha fields.

The review also confirmed that cursor bounds and wide-cursor validation should
remain in the existing `FrameRebuildPlan::draw_text_overlays` and
`FrameRebuildPlan::apply_cursor_uniforms` paths. The adapter should only map the
snapshot cursor viewport into `[x, y]` when both the snapshot cursor and caller
payload are present. The planned tests cover borrowed preedit identity, caller
field passthrough, cursor absence, preedit-derived suppression state, block
cursor mapping, and feeding the produced inputs into the existing drivers.
