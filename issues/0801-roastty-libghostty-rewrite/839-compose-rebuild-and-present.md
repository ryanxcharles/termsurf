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

# Experiment 839: Compose rebuild-and-present (add the Metal presentation stage)

## Description

Exp 838 composed the prepared frame rebuild
(`FrameTerminalSnapshot::rebuild_frame`) and **deliberately stopped before Metal
presentation**. This experiment adds the presentation stage so the standalone
composition spans the full snapshot → rebuilt-frame → **presented-frame** path.
It is still bottom-up and standalone (not yet wired into the renderer loop) —
the pieces stay individually testable before the larger renderer-thread
integration.

The one structural constraint: `present_metal_frame` (Exp 822) is a method on
`FrameRebuildPlan` and validates its input against `self.effective_grid`
(`frame_rebuild.rs:906`). So the presentation must use the **same plan** that
drove the rebuild — the composition must build the plan **once** and reuse it
for both the five rebuild stages and presentation (building a second plan would
be redundant and risks divergence).

## Changes

`roastty/src/renderer/frame_rebuild.rs` (production code + tests).

- **Refactor (additive, behavior-preserving):** extract the five-driver body of
  `rebuild_frame` into a private helper that takes a pre-built plan:

  ```rust
  fn run_rebuild_stages(
      &self,
      plan: &FrameRebuildPlan,
      targets: FramePreparedRebuildTargets<'_>,
      input: FramePreparedRebuildInput<'_>,
  ) -> Result<FramePreparedRebuildApplication, FramePreparedRebuildError>
  ```

  `rebuild_frame` becomes `self.build_plan()? ` then
  `run_rebuild_stages(&plan, …)` — its public signature, behavior, and the Exp
  838 tests are unchanged.

- **Add the presentation input bundle** (the presentation-only inputs not
  produced by the rebuild):

  ```rust
  pub(crate) struct FramePreparedPresentationInput<'a> {
      pub(crate) compositor: &'a mut MetalFrameCompositor,
      pub(crate) width: usize,
      pub(crate) height: usize,
      pub(crate) contents_scale: f64,
      pub(crate) grayscale_atlas: &'a Atlas,
      pub(crate) color_atlas: &'a Atlas,
  }
  ```

- **Add the composition** on `FrameTerminalSnapshot`:

  ```rust
  pub(crate) fn rebuild_and_present_frame(
      &self,
      targets: FramePreparedRebuildTargets<'_>,
      input: FramePreparedRebuildInput<'_>,
      presentation: FramePreparedPresentationInput<'_>,
  ) -> Result<FramePreparedFrameApplication, FramePreparedFrameError>
  ```

  which builds the plan once — routing the plan-build error through the rebuild
  error so it lands as `Rebuild(Plan(..))`:
  `let plan = self.build_plan().map_err(FramePreparedRebuildError::from)?;` —
  then calls `run_rebuild_stages(&plan, <reborrowed targets>, input)`, then
  `plan.present_metal_frame(presentation.compositor, FrameMetalPresentationInput { width, height, contents_scale, uniforms: targets.uniforms, contents: targets.contents, grayscale_atlas, color_atlas })`.
  The targets' `&mut` fields are **reborrowed** for the rebuild call so
  `targets.contents`/`uniforms` are then read immutably for presentation
  (disjoint immutable borrows of two fields — sound).

- **Add `FramePreparedFrameApplication`** =
  `{ rebuild: FramePreparedRebuildApplication, present: FrameMetalPresentationApplication }`.

- **Add `FramePreparedFrameError`** =
  `{ Rebuild(FramePreparedRebuildError), Present(FrameMetalPresentationError) }`
  with `From` impls so `?` flows. It is **`Debug`-only** (no `PartialEq`/`Eq`/
  `Clone`) because `FrameMetalPresentationError` derives only `Debug`; tests
  assert the variant via `matches!` (the Exp 822 style). Fail-fast: a
  rebuild-stage (or plan-build) failure returns `Rebuild(...)` and presentation
  does not run.

No driver/adapter change; presentation still goes through the existing
`present_metal_frame`; no renderer-thread wiring.

## Verification

Per the bounded-run convention (15-min cap, Central-stamped, single tracked
task, no poll-watcher). The presentation tests need a Metal device, so they
follow the existing `let Some(device) = metal_device() else { return; };` guard
(as the Exp 822 `present_metal_frame_*` tests do):

- **838 regression:** the existing `rebuild_frame` tests still pass (the
  refactor is behavior-preserving).
- **Happy path:** a dirty-row snapshot drives rebuild **and** present — the
  returned `FramePreparedFrameApplication` reports the rebuild stages applied
  (rows/overlay/uniforms/padding/cursor) **and** the presentation
  (`foreground_drawn`, dimensions), and the presented frame matches what
  `present_metal_frame` returns for the rebuilt contents.
- **Equivalence:** the composed `rebuild_and_present_frame` produces the same
  presentation application as `rebuild_frame` followed by a hand call to
  `plan.present_metal_frame` on identical inputs (same plan, same contents).
