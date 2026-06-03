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

# Experiment 262: Collection Index — the packed font-index type

## Description

With `renderGlyph` complete (Experiments 254–261), the next font sub-area is the
**Collection** — the set of faces (grouped by style) that a terminal renders
with, and the resolution of a codepoint to a specific face. The full
`Collection` pulls in `DeferredFace` and the `discovery` subsystem (CoreText
font matching); this experiment ports its **foundational value type** first: the
packed `Index` that names a font within a collection (`font/Collection.zig`
lines 875–939). It is pure bit-packing — no FFI, no discovery — and is used
everywhere a face is referenced, so it's the right atomic starting point.

### Upstream `Index` (`font/Collection.zig`)

- `Index = packed struct(u16) { style: Style = .regular, idx: IndexInt = 0 }`.
  `Style` is `enum(u3)`, so `idx` gets the remaining **13 bits**
  (`IndexInt = u13`, up to 8192 fonts per style). In Zig packed structs the
  first field is at the least-significant bits, so `style` occupies bits 0–2 and
  `idx` bits 3–15.
- `Special` (an `enum(IndexInt)`): `start = maxInt(IndexInt) = 8191`,
  `sprite = start`. Special indices don't map to a real face (sprite glyphs are
  drawn JIT via 2D graphics).
- `initSpecial(v)`: `{ .style = .regular, .idx = @intFromEnum(v) }`.
- `int(self)`: `@bitCast(self)` → the `u16` backing.
- `special(self)`:
  `if (self.idx < Special.start) null else @enumFromInt(self.idx)`.
- Invariants (its `test`): `@sizeOf(Index) == @sizeOf(u16)` and
  `idx_bits == 13`.

### Rust mapping (`roastty/src/font/collection.rs`, new)

A new `collection` module (the `Collection` struct itself lands in later
experiments; this slice is just `Index`):

- `pub(crate) struct Index { style: Style, idx: u16 }` — **private fields** so
  the `u13` `idx` invariant is enforced at the type boundary (upstream cannot
  represent `idx > 8191`; a `u16` could, so direct construction is closed off).
  `Style` is the existing `crate::font::Style` (`#[repr(u8)]`,
  `Regular..BoldItalic` = `0..3`, within the 3-bit field).
- Constants: `IDX_BITS = 13`, `STYLE_BITS = 3`,
  `IDX_MASK = (1 << 13) - 1 = 0x1FFF`. `Special::Sprite` with
  `START = IDX_MASK = 8191`.
- `Index::new(style, idx) -> Index`: the validated constructor —
  `assert!(idx <= IDX_MASK)` (a hard runtime check in **all** build modes, the
  analog of upstream's compile-time `u13` — `debug_assert!` would be compiled
  out in release and let an invalid `idx` through). All within-crate callers go
  through `new`/`special`/`from_int`, which only ever produce a valid `idx`.
- `Index::default()` → `Index::new(Regular, 0)` (upstream's field defaults).
- `Index::special(sprite)` → `{ style: Regular, idx: 8191 }` (valid;
  `idx == START`).
- accessors `style(&self) -> Style`, `idx(&self) -> u16`.
- `int(&self) -> u16`: `(self.style as u16) | (self.idx << STYLE_BITS)` — the
  faithful LSB-first layout (style low, idx high). **No masking**: `idx` is
  already a valid `u13` by construction, so there are no invalid bits to drop.
- `from_int(u16) -> Index`: `style` from bits 0–2 (mapped `0→Regular`, `1→Bold`,
  `2→Italic`, `3→BoldItalic`), `idx = v >> 3` (always `0..=8191`, a valid
  `u13`). (Only `0..=3` are valid styles; the 3-bit field's `4..=7` are unused
  by upstream and won't occur for a round-tripped `Index`.)
- `special_kind(&self) -> Option<Special>`:
  `if self.idx >= START { Some(Sprite) } else { None }` (faithful to
  `idx < start ⇒ null`).

`Style` gains a small `from_u3(u8) -> Option<Style>` helper (or the mapping
lives in `collection.rs`) so `from_int` doesn't duplicate a brittle match — to
be decided against `cargo build`; the mapping covers `0..=3`.

### Scope / faithfulness notes

- This is **only** the `Index` type. The `Collection` struct, `Entry`,
  `EntryOrAlias`, `add`/`getFace`/`getIndex`/`hasCodepoint`, `DeferredFace`, and
  `discovery` are later experiments.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/collection.rs` (new): the `Index` type, `Special`, and the
   `IDX_BITS`/`STYLE_BITS`/`IDX_MASK`/`START` constants, with `default`,
   `special`, `int`, `from_int`, `special_kind`.
2. `roastty/src/font/mod.rs`: `pub(crate) mod collection;` (and, if used, a
   `Style::from_u3` helper).
3. Tests in `collection.rs`:
   - `index_bit_layout`: `Index::new(Bold, 5).int() == 1 | (5 << 3) == 41`;
     `from_int(41) == Index::new(Bold, 5)`.
   - `index_round_trips`: for each style and a few `idx` values (incl. `0` and
     `8190`), `from_int(i.int()) == i`.
   - `index_default_is_zero`: `Index::default().int() == 0`.
   - `idx_bits_is_13`: `IDX_BITS == 13` (the invariant upstream pins) and the
     max non-special `idx` (`8190`) round-trips.
   - `special_index`: `Index::special(Special::Sprite)` has `idx() == 8191`, its
     `special_kind()` is `Some(Sprite)`, and a normal `Index::new(_, 0..=8190)`
     has `special_kind() == None`.
   - `from_int_idx_is_valid`: `from_int(u16::MAX).idx() == 8191` (any `u16`
     decodes to a valid `u13` `idx`, so `from_int` is total and safe).
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty collection
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Index` packs `style` (bits 0–2) and `idx` (bits 3–15) into a `u16` faithfully
  (`int`/`from_int` round-trip), with the `Special::Sprite = 8191` value and the
  `special_kind`/`special` helpers matching upstream;
- `IDX_BITS == 13` and a non-special `idx` up to `8190` round-trips;
- no FFI / C ABI changes (pure value type);
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the `Style` bit-width assumption needs
revisiting against the ported `Style`.

The experiment **fails** if the bit layout diverges from upstream's packed
struct or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation. It confirmed the **bit order**
is correct (Zig packed structs place the first field — `style`, `u3` — in the
least-significant bits, so `int() = style | (idx << 3)`;
`Index::new(Bold, 5).int() == 41`). It raised a **Medium** finding across two
passes: `idx: u16` could hold an invalid (`> 8191`) value that upstream's `u13`
cannot, and a `debug_assert` boundary is compiled out in release. The design was
revised to make the fields **private**, construct only through validated paths
(`new` with a hard `assert!(idx <= IDX_MASK)` in all build modes, `special`,
`from_int`), and drop the silent mask in `int()` (the `idx` is a valid `u13` by
construction; `from_int` always decodes to `0..=8191`). Codex's final pass
confirmed the finding is **fully resolved** and approved the design.

Review artifacts:

- Prompts: `logs/codex-review/20260602-214416-060779-prompt.md`,
  `…-214554-185064-prompt.md`, `…-214626-259496-prompt.md`
- Results: `logs/codex-review/20260602-214416-060779-last-message.md`,
  `…-214554-185064-last-message.md`, `…-214626-259496-last-message.md`
