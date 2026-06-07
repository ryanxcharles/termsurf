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

# Experiment 841: FrameRenderer drives update-and-present

## Description

Exp 840 gave `FrameRenderer` (`roastty/src/renderer/frame_renderer.rs`) the
persistent CPU-side state and `update_frame` (collect → `rebuild_frame` →
advance `current_grid`). This experiment adds the Metal half:
`update_and_present_frame`, which collects a snapshot from the live terminal and
drives Exp 839's `rebuild_and_present_frame` against the owned
`contents`/`uniforms` — so the renderer produces a **presented** frame end to
end, against its own persistent state, across frames.

This mirrors ghostty's renderer driving `updateFrame` then `drawFrame`. It is
still standalone (no `surface.draw()` wiring) and still takes the
`FramePreparedRebuildInput` as a parameter (surface-state derivation is a later
slice). The Metal compositor and atlases are supplied **per call** (the Exp 839
`FramePreparedPresentationInput` bundle) rather than owned by `FrameRenderer` —
owning them needs the device + atlas population, deferred to a later slice.

## Changes

`roastty/src/renderer/frame_renderer.rs` (production code + tests).

- Add `FrameRenderer::update_and_present_frame`:

  ```rust
  pub(crate) fn update_and_present_frame(
      &mut self,
      terminal: &Terminal,
      grid: &mut SharedGrid,
      dirty: RenderDirty,
      preedit: Option<Preedit>,
      input: FramePreparedRebuildInput<'_>,
      presentation: FramePreparedPresentationInput<'_>,
  ) -> Result<FramePreparedFrameApplication, FramePreparedFrameError>
  ```

  Body, parallel to `update_frame`:
  1. `let snapshot = FrameTerminalSnapshot::collect(terminal, self.current_grid, dirty, preedit);`
  2. re-seed `self.row_dirty` from `snapshot.row_dirty`;
  3. `let app = snapshot.rebuild_and_present_frame(FramePreparedRebuildTargets { contents: &mut self.contents, grid, row_dirty: &mut self.row_dirty, uniforms: &mut self.uniforms }, input, presentation)?;`
  4. on success advance `self.current_grid = snapshot.terminal_grid;`
  5. return `app`.

  Fail-fast: a rebuild-stage error (`FramePreparedFrameError::Rebuild(..)`)
  returns before presentation and **does not** advance `current_grid`; a
  presentation error (`Present(..)`) returns after the rebuild already mutated
  the owned state, but `current_grid` is still **not** advanced (the frame did
  not present) — this is consistent with `update_frame`'s "advance only on full
  success" and is the honest, simple rule (the next frame re-resizes
  idempotently).

No new types (reuses `FramePreparedPresentationInput` /
`FramePreparedFrameApplication` / `FramePreparedFrameError` from Exp 839). No
surface wiring; the compositor/atlases are caller-supplied.

## Verification

Per the bounded-run convention (15-min cap, Central-stamped, single tracked
task, no poll-watcher). The presentation tests need a Metal device, so they
follow the `let Some(device) = metal_device() else { return; };` guard (as Exp
839's `rebuild_and_present_frame_*` tests do). `frame_renderer.rs`'s test module
re-creates the small fixtures (terminal, `SharedGrid`, uniforms, input bundle,
and adds a `metal_device`/compositor/atlas helper).

- **Update-and-present happy path** (Metal-guarded): a fresh `FrameRenderer`
  against a 4×3 terminal rebuilds (full rebuild + resize, `current_grid` → 4×3)
  **and** presents at the requested drawable size; the returned
  `FramePreparedFrameApplication` reports both halves.
- **Across frames:** a second `update_and_present_frame` on a cleaned terminal
  is a partial (no resize) and still presents; `current_grid` stays 4×3.
- **Fail-fast before present does not advance `current_grid`** (Metal-guarded):
  a too-short `row_never_extend` → `Rebuild(PaddingExtend(..))`, `current_grid`
  unchanged, and the compositor was not reached (no `Present` variant).
- **Present-stage failure self-heals** (Metal-guarded — the experiment's novel
  behavior): an invalid `contents_scale` makes `compositor.draw_frame` error
  **after** the rebuild already mutated the owned `contents`/`uniforms`, so the
  result is `Present(..)` and `current_grid` is **not** advanced. A subsequent
  `update_and_present_frame` with a valid scale then **self-heals**: the stale
  `current_grid` makes it a full re-resize, `current_grid` advances to the
  terminal grid, and it presents. (Exp 839's
  `present_metal_frame_propagates_invalid_contents_scale_from_compositor`
  confirms invalid scale is the reachable post-rebuild present failure.)
- The Exp 840 `update_frame` tests still pass (untouched).
- `cargo build -p roastty` — no warnings. `cargo fmt -p roastty -- --check` —
  clean. Full suite via `scripts/bounded-run.sh` (default parallelism) stays
  green. No-ghostty grep on changed lines — clean. `git diff --check` — clean.

**Pass** = the new `update_and_present_frame` tests pass (Metal-device
permitting), the 840 tests still pass, and the full suite stays green.
**Partial/Fail** = any test fails or the suite regresses.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Verified the borrow plan compiles (identical shape to
`update_frame`; `FramePreparedFrameApplication` has no lifetime so the field
borrows end at `?`), the self-heal is sound (stale `current_grid` →
`resize_to.is_some()` skips the contents-size check → idempotent re-resize), the
test helpers are constructible (`Atlas::new`, `MetalFrameCompositor`, the Metal
enums are `pub(crate)`; the `metal_device` guard is honest), and the reuse of
Exp 839's types is exact (no new types).

**Verdict:** CHANGES REQUIRED → fixed. One Required:

- **Required — no present-stage failure test.** The three originally-listed
  tests (happy, across-frames, rebuild-error) all pass _vacuously_ w.r.t. the
  experiment's only novel behavior — a `Present(..)` error after the rebuild
  mutated the owned state, with `current_grid` left unadvanced and the next
  frame self-healing. The rebuild-error invariant is already covered non-Metal
  by Exp 840. **Fixed:** added a Metal-guarded present-error self-heal test
  (invalid `contents_scale` → `Present(..)`, `current_grid` unchanged; then a
  valid frame re-resizes and presents).
- **Optional/Nit (no change required):** the Metal test helpers duplicate
  frame_rebuild.rs's private ones (a shared test-support module could be
  considered later); with the present-error test added, the happy/across-frames
  tests are success-only smoke checks.