- **Fail-fast before presentation:** a rebuild-stage failure (e.g. truncated
  snapshot `row_dirty` → `Rebuild(Plan(..))`) returns the rebuild error and the
  compositor is **not** invoked (no presentation occurred — observable via an
  unchanged compositor / no foreground drawn).
- `cargo build -p roastty` — no warnings. `cargo fmt -p roastty -- --check` —
  clean. Full suite via `scripts/bounded-run.sh` (default parallelism) stays
  green. No-ghostty grep — clean. `git diff --check` — clean.

**Pass** = the new composition tests pass (Metal-device permitting), the 838
tests still pass after the refactor, and the full suite stays green.
**Partial/Fail** = any test fails or the suite regresses.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Verified the refactor is behavior-preserving, plan-reuse is correct
(`validate_metal_presentation_input` reads only `self.effective_grid` + the
input, no rebuild-consumed plan state), the reborrow/borrow plan compiles, the
new types match `present_metal_frame`, the Metal-device guard is honest, and
fail-fast holds.

**Verdict:** APPROVED, no Required findings. Two Optionals, adopted:

- **Plan-build error path.** `self.build_plan()?` yields
  `FrameRebuildPlanError`, which has no `From` into `FramePreparedFrameError`.
  **Fixed:** the design now routes it via
  `self.build_plan().map_err(FramePreparedRebuildError::from)?` so it lands as
  `Rebuild(Plan(..))`, consistent with `rebuild_frame`.
- **`FramePreparedFrameError` is `Debug`-only.** `FrameMetalPresentationError`
  derives only `Debug`, so the wrapper cannot derive `PartialEq`/`Eq`/`Clone`.
  **Fixed:** noted; tests assert via `matches!`.

## Result

**Result:** Pass

The refactor (`rebuild_frame` → `build_plan` + `run_rebuild_stages`),
`rebuild_and_present_frame`, and the `FramePreparedPresentationInput` /
`FramePreparedFrameApplication` / `FramePreparedFrameError` types landed.
Production `cargo build -p roastty` and `--tests` both clean (no warnings); fmt
clean, no-ghostty clean, `git diff --check` clean.

- **838 regression — the refactor is behavior-preserving:** all five
  `rebuild_frame_*` tests still pass unchanged.
- **`rebuild_and_present_frame_rebuilds_then_presents`** (Metal-device-guarded,
  ran on this GPU): a full-rebuild snapshot drives rebuild **and** present — the
  rebuild stages applied (`rows [0,1,2]`, block cursor applied) and the frame
  **presented** at the requested 8×6 drawable.
- **`rebuild_and_present_frame_fails_fast_before_presentation`**
  (Metal-guarded): a truncated snapshot `row_dirty` → `Rebuild(Plan(..))` —
  never `Present(..)`, proving presentation was not reached.

**Equivalence** is established by construction rather than a GPU side-by-side:
`rebuild_frame` and `rebuild_and_present_frame` call the **same**
`run_rebuild_stages` on the **same** plan, and presentation goes through the
unchanged `present_metal_frame` (Exp 822) — so the rebuild portion is provably
identical (and the 838 regression suite confirms `run_rebuild_stages` is
behavior-preserving). A literal two-compositor present comparison was omitted as
GPU-state-dependent and redundant.

**Full suite (default parallelism, `scripts/bounded-run.sh`):**
`4367 passed; 0 failed` (4360 + 5 from 838 + 2 from 839), 0 panics, 0
`PoisonError`, `STATUS=COMPLETED rc=0`, 191 s — green.

## Conclusion

The standalone composition now spans the full snapshot → rebuilt-frame →
**presented-frame** path: `rebuild_and_present_frame` builds one plan, runs the
five rebuild stages, and presents via Metal, with fail-fast error propagation.
The single-plan reuse keeps presentation validating against the same
`effective_grid` the rebuild targeted.

What remains for the live renderer path is the **renderer-thread wiring** the
815–839 pieces were built bottom-up for: calling `rebuild_and_present_frame`
from the renderer's actual frame loop (constructing the `FrameTerminalSnapshot`
from the live terminal each frame, threading the real
`Contents`/`SharedGrid`/uniforms/ atlases/compositor, and driving it on the
render thread). That integration is the next slice — the first one that touches
the live renderer rather than the standalone, individually-tested pipeline.

## Completion Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Independently confirmed: build + fmt clean; the slice
`frame_rebuild::tests::rebuild` → 7 passed / 0 failed (Metal device present,
both 839 tests and the five 838 `rebuild_frame_*` tests executed, not skipped);
the refactor is byte-identical (only `build_plan()?` moved into the caller);
`rebuild_and_present_frame` builds one plan, reborrows for the rebuild, presents
immutably, and reaches present only after the rebuild `?` succeeds; the suite
log shows 4367 passed / 0 failed, default parallelism, rc=0. **Verdict: CHANGES
REQUIRED → fixed.**

- **Required — stale README index status.** The 839 index line still read
  `Designed`. **Fixed:** flipped to `Pass`.
- **Optional/Nit (no change required):** the omitted GPU side-by-side
  equivalence test is honestly justified (equivalence holds by construction);
  the fail-fast test asserts the `Rebuild(Plan(_))` variant
  (presentation-not-reached is structurally guaranteed by `?`).
