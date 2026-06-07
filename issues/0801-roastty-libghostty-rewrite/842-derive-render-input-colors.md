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

# Experiment 842: Derive the render input's colors/palette from the live terminal

## Description

`FrameRenderer` (Exp 840/841) drives the frame end to end but still takes the
`FramePreparedRebuildInput` as a hand-built parameter. Replacing that parameter
with a value derived from the live surface/config/terminal is a **deep arc** of
its own: the bundle needs ~20 fields from config + terminal-dynamic state. Per
the issue's working directive, this experiment scopes that arc to its first
concrete, testable slice: **derive the terminal-effective colors and palette** —
the same data the existing GUI render path pulls via
`terminal.color_effective(...)` / `terminal.palette_current()` (`lib.rs:6578`) —
and assemble a complete input around them.

Two dependencies are made explicit (investigated, not deferred silently):

- **Config-sourcing is deferred for the whole knobs struct.** This slice takes
  no `Config` at all — so _every_ `FrameRenderKnobs` field is caller-supplied
  for now, including the ones that **do** exist in `Config` today (`bold_color`,
  `background_opacity`, `window_padding_color` at `config/mod.rs:78,80,98`);
  sourcing them from `Config` is a later slice. Separately, four of the knobs —
  `alpha`, `faint_opacity`, `thicken`, `thicken_strength` — are **not in
  roastty's `Config` at all** (confirmed: only `alpha_blending`, no
  `thicken`/`faint_opacity`), so they _additionally_ require porting new config
  options (a configuration-arc slice) before they can ever be config-sourced.
- **Deferred to later renderer slices, each named:** deriving the cursor
  sub-inputs from the terminal cursor state; `screen_fg` (the preedit/IME
  foreground) — left in knobs here, sourced with the cursor/preedit slice;
  selection/highlights/link-range derivation (these stay empty here — the common
  no-selection case); **`row_never_extend` via `row_never_extend_bg_flags`
  (`cell.rs:272`)** — a real per-row derivation stubbed all-false here; and
  sourcing the knobs from the surface config.

## Changes

`roastty/src/renderer/frame_renderer.rs` (production code + tests).

- Add `FrameRenderKnobs` — the caller-supplied, not-yet-config-sourced knobs:

  ```rust
  pub(crate) struct FrameRenderKnobs {
      pub(crate) bold: Option<BoldColor>,
      pub(crate) alpha: u8,
      pub(crate) faint_opacity: u8,
      pub(crate) thicken: bool,
      pub(crate) thicken_strength: u8,
      pub(crate) background_opacity_cells: bool,
      pub(crate) background_opacity: f64,
      pub(crate) padding_color: WindowPaddingColor,
      pub(crate) cursor: Option<FrameSnapshotCursorOverlayInput>,
      pub(crate) block_cursor: Option<FrameSnapshotBlockCursorUniformInput>,
      pub(crate) screen_fg: Rgb,
      pub(crate) overlay_alpha: u8,
  }
  ```

- Add `FrameRenderState` owning the **terminal-derived** data plus the empty
  dynamic buffers the input borrows:

  ```rust
  pub(crate) struct FrameRenderState {
      default_fg: Rgb,
      default_bg: Rgb,
      palette: Palette,
      highlights: Vec<Vec<Highlight>>,   // empty until the highlights slice
      link_ranges: Vec<Vec<[u16; 2]>>,   // empty until the links slice
      selection_config: SelectionConfig, // default until the selection slice
      row_never_extend: Vec<bool>,       // STUB: all-false until the slice that
                                         // derives it via row_never_extend_bg_flags
  }
  ```

- `FrameRenderState::from_terminal(terminal: &Terminal) -> Self` — `default_bg`
  from `color_effective(Background)` (fallback black), `default_fg` from
  `color_effective(Foreground)` (fallback white), `palette` from
  `palette_current()` (mapping the same way the GUI path does), the dynamic
  buffers empty/default and `row_never_extend` sized to `terminal.rows()`.

- `FrameRenderState::rebuild_input<'a>(&'a self, knobs: &'a FrameRenderKnobs) -> FramePreparedRebuildInput<'a>`
  — assemble the full bundle: `row_format` borrows
  `self.palette`/`highlights`/`link_ranges`/`selection_config` and the terminal
  `default_fg`/`default_bg`, with the knob fields threaded; `text_overlay` /
  `cursor_uniform` from `knobs.cursor` / `knobs.block_cursor`; `rebuild_uniform`
  and `padding_extend` from `knobs.padding_color` + `self.row_never_extend`.

No change to the existing pipeline or `FrameRenderer`; this is purely the input
assembly. (A follow-up slice can have `FrameRenderer::update_frame` accept a
`&FrameRenderState` + `&FrameRenderKnobs` instead of a raw input.)

## Verification

Per the bounded-run convention (15-min cap, Central-stamped, single tracked
task, no poll-watcher). Fast non-Metal unit tests in `frame_renderer.rs`:

- **Colors derive from the terminal:** a terminal with set background/foreground
  (via OSC or config palette) yields a `FrameRenderState` whose `default_bg`/
  `default_fg` equal the terminal's `color_effective` values, and whose
  `palette` matches `palette_current()`. Fallbacks (black/white) apply when
  unset.
- **`rebuild_input` drives a real rebuild:** `FrameRenderState::from_terminal` +
  `rebuild_input(&knobs)` feeds
  `FrameTerminalSnapshot::collect(...).rebuild_frame` (or
  `FrameRenderer::update_frame`) on a 4×3 terminal and rebuilds all rows —
  proving the assembled input is valid end to end.
- **`row_never_extend` is sized to the terminal rows** (so a full rebuild's
  padding-extend validation passes for any row).
- `cargo build -p roastty` — no warnings. `cargo fmt -p roastty -- --check` —
  clean. Full suite via `scripts/bounded-run.sh` (default parallelism) stays
  green. No-ghostty grep — clean. `git diff --check` — clean.

**Pass** = the new `FrameRenderState` tests pass, a terminal-derived input
rebuilds a frame, and the full suite stays green. **Partial/Fail** = any test
fails or the suite regresses.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Confirmed the color/palette derivation faithfully mirrors
`render_state_from_terminal` ((u8,u8,u8)→`color::Rgb` via `Rgb::new`,
black/white fallbacks, 256-entry palette); all five sub-inputs + every
`FrameSnapshotRowFormatInput` field are covered by state-or-knob; the
`rebuild_input<'a>(&'a self, &'a knobs)` lifetimes work; `row_never_extend`
sized to `terminal.rows()` passes full-rebuild validation; the four missing
config fields are genuinely absent; the tests are feasible.

**Verdict:** CHANGES REQUIRED → fixed. One Required + three Optionals, all
adopted:

- **Required — `row_never_extend` stub not named as deferred.** There is a real
  derivation `row_never_extend_bg_flags` (`cell.rs:272`); the all-false stub was
  presented as the natural value, not a behavioral stub. **Fixed:** the struct
  comment and the deferred list now name `row_never_extend_bg_flags` as the
  future derivation, like the other stubs.
- **Optional — `background_opacity` type.** The target field and `Config` are
  `f64`, not `f32`. **Fixed:** knob declared `f64`.
- **Optional — config rationale.** **Fixed:** clarified the whole struct is
  caller-supplied because config-sourcing is deferred; the four absent fields
  are an _additional_ future config-port.
- **Optional — `screen_fg`.** **Fixed:** noted it stays in knobs (preedit/IME
  foreground), sourced with the cursor/preedit slice.

## Conclusion

_(to be written after the run)_
