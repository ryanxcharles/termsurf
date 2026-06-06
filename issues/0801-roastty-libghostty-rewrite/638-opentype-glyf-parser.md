+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 638: OpenType Glyf Parser

## Description

Port Ghostty's `font/opentype/glyf.zig` parser into Roastty.

Experiment 637 confirmed that Roastty already has the metric/color OpenType
parsers (`sfnt`, `head`, `hhea`, `post`, `os2`, `svg`) but still lacks upstream
`opentype/glyf.zig`. This experiment should add the missing `glyf` module
without pulling in embedded fonts or a whole-file SFNT table-directory reader.

The goal is the same narrow behavior upstream currently implements: store a
borrowed `glyf` table byte slice, expose entries by byte offset, classify
entries as simple or composite, and compute/validate the byte size of simple
glyph entries. Composite glyphs and hinting instructions remain rejected,
matching Ghostty's current parser.

## Upstream behavior

`vendor/ghostty/src/font/opentype/glyf.zig` defines:

- `Glyf { data }` with `init(data)` and `entry(offset)`;
- `Entry { header, data }`, where `data` starts immediately after the 10-byte
  glyph header;
- `Entry::Header` with the five OpenType glyph-header fields;
- `Entry::Type::{simple, composite}` based on `numberOfContours >= 0`;
- `SimpleFlags` with `xBytes()`/`yBytes()` for simple-glyph coordinate byte
  accounting;
- `Entry::size()`, which:
  - returns the 10-byte header size for zero-contour glyphs with fewer than two
    remaining bytes;
  - validates increasing `endPtsOfContours`;
  - rejects non-zero instruction lengths;
  - expands repeated flags for coordinate byte accounting;
  - rejects repeats that define more points than the last endpoint allows;
  - skips the computed x/y coordinate byte spans;
  - rejects composite glyphs.

## Changes

1. Add `roastty/src/font/opentype/glyf.rs`:
   - `Glyf<'a>` borrowing the table data;
   - `Entry<'a>` borrowing an entry's post-header data;
   - `Header`, `EntryType`, `SimpleFlags`;
   - `SizeError` for `EndOfStream`, `InstructionsNotSupported`,
     `CompositeNotSupported`, `EndPointsOutOfOrder`, and `TooManyPoints`;
   - `Glyf::from_bytes`, `Glyf::entry`, `Entry::entry_type`, and `Entry::size`.
2. Update `roastty/src/font/opentype/mod.rs` to export `glyf`.
3. Tests using hand-built byte fixtures, not embedded fonts:
   - simple glyph with no instructions and mixed flag/coordinate encodings
     returns the expected size;
   - zero-contour header-only glyph returns the header size;
   - composite glyph returns `CompositeNotSupported`;
   - non-zero instruction length returns `InstructionsNotSupported`;
   - truncated header/data returns `EndOfStream`;
   - out-of-order endpoints return `EndPointsOutOfOrder`;
   - over-large flag repeat returns `TooManyPoints`;
   - `Glyf::entry(offset)` slices from the requested table offset.

## Verification

- `cargo test -p roastty font::opentype::glyf`
- `cargo test -p roastty font::opentype`
- `cargo test -p roastty font::face::coretext`
- `cargo test -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

Pass = Roastty has a tested `glyf` parser matching upstream's current narrow
validation behavior, composite/hinted glyphs remain rejected, no embedded fonts
or whole-file SFNT reader are introduced, and existing OpenType/CoreText tests
stay green.

Fail = the parser accepts composite or hinted glyphs, miscounts repeated
coordinate flags, treats malformed endpoints as valid, grows into embedded-font
or full-SFNT work, or regresses existing face/OpenType behavior.

## Design Review

**Reviewer:** Codex (gpt-5.5) · session `019e9a92-a2a2-72b0-85c6-d7842ba51409`

**Verdict:** APPROVED.

The reviewer found no blocking design issues. Non-blocking implementation notes:
preserve exact OpenType bit mapping for `SimpleFlags`, include fixture coverage
for all 0/1/2-byte coordinate cases, and preserve upstream's trailing-byte
behavior where `Entry::size()` returns the consumed size without requiring the
entry slice to be fully consumed.
