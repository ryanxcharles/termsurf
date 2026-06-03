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

# Experiment 247: Port the OpenType metric-table parsers (sfnt scalars + `head` + `hhea`)

## Description

Begin the font **face** path by porting the OpenType SFNT table parsers that
`Face::getMetrics` reads. Upstream's CoreText `getMetrics`
(`font/face/coretext.zig`) does **not** use CoreText's high-level metric
accessors — it pulls the raw `head`/`hhea`/`os2`/`post` table bytes from the
font (via `CTFontCopyTable`) and parses the OpenType tables itself, for precise
control over units-per-em, the typo/win ascent fallbacks, and the underline
metrics. So faithful face-metric extraction has a pure-Rust prerequisite: the
`opentype/` table parsers.

This first slice ports the foundation — the shared SFNT scalar types and a
big-endian reader, plus the two smallest, highest-value tables: **`head`**
(gives `unitsPerEm`, the scaling denominator for every metric) and **`hhea`**
(gives the primary `ascender`/`descender`/`lineGap`). `os2` and `post` are the
next slice; the CoreText FFI that feeds these parsers raw table bytes comes
after that. This is a small, deterministic, **no-FFI,
no-macOS-version-sensitivity** slice — exactly the kind to lead a new subsystem
with.

### Upstream shapes (`font/opentype/`)

Both tables are `extern struct`s with `align(1)` fields read big-endian via
`reader.readStructEndian(T, .big)` from a `[]const u8` (`init(data)` →
`error{EndOfStream}!T`). The fields are fixed-layout, in spec order.

- **`sfnt.zig` scalar aliases (used here):** `uint16 = u16`, `int16 = i16`,
  `uint32 = u32`, `int32 = i32`, `FWORD = i16`, `UFWORD = u16`,
  `LONGDATETIME = i64`, `Fixed = FixedPoint(i32, 16, 16)` (a 16.16 fixed-point
  with `to(Float)` / `from(float)`). (`F26Dot6`, the `Version16Dot16` packed
  struct, and the `SFNT` table-directory reader are **deferred** — `F26Dot6`
  lands with the CoreText size handling, and the directory reader is only needed
  for the non-CoreText whole-file path.)
- **`head.zig` (54 bytes):** `majorVersion`/`minorVersion` (u16), `fontRevision`
  (Fixed), `checksumAdjustment`/`magicNumber` (u32), `flags` (u16), `unitsPerEm`
  (u16), `created`/`modified` (i64), `xMin`/`yMin`/`xMax`/ `yMax` (i16),
  `macStyle` (u16), `lowestRecPPEM` (u16),
  `fontDirectionHint`/`indexToLocFormat`/`glyphDataFormat` (i16).
- **`hhea.zig` (36 bytes):** `majorVersion`/`minorVersion` (u16), `ascender`/
  `descender`/`lineGap` (FWORD i16), `advanceWidthMax` (UFWORD u16),
  `minLeftSideBearing`/`minRightSideBearing`/`xMaxExtent` (FWORD i16),
  `caretSlopeRise`/`caretSlopeRun`/`caretOffset` (i16), `_reserved0..3` (i16),
  `metricDataFormat` (i16), `numberOfHMetrics` (u16).

### Rust mapping

New `roastty/src/font/opentype/` module (`pub(crate) mod opentype;` in
`font/mod.rs`; `opentype/mod.rs` re-exports `Head`, `Hhea`, and declares
`sfnt`/`head`/`hhea`).

- `opentype/sfnt.rs`:
  - `pub(crate) enum OpenTypeError { EndOfStream }`
    (`Debug, Clone, Copy, PartialEq, Eq`) — the analog of upstream
    `error{EndOfStream}`.
  - `pub(crate) struct Fixed(pub i32)` (`Debug, Clone, Copy, PartialEq, Eq`),
    the 16.16 fixed-point, with `pub(crate) fn to_f64(self) -> f64`
    (`self.0 as f64 / 65536.0`) and `pub(crate) fn from_f64(v: f64) -> Fixed`
    (`Fixed((v * 65536.0).round() as i32)`). (Concrete `Fixed` rather than a
    generic `FixedPoint`; `F26Dot6` is added when CoreText needs it.)
  - `pub(crate) struct Reader<'a> { data: &'a [u8], pos: usize }` — a minimal
    big-endian cursor, the faithful analog of `fixedBufferStream` +
    `readStructEndian(.big)`: `read_u16`/`read_i16`/`read_u32`/`read_i32`/
    `read_i64` each consume the next N bytes big-endian and return
    `Err(OpenTypeError::EndOfStream)` if fewer than N remain. (Type aliases like
    `FWORD`/`UFWORD` are documented in comments and read via the matching
    `read_i16`/`read_u16`.)
- `opentype/head.rs`: `pub(crate) struct Head { … all 18 fields … }`
  (`Debug, Clone, Copy, PartialEq, Eq`; `fontRevision: Fixed`,
  `created`/`modified: i64`, the rest `u16`/`i16`/`u32` per spec). Field names
  match the spec (camelCase upstream → Rust `snake_case`: `units_per_em`,
  `lowest_rec_ppem`, `index_to_loc_format`, etc.).
  `pub(crate) fn from_bytes(data: &[u8]) -> Result<Head, OpenTypeError>` builds
  a `Reader` and reads each field in spec order.
