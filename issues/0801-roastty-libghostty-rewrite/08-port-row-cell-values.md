# Experiment 8: Port Packed Row and Cell Values

## Description

Port the packed `Row` and `Cell` value model from
`vendor/ghostty/src/terminal/page.zig`.

Experiments 3, 4, 6, and 7 now provide the prerequisites for this slice:
`Offset`, page-size integer aliases, `BitmapAllocator`, `Rgb`, `Underline`, and
`Style` IDs. This experiment should port only the value layer used by future
`Page.layout` and row/cell access. It should not allocate a `Page`, create page
backing memory, port `Capacity`, implement row/cell pointer access, or wire
grapheme/style/hyperlink maps.

The key architectural decision is to represent both values as raw `u64` wrapper
types with explicit bit masks and safe accessors:

```rust
#[repr(transparent)]
struct Row(u64);

#[repr(transparent)]
struct Cell(u64);
```

Do not use ordinary Rust structs or enums as the storage representation for
these values. Zig's upstream types are `packed struct(u64)`, and future page
layout math depends on the exact 8-byte value size, zero bit pattern, and field
packing. Rust has no native stable bitfield layout, so explicit raw-bit storage
is the clearest faithful port.

## Changes

1. Inspect upstream source.
   - Use `vendor/ghostty/src/terminal/page.zig` as the source of truth.
   - Re-read:
     - `Row`
     - `Row.SemanticPrompt`
     - `Row.cval`
     - `Row.managedMemory`
     - `Cell`
     - `Cell.ContentTag`
     - `Cell.RGB`
     - `Cell.Wide`
     - `Cell.SemanticContent`
     - `Cell.cval`
     - `Cell.init`
     - `Cell.isZero`
     - `Cell.hasText`
     - `Cell.codepoint`
     - `Cell.gridWidth`
     - `Cell.hasStyling`
     - `Cell.isEmpty`
     - `Cell.hasGrapheme`
     - `Cell.hasTextAny`
   - Do not modify `vendor/ghostty/`.

2. Add a page-value module.
   - Add `roastty/src/terminal/page.rs`.
   - Wire it from `roastty/src/terminal/mod.rs`.
   - Keep this file limited to value types and constants. It is acceptable for
     the module name to be `page` because these values are upstream `page.zig`
     types, but do not add a `Page` struct yet.

3. Port `Row`.
   - Represent `Row` as `#[repr(transparent)] Row(u64)`.
   - Preserve the bit layout:
     - `cells: Offset<Cell>`: 32 bits
     - `wrap`: 1 bit
     - `wrap_continuation`: 1 bit
     - `grapheme`: 1 bit
     - `styled`: 1 bit
     - `hyperlink`: 1 bit
     - `semantic_prompt`: 2 bits
     - `kitty_virtual_placeholder`: 1 bit
     - `dirty`: 1 bit
     - padding: remaining 23 bits
   - Add a `SemanticPrompt` enum matching upstream values: `none`, `prompt`,
     `prompt_continuation`.
   - Add safe getters/setters for the included fields.
   - Add `cval()` returning `u64`.
   - Add `managed_memory()` preserving upstream behavior:
     `styled || hyperlink || grapheme`.
   - Add layout tests for `size_of::<Row>() == 8` and
     `align_of::<Row>() == align_of::<u64>()`.

4. Port `Cell`.
   - Represent `Cell` as `#[repr(transparent)] Cell(u64)`.
   - Add layout tests for `size_of::<Cell>() == 8` and
     `align_of::<Cell>() == align_of::<u64>()`.
   - Preserve the bit layout:
     - `content_tag`: 2 bits
     - `content`: 24 bits
       - codepoint uses the low 21 bits of this field
       - palette background uses the low 8 bits
       - RGB background uses `r`, `g`, `b` bytes in upstream order
     - `style_id`: `StyleId` / `style::Id`, 16 bits
     - `wide`: 2 bits
     - `protected`: 1 bit
     - `hyperlink`: 1 bit
     - `semantic_content`: 2 bits
     - padding: remaining 16 bits
   - Add enums matching upstream values:
     - `ContentTag`
     - `Wide`
     - `SemanticContent`
   - Add constructors/helpers:
     - `Cell::init(codepoint)`
     - `Cell::bg_palette(index)`
     - `Cell::bg_rgb(rgb)`
   - Add safe getters/setters needed by future page operations:
     - content tag
     - codepoint
     - style ID
     - wide
     - protected
     - hyperlink
     - semantic content
   - Add upstream helper equivalents:
     - `cval`
     - `is_zero`
     - `has_text`
     - `grid_width`
     - `has_styling`
     - `is_empty`
     - `has_grapheme`
     - `has_text_any`
   - Reject invalid codepoints above `0x10FFFF` at construction time. Use a
     checked constructor or assertion; do not silently truncate.

5. Verify exact bit layout.
   - Add tests for raw `cval()` constants for representative fields. Include at
     least:
     - `Row::default().cval() == 0`
     - `Cell::init(0).cval() == 0`
     - `Cell::init('A').cval()` with the expected packed value
     - one non-zero `style_id`
     - each `Wide` value
     - each `ContentTag` value via constructors
     - each `SemanticContent` value
     - row `cells` offset
     - each row boolean flag
     - each `SemanticPrompt` value
   - Use Zig's declaration order as the packed field order. If there is any
     doubt about a raw constant, verify it with a temporary local Zig snippet or
     `zig test` probe before committing. Do not leave the probe in the repo.

6. Translate upstream tests and add direct equivalents.
   - Port upstream `Cell is zero by default`.
   - Add direct tests for:
     - `Cell::init`
     - `Cell::has_text`
     - `Cell::codepoint`
     - `Cell::grid_width`
     - `Cell::has_styling`
     - `Cell::is_empty`
     - `Cell::has_grapheme`
     - `Cell::has_text_any`
     - `Row::managed_memory`
   - Document deferred upstream tests:
     - `Page.layout can take a maxed capacity`
     - `Page capacity ...`
     - `Page init`
     - `Page read and write cells`
     - every grapheme/style/hyperlink/clone/move/integrity test

7. Preserve the unsafe policy.
   - Prefer fully safe Rust for this slice.
   - The raw `u64` wrappers should not require `unsafe`.
   - If an implementation uses `unsafe` for bit conversion, it must include a
     safety comment and a test proving the layout invariant. Prefer avoiding
     that entirely.

8. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - module added;
     - raw-bit layout approach;
     - upstream tests ported;
     - upstream tests deferred and why;
     - any `unsafe` code used and why;
     - verification command output summary.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `Row` and `Cell` exist as raw `u64` wrapper types;
- layout tests prove both values are 8 bytes and aligned like `u64`;
- raw `cval()` tests prove representative bit positions;
- raw zero-value tests prove both `Row` and `Cell` zero bit patterns;
- the upstream `Cell is zero by default` behavior is ported;
- row/cell helper behavior is tested;
- no `Page`, `Page.layout`, page allocation, grapheme map, style set, hyperlink
  map, clone, move, or integrity behavior is introduced;
- `cargo fmt`, targeted `cargo test -p roastty terminal::page`, and full
  `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- the value API is implemented, but exact packed constants cannot be confidently
  verified in this pass. In that case, do not mark the values complete; record
  the missing layout proof and design the next experiment around it.

The experiment fails if:

- it uses ordinary Rust struct layout for packed values;
- it silently truncates invalid codepoints or field values;
- it starts implementing `Page` allocation/layout or metadata maps;
- it leaves raw bit layout untested.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or implementing the
slice.
