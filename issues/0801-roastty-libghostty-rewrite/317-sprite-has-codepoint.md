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

# Experiment 317: the sprite has_codepoint predicate

## Description

The unifying `draw_codepoint` (Experiment 316) renders a sprite codepoint; its
natural pair is `has_codepoint`, which answers whether a codepoint is a drawable
sprite **without** rendering it. Upstream's sprite `Face.hasCodepoint(cp, p)` is
exactly `getDrawFn(cp) != null` — a single classifier shared with the render
path, ignoring the presentation. This experiment adds `has_codepoint`, the
predicate the collection's sprite-coverage check needs.

## Background

Upstream keeps one source of truth — `getDrawFn(cp)` (a single switch returning
the draw function or `null`) — and `hasCodepoint` is `getDrawFn(cp) != null`.
roastty's draw is split across the 16 family `draw_*` dispatchers, and
`draw_codepoint` (Experiment 316) reconstitutes that single classifier: it
returns `true` iff some family matches. So the DRY, behaviorally-faithful
predicate is "does `draw_codepoint` match" — evaluated against a throwaway
canvas so nothing is committed to a real surface. A future optimization can add
a range-only fast path (a non-drawing classifier) if the coverage check proves
hot; for now `draw_codepoint` stays the single source of truth so the predicate
can never diverge from what actually renders.

## Rust mapping (`roastty/src/font/sprite/draw.rs`)

`pub(crate) fn has_codepoint(cp: u32, metrics: &Metrics) -> bool` — render `cp`
into a scratch cell-sized `Canvas` via `draw_codepoint` and return whether it
matched:

```rust
let mut scratch = Canvas::new(metrics.cell_width, metrics.cell_height, 0, 0);
draw_codepoint(cp, metrics, &mut scratch)
```

(The presentation is ignored, matching upstream — the sprite font always
provides its codepoints regardless of the requested presentation.) Update the
module doc, noting the future range-classifier optimization.

## Scope / faithfulness notes

- **Ported**: the `has_codepoint` predicate (behaviorally faithful to upstream's
  `getDrawFn(cp) != null`, via the single `draw_codepoint` source of truth).
- **Deferred**: a range-only non-drawing fast path (an optimization), the
  sprite-kind special glyphs, and the resolver/atlas wiring.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/sprite/draw.rs`: add `has_codepoint`; update the module
   doc.
2. Tests (deterministic — the fixture `9×18` cell):
   - `has_codepoint_covers`: every representative codepoint from the
     `draw_codepoint` dispatch test (one per family — box line, box dashes,
     diagonal, arc, braille, sextant, octant, separated quadrant, block, corner
     triangle, outlined triangle, and the five powerline families) returns
     `true`.
   - `has_codepoint_excludes`: non-sprite codepoints (`'M'`, `0x0041`, `0x20`,
     `0x2603` ☃) return `false`.
   - `has_codepoint_matches_draw`: for a spread of codepoints (covered and not),
     `has_codepoint(cp)` equals whether `draw_codepoint(cp, …)` matched on a
     fresh canvas — pinning the predicate to the render path (no divergence).
3. Format and test (`cargo fmt`, accept output).

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

- `has_codepoint` returns `true` for every drawable sprite codepoint and `false`
  otherwise, ignoring presentation, never diverging from `draw_codepoint`;
- the covers, excludes, and matches-draw tests confirm the predicate;
- the range-only fast path, the special-sprite glyphs, and the resolver/atlas
  wiring stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if `has_codepoint` needs to consider presentation
(it should not — upstream ignores it).

The experiment **fails** if `has_codepoint` disagrees with `draw_codepoint`, or
any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no Required**
changes (and no Optional). It confirmed that rendering into a scratch `Canvas`
is behaviorally faithful for the ported codepoint-keyed sprite set —
`has_codepoint` returns the same boolean as `draw_codepoint`, including
blank-but-covered glyphs (e.g. Braille patterns), and ignores presentation as
upstream's `getDrawFn(cp) != null` does; that the DRY tradeoff (one scratch
render vs a duplicable range classifier that could diverge) is acceptable for
this slice with the resolver/atlas wiring and the range fast-path deferred; that
a cell-sized scratch canvas is safe for the existing families under normal
metrics (out-of-bounds geometry is clipped, and the scratch buffer has no
external side effects); and that the tests cover the right invariants (family
coverage, exclusions including a non-sprite symbol, and direct agreement with
`draw_codepoint`).

Review artifacts:

- Prompt: `logs/codex-review/20260603-092943-003682-prompt.md`
- Result: `logs/codex-review/20260603-092943-003682-last-message.md`
