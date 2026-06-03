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

# Experiment 338: the CoreText shaping core

## Description

With the shaper's output `shape::Cell` in place (Experiment 337), this
experiment ports the **heart of CoreText shaping**: turning a run of Unicode
codepoints into positioned glyphs via
`CFAttributedString → CTLine → CTRun → Cell`. This is the core of upstream
`Shaper.shape` — the glyph-and-cluster extraction — exposed as a `Face` method
(it needs the face's `CTFont`). The full `Shaper` orchestration (the run state,
the special-font path, advance-based positioning, RTL sorting, and the
`RunIterator` over terminal cells) builds on this.

## Upstream behavior (`shaper/coretext.zig` `Shaper.shape`)

```zig
// Build a CFAttributedString over the run's UTF-16 string with the font
// attribute, then a CTLine, then iterate its CTRuns extracting glyphs +
// string indices (clusters):
const attr_str = AttributedString.create(str, attr_dict);   // attr_dict: { font }
const line = Typesetter…createLine(...);                    // CTLine
for (line.getGlyphRuns()) |ctrun| {
    const glyphs  = ctrun.getGlyphsPtr() orelse ctrun.getGlyphs(...);
    const indices = ctrun.getStringIndicesPtr() orelse ctrun.getStringIndices(...);
    for (0..glyphs.len) |i| {
        // …advance-based x positioning + cluster→cell mapping…
        cell_buf.append(.{ .x = …cluster…, .glyph_index = glyphs[i] });
    }
}
```

`CTLine` shapes the attributed string (applying the font's `cmap`, ligatures,
and positioning). Each `CTRun` exposes its **glyphs** (`CGGlyph`/`u16`) and the
**string indices** mapping each glyph back to its source UTF-16 offset (the
cluster). Upstream computes a precise `x` from accumulated advances and a
cluster→cell map; this slice uses the **string index** as the `x`/cluster
(faithful for a simple, un-padded run) and defers the advance math.

## Rust mapping (`roastty/src/font/face/coretext.rs`)

- `roastty/Cargo.toml`: enable `CTLine`, `CTRun`, `CTStringAttributes`
  (`objc2-core-text`) and `CFAttributedString` (`objc2-core-foundation`).
- `pub(crate) fn shape_codepoints(&self, codepoints: &[u32]) -> Vec<shape::Cell>`:
  - Build a `String` from the scalars (`char::from_u32`, skipping invalid) and a
    `CFString`. The CoreText string indices are UTF-16 offsets into it.
  - Build a `CFMutableDictionary` with `kCTFontAttributeName → self.font`.
  - `CFAttributedString::new(None, &cfstring, &attrs)`.
  - `CTLine::with_attributed_string(&attr_str)`.
  - `line.glyph_runs()` → `CFArray`, cast to `CFArray<CTRun>`.
  - For each `CTRun`: `n = run.glyph_count()`; read the `n` glyphs (via
    `glyphs_ptr`, or `glyphs(CFRange, buf)` into a `Vec<CGGlyph>` if the pointer
    is null) and the `n` string indices (`string_indices_ptr`/`string_indices`).
    Emit
    `shape::Cell { x: index as u16, x_offset: 0, y_offset: 0, glyph_index: glyph as u32 }`
    for each.
  - The `unsafe` CoreText calls carry `// SAFETY:` notes.

## Scope / faithfulness notes

- **Ported**: the CoreText shaping core — `CFAttributedString` over the run's
  string with the font attribute, `CTLine`, and the per-`CTRun` glyph + string-
  index (cluster) extraction into `shape::Cell`s. This is what actually applies
  the font's shaping (cmap + ligatures) to a codepoint run.
- **Faithful simplifications (deferred)**: the precise **advance-based `x`** and
  the cluster→cell mapping (this slice uses the UTF-16 string index as `x`); the
  **special-font** fast path (codepoint == glyph); the **RTL/non-monotonic** run
  sorting; the **`x_offset`/`y_offset`** glyph positions; and the `Shaper`
  struct with its run state, caching, and the `RunIterator` over terminal cells.
  Those are subsequent experiments. The variation-axis score and variations also
  stay deferred.
- No C ABI/header/ABI-inventory change (`Face`/`shape::Cell` are internal Rust;
  only objc2 binding features are enabled).

## Changes

1. `roastty/Cargo.toml`: enable `CTLine`/`CTRun`/`CTStringAttributes` and
   `CFAttributedString`.
2. `roastty/src/font/face/coretext.rs`: add `Face::shape_codepoints`.
3. Tests (in `coretext.rs`):
   - `shape_ascii_monospace`:
     `Face::new("Menlo", 24.0).shape_codepoints(&['A' as u32, 'B' as u32, 'C' as u32])`
     returns 3 cells; each `cell.glyph_index` equals `face.glyph_index('A'+i)`
     (Menlo is monospace, 1:1, no ligatures), and the `x`s are `0, 1, 2` (the
     string indices).
   - `shape_single`: a single codepoint shapes to one cell whose `glyph_index`
     matches `face.glyph_index`.
   - `shape_empty`: an empty codepoint slice shapes to an empty `Vec`.
   - (Glyph ids are compared to `face.glyph_index`, not hard-coded, since
     CoreText glyph ids are font-specific.)
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty shape
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `shape_codepoints` reproduces the CoreText shaping core (the attributed string
  with the font attribute, the `CTLine`, and the per-`CTRun` glyph/string-index
  extraction into `shape::Cell`s);
- the ascii-monospace, single, and empty tests pass;
- the advance positioning, the special-font path, RTL sorting, and the `Shaper`/
  `RunIterator` stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if a non-monospace host font makes a glyph-id
assertion non-deterministic (the shaping call is still exercised; the assertions
compare against `face.glyph_index`).

The experiment **fails** if the attributed-string/`CTLine`/`CTRun` extraction
diverges from upstream, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
findings**. It verified: using the CoreText string index as `Cell.x` is a
documented, acceptable simplification for this slice (it proves glyph + cluster
extraction without taking on advance-based positioning yet); `String → CFString`
preserves the UTF-16 storage CoreText indexes into (the returned string indices
are UTF-16 offsets, so this is correct); `CFAttributedString` with
`kCTFontAttributeName → self.font` is the right mechanism to bind shaping to the
face's `CTFont`; `CTLine::with_attributed_string` is acceptable for the core
extraction (the `CTTypesetter` forced-LTR path stays deferred with RTL/non-
monotonic handling); the `glyphs_ptr`/`string_indices_ptr` with copy fallback is
the right pattern (`CFRange { location: 0, length: count }` is valid); and
reading `CTRun` data while the retained `CTLine`/runs array is alive is sound if
the slices do not escape the loop. The tests cover the intended deterministic
path (empty, single, ASCII monospace 1:1).

Two **implementation notes** (not Required, folded into the plan):
`CFAttributedString::new` returns `Option` — handle the impossible-failure path
cleanly (return an empty `Vec` on `None`); and for the copy fallback, allocate
the `Vec`'s length safely (e.g. `vec![0; n]`) before passing a `NonNull` buffer.

Review artifacts:

- Prompt: `logs/codex-review/20260603-125533-079572-prompt.md`
- Result: `logs/codex-review/20260603-125533-079572-last-message.md`
