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

# Experiment 256: Glyph constraint math — the `constrain` geometry, fixture-exact

## Description

`render_glyph` (Experiment 255) renders the **unconstrained** glyph. Upstream
`renderGlyph` calls
`opts.constraint.constrain(glyph_size, metrics, constraint_width)` to remap a
glyph's size and bearings so it fits/aligns within its grid cell(s) — the
machinery behind Nerd Font icons, box-drawing, emoji centering, and symbol
fitting.

This experiment ports that machinery as a **standalone, pure-math module**
(`roastty/src/font/face/constraint.rs`) — no CoreText, no FFI, no `render_glyph`
wiring. It is a faithful port of `RenderOptions.Constraint` and its
`constrain`/`constrainInner`/`scale_factors`/`aligned_x`/`aligned_y` functions
from `vendor/ghostty/src/font/face.zig` (lines 98–496). Because it is pure
arithmetic over `f64`, it can be pinned to upstream's own hardcoded
CoreText-derived fixtures **value-for-value**, making this a true parity test
(unlike the CoreText rasterization, which matches by construction). Experiment
257 will wire the constraint into `render_glyph` (baseline term, `scaleCTM`
stretch, the re-centering `dx`, and the `RenderOptions`/`grid_metrics`
parameter).

### What is ported (faithful to `font/face.zig`)

- `GlyphSize { width, height, x, y: f64 }` — a glyph's size and bottom-left
  bearings (the value `constrain` maps).
- `Constraint` — the rule struct: `size: Size`, `align_vertical`/
  `align_horizontal: Align`, `pad_{top,left,right,bottom}: f64`,
  `relative_{width,height,x,y}: f64`, `max_xy_ratio: Option<f64>`,
  `max_constraint_width: u8` (upstream `u2`, default 2), `height: Height`.
  `Default` mirrors upstream's `.{}` (all `.none`/`0.0`/`1.0` as upstream).
- `Size` = `None | Fit | Cover | FitCover1 | Stretch`; `Align` =
  `None | Start | End | Center | Center1`; `Height` = `Cell | Icon`.
