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

# Experiment 251: CoreText `Face` scalar metric accessors

## Description

Continue the CoreText `Face` toward `get_metrics` (Exp 250 proved the
`CTFont`/`CTFontCopyTable` path) by adding the **scalar metric accessors** that
`getMetrics` reads directly from the `CTFont`. These are the per-face values the
metric-assembly fallback chains depend on when a table is missing or as the
font's own size/units:

- `size` — the point size (`px_per_em`).
- `units_per_em` — the head-table fallback when `head` can't be read.
- `ascent` / `descent` / `leading` — the vertical-metrics fallback when the
  `hhea` table is absent (CoreText returns these already in pixels).
- `cap_height` / `x_height` — the cap/ex-height fallback when `OS/2` lacks
  `sCapHeight` / `sxHeight`.

This is a small FFI slice — seven thin `unsafe` wrappers over verified
`objc2-core-text` `CTFont` methods — with a live Menlo test. The
glyph-measurement accessors (`get_glyphs_for_characters` / advances / bounding
rects, for `cell_width`/`ascii_height`/`ic_width`) are the next slice; the full
`get_metrics` assembly follows that.

### The objc2 API (verified, `objc2-core-text` 0.3.2)

All are `unsafe` methods on `CTFont` returning `CGFloat` (`= f64`), except
`units_per_em` which returns `c_uint`:

- `size(&self) -> CGFloat`
- `ascent(&self) -> CGFloat`
- `descent(&self) -> CGFloat` (CoreText returns descent as a **positive**
  magnitude — the metric assembly negates it later)
- `leading(&self) -> CGFloat`
- `units_per_em(&self) -> c_uint`
- `cap_height(&self) -> CGFloat`
- `x_height(&self) -> CGFloat`

### Rust mapping (`roastty/src/font/face/coretext.rs`)

Add to `impl Face` (each wrapping the matching `unsafe` `CTFont` call in an
`unsafe` block with a one-line `SAFETY` note — the receiver is a live `CTFont`):

