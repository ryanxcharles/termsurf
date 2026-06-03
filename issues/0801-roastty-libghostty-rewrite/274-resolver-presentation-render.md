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

# Experiment 274: Resolver getPresentation + renderGlyph delegation

## Description

The `CodepointResolver`'s render surface: `getPresentation` (which atlas a glyph
needs — text vs emoji) and `renderGlyph` (render a resolved glyph into the
atlas). Both are thin layers over the already-ported `Collection` and
`Face::render_glyph`. This ports them (`font/CodepointResolver.zig` lines
301–351); the **sprite** render arm (a special sprite index drawn JIT) is
deferred to the sprite-font sub-area.

## Upstream behavior (`font/CodepointResolver.zig`)

- `getPresentation(index, glyph_index)` (lines 303–314): a special `sprite`
  index → `text`; otherwise load the face and return `emoji` if
  `isColorGlyph(glyph_index)` else `text`.
- `renderGlyph(alloc, atlas, index, glyph_index, opts)` (lines 327–351): a
  special `sprite` index renders via the sprite face (**deferred**); otherwise
  `collection.getFace(index).renderGlyph(atlas, glyph_index, opts)`.

## Rust mapping (`roastty/src/font/codepoint_resolver.rs`)

- `get_presentation(&self, index: Index, glyph: u16) -> Result<Presentation, EntryError>`:
  `if index.special_kind().is_some() { return Ok(Presentation::Text); }` (the
  only special kind is `Sprite` → `Text`); else
  `let face = self.collection.get_face(index)?; Ok(if face.is_color_glyph(glyph) { Presentation::Emoji } else { Presentation::Text })`.
- `enum ResolverRenderError { SpriteUnavailable, Entry(EntryError), Render(RenderGlyphError) }`
  with `From<EntryError>`/`From<RenderGlyphError>` (so `?` composes).
- `render_glyph(&self, atlas: &mut Atlas, index: Index, glyph: u16, opts: &RenderOptions) -> Result<Glyph, ResolverRenderError>`:
  `if index.special_kind().is_some() { return Err(ResolverRenderError::SpriteUnavailable); }`
  (sprite rendering deferred); else
  `let face = self.collection.get_face(index)?; Ok(face.render_glyph(atlas, glyph, opts)?)`.

## Scope / faithfulness notes

- **Deferred**: the **sprite** render arm — a sprite index needs the sprite font
  (box-drawing/braille JIT rendering), its own sub-area. `get_presentation`
  already handles the sprite index faithfully (→ `text`); `render_glyph` returns
  `SpriteUnavailable` until the sprite font lands.
- `Atlas`, `Glyph`, `RenderOptions`, `RenderGlyphError` come from
  `crate::font::{atlas, glyph, face::coretext}`.
- No C ABI/header/ABI-inventory change.

## Changes

1. `roastty/src/font/codepoint_resolver.rs`: add `get_presentation`,
   `ResolverRenderError` (+ `From` impls), and `render_glyph`; import `Atlas`,
   `Glyph`, `Presentation` is already in scope,
   `RenderOptions`/`RenderGlyphError` from `face::coretext`.
2. Tests (live CoreText, macOS):
   - `get_presentation_text`: a resolver over Menlo `Regular`; `'M'`'s glyph at
     `{Regular, 0}` → `Ok(Text)`.
   - `get_presentation_emoji`: a collection with Apple Color Emoji at
     `{Regular, 0}`; the `😀` glyph → `Ok(Emoji)`.
   - `get_presentation_sprite`: `get_presentation(Index::special(Sprite), 0)` is
     `Ok(Text)` without loading a face.
   - `render_glyph_via_resolver`: render `'M'` (resolved to `{Regular, 0}`) into
     a grayscale atlas with a `.none` `RenderOptions`; `Ok(g)` with
     `g.width > 0`, `g.height > 0`.
   - `render_glyph_sprite_unavailable`:
     `render_glyph(_, Index::special(Sprite), 0, _)` is
     `Err(SpriteUnavailable)`.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty codepoint_resolver
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `get_presentation` returns `text` for a sprite index, and `emoji`/`text` from
  the face's color state otherwise;
- `render_glyph` delegates a non-sprite index to `Face::render_glyph` and
  returns `SpriteUnavailable` for a sprite index;
- the sprite render arm is cleanly deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the error-composition shape needs adjustment.

The experiment **fails** if the presentation/render delegation diverges from
upstream or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no required
changes**.

Review artifacts:

- Prompt: `logs/codex-review/20260602-230721-531396-prompt.md`
- Result: `logs/codex-review/20260602-230721-531396-last-message.md`

Codex confirmed `get_presentation` is faithful for the scoped model (a special
index is the sprite → `Text`; otherwise the face's `is_color_glyph(glyph)`
chooses `Emoji`/`Text`), that `render_glyph` correctly delegates non-special
indices through `collection.get_face(index)?.render_glyph(...)`, that returning
`SpriteUnavailable` for the sprite arm is a sound scoped deviation while sprite
rendering is deferred, that the `EntryError`/`RenderGlyphError` composition is
appropriate, and that the tests cover the special fast path and the deferred
sprite-render error.
