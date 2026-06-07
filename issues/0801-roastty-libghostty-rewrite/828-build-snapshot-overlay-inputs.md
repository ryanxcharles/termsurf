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

## Result

**Result:** Pass

Added snapshot adapter inputs for text overlays and cursor uniforms.
`FrameTerminalSnapshot::text_overlay_input` now borrows snapshot preedit, maps
snapshot cursor viewport into cursor grid position when a caller cursor payload
is present, and copies caller-supplied cursor style, cursor width, cursor color,
screen foreground, and alpha. `FrameTerminalSnapshot::cursor_uniform_input` now
derives `preedit_active` from snapshot preedit and maps snapshot cursor viewport
into an optional block cursor uniform when a caller block-cursor payload is
present.

Implementation changes:

- `roastty/src/renderer/frame_rebuild.rs`
  - Added `FrameSnapshotCursorOverlayInput`.
  - Added `FrameSnapshotTextOverlayInput`.
  - Added `FrameSnapshotBlockCursorUniformInput`.
  - Added `FrameSnapshotCursorUniformInput`.
  - Added `FrameTerminalSnapshot::text_overlay_input`.
  - Added `FrameTerminalSnapshot::cursor_uniform_input`.
  - Added shared cursor grid-position conversion from snapshot cursor viewport.
  - Added tests proving preedit borrowing, cursor mapping, caller-field
    passthrough, cursor omission, preedit-active derivation, block-cursor
    mapping, and live-driver use for cursor drawing and preedit suppression.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - Marked Experiment 828 as `Pass`.
  - Updated the renderer tracker to say terminal frame snapshots can now feed
    rebuild planning, row formatting, text overlay, and cursor uniform inputs
    while live renderer-loop orchestration remains open.

Verification:

- `cargo fmt -p roastty`
- `cargo test -p roastty renderer::frame_rebuild::tests::snapshot_overlay -- --nocapture`
  - 8 passed
- `cargo test -p roastty renderer::frame_rebuild -- --nocapture`
  - 107 passed
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/828-build-snapshot-overlay-inputs.md`
- `git diff --check`

## Conclusion

Terminal snapshots now bridge all prepared front-half frame rebuild inputs that
depend on live terminal state: rebuild planning, row formatting, text overlays,
and cursor uniforms. The adapters still only package data. The existing
`draw_text_overlays` and `apply_cursor_uniforms` drivers remain responsible for
bounds validation, wide cursor validation, mutation, and rendering behavior.

The next useful experiment can start composing a single prepared frame rebuild
sequence that collects a snapshot, builds a plan, formats rows, draws overlays,
updates rebuild/cursor uniforms, refines padding extension rows, and then stops
before Metal presentation or renderer-thread orchestration.

## Completion Review

Codex reviewed the completed implementation and recorded result, and approved
the experiment with no findings. The review confirmed that the implementation is
scoped to the intended adapter layer and does not add renderer-loop wiring,
Metal presentation orchestration, or C ABI changes.

The review also confirmed the cursor/preedit behavior: text overlay input
borrows snapshot preedit, maps cursor position from `cursor_viewport`, and
copies caller-owned style, width, color, and screen fields. Cursor uniforms
derive `preedit_active` from snapshot preedit and only build a block cursor when
both snapshot cursor position and caller payload are present. Validation remains
in the existing drivers.

The review found the `snapshot_overlay` tests adequate for this slice and
confirmed that the recorded verification matches the requested commands.
