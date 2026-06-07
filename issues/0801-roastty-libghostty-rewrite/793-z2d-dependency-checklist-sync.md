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

# Experiment 793: z2d Dependency Checklist Sync

## Description

The Issue 801 dependency section still calls the `z2d` fidelity decision open
and says sprite path rendering is pending. The current Roastty tree has a
from-scratch Rust sprite rasterizer in `font/sprite/raster.rs`, wired into
`Canvas::line`, `Canvas::stroke_path`, `Canvas::fill_path`, and
`Canvas::inner_stroke_path`. This satisfies the `z2d` dependency for the sprite
path rasterizer named by the dependency checklist.

The CPU debug overlay remains open in the renderer checklist row for z2d debug
`Overlay`, link highlighting, render `Thread`, and custom shaders. This
experiment updates only the dependency decision/checklist wording for sprite
rasterization and does not claim that debug overlay or renderer integration is
complete.

## Changes

- `issues/0801-roastty-libghostty-rewrite/README.md`
  - Change the Zig-origin `z2d` decision from open to satisfied for sprite path
    rasterization via the Rust rasterizer.
  - Mark the dependency checklist row for `z2d` complete with scoped wording.
  - Leave the renderer debug overlay row unchanged.
  - Add the Experiment 793 index entry.
- `issues/0801-roastty-libghostty-rewrite/793-z2d-dependency-checklist-sync.md`
  - Record the verification evidence and review result.

## Verification

- Inspect current sprite rasterizer and Canvas path methods:
  - `roastty/src/font/sprite/raster.rs`
  - `roastty/src/font/sprite/canvas.rs`
  - `roastty/src/font/sprite/draw.rs`
- Run focused raster/path tests:
  - `cargo test -p roastty font::sprite::raster -- --nocapture --test-threads=1`
  - `cargo test -p roastty font::sprite::canvas -- --nocapture --test-threads=1`
  - `cargo test -p roastty font::sprite::draw -- --nocapture --test-threads=1`
- Run representative path-rendered glyph category checks:
  - `cargo test -p roastty powerline -- --nocapture --test-threads=1`
  - `cargo test -p roastty render_codepoint -- --nocapture --test-threads=1`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/793-z2d-dependency-checklist-sync.md`
- Run:
  - `git diff --check`

The experiment passes if the Rust sprite rasterizer exists, path-rendered sprite
tests pass, and the README marks `z2d` complete only for sprite path
rasterization while keeping renderer debug overlay work open. It is Partial if
the rasterizer exists but representative glyph paths do not verify. It fails if
the original "path rendering pending" wording remains accurate.

## Design Review

Codex reviewed the design and found no blocking findings. The review approved
the scoped sprite path rasterization claim, unchanged renderer debug overlay
row, open renderer glyph upload/live render loop/custom shader work, and
non-empty focused test filters.
