# Experiment 148: Phase I — sprite legacy-computing coverage

## Description

Complete the remaining sprite-font coverage called out in Phase I for Symbols
for Legacy Computing and the initial branch-drawing glyphs.

Roastty's Rust sprite port already covers box drawing, dashes, rounded corners,
diagonals, blocks, Braille, sextants (`U+1FB00`-`U+1FB3B`), separated quadrants,
octants, powerline glyphs, geometric corner triangles, and special
cursor/underline sprites. The remaining checklist item names two missing
families:

- Symbols for Legacy Computing after sextants: `U+1FB3C`-`U+1FBEF`
- Branch drawing glyphs: `U+F5D0`-`U+F5E3`

This experiment ports those upstream draw dispatches into
`roastty/src/font/sprite/draw.rs`, reusing the existing Rust `Canvas` path,
line, invert, block, arc, and shade primitives where possible.

Upstream coverage is defined by `Face.zig`'s compile-time collection of draw
function names, not by treating the Unicode span as dense. This means the
Symbols for Legacy Computing slice must include the exact upstream functions
inside the `U+1FB3C`-`U+1FBEF` envelope and must keep upstream gaps excluded
(for example, the gaps between `draw1FBAF` and `draw1FBBD`, and between
`draw1FBBF` and `draw1FBCE`).

Out of scope:

- the branch glyphs after `U+F5E3` (`U+F5E4`-`U+F60D`), because the roadmap item
  explicitly calls out only `U+F5D0`-`U+F5E3`;
- Symbols for Legacy Computing segmented digits (`U+1FBF0`-`U+1FBF9`), because
  the roadmap item ends at `U+1FBEF`;
- renderer/UI screenshot coverage beyond sprite unit tests and shared-grid glyph
  rendering, because this slice is the procedural sprite renderer.

## Changes

- `roastty/src/font/sprite/draw.rs`
  - Port upstream
    `vendor/ghostty/src/font/sprite/draw/symbols_for_legacy_computing.zig`
    dispatches for:
    - smooth mosaics `draw1FB3C_1FB67`;
    - edge triangles and inverted edge triangles `draw1FB68_1FB6F`;
    - vertical eighth blocks `draw1FB70_1FB75`;
    - horizontal eighth blocks `draw1FB76_1FB7B`;
    - combined eighth/shade/checkerboard/diagonal-fill glyphs `draw1FB7C_1FB97`,
      `draw1FB98`, `draw1FB99`, `draw1FB9A_1FB9F`;
    - corner diagonal line sets `draw1FBA0_1FBAE` and `draw1FBAF`;
    - inverted diagonal glyphs `draw1FBBD`, `draw1FBBE`, `draw1FBBF`;
    - thirds `draw1FBCE`, `draw1FBCF`;
    - cell diagonals `draw1FBD0_1FBDF`;
    - circle/half-circle/quadrant pieces `draw1FBE0_1FBEF`.
  - Port upstream `vendor/ghostty/src/font/sprite/draw/branch.zig` dispatches
    for `U+F5D0`-`U+F5E3`.
  - Reuse existing primitives (`fill`, `block`, `hline_middle`, `vline_middle`,
    `draw_box_arc`, `line`, `fill_path`, `stroke_path`, `invert`) rather than
    duplicating equivalent geometry.
  - Add any narrowly needed helpers that upstream depends on but Roastty has not
    yet exposed in this module, such as smooth-mosaic pattern decoding,
    edge-triangle paths, fading lines, branch nodes, checkerboard fill, corner
    diagonal lines, cell diagonals, and circle pieces.
  - Update `draw_codepoint` so the new families participate in the same dispatch
    chain as the already-ported sprite families.
  - Update `has_codepoint` coverage representatives and boundary tests so the
    predicate cannot diverge from the new dispatch.
  - Add explicit gap tests for upstream-uncovered scalars inside the
    `U+1FB3C`-`U+1FBEF` envelope, including `U+1FBB0`, `U+1FBBC`, `U+1FBC0`, and
    `U+1FBCD`.
- `roastty/src/font/sprite/canvas.rs`
  - Only if required, add a small crate-private clip-reset/set helper so
    inverted glyphs can preserve upstream's post-invert clipping behavior
    without exposing private fields broadly.
- `roastty/src/font/codepoint_resolver.rs` and/or
  `roastty/src/font/shared_grid.rs`
  - Add focused tests proving representative new codepoints resolve to the
    sprite face and render non-empty glyphs through the normal sprite atlas
    path.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - After the result, mark the Phase I sprite legacy-computing/branch checklist
    item complete only if the exact upstream draw-function inventory in the
    named ranges is covered by dispatch and representative rendering tests, and
    upstream gaps are proven excluded.

## Verification

- Format markdown:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/148-sprite-legacy-computing-coverage.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Format Rust:
  - `cargo fmt`
- Run focused sprite tests:
  - `cargo test -p roastty font::sprite::draw -- --test-threads=1`
  - `cargo test -p roastty sprite -- --test-threads=1`
  - `cargo test -p roastty render_glyph_sprite -- --test-threads=1`
- Run ABI harness:
  - `cargo test -p roastty --test abi_harness`