- `does_anything`, `constrain` (incl. the `.stretch` "fib the metrics to the
  grid" special-case and the non-negative padding clamp), `constrain_inner`
  (min-constraint-width rule, the scale group via `relative_*`,
  center-preserving scale, then alignment), `scale_factors` (`fit`/`cover`/
  `fit_cover1` — including its single-cell recursive call — `stretch`, and the
  `max_xy_ratio` reduction), `aligned_y`, and `aligned_x` (incl. `center1`).
- All reads are against the already-ported `crate::font::metrics::Metrics`,
  which already carries every field used: `cell_width`, `cell_height`,
  `face_width`, `face_height`, `face_y`, `icon_height`, `icon_height_single`.

`constraint_width` is the `u8` count of cells available (upstream `u2`).

### What is deferred

- Wiring `constrain` into `render_glyph` (Experiment 257): the baseline term
  added to `y` before constraining, the `scaleCTM(width/rect.w, height/rect.h)`
  stretch, the `dx` re-centering, and the `RenderOptions`/`grid_metrics`
  rasterizer parameter.
- The Nerd Font per-codepoint attribute table (`nerd_font_attributes.zig`,
  `getConstraint`) — a large lookup table. This experiment's fixtures that
  upstream sources from `getConstraint(...)` instead **construct the resulting
  `Constraint` literally** (with the exact field values upstream's own test
  asserts `getConstraint` returns), so the `fit_cover1`/`icon`/`center1` and
  `stretch`/`start`/`center1` math is still exercised. The table itself lands in
  a later experiment.

### Parity fixtures (ported from `font/face.zig` test "Constraints")

Upstream pins `constrain` against CoreText metrics at size 12 / DPI 96 with
JetBrains Mono grid metrics. The Rust test ports the same `Metrics` and the same
input → expected `GlyphSize` pairs, compared with upstream's tolerance —
`approxEqRel` with `sqrt(f64::EPSILON)` (≈ `1.49e-8`):
`(a - b).abs() <= a.abs().max(b.abs()) * sqrt_eps`, field by field.

Cases (each an exact value match):

1. **ASCII, `.none`**: `'x'` BBox is returned unchanged for `constraint_width` 1
   and 2.
2. **Symbol, `.fit`**: `'■'` (0x25A0) scales down + shifts to one cell at width
   1 (`{face_width, face_width, 0, 5.64}`); unchanged at width 2.
3. **Emoji, `.cover` + center + 0.025 L/R pad**: `'🥸'` (0x1F978) at width 2 →
   `{18.72, 18.72, 0.44, 1.4}`.
4. **`fit_cover1` + `icon` height + `center1`** (NF lightbulb 0xEA61, constraint
   constructed literally): width 1 →
   `{7.2125, 10.4125, 0.8125, 5.950695224719102}`; width 2 →
   `{9.015625, 13.015625, 1.015625, 4.7483690308988775}`.
5. **`stretch` + `cell` height + `start`/`center1`** (NF flame 0xE0C0,
   constraint constructed literally): width 1 →
   `{cell_width, cell_height, 0, 0}`; width 2 →
   `{2*cell_width, cell_height, 0, 0}`.

These cover every `Size` variant and the `none`/`start`/`center`/`center1`
alignments — a thorough pin on the ported arithmetic.

## Changes

1. `roastty/src/font/face/constraint.rs` (new): `GlyphSize`, `Constraint`,
   `Size`, `Align`, `Height`, and the `does_anything`/`constrain`/
   `constrain_inner`/`scale_factors`/`aligned_y`/`aligned_x` methods. Module-
   level `#![allow(dead_code)]` is unnecessary (the crate already allows it at
   `font` root); the module is consumed by Experiment 257.
2. `roastty/src/font/face/mod.rs`: add `pub(crate) mod constraint;`.
3. Tests in `constraint.rs`: a private `approx_eq`/`expect_approx_eq` helper
   matching upstream's `approxEqRel(sqrt_eps)`, the shared JetBrains-Mono
   `Metrics` fixture, and the five cases above.
4. Format and test (`cargo fmt`, accept output).

No CoreText/FFI, no `render_glyph` change, no C ABI/header/ABI-inventory change.

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty constraint
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `constraint.rs` faithfully ports the constraint types and the
  `constrain`/`constrain_inner`/`scale_factors`/`aligned_x`/`aligned_y` logic,
  including the `.stretch` metric-fib special-case, the `fit_cover1` recursive
  single-cell call, the `max_xy_ratio` reduction, and `center1`;
- all five ported fixtures match upstream value-for-value within
  `approxEqRel(sqrt(f64::EPSILON))`;
- `render_glyph` and the C ABI are untouched (pure-math module only);
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if a fixture cannot be made to match without a
faithfulness deviation (e.g. an upstream-specific rounding) that must be
documented.

The experiment **fails** if the ported arithmetic diverges from upstream on any
fixture, or if any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no required
changes**.

Review artifacts:

- Prompt: `logs/codex-review/20260602-204624-937735-prompt.md`
- Result: `logs/codex-review/20260602-204624-937735-last-message.md`

Codex confirmed the scoped pure-math port is faithful: every required `Metrics`
field exists, the `.stretch` metrics-fib and the non-negative padding clamp are
covered, `constrain_inner`/the `relative_*` scale group/the recursive
`fit_cover1`/`max_xy_ratio`/alignment (incl. `center1`) are all in scope, and
constructing the two Nerd-Font fixture `Constraint`s literally (deferring
`getConstraint`) is reasonable. It confirmed the `sqrt(f64::EPSILON)` relative
tolerance matches upstream's intent and noted the exact-zero `stretch`
expectations require the arithmetic to produce exact `0.0` — which the
`stretch`/`start`/`center1` path should.
