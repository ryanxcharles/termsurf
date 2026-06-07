+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.result]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"
+++

# Experiment 840: A persistent FrameRenderer that owns state and drives update_frame

## Description

Exps 815–839 built the frame-rebuild pipeline (`FrameTerminalSnapshot`,
`FrameRebuildPlan`, `rebuild_frame`, `rebuild_and_present_frame`) bottom-up and
**standalone** — no caller owns the per-frame state across frames. The live draw
path (`roastty_surface_draw` → `surface.draw()`) does not yet use it.

Ghostty's renderer (`vendor/ghostty/src/renderer/generic.zig`) owns the
persistent frame state (`cells: Contents`, `uniforms`, `font_grid`, `size`) and
drives it via `updateFrame` (rebuild the frame from the terminal) then
`drawFrame` (present). The roastty pipeline is the _guts_ of those; this
experiment begins the integration scaffold by introducing the **persistent
owner**.

This is the **first** renderer-integration slice. It is deliberately small and
Metal-free: a `FrameRenderer` that owns the CPU-side rebuild state and drives
`rebuild_frame` from a live terminal. Four things are explicitly **deferred** to
later slices: (a) deriving the `FramePreparedRebuildInput` from surface config/
state (this slice takes it as a parameter); (b) Metal presentation
(`draw_frame`/compositor/atlases); (c) wiring into `surface.draw()`; (d)
clearing the **terminal's** dirty bits after a frame — ghostty's `updateFrame`
clears row dirty, but `update_frame` here only clears its own scratch
`row_dirty` (the terminal is re-snapshotted each frame), so until that machinery
exists a persistently-dirty terminal re-rebuilds those rows every frame. The
tests seed a clean terminal explicitly (`clear_dirty_for_tests`) to exercise the
partial path.

## Changes

New module `roastty/src/renderer/frame_renderer.rs` (declared
`mod frame_renderer;` in `roastty/src/renderer/mod.rs`).

- `pub(crate) struct FrameRenderer` owning the persistent CPU-side state:

  ```rust
  pub(crate) struct FrameRenderer {
      contents: Contents,
      uniforms: MetalUniforms,
      current_grid: GridSize,   // last-rendered grid, for resize detection
      row_dirty: Vec<bool>,     // scratch dirty buffer the rebuild clears
  }
  ```

