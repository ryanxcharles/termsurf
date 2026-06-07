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

# Experiment 838: Compose the prepared frame rebuild sequence

## Description

Feature work resumes here (the test suite is green again after Exps 829–837).
Experiments 815–828 built every adapter and driver of the renderer's frame
rebuild as independent, individually-tested pieces in
`roastty/src/renderer/frame_rebuild.rs`:

- adapters on `FrameTerminalSnapshot`: `collect`, `build_plan`,
  `row_format_input`, `text_overlay_input`, `cursor_uniform_input`;
- drivers on `FrameRebuildPlan`: `format_rows`, `draw_text_overlays`,
  `apply_rebuild_uniforms`, `refine_padding_extend_rows`,
  `apply_cursor_uniforms`.

Exp 828's conclusion names the next step exactly: "compose a single prepared
frame rebuild sequence that collects a snapshot, builds a plan, formats rows,
draws overlays, updates rebuild/cursor uniforms, refines padding extension rows,
and then **stops before Metal presentation or renderer-thread orchestration**."

This experiment adds that one composition entry point. It introduces **no new
rendering behavior** — every validation and mutation stays in the existing
drivers; the composition only sequences them and threads the snapshot-derived
and caller-supplied inputs, so a caller no longer has to hand-wire six calls in
the right order.

### Ordering (driven by uniform data dependencies)

1. `format_rows` — rebuilds dirty row contents (`Contents`, `SharedGrid`,
   `row_dirty`).
2. `draw_text_overlays` — cursor/preedit into `Contents`/`SharedGrid`.
3. `apply_rebuild_uniforms` — grid-size + **reset** padding-extend on full
   rebuild (`MetalUniforms`).
4. `refine_padding_extend_rows` — **refines** the padding-extend the previous
   step reset, so it must run **after** `apply_rebuild_uniforms`.
5. `apply_cursor_uniforms` — block-cursor uniform (independent of 3–4).

`present_metal_frame` and `apply_custom_shader_frame` are intentionally **not**
called — that is the renderer-thread orchestration the sequence stops before.

## Changes

`roastty/src/renderer/frame_rebuild.rs` (production code — the composition, plus
tests).

- Add a mutable-target bundle (so the signature stays readable):

  ```rust
  pub(crate) struct FramePreparedRebuildTargets<'a> {
      pub(crate) contents: &'a mut Contents,
      pub(crate) grid: &'a mut SharedGrid,
      pub(crate) row_dirty: &'a mut [bool],
      pub(crate) uniforms: &'a mut MetalUniforms,
  }
  ```

- Add a caller-supplied input bundle, mixing the snapshot-adapter inputs
  (827/828) with the two drivers whose inputs are not snapshot-derived (rebuild
  uniforms, padding extend):

  ```rust
  pub(crate) struct FramePreparedRebuildInput<'a> {
      pub(crate) row_format: FrameSnapshotRowFormatInput<'a>,
      pub(crate) text_overlay: FrameSnapshotTextOverlayInput,
      pub(crate) cursor_uniform: FrameSnapshotCursorUniformInput,
      pub(crate) rebuild_uniform: FrameRebuildUniformInput,
      pub(crate) padding_extend: FramePaddingExtendInput<'a>,
  }
  ```

- Add the composition on `FrameTerminalSnapshot`:

  ```rust
  pub(crate) fn rebuild_frame(
      &self,
      targets: FramePreparedRebuildTargets<'_>,
      input: FramePreparedRebuildInput<'_>,
  ) -> Result<FramePreparedRebuildApplication, FramePreparedRebuildError>
  ```

  which: builds the plan (`self.build_plan()?`), then calls the five drivers in
  the order above — passing `self.row_format_input(input.row_format)`,
  `self.text_overlay_input(input.text_overlay)`,
  `self.cursor_uniform_input(input.cursor_uniform)` for the snapshot-derived
  stages and `input.rebuild_uniform` / `input.padding_extend` for the other two
  — reborrowing `targets.contents`/`grid`/`uniforms` across the calls.

- Add `FramePreparedRebuildApplication` collecting each stage's existing
  application struct (`FrameRowRebuildApplication<FrameRowRenderError>`,
  `FrameTextOverlayApplication`, `FrameRebuildUniformApplication`,
  `FramePaddingExtendApplication`, `FrameCursorUniformApplication`).

