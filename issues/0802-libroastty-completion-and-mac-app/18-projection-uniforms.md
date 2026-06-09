+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"
+++

# Experiment 18: Phase C — wire the projection/screen-size uniforms (live text)

## Description

Exp 17 fixed atlas coherence and proved (by a GPU-readback test) that glyphs
render **given a correct projection** — but discovered the live path never sets
up the projection/screen-size uniforms, so glyphs map outside NDC and render
off-screen. The `FrameRenderer`'s rebuild only touches `grid_size`
(`FrameRebuildUniformInput` carries just `padding_color`); the **projection
matrix, `screen_size`, and `cell_size`** come from
`MetalUniforms::update_screen_size` / `update_font_grid`, which the app normally
drives (upstream `updateScreenSizeUniforms` / `updateFontGridUniforms`).
`build_live_renderer` / `present_live` call **neither**. This is the last gap
before the launched app shows real terminal text.

## Approach

Drive the screen-size + font-grid uniforms from the live surface state, every
present (so it is correct on first frame and on resize) — in **physical-pixel
space**, matching the compositor target.

0. **Retina-scale reconciliation (Required, from the review).** The compositor
   target + `screen_size` are in **physical pixels** (`width_px`/`height_px`),
   but the grid font is rasterized at **point** size
   (`Face::new("Menlo", font_size)`), so `metrics.cell_*` are point-space and
   the shader positions glyphs as `projection × (cell_size × grid_pos)` with no
   scale factor — on a 2× display the grid would fill only the top-left quarter
   at half size. Fix: in `build_live_renderer`, rasterize the grid font at
   **`font_size × scale`** so `metrics.cell_*` (hence the `cell_size` uniform)
   are physical pixels matching the `width_px` screen. Then
   `cols × cell_phys == width_px` and the grid fills the window. (`scale` is
   already a `build_live_renderer` arg, used for the layer bounds.)
1. **`FrameRenderer`**: add
   `update_screen(&mut self, size: Size, grid: GridSize, metrics: &Metrics)`
   that calls `self.uniforms.update_screen_size(size, grid)` +
   `self.uniforms.update_font_grid(metrics)`. (The rebuild then updates
   `grid_size`/contents on top; these set the projection + cell pixel size the
   rebuild does not.)
2. **`present_live`** (`lib.rs`): before `render_and_present_frame`, build a
   `Size` from the surface — `screen: ScreenSize { width, height }` using the
   **clamped `width`/`height` locals** already computed for the present
   (`.max(1)`, physical px — never the raw `self.size.*_px` which are 0
   pre-`set_size` and would feed `inf` into `ortho2d`),
   `cell: shared_grid.cell_size()` (physical px after step 0),
   `padding: Padding::default()` (zero for now) — and a
   `GridSize { columns, rows }` from `self.size.columns`/`rows`, then call
   `frame_renderer.update_screen(size, grid, &shared_grid.metrics)`. Same
   `width`/`height` as the present, so the projection and the target agree in
   physical pixels.
3. **Leave the background color** at the `from_config` default for this slice
   (the cells' own bg comes from the terminal via `Contents`); a faithful
   `update_bg_color` from the terminal is a follow-up Nit if the padding/clear
   color looks wrong.

This touches **only `libroastty`** (`FrameRenderer` + `present_live`). No app
changes.

## Verification

1. **`cargo test -p roastty`** (full) green — the new `update_screen` method is
   exercised by the Exp-17 readback test path indirectly; add/extend a unit test
   if useful, but the existing present tests must stay green.
2. **App launch (the payoff):** rebuild RoasttyKit + app, launch (Exp-16 gives a
   live shell), capture the window (full-screen `screencapture` + `sips` crop,
   per Exp 15). **The shell prompt renders as actual text** on screen — the
   first real terminal frame from libroastty. Kill the spawned app + children (0
   dangling PIDs); screenshot out-of-repo.
3. **Cross-check scale parity (not just "text"):** the captured crop shows
   readable glyphs that **fill the window** (not confined to the top-left
   quadrant / half size — which would mean the Retina reconciliation in step 0
   is wrong), anchored at the origin spanning the expected width; compare
   against a real-Ghostty capture for sanity.

**Pass** = the projection/screen uniforms are driven from the surface in
physical-pixel space (Retina-correct), the suite is green, and the launched app
**renders the shell prompt as text** filling the NSView (not a half-size
top-left quadrant).

**Partial** = uniforms are wired + tests green, but the live capture still
doesn't show text for a further pinned reason — most likely the **one-shot
present timing** (the present fires from `set_size`/`start_termio` before the
shell emits its prompt, and nothing re-presents), which is the Exp-19 continuous
`CVDisplayLink` driver. Documented with evidence (e.g. a present forced after a
delay does show text).

**Fail** = text still won't render with a correct projection (documented as the
real blocker).

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: CHANGES REQUIRED → addressed.** It **confirmed the core
mechanism**: the rebuild's only geometry write is `update_grid_size`
(`frame_rebuild.rs:1149`) — it never touches
`projection_matrix`/`screen_size`/`cell_size`, so the Exp-18 values survive to
the present (the Exp-17 test already proves the ordering); per-present recompute
is fine (FrameRenderer owns uniforms across frames); the `Size`/grid
construction units are right and `padding:0` doesn't divide-by-zero; the borrows
are sound; and the one-shot-present **timing** (the async prompt sets `dirty`
but nothing re-presents — `apply_termio_event` doesn't `wakeup`) is honestly
flagged as the likely Partial → Exp 19. One Required + two Optional, folded in:

- **Required — Retina scale mismatch.** `screen` is physical-pixel (`width_px`)
  but the font is rasterized at point size (`scale` only used for layer bounds;
  the shader has no scale factor), so on 2× the grid fills only the top-left
  quarter at half size. **Fixed:** step 0 rasterizes the grid font at
  `font_size × scale` so cell metrics are physical pixels.
- **Optional — pre-`set_size` `inf` projection.** **Fixed:** use the clamped
  `width`/`height` locals, not raw `self.size.*_px`.
- **Optional — verification wouldn't catch a 2× error** (glyphs still show).
  **Fixed:** the cross-check now asserts the text **fills the window** (not a
  half-size quadrant).

## Result

**Result:** Pass — **the launched Roastty app renders the live shell prompt as
real text.** This is the first real terminal frame from libroastty in the
renamed Ghostty app — the culmination of Phase C's render path (Exp 14 runs → 15
layer attach → 16 shell → 17 atlas → 18 projection).

