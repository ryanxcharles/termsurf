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

# Experiment 843: Derive the cursor sub-inputs and screen_fg from the terminal

## Description

Exp 842 derived the render input's colors/palette from the live terminal but
left the cursor (`text_overlay.cursor`, `cursor_uniform.block_cursor`) and
`screen_fg` as caller-supplied `FrameRenderKnobs`. This experiment, the next
slice of the input-derivation arc, derives them from the terminal cursor state —
the same sources the existing GUI render path uses (`terminal.cursor_visible()`,
`cursor_visual_style()`, `color_effective(Cursor)`; `render_cursor_visual_style`
at `lib.rs`). The reusable mapping
`renderer::cursor::Style::from_terminal( VisualStyle)` already exists.

Derivation rules (mirroring the existing path; ghostty's exact cursor color
resolution and wide-cell handling are simplified, see deferrals):

- **Visibility gates it:** when `terminal.cursor_visible()` is false, both the
  overlay cursor and the block-cursor uniform are `None`.
- **Style:** `Style::from_terminal(terminal.cursor_visual_style())`.
- **Color:** `color_effective(TerminalColorKind::Cursor)` mapped to `Rgb`,
  falling back to the terminal foreground (`default_fg`) when unset.
- **Block-cursor uniform only for `Block`:** the `cursor_uniform.block_cursor`
  is set only when the derived style is `Style::Block` (other styles render
  purely as the overlay glyph); the overlay cursor is set for every visible
  style.
- **`screen_fg`** = the terminal effective foreground (`default_fg`).

Deferred (named, not hidden):

- **Cursor `wide`** — both the overlay `wide: bool` and the block `wide: Wide`
  are set to narrow (`false` / `Wide::Narrow`) here; the real value comes from
  the cell under the cursor and is a follow-up slice.
- **Cursor color nuance** — ghostty also supports an inverted/contrast cursor
  color when no explicit color is configured; this slice uses
  `color_effective(Cursor)` + foreground fallback only.
- **Blink/focus gating** — `cursor_blinking()` / focus are not consulted yet.

## Changes

`roastty/src/renderer/frame_renderer.rs` (production code + tests).

- **Remove** `cursor`, `block_cursor`, and `screen_fg` from `FrameRenderKnobs`
  (now derived). The knobs keep the not-yet-config-sourced fields (`bold`,
  `alpha`, `faint_opacity`, `thicken`, `thicken_strength`,
  `background_opacity_cells`, `background_opacity`, `padding_color`,
  `overlay_alpha`). The 842 test helper `render_knobs()` drops those three
  initializers accordingly (compiler-enforced).

- **Add a derived-cursor field** to `FrameRenderState`:

  ```rust
  // Some(style, color) when the terminal cursor is visible; None otherwise.
  cursor: Option<(CursorStyle, Rgb)>,
  screen_fg: Rgb,
  ```

- **`from_terminal`** additionally derives `cursor` (visibility-gated, style via
  `Style::from_terminal`, color via `color_effective(Cursor)` → `default_fg`
  fallback) and `screen_fg = default_fg`.

- **`rebuild_input`** builds the cursor sub-inputs from `self.cursor`:
  `text_overlay.cursor = self.cursor.map(|(style, color)| FrameSnapshotCursorOverlayInput { style, wide: false, color })`;
  `cursor_uniform.block_cursor = self.cursor.filter(|(style, _)| matches!(style, CursorStyle::Block)).map(|(_, color)| FrameSnapshotBlockCursorUniformInput { wide: Wide::Narrow, color })`;
  `text_overlay.screen_fg = self.screen_fg`. `text_overlay.alpha` stays
  `knobs.overlay_alpha`.

No change to the pipeline or `FrameRenderer`.

## Verification

Per the bounded-run convention (15-min cap, Central-stamped, single tracked
task, no poll-watcher). Fast non-Metal unit tests in `frame_renderer.rs`:

- **Visible cursor derives an overlay:** a terminal with a visible cursor yields
  `from_terminal().cursor == Some((Style::from_terminal(visual_style), color))`;
  `rebuild_input`'s `text_overlay.cursor` is `Some` with that style/color and
  `screen_fg == default_fg`. With no OSC-12 set, the color is the `default_fg`
  fallback.
- **Cursor color flows from `color_effective(Cursor)`:** feed `\x1b]12;rgb:...`
  (OSC-12, set cursor color) and assert the derived cursor color is that value
  (not the `default_fg` fallback) — proving the color genuinely comes from
  `color_effective(Cursor)`.
- **Block style sets the uniform, non-block does not:** with the terminal cursor
  style Block, `cursor_uniform.block_cursor` is `Some`; force a non-block visual
  style and assert it is `None` while the overlay cursor remains `Some`.
- **Hidden cursor → no overlay/uniform:** when `cursor_visible()` is false, both
  are `None`.
- **Still drives a frame:** `FrameRenderState::from_terminal` + `rebuild_input`
  feeds `FrameRenderer::update_frame` on a 4×3 terminal and rebuilds the full
  frame (the derived cursor input is valid end to end).
- `cargo build -p roastty` — no warnings. `cargo fmt -p roastty -- --check` —
  clean. Full suite via `scripts/bounded-run.sh` (default parallelism) stays
  green. No-ghostty grep — clean. `git diff --check` — clean.

**Pass** = the new cursor-derivation tests pass, a terminal-derived input (now
with the cursor derived) rebuilds a frame, and the full suite stays green.
**Partial/Fail** = any test fails or the suite regresses.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). Verified on six axes and confirmed: the cursor derivation is correct
(`Style::from_terminal` maps the three terminal-reachable styles; visibility is
double-gated by `cursor_visible()` + the existing `cursor_viewport` gate); **no
double-draw** for Block (the overlay rect + the block uniform's under-text
recolor are the established Exp-838 contract, and gating the uniform on exactly
`Block` is forward-correct for unfocused `BlockHollow`); the
`color_effective(Cursor)` → `default_fg` fallback mirrors
`render_state_from_terminal`/upstream; `screen_fg = default_fg` is the right
preedit source; `CursorStyle` is `Copy` so the tuple field

- `.map`/`.filter`/`matches!` work; tests are controllable (`CursorVisible`
  defaults true, toggles via `\x1b[?25l`; DECSCUSR `\x1b[2 q`/`\x1b[4 q` wired);
  `FrameRenderKnobs` has no callers outside `frame_renderer.rs`.

**Verdict:** APPROVED, no Required findings. Two adopted:

- **Optional — color test was vacuous.** A fresh terminal has no OSC-12, so the
  color assertion collapsed to the fallback. **Fixed:** added a test feeding
  `\x1b]12;rgb:...` and asserting the derived cursor color is that value,
  exercising the `color_effective(Cursor)`-is-`Some` branch.
- **Nit — name the helper to update.** **Fixed:** the Changes section names the
  842 `render_knobs()` helper as dropping the three initializers.

## Conclusion

_(to be written after the run)_