- `FrameRenderer::new(uniforms: MetalUniforms) -> Self` — `Contents::default()`,
  the caller-built `uniforms` (config-derived construction via
  `MetalUniforms::new` is the surface's job, deferred to a later slice),
  `current_grid = GridSize { columns: 0, rows: 0 }` (so the first frame is a
  full rebuild + resize), empty `row_dirty`. (`MetalUniforms` has no `Default`;
  its `new` needs config-derived values — `min_contrast`, `background`,
  `background_opacity`, `colorspace`, `blending` — so `FrameRenderer` takes it
  as an argument rather than hardcoding config.)

- `FrameRenderer::update_frame(&mut self, terminal: &Terminal, grid: &mut SharedGrid, dirty: RenderDirty, preedit: Option<Preedit>, input: FramePreparedRebuildInput<'_>) -> Result<FramePreparedRebuildApplication, FramePreparedRebuildError>`:
  1. `let snapshot = FrameTerminalSnapshot::collect(terminal, self.current_grid, dirty, preedit);`
  2. size `self.row_dirty` to `snapshot.terminal_grid.rows` and seed it from
     `snapshot.row_dirty` (the scratch buffer the rebuild marks clean);
  3. `let app = snapshot.rebuild_frame(FramePreparedRebuildTargets { contents: &mut self.contents, grid, row_dirty: &mut self.row_dirty, uniforms: &mut self.uniforms }, input)?;`
  4. on success advance `self.current_grid = snapshot.terminal_grid;` (so the
     next frame only resizes when the terminal grid actually changes);
  5. return `app`.

  On a rebuild error, `current_grid` is **not** advanced (the frame did not
  complete) — fail-fast, consistent with `rebuild_frame`.

- Read-only accessors used by tests / future presentation slices:
  `contents(&self) -> &Contents`, `uniforms(&self) -> &MetalUniforms`,
  `current_grid(&self) -> GridSize`.

No change to the existing pipeline; `update_frame` is a thin owner over
`rebuild_frame`. The `SharedGrid` is borrowed per frame (the font system owns
it), not held by `FrameRenderer`. No Metal, no surface wiring.

## Verification

Per the bounded-run convention (15-min cap, Central-stamped, single tracked
task, no poll-watcher). These are fast unit tests in `frame_renderer.rs` (no
Metal device needed — rebuild is CPU-side):

- **First frame is a full rebuild + resize:** a fresh `FrameRenderer` (0×0 grid)
  `update_frame` against a populated test terminal rebuilds all rows, resizes
  `contents` to the terminal grid, and advances `current_grid` to the terminal
  grid; the returned application reports `reset_contents` / all rows rebuilt.
- **Second frame with no terminal change is a partial (no resize):** a follow-up
  `update_frame` on the same (now-clean) terminal does **not** resize and
  rebuilds only dirty rows (or none) — `current_grid` already equals the
  terminal grid.
- **Resize is detected:** after a terminal grid change, `update_frame` resizes
  `contents`/`uniforms` and re-advances `current_grid`.
- **Error does not advance `current_grid`:** force a rebuild error via an
  input-controlled trigger — `WindowPaddingColor::Extend` with a too-short
  `row_never_extend` (`&[]`) → `PaddingExtend` after `format_rows`/overlays
  succeed — and assert `current_grid` is unchanged and the error propagates.
  (The plan-reject path is unreachable from `update_frame`, which always builds
  a valid `row_dirty`.)
- The `frame_rebuild.rs` test helpers (`menlo_grid`, `snapshot_format_input`,
  etc.) are private to that module's `mod tests`, so `frame_renderer.rs`'s tests
  re-create the small fixtures they need (a test terminal, a `SharedGrid`,
  `MetalUniforms::new`, and a `FramePreparedRebuildInput` from `pub(crate)`
  fields).
- `cargo build -p roastty` — no warnings. `cargo fmt -p roastty -- --check` —
  clean. Full suite via `scripts/bounded-run.sh` (default parallelism) stays
  green. No-ghostty grep on changed lines — clean. `git diff --check` — clean.

**Pass** = the new `FrameRenderer` tests pass and the full suite stays green.
**Partial/Fail** = any test fails or the suite regresses.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Traced the resize/`current_grid` state machine, `row_dirty` seeding,
the error path, and the borrow shape against the actual pipeline code.

**Verdict:** APPROVED, no Required findings. Confirmed: frame 1 (0×0 →
`grid_changed`) takes `validate_application`'s `resize_to.is_some()` branch
(skips the contents-size check), the drivers resize/reset `Contents` and update
the uniform grid, and advancing `current_grid` makes frame 2 a non-resize
partial that re-validates `contents.size() == effective_grid` (no off-by-one); a
mid-frame error leaves `current_grid` stale but the next frame's resize path
self-heals idempotently. `row_dirty` re-seeding each frame is correct (the dirty
truth lives in the terminal). The `input` parameter is constructible from
`pub(crate)` fields (Metal-free). The error path is forceable (Extend +
too-short `row_never_extend` → `PaddingExtend`). Borrows compile. Three
Optionals/Nit, all adopted:

- **Terminal dirty-clearing divergence.** Added to the deferred list (the test
  seeds a clean terminal explicitly).
- **Test-helper re-creation.** Noted that `frame_renderer.rs` tests re-create
  the small fixtures (the `frame_rebuild.rs` helpers are private).
- **Vague error trigger.** Specified the reachable `PaddingExtend` trigger.

## Conclusion

_(to be written after the run)_
