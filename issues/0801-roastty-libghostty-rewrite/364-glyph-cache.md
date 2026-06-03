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

# Experiment 364: the shared grid's glyph cache

## Description

Experiment 363 built `SharedGrid::render_glyph` but deferred the **glyph
cache**: every call re-rasterizes and reserves a fresh atlas region, even for a
glyph already rendered — so a screen full of repeated characters would fill the
atlas with duplicates. This experiment adds upstream's cache: a map keyed by the
glyph's identity that returns the already-rendered `Glyph` on a hit, so each
distinct glyph is rasterized and packed into the atlas exactly once.

## Upstream behavior

Upstream `SharedGrid` holds `glyphs: HashMap<GlyphKey, Render>`. `renderGlyph`
builds `GlyphKey { index, glyph, opts }`, returns the cached `Render` on a hit,
and otherwise renders, inserts, and returns (rolling the entry back on a render
error). The key's hash/eq use a **`Packed` u64** that includes only the `index`
(u16), `glyph` (u32), and the **integer** option fields — `cell_width`,
`thicken`, `thicken_strength`, `constraint_width`. It deliberately excludes the
float-bearing `grid_metrics` and `constraint`: `grid_metrics` is constant for a
grid, and the `constraint` is derived deterministically from the glyph's
presentation (itself a function of `index`/`glyph`), so they cannot vary
independently of the key's fields.

## Rust mapping (`roastty/src/font/shared_grid.rs`)

A hashable `GlyphKey` mirroring upstream's `Packed` (the float fields excluded,
so no float hashing is needed):

```rust
use std::collections::HashMap;

/// The glyph cache key. Mirrors upstream `GlyphKey.Packed`: the packed font
/// index, the glyph id, and the **integer** render options. The float-bearing
/// `grid_metrics`/`constraint` are excluded — `grid_metrics` is constant per
/// grid and `constraint` is derived from the glyph's presentation, so neither
/// varies independently of these fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphKey {
    index: u16, // `Index::int()`
    glyph: u32,
    cell_width: u8, // `opts.cell_width.unwrap_or(0)`, as upstream's `orelse 0`
    thicken: bool,
    thicken_strength: u8,
    constraint_width: u8,
}

impl GlyphKey {
    fn new(index: Index, glyph: u32, opts: &RenderOptions) -> GlyphKey {
        GlyphKey {
            index: index.int(),
            glyph,
            cell_width: opts.cell_width.unwrap_or(0),
            thicken: opts.thicken,
            thicken_strength: opts.thicken_strength,
            constraint_width: opts.constraint_width,
        }
    }
}
```

`SharedGrid` gains a `glyphs: HashMap<GlyphKey, Glyph>` field (initialized empty
in `new`), and `render_glyph` gets a cache check at the top and an insert on a
successful render:

```rust
pub(crate) fn render_glyph(
    &mut self,
    index: Index,
    glyph_index: u32,
    opts: &RenderOptions,
) -> Result<Glyph, ResolverRenderError> {
    let key = GlyphKey::new(index, glyph_index, opts);
    if let Some(&glyph) = self.glyphs.get(&key) {
        return Ok(glyph); // cache hit — no re-rasterization, no atlas reservation
    }

    let presentation = self.resolver.get_presentation(index, glyph_index as u16)?;
    let glyph = match presentation {
        Presentation::Emoji => { /* …emoji constraint, atlas_color… */ }
        Presentation::Text => { /* …atlas_grayscale… */ }
    }?; // a render error is propagated WITHOUT caching

    self.glyphs.insert(key, glyph);
    Ok(glyph)
}
```

## Scope / faithfulness notes

- **Ported (bridged)**: the glyph cache — `SharedGrid` holds a
  `HashMap<GlyphKey, Glyph>`, returns the cached glyph on a hit, and otherwise
  renders, inserts, and returns. This is upstream's `glyphs` map and the
  fast/slow path of `renderGlyph`.
- **Faithful**: the key mirrors upstream's `Packed` exactly — `index.int()`,
  `glyph`, and the integer options (`cell_width` with `unwrap_or(0)`, `thicken`,
  `thicken_strength`, `constraint_width`); the float-bearing `grid_metrics` and
  `constraint` are excluded for the same reason upstream excludes them (constant
  per grid / derived from presentation). A cache hit returns the identical
  `Glyph` and performs **no** atlas reservation.
- **Invariant (documented on `GlyphKey`)**: the cache is correct only for the
  grid/renderer path, where `grid_metrics` is the grid's constant and the
  `constraint` is derived from the glyph's presentation. It is **not** a general
  "same glyph, arbitrary constraint" renderer — a caller that renders the same
  `(index, glyph, integer-opts)` with a deliberately different `constraint`/
  `grid_metrics` would wrongly hit the cache. The grid never does this.
- **Faithful adaptation**: roastty renders **then** inserts on success (so a
  failed render is simply never cached), rather than upstream's insert-then-
  `errdefer`-remove. The end state is identical — a render error leaves no cache
  entry — without the rollback machinery. The map is single-threaded (no
  upstream read/write lock); roastty's `SharedGrid` is `&mut`-accessed, so the
  lock is unnecessary.