- Run full Roastty Rust coverage:
  - `cargo test -p roastty -- --test-threads=1`
- Run hosted app coverage:
  - `cd roastty && macos/build.nu --action test`
- Run hygiene checks:
  - `cargo fmt --check`
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/148-sprite-legacy-computing-coverage.md issues/0802-libroastty-completion-and-mac-app/README.md`

**Pass** = every codepoint covered by the upstream draw functions listed above
and every branch codepoint in `U+F5D0`-`U+F5E3` returns `true` from
`draw_codepoint`/`has_codepoint`, upstream-uncovered gaps inside the
`U+1FB3C`-`U+1FBEF` envelope and out-of-range boundary codepoints remain
excluded, representative glyphs from each newly ported subfamily draw the
expected non-empty or intentionally empty shape, normal sprite atlas rendering
works for representative new codepoints, and focused, full, hosted, formatting,
and hygiene checks pass.

**Partial** = the named ranges are recognized and mostly render, but one or more
subfamilies need geometry follow-up or hosted/full verification cannot be
completed.

**Fail** = the Rust sprite primitives are missing a required rendering
capability that prevents a faithful port without a broader canvas/rasterization
experiment.

## Design Review

**Reviewer:** Codex-native adversarial review subagents `Laplace`, `Pasteur`,
and `Anscombe`, fresh context.

**Initial verdict:** Changes required.

**Findings and fixes:**

- **Required:** the initial pass criteria treated every scalar in
  `U+1FB3C`-`U+1FBEF` as covered, but upstream coverage is based on collected
  draw-function names and has intentional gaps inside that envelope. Fixed by
  requiring exact upstream draw-function inventory coverage and explicit gap
  tests for upstream-uncovered scalars such as `U+1FBB0`, `U+1FBBC`, `U+1FBC0`,
  and `U+1FBCD`.
- **Required:** the first fix still collapsed upstream's vertical and horizontal
  eighth block functions into `draw1FB70_1FB7B`. Fixed by listing the exact
  upstream functions `draw1FB70_1FB75` and `draw1FB76_1FB7B`.

**Final verdict:** Approved.

## Result

**Result:** Pass

Ported the remaining Phase I sprite coverage for the exact upstream
Symbols-for-Legacy-Computing draw-function inventory inside the
`U+1FB3C`-`U+1FBEF` envelope and for branch glyphs `U+F5D0`-`U+F5E3`.

`roastty/src/font/sprite/draw.rs` now dispatches smooth mosaics, edge triangles,
vertical and horizontal eighth blocks, combined eighth/shade/checkerboard
glyphs, diagonal fills, corner diagonal line sets, inverted diagonal glyphs,
thirds, cell diagonals, circle pieces, and the initial branch drawing subset.
The implementation reuses the existing sprite raster/path pipeline, the box-arc
geometry, block/shade primitives, and adds narrow helpers for smooth-mosaic
pattern decoding, edge triangles, checkerboards, cell clipping, medium-alpha
path fills, fading branch lines, and circle pieces.

The tests prove the important inventory rule from the design review:
`U+1FB3C`-`U+1FBEF` is not dense. Every upstream-covered draw-function range in
that envelope is recognized, while upstream gaps such as `U+1FBB0`, `U+1FBBC`,
`U+1FBC0`, and `U+1FBCD` remain excluded. Representative pixel tests cover the
new path, eighth-block, intentional-empty, medium-alpha, and clipped-circle
paths, and resolver tests prove representative legacy/branch glyphs render
through the normal sprite atlas path.

Verification completed:

- `cargo fmt`
- `cargo test -p roastty font::sprite::draw -- --test-threads=1` — 142 passed
- `cargo test -p roastty sprite -- --test-threads=1` — 269 passed
- `cargo test -p roastty render_glyph_sprite -- --test-threads=1` — 5 passed
- `cargo test -p roastty --test abi_harness` — 1 passed, with existing C enum
  conversion warnings
- `cargo test -p roastty -- --test-threads=1` — 4822 unit tests plus ABI harness
  and doc tests passed, with existing C enum conversion warnings
- `cd roastty && macos/build.nu --action test` — 211 hosted macOS tests passed
  (`TEST SUCCEEDED`), with existing SwiftLint, Swift concurrency/sendability,
  main-thread-checker, App Intents, missing testing config, and pasteboard
  warnings/noise
- `cargo fmt --check`
- `git diff --check`
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/148-sprite-legacy-computing-coverage.md issues/0802-libroastty-completion-and-mac-app/README.md`

## Conclusion

Phase I sprite legacy-computing coverage is complete. Roastty now covers the
upstream sprite draw-function set for the named legacy-computing and branch
ranges, preserves upstream's intentional holes, and proves the new glyphs
through both direct sprite tests and the normal sprite atlas rendering path.

## Completion Review

**Reviewer:** Codex-native adversarial review subagent `Boole`, fresh context.

**Verdict:** Approved.

**Findings:** None.

**Independent verification:** `cargo fmt --check`, `git diff --check`, focused
sprite draw tests, focused sprite tests, focused sprite atlas tests, and
markdown prettier check all passed.
