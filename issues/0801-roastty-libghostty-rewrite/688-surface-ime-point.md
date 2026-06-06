+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5"
reasoning = "medium"
+++

# Experiment 688: Surface IME Point

## Description

Experiment 687 added the surface key-translation modifier query. Another small
upstream surface entry point Roastty lacks is
`ghostty_surface_ime_point(surface, x, y, width, height)`, which tells a
frontend where to place the platform IME composition window.

Upstream computes the point from the renderer state's active terminal cursor,
surface cell geometry, content scale, and current preedit width. Roastty already
has surface pixel/cell size storage, content-scale setters, terminal cursor
position through the worker, and surface preedit state. Roastty does not yet
have the full renderer state, renderer padding, visible-scroll handling, or a
width-aware surface preedit representation. This experiment adds the C ABI entry
point and a conservative geometry calculation using the data Roastty currently
owns.

The result is intentionally not full upstream parity yet. It does not implement
renderer padding, scrolling visibility adjustments, renderer-owned preedit
ranges, or full East Asian width handling for preedit strings. It also does not
add mouse dispatch, key dispatch, splits, inspector, or quicklook APIs.

## Changes

- `roastty/include/roastty.h`
  - Add
    `ROASTTY_API void roastty_surface_ime_point(roastty_surface_t, double*, double*, double*, double*);`
    next to `roastty_surface_preedit`.
- `roastty/src/lib.rs`
  - Add a small `Surface::ime_point()` helper returning `(x, y, width, height)`.
  - Use the current terminal cursor from the attached worker when available;
    otherwise use `(0, 0)`.
  - Sanitize content scale before division: `0`, negative, NaN, and infinite
    scale values are treated as `1.0`.
  - Compute `x` like upstream's unpadded midpoint:
    `(cursor_x * cell_width_px + cell_width_px / 2) / scale_factor_x`.
  - Compute `y` like upstream's unpadded cell bottom:
    `(cursor_y * cell_height_px + cell_height_px) / scale_factor_y`.
  - Compute `height` as `cell_height_px / scale_factor_y`.
  - Compute `width` from the current preedit text's scalar count times
    `cell_width_px`, clamped to the remaining pixel width after
    `(cursor_x + 1) * cell_width_px`, matching upstream's no-wrap behavior for
    the currently available data. The remaining-width calculation is saturating:
    if the cursor-derived offset is at or beyond `width_px`, the returned width
    is `0`.
  - Treat missing or zero cell geometry as a zero rectangle.
  - Make null surfaces and null output pointers safe no-ops; non-null output
    pointers receive zero for null surfaces. Each output pointer is handled
    independently so callers may request only some fields.
- `roastty/tests/abi_harness.c`
  - Exercise null and live `roastty_surface_ime_point` calls through
    `roastty.h`.
- Tests
  - Null surface writes zero to non-null outputs and tolerates null output
    pointers.
  - Missing cell geometry returns a zero rectangle.
  - Invalid content scale values fall back to `1.0`.
  - Live surface geometry uses cursor, cell size, content scale, and preedit
    width.
  - Width clamps at the right edge and saturates to zero when the cursor is at
    or beyond the right edge.
  - Detached surfaces still use stored surface geometry and preedit state
    without querying a worker.
  - Partial null output pointers on a live surface leave the null fields alone
    while writing the non-null fields.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/688-surface-ime-point.md`
- `cargo fmt -p roastty`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

**Result:** Approved after design fixes.

Codex initially blocked the design on three underspecified edge cases: invalid
content-scale values, remaining-width clamping when the cursor offset reaches or
exceeds the surface width, and verification for mixed null/non-null output
pointers. The design now treats invalid scale values as `1.0`, makes the
remaining-width calculation saturating, and requires partial-null pointer
coverage.

Codex approved the updated design as appropriately incremental. It also
confirmed the explicit exclusions are acceptable for this slice: renderer
padding, visible-scroll handling, renderer-owned preedit ranges, and full
width-aware preedit parity remain future work.
