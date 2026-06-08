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

# Experiment 844: Derive row_never_extend from the terminal rows

## Description

Exp 842/843 derive the render input's colors, palette, and cursor from the live
terminal but `FrameRenderState.row_never_extend` is still the all-false stub
(named deferred in 842). This experiment, the next slice of the input-derivation
arc, replaces it with the real per-row derivation
`cell::row_never_extend_bg_flags` (`cell.rs:272`) — the **last** stubbed field
in the assembled input.

`row_never_extend_bg(row, palette, default_bg)` returns true when a row should
not have window padding extended into it: a semantic-prompt row, a perfect-fit
powerline cell, or any cell whose background resolves to the default background
(`cell.rs:238`). It needs the shaped rows (`&[RunOptions]`), which the terminal
provides via `terminal.shape_run_options()` — the same call the snapshot makes.

**Known perf cost (deferred, not hidden):** `from_terminal` will call
`shape_run_options()` to derive `row_never_extend`, and
`FrameTerminalSnapshot:: collect` calls it again during the frame — two shapings
per frame. They align because both shape the same terminal state. Sharing one
shaping (e.g. threading the snapshot's rows into the input, or moving the input
derivation into the snapshot) is a later refactor slice; this slice keeps
`FrameRenderState` independent of the snapshot.

## Changes

`roastty/src/renderer/frame_renderer.rs` (production code + tests).

- In `FrameRenderState::from_terminal`, replace the
  `row_never_extend: vec![false; terminal.rows() as usize]` stub with:

  ```rust
  let rows = terminal.shape_run_options();
  let row_never_extend = row_never_extend_bg_flags(&rows, &palette, default_bg);
  ```

  (`row_never_extend_bg_flags` imported from `crate::renderer::cell`.)

- Update the `FrameRenderState` doc comment: `row_never_extend` is no longer a
  stub — it is derived; the doc's "stubs until their own slices" list drops it.

No change elsewhere; `row_never_extend` length is still `terminal.rows()` (one
flag per shaped row), so padding-extend validation is unaffected.

## Verification

Per the bounded-run convention (15-min cap, Central-stamped, single tracked
task, no poll-watcher). Fast non-Metal unit tests in `frame_renderer.rs`:

- **Derived row_never_extend matches the per-row helper:** for a populated
  terminal,
  `from_terminal().row_never_extend == row_never_extend_bg_flags(&terminal.shape_run_options(), &palette, default_bg)`
  (faithful wiring), and length `== terminal.rows()`.
- **Concrete flags (all-true default case):** a 4×3 terminal yields
  `row_never_extend == [true, true, true]` — **every** row never-extends,
  because a blank cell is a `Codepoint` cell with the default style whose
  `resolve_bg` is `None`, hitting `row_never_extend_bg`'s default-background arm
  (matching upstream `row.zig`). So the all-false stub was behaviorally wrong,
  not just incomplete.
- **A non-default-background row is false:** fill one row's columns with an
  explicit non-default background (e.g. `\x1b[2;1H\x1b[41mBBBB`, palette-red bg
  on row 1) and assert that row's flag is `false` while the others stay `true` —
  proving the derivation distinguishes extend from never-extend, not just
  returns all-true.
- **Still drives a frame:** `FrameRenderState::from_terminal` + `rebuild_input`
  feeds `FrameRenderer::update_frame` on a 4×3 terminal and rebuilds the full
  frame with the derived (non-stub) `row_never_extend` (the padding-extend stage
  accepts it).
- `cargo build -p roastty` — no warnings. `cargo fmt -p roastty -- --check` —
  clean. Full suite via `scripts/bounded-run.sh` (default parallelism) stays
  green. No-ghostty grep — clean. `git diff --check` — clean.

**Pass** = the new row_never_extend tests pass, a terminal-derived input still
rebuilds a frame, and the full suite stays green. **Partial/Fail** = any test
fails or the suite regresses.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Verified the args (`&palette`/`default_bg` match the helper), the
one-flag-per-row length, the honest duplicate-shaping flag and alignment claim,
and that the 842 frame test's assertions are insensitive to the padding-extend
path.

**Verdict:** CHANGES REQUIRED → fixed. One Required + one Optional:

