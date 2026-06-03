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

# Experiment 324: SVG color detection

## Description

roastty's `ColorState` (`face/coretext.rs`) currently detects **only** sbix
color fonts; SVG-table color fonts are explicitly deferred ("non-sbix color
fonts are not yet flagged"). Upstream's `ColorState` also reads the `SVG ` table
and parses it via `opentype.SVG`, so `isColorGlyph` returns `true` for a glyph
that has an SVG document. This experiment ports the `opentype.SVG` parser and
wires it into `ColorState`, completing the color-glyph detection: a face is a
color face if it has sbix **or** a parseable `SVG ` table, and a glyph is a
color glyph if it is sbix or present in the SVG table.

## Upstream behavior (`opentype/svg.zig`, `face/coretext.zig` `ColorState`)

`opentype.SVG` is a minimal reader of the OpenType `SVG ` table — just enough to
answer "is glyph N in the table?":

```zig
pub fn init(data) !SVG {
    if (readInt(u16, .big) != 0) return error.SVGVersionNotSupported; // version
    const offset = readInt(u32, .big);                                // doc-list offset
    seekTo(offset);
    const len = readInt(u16, .big);                                   // numEntries
    const records: [*]const [12]u8 = @ptrCast(data[getPos()..]);      // 12-byte records
    const start_range = glyphRange(&records[0]);
    const end_range = if (len == 1) start_range else glyphRange(&records[len - 1]);
    return .{ .start_glyph_id = start_range[0], .end_glyph_id = end_range[1],
              .records = records[0..len] };
}
pub fn hasGlyph(self, glyph_id: u16) bool {
    if (glyph_id < start_glyph_id or glyph_id > end_glyph_id) return false; // fast out-of-range
    if (glyph_id == start_glyph_id or glyph_id == end_glyph_id) return true; // fast endpoints
    return binarySearch(records, glyph_id, compareGlyphId) != null;          // by [start,end]
}
```

Each 12-byte record is an `SVGDocumentRecord`
`{ startGlyphID: u16, endGlyphID: u16, svgDocOffset: u32, svgDocLength: u32 }`
(big-endian); `glyphRange` reads its first two `u16`s. The records are sorted by
glyph id, so `hasGlyph` binary- searches by the `[start, end]` range.

`ColorState`:

```zig
sbix: bool,        // the `sbix` table exists and is non-empty
svg: ?opentype.SVG // the parsed `SVG ` table (if any)
pub fn isColorGlyph(self, glyph_id: u32) bool {
    const g = cast(u16, glyph_id) orelse return false;
    if (self.sbix) return true;
    if (self.svg) |svg| if (svg.hasGlyph(g)) return true;
    return false;
}
```

## Rust mapping

- `roastty/src/font/opentype/svg.rs` (new): port `opentype.SVG`.
  ```rust
  pub(crate) struct Svg {
      start_glyph_id: u16,
      end_glyph_id: u16,
      records: Vec<[u8; 12]>, // owned copies (small); avoids a data lifetime
  }
  impl Svg {
      pub(crate) fn from_bytes(data: &[u8]) -> Result<Svg, OpenTypeError> { … }
      pub(crate) fn has_glyph(&self, glyph_id: u16) -> bool { … }
  }
  ```
  `from_bytes`: read the big-endian `u16` version (`!= 0` ⇒
  `OpenTypeError::UnsupportedVersion`); read the `u32` document-list offset; at
  that offset read the `u16` entry count `len`; require `len >= 1` and the
  `len × 12` record bytes to fit (else `OpenTypeError::EndOfStream`); copy the
  records; `start_glyph_id` from record 0, `end_glyph_id` from record `len - 1`
  (or record 0 when `len == 1`). All reads are bounds-checked (the analog of
  upstream's `EndOfStream`). `has_glyph`: the two fast paths (out-of-range ⇒
  `false`, endpoints ⇒ `true`), then a `binary_search_by` over the records
  comparing the glyph id against each record's `[start, end]` range
  (`glyph_range(record)` reads the first two big-endian `u16`s).
- `roastty/src/font/opentype/mod.rs`: add `pub(crate) mod svg;`.
- `roastty/src/font/face/coretext.rs` `ColorState`: add `svg: Option<Svg>`;
  `is_color_glyph(glyph)` returns `true` if `sbix`, else
  `self.svg.as_ref().is_some_and(|s| s.has_glyph(glyph))`. `detect_color` reads
  the `SVG ` table (`copy_table(b"SVG ")`), parses it
  (`Svg::from_bytes(..).ok()`), and returns `Some(ColorState { sbix, svg })`
  when **either** `sbix` is true **or** `svg.is_some()` (so an SVG-only color
  font is now flagged). Update the `ColorState`/`detect_color` doc comments (SVG
  is no longer deferred).

## Scope / faithfulness notes

- **Ported**: the `opentype.SVG` table reader (`from_bytes` + `has_glyph`) and
  its use in `ColorState` — completing color detection for sbix **and** SVG
  fonts. `is_color_glyph` now matches upstream's `isColorGlyph`.
- **Deferred**: actually _rendering_ SVG glyphs (rasterizing the SVG document)
  is a separate concern — CoreText renders the color glyph through the normal
  `CTFontDrawGlyphs`/bitmap path, and this detection only decides the
  presentation/atlas-format choice (BGRA vs grayscale), exactly as upstream's
  detection does. The COLR table (a third color format) is not read by upstream
  `ColorState` either, so it stays out of scope.
- The records are copied into an owned `Vec<[u8; 12]>` rather than referencing
  the table data (upstream keeps `svg_data` alive and slices it); behavior is
  identical, and it avoids threading a lifetime through `ColorState`.
- No C ABI/header/ABI-inventory change (`ColorState`/`Svg` are internal Rust).

## Changes

1. `roastty/src/font/opentype/svg.rs`: the `Svg` parser.
2. `roastty/src/font/opentype/mod.rs`: declare the module.
3. `roastty/src/font/face/coretext.rs`: extend `ColorState`/`detect_color`/
   `is_color_glyph` for SVG; update the doc comments.
4. Tests:
   - `svg.rs` `from_bytes_single_record`: a hand-built minimal `SVG ` table
     (version `0`, a doc-list at some offset, one record
     `start=11482, end=11482`) parses to
     `start_glyph_id == end_glyph_id == 11482`, and `has_glyph(11482)` is
     `true`, `has_glyph(11481)`/`has_glyph(11483)` `false` (mirrors upstream's
     JuliaMono assertion without needing the font).
   - `svg.rs` `from_bytes_multi_record`: three records (e.g. `[10,12]`,
     `[20,22]`, `[40,42]`) — `has_glyph` is `true` inside each range (including
     a non-endpoint middle record `21`) and `false` in the gaps (`13`, `30`,
     `43`), exercising the binary search and the `[start, end]` comparison.
   - `svg.rs` `from_bytes_bad_version` / `from_bytes_truncated`: a non-zero
     version returns `UnsupportedVersion`; a buffer too short for the header,
     the count, or the declared records returns `EndOfStream`.
   - `coretext.rs` `color_state_svg` (integration, best-effort): a known
     non-color text font (`Menlo`) yields `has_color() == false`; the sbix path
     (`Apple Color Emoji`) still yields `has_color() == true` and a color glyph.
     (The SVG-table path is fully covered by the synthetic `svg.rs` tests, since
     a macOS system font with an `SVG ` table is not guaranteed.)
5. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty svg
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `opentype::svg::Svg` reproduces upstream's `opentype.SVG` (`from_bytes`
  parsing and the `has_glyph` fast paths + binary search), and `ColorState`
  flags a face as color for sbix **or** a parseable `SVG ` table, with
  `is_color_glyph` matching upstream's `isColorGlyph`;
