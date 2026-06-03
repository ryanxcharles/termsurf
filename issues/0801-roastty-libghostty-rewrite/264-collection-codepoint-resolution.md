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

# Experiment 264: Collection codepoint resolution — get_index / has_codepoint

## Description

The heart of the `Collection`: resolving a Unicode codepoint to the face that
renders it, honoring the requested **presentation** (text vs emoji). This ports
`Collection.getIndex`/`hasCodepoint` and `Entry.hasCodepoint`
(`font/Collection.zig` lines 272–308, 803–834), plus the `Face.glyphIndex`
primitive they need (`font/face/coretext.zig`). With this, the Collection can
answer "which face has codepoint X in presentation P?" — what the
`CodepointResolver` and shaper sit on.

### Upstream behavior

- `Face.glyphIndex(cp) -> ?u32`: UTF-32 → UTF-16 (surrogate pair for non-BMP),
  `getGlyphsForCharacters` (returns false ⇒ no glyph ⇒ `null`), else the glyph
  id.
- `Entry.hasCodepoint(cp, p_mode)` (lines 803–834):
  - `default(p)`: a **fallback** face requires explicit presentation matching
    (treat as `explicit(p)`); a non-fallback face accepts `any`.
  - `explicit(p)`: `glyphIndex(cp)` then match the presentation —
    `text ⇒ !isColorGlyph(idx)`, `emoji ⇒ isColorGlyph(idx)`.
  - `any`: `glyphIndex(cp) != null`.
- `Collection.getIndex(cp, style, p_mode)` (lines 272–293): iterate the style's
  entries in order, return the first matching `Index`, else `null`.
- `Collection.hasCodepoint(index, cp, p_mode)` (lines 299–308): bounds-check the
  index, then `entry.hasCodepoint`.

### Rust mapping

1. **`Face::glyph_index(&self, cp: u32) -> Option<u16>`** (`face/coretext.rs`):
   `char::from_u32(cp)?` (an invalid scalar has no glyph), `encode_utf16` into a
   `[u16; 2]`, call `glyphs_for_characters` (the FFI returns `bool`;
   `false ⇒ None`), return `Some(glyphs[0])`. (For a surrogate pair the trailing
   unit maps to `0`; the glyph is `glyphs[0]`.)
2. **`PresentationMode`** (`collection.rs`, new):
   `enum PresentationMode { Explicit(Presentation), Default(Presentation), Any }`
   (`Presentation` is the existing `crate::font::Presentation`).
3. **`Entry::has_codepoint(&self, cp: u32, p_mode: PresentationMode) -> bool`**:
   the faithful match — `Default(p)` resolves to `Explicit(p)` when
   `self.fallback` else `Any`; `Explicit(p)` does `glyph_index` + the color
   check; `Any` is `glyph_index(cp).is_some()`.
4. **`Collection::get_index(&self, cp: u32, style: Style, p_mode: PresentationMode) -> Option<Index>`**:
   iterate `faces[style]`, return `Index::new(style, i)` for the first entry
   whose `has_codepoint` is true, else `None`.
5. **`Collection::has_codepoint(&self, index: Index, cp: u32, p_mode: PresentationMode) -> bool`**:
   `index.idx() as usize >= faces[index.style()].len() ⇒ false`, else the
   entry's `has_codepoint`. (A special index has `idx == 8191 ≥ len`, so it's
   `false`, matching upstream's direct-index behavior.)

### Scope / faithfulness notes

- **Deferred**: the `deferred`-face arm of `hasCodepoint` (lazy `DeferredFace`
  search) — all faces are eagerly loaded here.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/face/coretext.rs`: add `glyph_index`.
2. `roastty/src/font/collection.rs`: add `PresentationMode`,
   `Entry::has_codepoint`, `Collection::get_index`/`has_codepoint`; import
   `crate::font::Presentation`.
3. Tests (live CoreText, macOS):
   - `glyph_index_basic`: `Face::new("Menlo", 32.0).glyph_index('M' as u32)` is
     `Some(non-zero)`; a Private-Use codepoint Menlo lacks (e.g. `0xE000`) is
     `None`; the `U+1F600` emoji resolves in Apple Color Emoji.
   - `get_index_text`: a collection with Menlo `Regular` —
     `get_index('M', Regular, Any)` is `Some({Regular,0})`; `Explicit(Text)` is
     `Some({Regular,0})` (`'M'` is not color); `Explicit(Emoji)` is `None`.
   - `get_index_emoji`: Menlo `Regular` (idx 0) + Apple Color Emoji `Regular`
     (idx 1) — `get_index(😀, Regular, Any)` and `Explicit(Emoji)` are
     `Some({Regular,1})` (Menlo lacks it; the emoji face has it as color);
     `Explicit(Text)` is `None`.
   - `default_presentation_fallback`: the emoji `Entry` added as
     **non-fallback** answers `has_codepoint(😀, Default(Text)) == true`
     (Default ⇒ Any), but added as **fallback** answers `false` (Default ⇒
     Explicit(Text); the emoji glyph is color, not text).
   - `has_codepoint_bounds`:
     `Collection::has_codepoint(Index::new(Regular, 5), 'M', Any)` on a one-face
     collection is `false`.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty collection
cargo test -p roastty face
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Face::glyph_index` faithfully maps a codepoint (incl. non-BMP) to a glyph or
  `None`;
- `Entry::has_codepoint` implements the `default`/`explicit`/`any` +
  fallback/color logic, and `Collection::get_index`/`has_codepoint` resolve in
  list order with the bounds guard;
- a text codepoint resolves to a text face and an emoji codepoint to the color
  face, with the presentation filters behaving as upstream;
- the deferred-face arm is cleanly deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if `glyph_index`'s surrogate-pair handling needs a
different shape than expected.

The experiment **fails** if the presentation/fallback resolution diverges from
upstream or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no required
changes**.

Review artifacts:

- Prompt: `logs/codex-review/20260602-215746-122426-prompt.md`
- Result: `logs/codex-review/20260602-215746-122426-last-message.md`

Codex confirmed the resolution logic matches upstream: `Default(p)` becomes
`Explicit(p)` only for fallback entries, `Explicit(p)` filters through
`glyph_index` + `is_color_glyph`, and `Any` accepts any mapped glyph; returning
`glyphs[0]` after `CTFontGetGlyphsForCharacters` returns true is faithful
(including non-BMP surrogate-pair handling), and `char::from_u32(cp)?` is an
acceptable Rust scalar-value gate for invalid/surrogate codepoints. `get_index`
ordering, the `has_codepoint` bounds behavior, and the special-index `false`
behavior are also aligned.