- **Required — wrong concrete expectation.** `[true, false, false]` was wrong: a
  blank cell is a `Codepoint` cell (`page.rs:3289`, default content tag 0) with
  the default style whose `resolve_bg` is `None`, so `row_never_extend_bg`
  returns **true** for empty rows too — a default 4×3 terminal is **all-true**
  `[true, true, true]` (faithful to upstream `row.zig:55-57`). **Fixed:** the
  concrete case now asserts `[true, true, true]`, and a separate test fills a
  row with a non-default explicit background to exercise the `false` branch.
- **Optional — stale line refs.** **Fixed:** `row_never_extend_bg_flags` is
  `cell.rs:272`, `row_never_extend_bg` is `cell.rs:238`.

(The reviewer could not, under the read-only constraint, prove
`refine_padding_extend_rows` does not panic on a `true` flag; the implementation
will verify the frame test passes before recording the result.)

## Result

**Result:** Pass

`from_terminal` now derives `row_never_extend` via
`row_never_extend_bg_flags(&terminal.shape_run_options(), &palette, default_bg)`
(replacing the all-false stub); the `FrameRenderState` doc comment moved
`row_never_extend` out of the stub list. Production `cargo build -p roastty` and
`--tests` both clean (no warnings); fmt clean, no-ghostty clean,
`git diff --check` clean.

Three new tests, all passing (and the 842 frame test still rebuilds a frame with
the all-true flags — no `refine_padding_extend_rows` panic, resolving the design
review's open question):

- **`render_state_row_never_extend_matches_helper`** — the derived flags equal
  `row_never_extend_bg_flags` applied to the terminal's shaped rows; length 3.
- **`render_state_default_terminal_never_extends_every_row`** — a default 4×3 is
  `[true, true, true]` (blank cells are default-bg codepoint cells).
- **`render_state_non_default_background_row_may_extend`** — a row filled with a
  palette-red background (`\x1b[2;1H\x1b[41mBBBB`) is `false` while the
  default-background rows around it are `true` — the derivation distinguishes
  extend from never-extend.

**Full suite (default parallelism, `scripts/bounded-run.sh`):**
`4384 passed; 0 failed` (4381 + 3 new), 0 panics, 0 `PoisonError`,
`STATUS=COMPLETED rc=0`, 281 s — green.

## Conclusion

`row_never_extend` — the last stubbed input field — is now derived. Every field
of `FramePreparedRebuildInput` except the three dynamic buffers
(`highlights`/`link_ranges`/`selection_config`, still empty/default) and the
config-knobs now comes from the live terminal.

Continuing the input-derivation arc, in order:

- **Exp 845:** derive selection / highlights / link ranges from the terminal
  (the remaining empty dynamic buffers).
- **Exp 846+:** the **configuration sub-arc** — port `font-thicken`,
  `font-thicken-strength`, `minimum-contrast` (→ `alpha`/`faint_opacity`);
  source the remaining knobs (`bold_color`, `background_opacity`,
  `window_padding_color`) from `Config`; then have `FrameRenderer::update_frame`
  take `&FrameRenderState` + `&FrameRenderKnobs` directly (and consider sharing
  one `shape_run_options` between `FrameRenderState` and the snapshot — the perf
  cost noted above); finally build the input from live surface state in
  `surface.draw()`.

## Completion Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Independently confirmed: the diff matches the design (the
`row_never_extend_bg_flags` import, the `from_terminal` derivation with the
derived `&palette`/`default_bg`, the doc-comment update, 3 new tests); only
`frame_renderer.rs` changed; 17/17 frame_renderer tests pass (3 new + 840–843);
the non-default-bg test is genuine (row 1 red-filled → `false`, rows 0/2 →
`true`); length == `terminal.rows()`; v1.log shows 4384 passed / 0 failed, rc=0,
default parallelism, no timeout; fmt/build clean. **Verdict: CHANGES REQUIRED →
fixed.**

- **Required — stale README index status.** Flipped 844 `Designed → Pass`.
- **Optional — self-citing comment.** The "sharing one shaping is a later
  refactor" comment cited Exp 844 (itself); **fixed** to Exp 846+ (where the
  shared-shaping refactor is scheduled).