- the single/multi-record, bad-version, truncated, and ColorState tests pass;
- SVG glyph _rendering_ and the COLR table stay out of scope;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if an integration test against a real SVG-table
font cannot be expressed on the available system fonts (the parser is still
proven directly against synthetic tables).

The experiment **fails** if the `SVG ` parsing, the `has_glyph` predicate, or
the `ColorState` integration diverges from upstream, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
findings**. It confirmed: the `SVG ` parser layout is correct for the fields
upstream reads (`u16` version, `u32` document-list offset, then at that offset a
`u16 numEntries` followed by 12-byte records — the header's reserved field after
the offset need not be read because the parser seeks by the offset); the Rust
`binary_search_by` direction is correct (for an element comparator,
`gid < start` ⇒ `Greater`, `gid > end` ⇒ `Less`, in-range ⇒ `Equal` — not
inverted vs upstream's Zig); the `len == 1` start/end handling matches upstream
and the explicit `len >= 1` + record-fit bounds checks are a safe Rust
equivalent of upstream's `EndOfStream`; owning a `Vec<[u8; 12]>` is behaviorally
identical because `has_glyph` only needs each record's first two big-endian
`u16`s (not the SVG document payload); and `ColorState` detecting
`sbix || parseable SVG` with `sbix` checked before `svg.has_glyph` matches
upstream's color-glyph decision.

One **implementation note** (not Required): adding `Option<Svg>` (which owns a
`Vec`) means `ColorState` can no longer be `Copy`, so `Face::is_color_glyph`
must switch from `self.color.is_some_and(...)` to
`self.color.as_ref().is_some_and(...)`. Folded into the implementation plan —
`ColorState` drops its `Copy`/`Clone` derive as needed and the call site uses
`.as_ref()`.

Review artifacts:

- Prompt: `logs/codex-review/20260603-110620-209869-prompt.md`
- Result: `logs/codex-review/20260603-110620-209869-last-message.md`

## Result

**Result:** Pass

SVG color detection lands.

- `roastty/src/font/opentype/svg.rs` (new): `Svg::from_bytes` parses the `SVG `
  table (big-endian `u16` version → `UnsupportedVersion` if non-zero; `u32`
  document-list offset; sub-slice at the offset; `u16` `numEntries`; the
  `numEntries × 12`-byte records, all bounds-checked → `EndOfStream` on
  truncation/over-declared count/zero records); `start_glyph_id` from record 0,
  `end_glyph_id` from the last record. `has_glyph` does the two fast paths
  (out-of-span → `false`, endpoints → `true`) then a `binary_search_by` over the
  records by `[start, end]`. `opentype/mod.rs` declares the module.
- `roastty/src/font/face/coretext.rs`: `ColorState` gained `svg: Option<Svg>`
  (and dropped its `Copy`/`Clone`/`Eq` derives — it now owns a `Vec`);
  `is_color_glyph` is `sbix → true`, else `svg.has_glyph(glyph)`. `detect_color`
  reads the `SVG ` table, parses it, and returns `Some` when **sbix OR svg**.
  `Face::is_color_glyph` uses `self.color.as_ref()`.

Tests: `svg.rs` `from_bytes_single_record` (start=end=11482, mirroring
upstream's JuliaMono assertion without the font), `from_bytes_multi_record`
(non-endpoint middle hit + gap misses — exercises the binary search),
`from_bytes_offset_ beyond_header`, `from_bytes_bad_version`
(`UnsupportedVersion`), `from_bytes_truncated` (4 `EndOfStream` cases);
`coretext.rs` `color_state_svg_branch` (a synthetic SVG-only `ColorState`: glyph
5 is color, 6 is not; sbix short-circuits). The existing
`text_font_has_no_color` (Menlo) and `emoji_font_has_color` (Apple Color Emoji,
sbix) still pass.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2695 passed, 0 failed (+6, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

Color-glyph detection is now complete for both color formats CoreText's
`ColorState` reads: a face is colored if it has sbix **or** a parseable `SVG `
table, and a glyph is a color glyph if it is sbix or has an SVG document —
matching upstream's `isColorGlyph`. The result-review caught a real latent bug
the experiment exposed (`render_glyph` conflated `is_color` with `sbix`, which
was harmless while only sbix existed) and it is now the faithful upstream split.

The remaining font-subsystem work is the larger arc: **font discovery** (the
CoreText `Descriptor → CTFontDescriptor → CTFontCollection` matching that gates
the resolver's discovery fallback and codepoint overrides), wiring
`get_constraint` into the render path, and the **shaper**.

## Completion Review

Codex reviewed the completed implementation and result and raised **one Required
finding**, since fixed: `render_glyph` used `let sbix = is_color;`, which was
only correct while color meant sbix-only — after this experiment an SVG glyph is
color but not sbix, so it would wrongly take the sbix-only branches (skipping
synthetic bold, skipping thicken padding, quantizing position as a bitmap).
Fixed with the upstream-equivalent split:
`let sbix = is_color && self.color.as_ref().is_some_and(|c| c.sbix);`
(`is_color` selects BGRA; `sbix` additionally gates the bitmap-only branches). A
follow-up review **confirmed the fix fully resolves the finding** and matches
upstream's `sbix = is_color and self.color.?.sbix`, with no remaining issues.
Codex also confirmed the parser is sound (version/offset/count/record parsing,
`len == 1`, bounds checks, `binary_search_by` direction, `sbix || svg`
detection, and dropping `Copy/Eq` are all consistent with the port).

Review artifacts:

- Result review: `logs/codex-review/20260603-111218-196044-last-message.md`
- Fix confirmation: `logs/codex-review/` (follow-up in session
  `019e8e1b-1019-72f3-b15b-3259f8aabd15`)