- Add `FramePreparedRebuildError` — one variant per stage wrapping that stage's
  existing error (`Plan(FrameRebuildPlanError)`,
  `FormatRows(FrameRowFormatValidationError)`,
  `TextOverlays(FrameTextOverlayError)`,
  `RebuildUniforms(FrameRebuildUniformValidationError)`,
  `PaddingExtend(FramePaddingExtendValidationError)`,
  `CursorUniforms(FrameCursorUniformValidationError)`), with `From` impls so the
  body can use `?`. **Fail-fast:** the first failing stage returns its error and
  later stages do not run (the early stages' mutations have already landed in
  `targets`, exactly as if the caller had hand-sequenced them — the composition
  changes ordering ergonomics, not failure semantics).

No change to any driver/adapter, and no Metal-presentation or renderer-thread
wiring.

## Verification

Per the bounded-run convention (15-min cap, Central-stamped, single tracked
task, no poll-watcher); these are fast unit tests in `frame_rebuild.rs`:

- **Happy path:** a dirty-row snapshot drives a full sequence — assert the
  returned `FramePreparedRebuildApplication` reports the rows formatted, the
  overlay drawn, the rebuild/cursor uniforms applied, and padding rows refined,
  and that the order is correct (e.g. padding-extend reflects
  refine-after-reset, not reset-after-refine).
- **Equivalence:** the composed sequence produces the **same** `Contents`,
  `SharedGrid`, `MetalUniforms` mutations as calling the five drivers by hand in
  the same order (a golden side-by-side on identical inputs).
- **Fail-fast per stage:** inject a validation failure at each stage in turn and
  assert (a) the matching `FramePreparedRebuildError` variant is returned, and
  (b) later stages did not run (observable via the unmutated later-stage
  target).
- **Stops before presentation:** assert (by construction / no call) that
  `present_metal_frame` and `apply_custom_shader_frame` are not invoked.
- `cargo build -p roastty` — no warnings (production code).
  `cargo fmt -p roastty -- --check` — clean. The full suite via
  `scripts/bounded-run.sh` (default parallelism) stays green. No-ghostty grep on
  changed lines — clean. `git diff --check` — clean.

**Pass** = the new composition tests pass, the equivalence test shows identical
mutations, fail-fast works per stage, and the full suite stays green.
**Partial/Fail** = any composition test fails or the suite regresses.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Verified every type name, signature, error variant, the borrow plan,
and the ordering against `frame_rebuild.rs` and `metal/shaders.rs`.

**Verdict:** APPROVED, no Required/Optional/Nit findings. Confirmed: the
reset-then-refine dependency is real (`apply_rebuild_uniforms` calls
`reset_padding_extend`, `refine_padding_extend_rows` then refines), so step 3
before step 4 is correct; `apply_cursor_uniforms` touches disjoint uniform
fields (`cursor_pos`/`wide`/`color`), so it is genuinely order-independent and
safe last. All five input bundle types and the six error types match the drivers
exactly (`draw_text_overlays` → `FrameTextOverlayError`, correctly _not_ the
`...ValidationError`). The mixed-lifetime input bundle and the `&mut` reborrows
across the five sequential calls compile cleanly. "No new behavior" + fail-fast
is honest (composing with `?` is identical to hand-sequencing). Scope correctly
stops before `present_metal_frame` / `apply_custom_shader_frame`.

**Note on ordering vs Exp 828's prose:** 828's conclusion listed "rebuild/cursor
uniforms, refines padding extension rows" (cursor before padding); this design
reorders to rebuild → padding → cursor, justified by the data dependency. The
reviewer confirmed both orders are functionally identical because
`apply_cursor_uniforms` mutates uniform fields disjoint from padding-extend and
grid size — so the reorder is safe and better expresses the real dependency.

## Result

**Result:** Pass

`FrameTerminalSnapshot::rebuild_frame` landed with its target/input bundles,
`FramePreparedRebuildApplication`, and `FramePreparedRebuildError` (six `From`
impls). Production `cargo build -p roastty` and `cargo build -p roastty --tests`
both clean (no warnings); fmt clean, no-ghostty clean, `git diff --check` clean.

Four new tests in `frame_rebuild.rs`, all passing:

