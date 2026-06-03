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

# Experiment 334: discover a font for a codepoint (CTFontCreateForString)

## Description

The resolver's discovery fallback (Experiment 333) uses the **general**
`CTFontCollection` match, which misses some codepoints — notably CJK unified
ideographs, where CoreText's locale-aware `CTFontCreateForString` is the only
reliable way to pick the right font. This experiment ports the core of
upstream's `discoverCodepoint`: given an existing ("original") face and a
codepoint, ask CoreText for the font it would use to render that codepoint,
rejecting the `LastResort` (replacement-glyph) font. The `discoverFallback`
orchestration (the CJK gate and the general-then-codepoint fallback) and its
resolver wiring are the next experiments.

## Upstream behavior (`discovery.zig` `discoverCodepoint`)

```zig
fn discoverCodepoint(self, collection, desc) !?*FontDescriptor {
    // Pick the original font by the requested style (bold_italic → bold →
    // italic → regular, each falling back when absent).
    const original = …collection.getFace(style)…;

    // UTF-8 → CFString; the CTFontCreateForString range is in UTF-16 units.
    const str = String.createWithBytes(utf8(desc.codepoint));
    const range_len = if (surrogate_pair(desc.codepoint)) 2 else 1;

    const font = original.font.createForString(str, Range.init(0, range_len))
        orelse return null;

    // Reject the LastResort font (only replacement chars).
    if (eql(font.copyPostScriptName(), "LastResort")) return null;

    return font.copyDescriptor();
}
```

`CTFontCreateForString` starts from the original font and returns the font
CoreText would use for the string. The `range` length is the codepoint's
**UTF-16 unit count** (`2` for a supplementary codepoint, else `1`). The
`LastResort` font is CoreText's final fallback (it renders only replacement
boxes), so a result named `"LastResort"` means "no real font" — treated as
`null`.

## Rust mapping (`roastty/src/font/face/coretext.rs`)

The original-font selection lives in the collection/resolver; the **CoreText
call** is naturally a method on the original [`Face`] (it needs the face's
private `CTFont`):

- `pub(crate) fn font_for_codepoint(&self, cp: u32) -> Option<Face>`:
  - `let c = char::from_u32(cp)?;`
  - `let s = CFString::from_str(&c.to_string());` (the UTF-8 → CFString).
  - `let range = CFRange { location: 0, length: c.len_utf16() as isize };` (the
    UTF-16 unit count — `1` or `2`).
  - `let font = unsafe { self.font.for_string(&s, range) };`
    (`CTFontCreateForString` — the objc2 binding is non-`Option`: it always
    returns a font, falling back to `LastResort`).
  - `if unsafe { font.post_script_name() }.to_string() == "LastResort" { return None; }`
    (reject the replacement font).
  - `Some(Face::from_ct_font(font))` — wrap the discovered font.

## Scope / faithfulness notes

- **Ported**: `discoverCodepoint`'s CoreText call — `CTFontCreateForString` from
  an original face for a codepoint, with the UTF-16 range length and the
  `LastResort` rejection, returning the discovered face (or `None`).
- **Deferred**: the **original-font style selection** (bold_italic → … →
  regular) and the `discoverFallback` orchestration (the CJK `0x4E00..=0x9FFF`
  gate and the empty-general-discover fallback) — those live in discovery/the
  resolver and are the next experiment. Here the caller supplies the original
  face. Applying variations is also deferred.
- The objc2 `for_string` is non-`Option` (always returns a font), so the
  `LastResort` name check is the sole "no font" signal — exactly upstream's
  effective behavior (its `orelse` rarely fires; the `LastResort` guard is the
  real filter).
- No C ABI/header/ABI-inventory change (`Face` is internal Rust).

## Changes

1. `roastty/src/font/face/coretext.rs`: add `Face::font_for_codepoint`.
2. Tests (in `coretext.rs`):
   - `font_for_codepoint_cjk`:
     `Face::new("Menlo", 24.0).font_for_codepoint(0x4E00)` (`一`, a CJK
     ideograph Menlo lacks) returns `Some(face)`, and that face **renders**
     `0x4E00` (`face.glyph_index(0x4E00).is_some()`) — CoreText found a CJK
     font.
   - `font_for_codepoint_ascii`: `font_for_codepoint('M' as u32)` returns
     `Some(face)` rendering `'M'` (Menlo itself, or another font, has it).
   - `font_for_codepoint_supplementary`: an emoji `0x1F600` (supplementary,
     UTF-16 surrogate pair, `len == 2`) returns `Some(face)` rendering it —
     exercising the 2-unit range.
   - `font_for_codepoint_none`: a codepoint no font covers (a noncharacter, e.g.
     `0xFFFFF` plane-15 with no font, or `char::from_u32` of a noncharacter)
     returns `None` (the `LastResort` rejection). (The exact codepoint is
     confirmed against the host during implementation; if every probed codepoint
     resolves, the test asserts the `LastResort` guard via a documented
     unsupported codepoint.)
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty font_for_codepoint
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `font_for_codepoint` reproduces `discoverCodepoint`'s CoreText call (the
  CFString, the UTF-16 range length, `CTFontCreateForString`, and the
  `LastResort` rejection), returning the discovered face or `None`;
- the CJK, ascii, supplementary, and none tests pass;
- the original-font style selection and the `discoverFallback` orchestration
  stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if the host cannot reproduce the `None`
(LastResort) case deterministically (the positive cases still prove the CoreText
call).

The experiment **fails** if the `CTFontCreateForString` call, the range length,
or the `LastResort` rejection diverges from upstream, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
findings**. It confirmed the slice is faithful: `c.len_utf16()` is the correct
`CFRange.length` (`CTFontCreateForString` ranges are over UTF-16 units — `1` for
BMP, `2` for supplementary); `CFString::from_str(&c.to_string())` is equivalent
to upstream's UTF-8 `createWithBytes` of the single scalar; the `LastResort`
PostScript-name rejection matches upstream's real no-font filter, and checking
it after the (non-`Option`) `for_string` call is the correct Rust shape; a
`Face` method is the right local API (the call needs the private source
`CTFont`) and returning `Face::from_ct_font(font)` preserves the color-detection
wrapping; and deferring the original-font style selection and the
`discoverFallback` orchestration is cleanly scoped.

One **caveat** (folded into the test plan): the `None`-case test determinism —
macOS font fallback can vary by host/OS version, so the no-font test uses a
**noncharacter** (e.g. `U+FDD0`, a permanently-unassigned scalar no font covers)
to force the `LastResort` path, and stays adaptive (the positive cases prove the
CoreText call regardless).

Review artifacts:

- Prompt: `logs/codex-review/20260603-122831-868161-prompt.md`
- Result: `logs/codex-review/20260603-122831-868161-last-message.md`