- **Deferred**: cache invalidation on metrics/font reload (the whole grid is
  rebuilt then, as upstream) and the Metal draw-path consumer. (Consumed by
  tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/font/shared_grid.rs`: add the `GlyphKey` struct (+`new`), the
   `glyphs: HashMap<GlyphKey, Glyph>` field (init empty in `new`), and the cache
   check/insert in `render_glyph`; import `std::collections::HashMap`.
2. Test (in `shared_grid.rs`): with a Menlo `SharedGrid`:
   - render `'M'` twice; assert the two returned `Glyph`s are equal and the
     cache holds exactly **one** entry (the second call was a hit — no second
     rasterization);
   - render a different glyph (`'N'`); assert the cache now holds **two**
     entries (distinct glyphs are distinct keys).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty shared_grid
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `SharedGrid::render_glyph` caches by the upstream-shaped `GlyphKey`, returning
  the cached `Glyph` on a hit (no re-rasterization) and inserting only on a
  successful render — faithful to upstream's `glyphs` map;
- the cache test passes (one entry for a repeated glyph, two for distinct), and
  the existing tests still pass;
- cache invalidation and the Metal consumer stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the key collides distinct glyphs or distinguishes
identical ones, a failed render is cached, a hit re-rasterizes, or any public C
API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed: excluding `grid_metrics` and `constraint` from the key
is faithful to upstream and sound for the `SharedGrid` contract (one grid has
one metrics set, and the effective constraint is deterministic for the glyph
path, with emoji overridden internally); `render-then-insert-on-success` is
equivalent to upstream's insert-plus-rollback end state (a failed render never
reaches the insert, so no stale entry is possible) and no lock is needed because
roastty has `&mut SharedGrid`; the key fields are complete and correctly typed
(using full `u8` values is fine since roastty's `RenderOptions` fields are wider
than upstream's packed `u2`, avoiding truncation); and the test is sufficient
for the cache happy path. Its one emphasis — that this key is for the
renderer/grid path, not a general "same glyph, arbitrary constraint" renderer —
is now recorded as a documented invariant on `GlyphKey` (and in the scope notes
above).

Review artifacts:

- Prompt: `logs/codex-review/20260603-174110-544223-prompt.md` (design)
- Result: `logs/codex-review/20260603-174110-544223-last-message.md` (design)

## Result

**Result:** Pass

The shared grid now rasterizes each distinct glyph exactly once.

- `roastty/src/font/shared_grid.rs`:
  - `GlyphKey { index: u16, glyph: u32, cell_width: u8, thicken: bool, thicken_strength: u8, constraint_width: u8 }`
    (derives `Hash`/`Eq`), built via `GlyphKey::new(index, glyph, opts)` from
    `index.int()` and the integer render options (`cell_width.unwrap_or(0)`, …)
    — mirroring upstream's `Packed`, with the float-bearing
    `grid_metrics`/`constraint` excluded. Its doc records the grid-path-only
    invariant.
  - `SharedGrid` gains a private `glyphs: HashMap<GlyphKey, Glyph>` (empty in
    `new`). `render_glyph` checks the cache first (returning the cached `Glyph`
    on a hit, with no re-rasterization or atlas reservation), renders on a miss,
    and inserts only on success — the `?` after the render match propagates a
    render error before the insert, so a failed render is never cached.
  - Module and struct docs updated to reflect that the grid now owns the cache
    (invalidation on reload remains deferred).

Test (in `shared_grid.rs`): `render_glyph_caches_by_key` renders `'M'` twice
(asserts the two glyphs are equal and `glyphs.len() == 1` — the second call was
a hit) then `'N'` (asserts `glyphs.len() == 2` — a distinct key).

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2812 passed, 0 failed (+1, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

`SharedGrid` is complete: it resolves, shapes (via the run pipeline),
rasterizes, and now caches glyphs — each distinct glyph hits the atlas once. The
`font/` subsystem is feature-complete for the render path; the renderer can
shape a viewport into `ShapedRun`s (Experiments 358–362) and `render_glyph` each
glyph index through the cache into an atlas region (Experiments 363–364).

The remaining work is outside `font/`'s core render path: cache invalidation on
metrics/font reload, and the **Metal draw path** that consumes the atlases + the
`ShapedRun`s to fill the GPU cell buffer.

## Completion Review

Codex reviewed the completed implementation and result and **approved**. It
found the cache implementation functionally sound: the key mirrors upstream's
packed shape with `index.int()` preserving the full `Index` representation (no
collision versus the full index), the hit path returns the copied cached
`Glyph`, and the `?` after the render match is correctly placed so render errors
return before the insert (failed renders are never cached). It confirmed the
test proves the main cache behavior (same key → same glyph, one entry; a
different glyph → a second entry). Its one **Low** finding — that the
module/struct doc comments still called the glyph cache a "later sub-area"
(stale after this implementation) — was fixed: both now state the grid owns the
cache, with only invalidation/reload deferred.

Review artifacts:

- Result review: `logs/codex-review/20260603-174343-614139-last-message.md`