- `pub(crate) fn size(&self) -> f64`
- `pub(crate) fn units_per_em(&self) -> u32` (`as u32` from `c_uint`)
- `pub(crate) fn ascent(&self) -> f64`
- `pub(crate) fn descent(&self) -> f64` (returns CoreText's raw positive value;
  the sign is the assembly's concern, kept faithful here)
- `pub(crate) fn leading(&self) -> f64`
- `pub(crate) fn cap_height(&self) -> f64`
- `pub(crate) fn x_height(&self) -> f64`

### Faithfulness and scope notes

- The accessors are thin and return exactly what CoreText returns (e.g.
  `descent` stays positive, matching upstream's `self.font.getDescent()` before
  its later `-`). No metric assembly or sign-flipping here.
- `units_per_em` is `c_uint` upstream and is cast to `u32` (the OpenType
  `unitsPerEm` is a `u16`, so `u32` is ample).
- These are the table-absent fallbacks `get_metrics` will call; the
  glyph-measurement accessors and the assembly are separate slices.
- No C ABI, header, or ABI inventory changes; no new dependencies.

## Changes

1. `roastty/src/font/face/coretext.rs`: add the seven scalar accessors to
   `impl Face`.
2. Tests in `coretext.rs` (live CoreText, macOS):
   - `scalar_metrics_are_plausible`: `Face::new("Menlo", 12.0)` →
     `size() == 12.0`; `(16..=16384).contains(&units_per_em())`;
     `ascent() > 0.0`; `descent() > 0.0` (CoreText's positive convention);
     `leading() >= 0.0`; `cap_height() > 0.0`; `x_height() > 0.0`; and
     `cap_height() > x_height()` (capitals are taller than the x-height). Values
     are asserted as ranges/ relations, not font-version-pinned numbers (except
     the size we set).

3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo test -p roastty face
cargo test -p roastty
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- the seven accessors wrap the matching `CTFont` methods and return plausible
  values for a live font (`size` exact, the rest in valid ranges,
  `cap_height > x_height`);
- `descent` is returned as CoreText's raw positive value (faithful);
- the glyph-measurement accessors and the assembly are cleanly deferred;
- no C ABI, header, or ABI inventory changes;
- `cargo fmt` accepted and `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if an accessor's objc2 signature differs from the
verified one and needs adjusting.

The experiment **fails** if an accessor returns an implausible value, if
`descent`'s sign is altered here, or if any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no required
changes**.

Review artifacts:

- Prompt: `logs/codex-review/20260602-200821-242523-prompt.md`
- Result: `logs/codex-review/20260602-200821-242523-last-message.md`

Codex confirmed the seven accessors are exactly the scalar CoreText fallbacks
upstream `getMetrics` uses (`size` → `px_per_em`; `units_per_em` for the
`head`-absent case; `ascent`/`descent`/`leading` for the `hhea`-absent case;
`cap_height`/`x_height` for the OS/2 cap/ex fallback), that returning `descent`
as CoreText's raw positive value is correct (upstream negates it later with
`-self.font.getDescent()`, so the flip belongs in the assembly slice), that the
`c_uint → u32` cast is fine, and that the range/relation test assertions are
appropriately robust for a live font. Deferring glyph measurement and the full
assembly keeps the slice clean.

## Result

**Result:** Pass

Added the seven scalar accessors to `impl Face` in
`roastty/src/font/face/coretext.rs` — `size`, `units_per_em` (`c_uint → u32`),
`ascent`, `descent` (CoreText's raw positive value), `leading`, `cap_height`,
`x_height` — each a thin `unsafe` wrapper over the matching `CTFont` method with
a `SAFETY` note.

Test added (1): `scalar_metrics_are_plausible` — `Face::new("Menlo", 12.0)` →
`size() == 12.0`, `units_per_em()` in `16..=16384`, `ascent()`/`descent()`/
`cap_height()`/`x_height()` all `> 0.0`, `leading() >= 0.0`, and
`cap_height() > x_height()`.

### Verification

```bash
cargo fmt -p roastty
cargo test -p roastty face
cargo test -p roastty
```

Observed:

- `face`: 3 passed (the table spike + missing-table + the new scalar metrics).
- Full `roastty`: 2358 unit tests passed (2357 prior + 1 new), plus the C ABI
  harness passed.
- `cargo fmt -p roastty -- --check`: clean.
- `cargo build -p roastty`: no warnings.
- No-`ghostty`-name gates passed for `roastty/src/font` (and
  `lib.rs`/header/abi).
- `git diff --check`: clean.

No C ABI, header, or ABI inventory changes; the glyph-measurement accessors and
the full `get_metrics` assembly cleanly deferred.

### Completion Review

Codex reviewed the completed implementation and found **no issues** ("nothing
needs to change before the result commit").

Review artifacts:

- Prompt: `logs/codex-review/20260602-201051-081525-prompt.md`
- Result: `logs/codex-review/20260602-201051-081525-last-message.md`

Codex confirmed each accessor is a thin wrapper over the matching `CTFont`
method, that `units_per_em` casts to `u32` and `descent` preserves CoreText's
raw positive magnitude (no premature sign flip), that the `unsafe` blocks are
scoped to the FFI calls with accurate `SAFETY` notes, and that the test checks
stable ranges/relations rather than version-specific values. Scope is clean.

## Conclusion

Experiment 251 succeeds. The `Face` now exposes the seven scalar metrics
`get_metrics` reads from the `CTFont` as fallbacks. Both Codex gates passed with
zero findings.

The next slice adds the **glyph-measurement accessors**:
`get_glyphs_for_characters` (map a `&[u16]` of codepoints to `CGGlyph`s),
`advances_for_glyphs` (horizontal advances → `cell_width` as the max printable-
ASCII advance), and `bounding_rects_for_glyphs` (the overall ASCII bounding box
→ `ascii_height`, and the `水`/`H` glyph bounds for `ic_width`). These add
`objc2-core-graphics` (`CGGlyph`/`CGSize`/`CGRect`). The full `get_metrics`
assembly — copy the four tables (with the `bhed` fallback), run the
ascent/descent/underline/cap-height fallback chains, measure the glyph metrics,
build `FaceMetrics`, and feed `Metrics::calc` — is the slice after that.