### Changes (only `libroastty`)

- **`build_live_renderer`** rasterizes the grid font at **`font_size × scale`**,
  so `metrics.cell_*` are physical pixels matching the px-space compositor
  target/projection (Retina reconciliation).
- **`FrameRenderer::update_screen(size, grid, metrics)`** → `update_screen_size`
  (ortho projection + `screen_size`) + `update_font_grid` (cell pixel size) —
  the geometry uniforms the rebuild never touches.
- **`present_live`** drives `update_screen` every present from the surface
  state, in physical-pixel space (clamped `width`/`height`,
  `shared_grid.cell_size()`, grid from `self.size.columns/rows`).

### Evidence (live launch, captured out-of-repo, app + children killed — 0 dangling PIDs)

The window (`name="👻"`, 800×632) renders, top-left, **Retina-correct (full
size, filling the width — not a half-size quadrant)**:

- the **shell prompt `ryan@rxc termsurf %`** with a block cursor after it;
- Ghostty's own debug-build banner: "⚠️ You're running a debug build of Roastty!
  Performance will be degraded." (proof the _unaltered_ app's own UI text
  renders through libroastty);
- crisp glyphs at the correct cell pitch. No present error in stderr.

(Capture note: `screencapture -l`/`-R` fail on the IOSurface-layer window, and
`winid.swift` resolved the wrong window — the real Roastty window is
`name="👻"`. Reliable path: enumerate `CGWindowListCopyWindowInfo` by the
Roastty PID to get the `👻` window bounds, then full-screen `screencapture` +
the new `scripts/roastty-app/crop.swift` to crop the region.)

### Verification

- **Full `cargo test -p roastty`** green (the present tests + the Exp-17
  readback test pass; the new `update_screen` is on the live path, exercised by
  the launch).
- **App renders the prompt as text** (above), Retina-correct — the Pass bar
  (text fills the NSView, not a quadrant) is met.

## Conclusion

**libroastty puts a real terminal on screen.** The copied-and-renamed Ghostty
app — unaltered except for the `ghostty→roastty` rename — boots
`roastty_app_new`/`surface_new`, starts a shell, and renders its prompt as text
into its own NSView, all through the reconciled embedded ABI and the Rust
renderer. The conformance oracle is now _live_.

**What renders is the first frame.** The present fires only from
`set_size`/`start_termio`/`draw` (Exp 16); the async shell output that arrives
later sets `dirty` but nothing re-presents, so **typing / new output won't
update live** yet. **Next (Exp 19): the continuous `CVDisplayLink` render
driver** — a library-internal loop that presents on dirty, matching upstream's
`renderer/Thread.zig` — after which the terminal is fully live, and
feature-by-feature conformance testing (typing, selection, scrollback, colors,
resize, …) can begin.

## Result Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED.** It independently verified:
`cargo build -p roastty` clean (0 warnings), `present_samples_grid_atlas_…` +
`render_and_present_frame_presents` pass, `cargo fmt --check` clean; the
projection is **not clobbered** (the rebuild's only geometry write is
`update_grid_size` at `frame_rebuild.rs:1149` — never
`projection_matrix`/`screen_size`/ `cell_size`; `update_screen` runs before the
present); the **Retina scale is consistent** (the same `scale_factor_x` drives
the font rasterization, the cell metrics, the layer bounds, the compositor
target, and `contents_scale` — `cols × cell_phys ≈ width_px`, no half/double
mismatch); the **borrows are sound** and `columns`/`rows`/`width`/`height` are
captured as locals before the borrows (clamped, so no `inf`); **"Pass" is
honest** (`apply_termio_event` only sets `dirty` and never presents, so live
updates genuinely need Exp 19; the prompt was caught by an initial
`set_size`/`start_termio`/`draw` present, evidenced by the specific content);
and the **scope is clean** (only the two `libroastty` edits; the new
`scripts/roastty-app/{crop,list-windows}.swift` are standalone tooling, no app
edits). Three non-blocking findings: one Optional **fixed** (added a direct unit
test `update_screen_sets_projection_screen_and_cell` asserting the projection /
screen_size / cell_size); two noted for later slices (the `GridSize` from
`self.size` only perturbs `grid_padding` cosmetically; the font cell binds
`scale` at first build so a later DPI-change would desync — a pre-existing gap
for a rebuild-on-scale-change slice).