- `rebuild_frame_runs_the_full_prepared_sequence` — a full-rebuild snapshot
  drives all five stages: rows `[0,1,2]` rebuilt + contents reset, overlay
  cursor drawn, rebuild-uniform padding-extend reset, padding rows `[0,2]`
  refined, block cursor applied, `row_dirty` cleared.
- `rebuild_frame_matches_hand_sequenced_drivers` — **equivalence**: the composed
  sequence yields identical per-stage applications **and** identical `Contents`
  (`bg_cells`/`fg_rows`), `MetalUniforms.padding_extend`, and `row_dirty` to
  calling the five drivers by hand in the same order. Proves no behavior change.
- `rebuild_frame_fails_fast_on_plan_error_without_mutating_targets` — a
  truncated snapshot `row_dirty` makes `build_plan` fail → `Plan` variant,
  **no** target mutated.
- `rebuild_frame_fails_fast_on_format_rows_error_and_skips_later_stages` — a
  too-short target `row_dirty` makes `format_rows` reject → `FormatRows`
  variant, the later uniform stages did **not** run.
- `rebuild_frame_fails_fast_on_padding_extend_after_earlier_stages_ran` — a
  too-short `row_never_extend` makes `refine_padding_extend_rows` (stage 4)
  reject → `PaddingExtend` variant — and `format_rows` (stage 1) had already
  cleared `row_dirty`, so this proves a genuine **mid-sequence** failure
  (earlier stage mutated, the failing stage and later stages handled by `?`).

(Three of the six stages — `Plan`, `FormatRows`, `PaddingExtend` — have explicit
fail-fast tests, including a mid-sequence one. The remaining three propagate
identically: `TextOverlays` and `CursorUniforms` by the same `?`+`From`
mechanism the three tests exercise and the compiler enforces, and
`RebuildUniforms` cannot be made to fail from a valid snapshot's plan (its
validation only rejects an internally-inconsistent plan, which `build_plan`
never produces). The happy path proves each stage's `?` is on the success path.
"Stops before presentation" holds by construction: `rebuild_frame` never
references `present_metal_frame` or `apply_custom_shader_frame`.)

**Full suite (default parallelism, `scripts/bounded-run.sh`):**
`4365 passed; 0 failed` (the prior 4360 + 5 new), 0 panics, 0 `PoisonError`,
`STATUS=COMPLETED rc=0` — the suite stays green with the composition added.

## Conclusion

The renderer's frame rebuild now has one composition entry point: a caller hands
`rebuild_frame` a snapshot, the mutable targets, and the per-stage inputs, and
it builds the plan and runs the five drivers in dependency order — with
fail-fast error propagation and proven equivalence to hand-sequencing. The
adapter/driver pieces built across 815–828 are now a single callable sequence.

The next experiment continues from where this one deliberately stops:
**renderer- thread orchestration / Metal presentation** — feeding the rebuilt
`Contents` / `SharedGrid` / `MetalUniforms` into `present_metal_frame` (Exp 822)
within the renderer loop, and wiring `rebuild_frame` to where the renderer
currently rebuilds frames. That is the next slice toward the live Metal renderer
path.

## Completion Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Verified the diff matches the approved design (the bundles,
`rebuild_frame` calling the five drivers in the stated order with
`apply_rebuild_uniforms` before `refine_padding_extend_rows`, the application
struct, the error enum + six `From` impls; production-only, no `#[cfg(test)]` on
the composition); confirmed the equivalence test is substantive (identical
per-stage applications + `Contents`/`uniforms`/`row_dirty`); "stops before
presentation" holds by construction; and the suite evidence (4364 → now 4365
passed, rc=0, default parallelism). **Verdict: CHANGES REQUIRED → fixed.**

- **Required — stale README index status.** The 838 index line still read
  `Designed`. **Fixed:** flipped to `Pass`.
- **Optional — fail-fast under-delivered vs the design's "each stage in turn."**
  **Addressed:** added a third, mid-sequence fail-fast test (`PaddingExtend`,
  after `format_rows` mutated), and documented that the remaining variants
  (`TextOverlays`/`CursorUniforms`) propagate by the same compiler-enforced
  `?`+`From` mechanism while `RebuildUniforms` cannot fail from a valid plan.
  The narrowed-but-justified criterion is recorded.