- `opentype/hhea.rs`: `pub(crate) struct Hhea { … all 18 fields … }` (reserved
  fields kept as `_reserved0..3` for a faithful layout), same `Debug, …, Eq`
  derives and `from_bytes` reading in spec order.

### Faithfulness and scope notes

- Reading is **field-by-field big-endian** (a safe `Reader`), not a
  `transmute`/`readStructEndian` over a packed struct — same bytes consumed in
  the same order, but no `unsafe` and no alignment assumptions.
- The `SFNT` table-**directory** reader (`SFNT.init`/`getTable`) is deferred:
  the CoreText path supplies individual table bytes via `CTFontCopyTable`, so
  the per-table `from_bytes` parsers are what `getMetrics` needs. The whole-file
  directory reader is only for the non-CoreText path.
- Upstream's `head`/`hhea` tests parse a real embedded font (`julia_mono`);
  since the embedded fonts aren't ported, this slice uses **hand-built table
  bytes** with known field values (an equivalent Roastty test per the Test
  Parity rule) — which also tests the parser in isolation more directly.
- No `os2`/`post`/`glyf`/`svg`, no CoreText FFI, no rasterization.
- No C ABI, header, or ABI inventory changes; no new dependencies (std only).

## Changes

1. `roastty/src/font/mod.rs`: add `pub(crate) mod opentype;` and a one-line doc
   note.

2. `roastty/src/font/opentype/mod.rs` (new): module doc;
   `pub(crate) mod sfnt; pub(crate) mod head; pub(crate) mod hhea;`; re-export
   `Head`/`Hhea` and the `sfnt` types.

3. `roastty/src/font/opentype/sfnt.rs` (new): `OpenTypeError`, `Fixed`
   (+`to_f64`/`from_f64`), `Reader` (big-endian).

4. `roastty/src/font/opentype/head.rs` (new): `Head` + `from_bytes`.

5. `roastty/src/font/opentype/hhea.rs` (new): `Hhea` + `from_bytes`.

6. Tests in the respective files:
   - `sfnt`: `fixed_round_trip` (`Fixed::from_f64(0.05499267578125).0 == 3604`
     and `to_f64` back); `reader_big_endian` (reads `u16`/`i16`/`u32`/`i64`
     correctly); `reader_end_of_stream` (truncated input → `Err(EndOfStream)`).
   - `head`: `parse_head` — a hand-built 54-byte big-endian `head` with known
     values (`units_per_em = 2048`, `font_revision = Fixed::from_f64(1.0)`,
     `magic_number = 0x5F0F3CF5`, `x_min = -10`, `index_to_loc_format = 1`, …) →
     all fields equal; `head_truncated` (53 bytes → `Err(EndOfStream)`).
   - `hhea`: `parse_hhea` — a hand-built 36-byte big-endian `hhea`
     (`ascender = 1900`, `descender = -450`, `line_gap = 0`,
     `number_of_h_metrics = 2`, …) → all fields equal; `hhea_truncated`.

7. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo test -p roastty opentype
cargo test -p roastty
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Fixed` round-trips 16.16 values; the big-endian `Reader` reads each width
  correctly and returns `EndOfStream` on short input;
- `Head` and `Hhea` parse their fixed layouts field-by-field big-endian, in spec
  order, matching the hand-built fixtures exactly, and error on truncation;
- the `sfnt` directory reader, `os2`/`post`, and CoreText FFI are cleanly
  deferred;
- no C ABI, header, or ABI inventory changes;
- `cargo fmt` accepted and `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if `os2`/`post` or the CoreText path forces a
representation change on `Reader`/`Fixed` or the table structs.

The experiment **fails** if a field is read at the wrong
offset/width/endianness, if `EndOfStream` is not returned on truncation, or if
any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no required
changes**.

Review artifacts:

- Prompt: `logs/codex-review/20260602-193913-678777-prompt.md`
- Result: `logs/codex-review/20260602-193913-678777-last-message.md`

Codex confirmed the `head` (54 bytes) and `hhea` (36 bytes) field order, widths,
and totals match upstream — including `Fixed` as 4 bytes, `LONGDATETIME` as
signed 8-byte values, `FWORD`/`UFWORD`, and the four reserved `i16` fields in
`hhea`; that the field-by-field big-endian `Reader` is a faithful safe
substitute for Zig's `readStructEndian(.big)`; that `Fixed(i32)` with
`to_f64 = raw/65536` and `from_f64 = round(v*65536)` matches the 16.16 behavior
and `0.05499267578125 * 65536 == 3604` exactly; and that deferring the SFNT
directory reader, `os2`/`post`, `F26Dot6`, and the CoreText FFI is sound because
CoreText supplies individual tables via `CTFontCopyTable`. Hand-built test bytes
were endorsed as tighter coverage than the embedded-font fixture.
