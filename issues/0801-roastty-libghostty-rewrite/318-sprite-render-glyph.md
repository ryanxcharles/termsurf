+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 318: the sprite render-to-atlas (renderGlyph)

## Description

With the codepoint dispatch (`draw_codepoint`, Experiment 316) and the coverage
predicate (`has_codepoint`, Experiment 317) in place, this experiment ports the
sprite **`renderGlyph`**: size a padded `Canvas`, draw the codepoint, write the
trimmed result to the atlas, and return a `Glyph` with the atlas region and the
bearings. This is the rendering half of the sprite `Face` — the piece the
resolver will call to fill its deferred `SpriteUnavailable` arm (the resolver
wiring itself is a later experiment).

## Upstream behavior (`sprite/Face.zig` `renderGlyph`)

- `width = cell_width` (× the run's cell count for wide glyphs — deferred here);
  `height = cell_height`.
- `padding_x = width / 4`; `padding_y = height / 4`.
- `canvas = Canvas.init(width, height, padding_x, padding_y)`.
- `draw(cp, canvas, width, height, metrics)` (the codepoint dispatch).
- `region = canvas.writeAtlas(atlas)` (trims the clip margins, reserves an atlas
  region, blits the trimmed buffer).
- the returned `Glyph`:
  - `width = region.width`, `height = region.height`;
  - `offset_x = clip_left − padding_x`;
  - `offset_y = (region.height +| clip_bottom) − padding_y` (saturating add);
  - `atlas_x = region.x`, `atlas_y = region.y`.

## Rust mapping (`roastty/src/font/sprite/mod.rs`, `canvas.rs`)

- `roastty/src/font/sprite/canvas.rs`: add
  `pub(crate) fn clip_left(&self) -> u32` and
  `pub(crate) fn clip_bottom(&self) -> u32` (the `Glyph` bearings need the
  post-`write_atlas` trim margins; the fields are private to the `canvas`
  module).
- `roastty/src/font/sprite/mod.rs`:
  `pub(crate) fn render_codepoint(cp: u32, metrics: &Metrics, atlas: &mut Atlas) -> Result<Option<Glyph>, AtlasError>`
  — `width = metrics.cell_width`, `height = metrics.cell_height`;
  `padding_x = width / 4`, `padding_y = height / 4`; build a
  `Canvas::new(width, height, padding_x, padding_y)`; if
  `!draw::draw_codepoint(cp, metrics, &mut canvas)` return `Ok(None)`;
  `let region = canvas.write_atlas(atlas)?`; return
  `Ok(Some(Glyph { width: region.width, height: region.height, offset_x: canvas.clip_left() as i32 − padding_x as i32, offset_y: (region.height .saturating_add(canvas.clip_bottom())) as i32 − padding_y as i32, atlas_x: region.x, atlas_y: region.y }))`.

## Scope / faithfulness notes

- **Ported**: the sprite `renderGlyph` — the codepoint → padded canvas → trimmed
  atlas region → `Glyph` pipeline.
- **Deferred**: the wide-glyph `cell_width` factoring (single-cell only here),
  the sprite-kind special glyphs, and the resolver wiring that calls this to
  fill `SpriteUnavailable`.
- No C ABI/header/ABI-inventory change (the `Glyph`/`Atlas` types are internal
  Rust).

## Changes

1. `roastty/src/font/sprite/canvas.rs`: add the `clip_left`/`clip_bottom`
   accessors.
2. `roastty/src/font/sprite/mod.rs`: add `render_codepoint`; note it in the
   module doc.
3. Tests (deterministic — the fixture `9×18` cell metrics, a fresh
   `Atlas::new(64, Grayscale)`):
   - `render_codepoint_box_line`: `render_codepoint(0x2500, …)` returns
     `Ok(Some(glyph))` with `glyph.width > 0` and `glyph.height > 0` (the
     trimmed horizontal line), a reserved atlas region (`glyph.width/height`
     equal the trimmed dimensions), and the atlas pixels are non-blank in that
     region.
   - `render_codepoint_offsets`: the bearings are computed from the clip margins
     and padding (`offset_x = clip_left − padding_x`,
     `offset_y = (height +| clip_bottom) − padding_y`) — assert against the
     values read back from a directly-rendered `Canvas` (same `draw_codepoint` +
     `write_atlas`).
   - `render_codepoint_blank`: a **covered-but-blank** glyph (`0x2800`, the
     blank Braille pattern — `draw_codepoint` returns `true` but draws no ink)
     returns `Ok(Some(_))` (not `Ok(None)`), and `write_atlas` tolerates the
     fully-trimmed blank canvas without panicking (the `Glyph` may have
     `width`/`height` `0`) — proving covered-but-empty glyphs are not mistaken
     for unsupported (per the design review).
   - `render_codepoint_none`: a non-sprite (`'M'`) returns `Ok(None)`; the next
     successful glyph's atlas placement is unchanged across an intervening
     `None` render (no region reserved).
   - (The exact numbers are confirmed against the render during implementation.)
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty sprite
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `render_codepoint` reproduces z2d's sprite `renderGlyph` — the padded canvas,
  the codepoint draw, the trimmed atlas write, and the `Glyph` bearings — and
  returns `Ok(None)` for un-handled codepoints;
- the box-line, offsets, and none tests confirm the pipeline;
- the wide-glyph factoring, the special-sprite glyphs, and the resolver wiring
  stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the `Glyph` metrics need the wide-glyph
factoring the single-cell path does not cover.

The experiment **fails** if the render pipeline or the `Glyph` bearings diverge
from z2d, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and raised one **Required**
finding: the tests must cover a **covered-but-blank** render (e.g. `0x2800`, the
blank Braille pattern), since `draw_codepoint(0x2800)` returns `true` but draws
no ink — `render_codepoint` must return `Ok(Some(_))` (not `Ok(None)`), and
`write_atlas` must tolerate the fully-trimmed blank canvas without panicking or
treating it as unsupported. Fixed: added `render_codepoint_blank`. One
**Optional** suggestion — for `render_codepoint_none`, prove no atlas region is
reserved by comparing the next successful glyph's placement before/after an
intervening unsupported render — folded into the `none` test. Codex confirmed
the rest is faithful for the single-cell path: the padding, the
post-`write_atlas` clip-margin use, `offset_x`, the saturating `offset_y`, the
atlas coordinates, and the early `Ok(None)` for non-sprites all match the
intended Rust API shape; and deferring the wide-glyph factoring is acceptable
since the signature is explicitly single-cell.

Review artifacts:

- Prompt: `logs/codex-review/20260603-093552-125296-prompt.md`
- Result: `logs/codex-review/20260603-093552-125296-last-message.md`

## Result

**Result:** Pass

`roastty/src/font/sprite/canvas.rs` gained `clip_left()`/`clip_bottom()`
accessors. `roastty/src/font/sprite/mod.rs` gained
`render_codepoint(cp, metrics, atlas) -> Result<Option<Glyph>, AtlasError>`: it
sizes a padded `Canvas` (`width = cell_width`, `height = cell_height`,
`padding = width/4 × height/4`), calls `draw_codepoint` (returning `Ok(None)` if
unmatched), writes the trimmed buffer to the atlas, and builds the `Glyph` from
the region and the trim margins (`offset_x = clip_left − padding_x`,
`offset_y = (region.height +| clip_bottom) − padding_y`).

Tests (the fixture `9×18` metrics, a fresh `Atlas::new(64, Grayscale)`):

- `render_codepoint_box_line` — `0x2500` returns `Ok(Some(glyph))` with a
  non-empty trimmed region and non-blank atlas pixels.
- `render_codepoint_offsets` — the bearings equal an independent direct
  `Canvas` + `write_atlas` render's `clip_left − padding_x` /
  `(region.height +| clip_bottom) − padding_y`.
- `render_codepoint_blank` — `0x2800` (covered-but-blank Braille) returns
  `Ok(Some(_))` (not `None`) with `width`/`height` `0`, no panic.
- `render_codepoint_none` — `'M'` returns `Ok(None)`, and the next glyph's atlas
  placement matches a fresh-atlas baseline (no region wasted on the `None`).

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2668 passed, 0 failed (+4, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The sprite render-to-atlas lands: `render_codepoint` is the codepoint path of
the sprite `Face.renderGlyph` — a codepoint becomes an atlas `Glyph` with
correct bearings. The sprite font is now complete end-to-end for codepoint-keyed
glyphs: `has_codepoint` (cover) → `render_codepoint` (draw + atlas → `Glyph`).

The remaining work is: the **wide-glyph `cell_width` factoring** (a `Canvas`
`width = cell_width × cell_count` for double-width sprites, threading the count
into the dispatch); the **sprite-kind special glyphs**
(underlines/strikethrough/ overline/cursors, keyed by a `Sprite` enum — a
parallel render entry point that takes the kind, the glyph width/height, and the
metrics); and the **resolver wiring** — `codepoint_resolver.render_glyph`
replacing its `SpriteUnavailable` arm with a call into `render_codepoint` (and a
`has_codepoint`-backed coverage check in the collection), threading the metrics
and the atlas. After the sprite font: the discovery consumer, the UCD
emoji-presentation default, codepoint overrides, the shaper, the Nerd Font
attribute table, and SVG color detection.

## Completion Review

Codex reviewed the completed implementation and result and found **no Required
changes** (and no Optional). It confirmed `render_codepoint` faithfully ports
the single-cell codepoint path of upstream `renderGlyph` (the padded canvas
sizing, the dispatch draw, the post-`write_atlas` trimmed region,
`offset_x = clip_left − padding_x`, and the saturating
`offset_y = region.height + clip_bottom − padding_y`); that the `0x2800` test
covers the important covered-but-blank case; that the `None` test verifies
unsupported codepoints do not consume atlas space; and that the offset test is
well-targeted (comparing against an independent direct canvas render with the
same trim-state semantics).

Review artifacts:

- Result review: `logs/codex-review/20260603-093858-841658-last-message.md`
