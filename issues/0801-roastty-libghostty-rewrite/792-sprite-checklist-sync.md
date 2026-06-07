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

# Experiment 792: Sprite Checklist Sync

## Description

The Issue 801 font/text checklist still describes sprite support as partial or
missing: `Canvas` says z2d path rendering is missing, and `draw/` glyph tables
say box/block/braille/powerline/geometric/legacy tables are missing. The current
Roastty tree has moved past that wording. `font/sprite/canvas.rs` provides the
sprite canvas, `font/sprite/raster.rs` provides path rasterization, and
`font/sprite/draw.rs` contains the procedural glyph inventory for box drawing,
blocks, Braille, Powerline, geometric/corner shapes, sextants, octants, cursor
glyphs, and text-decoration strokes.

This experiment verifies the existing sprite modules and updates the checklist
to mark the sprite canvas/draw rows complete for the current procedural sprite
implementation. It does not claim that renderer glyph upload, the live render
loop, z2d debug overlay, or custom shader integration is complete; those remain
tracked by renderer rows.

## Changes

- `issues/0801-roastty-libghostty-rewrite/README.md`
  - Mark the Sprite `Canvas` row complete for canvas plus raster path support.
  - Mark the Sprite `draw/` glyph table row complete for the current procedural
    inventory.
  - Keep renderer integration rows unchanged.
  - Add the Experiment 792 index entry.
- `issues/0801-roastty-libghostty-rewrite/792-sprite-checklist-sync.md`
  - Record the verification evidence and review result.

## Verification

- Inspect current sprite modules:
  - `roastty/src/font/sprite/canvas.rs`
  - `roastty/src/font/sprite/raster.rs`
  - `roastty/src/font/sprite/draw.rs`
  - `roastty/src/font/sprite/mod.rs`
- Run focused sprite tests:
  - `cargo test -p roastty font::sprite::canvas -- --nocapture --test-threads=1`
  - `cargo test -p roastty font::sprite::raster -- --nocapture --test-threads=1`
  - `cargo test -p roastty font::sprite::draw -- --nocapture --test-threads=1`
- Run representative category checks:
  - `cargo test -p roastty powerline -- --nocapture --test-threads=1`
  - `cargo test -p roastty braille -- --nocapture --test-threads=1`
  - `cargo test -p roastty octant -- --nocapture --test-threads=1`
  - `cargo test -p roastty render_codepoint -- --nocapture --test-threads=1`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/792-sprite-checklist-sync.md`
- Run:
  - `git diff --check`

The experiment passes if the sprite canvas, path rasterizer, and procedural draw
inventory exist, focused tests pass, and the README rows are checked with
wording scoped to sprite generation rather than renderer integration. It is
Partial if only canvas/raster or only draw inventory verifies. It fails if the
original partial/missing wording remains accurate.

## Design Review

Codex reviewed the design and found no blocking findings. The review approved
the sprite-generation scope, unchanged renderer rows, explicit open work for
renderer glyph upload/integration, live render loop, z2d debug overlay, and
custom shaders, and the relevant focused test filters.
