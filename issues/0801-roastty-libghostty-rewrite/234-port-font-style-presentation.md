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

# Experiment 234: Port Font `Style` and `Presentation` Enums

## Description

Continue the font subsystem (Experiment 233) by porting the two foundational
enums from upstream `font/main.zig`: `Style` (the font weight/slant) and
`Presentation` (text vs. emoji rendering of a codepoint). Both are leaf value
types used throughout the font stack — faces, the collection, the resolver, and
shaping all key off them.

### Types to port

Upstream `font/main.zig`:

```
pub const Style = enum(u3) {
    regular = 0,
    bold = 1,
    italic = 2,
    bold_italic = 3,
};

/// The presentation for an emoji.
pub const Presentation = enum(u1) {
    text = 0,   // U+FE0E
    emoji = 1,  // U+FE0F
};
```

The explicit discriminants matter: `Style` is a `u3` and `Presentation` a `u1`,
and their numeric values are part of the encoding (e.g. used as indices or
packed into other state by later font/collection code). The Rust port preserves
them with explicit discriminants and `#[repr(u8)]` (Rust has no `u3`/`u1`).

(Note: upstream's `Presentation.emoji` comment reads `U+FEOF`, an obvious typo
for the variation selector `U+FE0F`; the port uses the correct value in its
comment.)

### Scope and faithfulness notes

- `Style { Regular = 0, Bold = 1, Italic = 2, BoldItalic = 3 }` and
  `Presentation { Text = 0, Emoji = 1 }`, `#[repr(u8)]`, deriving
  `Debug, Clone, Copy, PartialEq, Eq`. No `Default` is added (upstream declares
  none).
- Placed in `roastty/src/font/mod.rs` (the font module root), mirroring
  upstream's `main.zig` placement of these module-level types.
- No face/collection/resolver/shaping behavior — only the two enums.
- No C ABI, header, or ABI inventory changes; no new dependencies.

## Changes

1. `roastty/src/font/mod.rs`:
   - Add
     `pub(crate) enum Style { Regular = 0, Bold = 1, Italic = 2, BoldItalic = 3 }`
     and `pub(crate) enum Presentation { Text = 0, Emoji = 1 }`, both
     `#[repr(u8)]` with `Debug, Clone, Copy, PartialEq, Eq`, and the upstream
     doc comments (with the `U+FE0F` correction).

2. Tests in `roastty/src/font/mod.rs` (a `#[cfg(test)] mod tests`):
   - `style_discriminants`: `Style::Regular as u8 == 0`, `Bold == 1`,
     `Italic == 2`, `BoldItalic == 3`.
   - `presentation_discriminants`: `Presentation::Text as u8 == 0`,
     `Emoji as u8 == 1`.

3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo test -p roastty font
cargo test -p roastty
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Style` and `Presentation` are ported with the exact variants and explicit
  discriminants (`0..3` and `0..1`);
- the discriminant tests pass;
- no face/collection/shaping scope is pulled in;
- no C ABI, header, or ABI inventory changes;
- `cargo fmt` accepted and `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if a later font slice shows these enums need an
associated method (e.g. a `from`/`to` mapping) that should be its own change.

The experiment **fails** if a discriminant diverges from upstream, if extra font
behavior leaks in, or if any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no issues**.

Review artifacts:

- Prompt: `logs/codex-review/20260602-081536-392278-prompt.md`
- Result: `logs/codex-review/20260602-081536-392278-last-message.md`

Codex confirmed the variants, order, and discriminants exactly match upstream
(`Style` 0–3, `Presentation` 0–1), that `#[repr(u8)]` with explicit
discriminants is the right Rust shape for the `u3`/`u1` encodings, that placing
them in `font/mod.rs` mirrors upstream `main.zig`, that omitting `Default` is
faithful, that the `U+FE0F` comment correction is documentation-only, and that
the two discriminant tests are adequate. No changes required.