## Result

**Result:** Pass

`FrameRenderer::update_and_present_frame` landed (reusing the Exp 839 types; the
test module added `metal_device`/`metal_compositor`/atlas helpers). Production
`cargo build -p roastty` and `--tests` both clean (no warnings); fmt clean,
no-ghostty clean, `git diff --check` clean.

Four new Metal-device-guarded tests (ran on this GPU), all passing; the four Exp
840 `update_frame` tests still pass untouched:

- **`update_and_present_rebuilds_and_presents`** — fresh `FrameRenderer` against
  a 4×3 terminal: rebuild (`rows [0,1,2]`) + present at 8×6, `current_grid` →
  4×3.
- **`update_and_present_second_frame_is_partial_and_still_presents`** — after a
  `clear_dirty_for_tests`, a no-reset partial that still presents;
  `current_grid` stays 4×3.
- **`update_and_present_rebuild_error_skips_present_and_grid`** — too-short
  `row_never_extend` → `Rebuild(PaddingExtend(..))`, `current_grid` stays 0×0
  (present never reached).
- **`update_and_present_present_error_does_not_advance_grid_then_self_heals`**
  (the novel behavior) — invalid `contents_scale` (0.0) → `Present(..)`
  **after** the rebuild mutated the owned state, `current_grid` stays 0×0; a
  subsequent valid frame **self-heals** (full re-resize, `current_grid` → 4×3,
  presents).

**Full suite (default parallelism, `scripts/bounded-run.sh`):**
`4375 passed; 0 failed` (4371 + 4 new), 0 panics, 0 `PoisonError`,
`STATUS=COMPLETED rc=0`, 185 s — green.

## Conclusion

`FrameRenderer` now drives a full frame end to end against its own persistent
state — `update_frame` (rebuild only) and `update_and_present_frame` (rebuild +
Metal present) — the roastty analogue of ghostty's `updateFrame` + `drawFrame`,
with the present-error self-heal proven. The frame-rebuild pipeline (815–839)
now has a stateful owner (840) that can present (841).

Remaining toward the live draw path (`surface.draw()`):

- **Exp 842:** derive the `FramePreparedRebuildInput` from the surface's
  config/state (selection, cursor, colors, font knobs) — replacing the
  caller-supplied parameter — so a caller no longer hand-builds the input
  bundle.
- **Later:** `FrameRenderer` owns the `MetalFrameCompositor` + atlases (needs
  the device + atlas population); clear the terminal's dirty bits after a frame;
  then wire `FrameRenderer` into `surface.draw()` / the C ABI so the live path
  renders through the new pipeline.

## Completion Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Independently reproduced the gates: build clean (no warnings), fmt
clean, the slice = 8 passed / 0 failed (4 new + 4 untouched 840); v1.log shows
4375 passed / 0 failed, rc=0, default parallelism, no timeout, the 4 new tests
`... ok` (Metal present, not skipped). Confirmed: only `frame_renderer.rs`
changed (`update_frame` untouched); `?` returns before `self.current_grid = ...`
so a `Present` error provably does not advance the grid; the self-heal test is
genuine (asserts `Present(_)` + grid 0×0, then `reset_contents` + grid → 4×3 +
width 8). **Verdict: CHANGES REQUIRED → fixed.**

- **Required — stale README index status.** The 841 index line still read
  `Designed`. **Fixed:** flipped to `Pass`.
- **Nit (no change required):** the happy/across-frames tests are success-only
  smoke checks; the present-error test carries the novel-behavior coverage.
