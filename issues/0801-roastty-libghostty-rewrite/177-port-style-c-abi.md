+++
[implementer]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 177: Port Style C ABI

## Description

Port the small style C ABI from upstream:

- `vendor/ghostty/src/terminal/c/style.zig`.

Experiment 176 added packed row/cell value getters. The next obvious consumer is
render-state row/cell iteration, but upstream render row-cell data includes a
`style` selector whose output type is the C style struct from `style.zig`.
Roastty currently has internal style values but no public style C struct. Port
that ABI first so the later render-state experiment can expose style data
without inventing a placeholder or leaving a required render selector partial.

Public names must use Roastty naming only:

- `roastty_style_color_*`;
- `roastty_style_s`;
- `roastty_style_default`;
- `roastty_style_is_default`.

Upstream names may appear only in this issue document as source citations.

## Changes

1. Re-read upstream source:
   - `vendor/ghostty/src/terminal/c/style.zig`;
   - `roastty/src/terminal/style.rs`;
   - `roastty/src/terminal/sgr.rs`;
   - `roastty/src/terminal/color.rs`.

2. Add public style C types in `roastty/include/roastty.h` and mirrored Rust
   `repr(C)` types in `roastty/src/lib.rs`.
   - `roastty_style_color_tag_e`:
     - `ROASTTY_STYLE_COLOR_NONE = 0`;
     - `ROASTTY_STYLE_COLOR_PALETTE = 1`;
     - `ROASTTY_STYLE_COLOR_RGB = 2`.
   - `roastty_style_color_value_u`:
     - `uint8_t palette`;
     - `roastty_rgb_s rgb`;
     - `uint64_t _padding`.
   - `roastty_style_color_s`:
     - `roastty_style_color_tag_e tag`;
     - `roastty_style_color_value_u value`.
   - `roastty_style_s`:
     - `size_t size`;
     - `roastty_style_color_s fg_color`;
     - `roastty_style_color_s bg_color`;
     - `roastty_style_color_s underline_color`;
     - `bool bold`;
     - `bool italic`;
     - `bool faint`;
     - `bool blink`;
     - `bool inverse`;
     - `bool invisible`;
     - `bool strikethrough`;
     - `bool overline`;
     - `int underline`.

3. Preserve the upstream macOS C ABI layout:
   - `sizeof(roastty_style_color_value_u) == 8`;
   - `_Alignof(roastty_style_color_value_u) == 8`;
   - `sizeof(roastty_style_color_s) == 16`;
   - `_Alignof(roastty_style_color_s) == 8`;
   - `offsetof(roastty_style_color_s, tag) == 0`;
   - `offsetof(roastty_style_color_s, value) == 8`;
   - `sizeof(roastty_style_s) == 72`;
   - `_Alignof(roastty_style_s) == 8`;
   - `offsetof(roastty_style_s, size) == 0`;
   - `offsetof(roastty_style_s, fg_color) == 8`;
   - `offsetof(roastty_style_s, bg_color) == 24`;
   - `offsetof(roastty_style_s, underline_color) == 40`;
   - `offsetof(roastty_style_s, bold) == 56`;
   - `offsetof(roastty_style_s, italic) == 57`;
   - `offsetof(roastty_style_s, faint) == 58`;
   - `offsetof(roastty_style_s, blink) == 59`;
   - `offsetof(roastty_style_s, inverse) == 60`;
   - `offsetof(roastty_style_s, invisible) == 61`;
   - `offsetof(roastty_style_s, strikethrough) == 62`;
   - `offsetof(roastty_style_s, overline) == 63`;
   - `offsetof(roastty_style_s, underline) == 64`;
   - `roastty_style_s.size` is initialized to `sizeof(roastty_style_s)`.

4. Add conversion helpers inside Rust:
   - internal `style::Color::None` -> `ROASTTY_STYLE_COLOR_NONE`;
   - internal `style::Color::Palette(idx)` -> `ROASTTY_STYLE_COLOR_PALETTE` with
     `palette = idx`;
   - internal `style::Color::Rgb(rgb)` -> `ROASTTY_STYLE_COLOR_RGB` with `rgb`;
   - internal `style::Style` -> `roastty_style_s`;
   - underline flag maps to the upstream numeric SGR underline values already
     used by internal style formatting:

     | Internal underline | C `underline` value |
     | ------------------ | ------------------- |
     | `None`             | 0                   |
     | `Single`           | 1                   |
     | `Double`           | 2                   |
     | `Curly`            | 3                   |
     | `Dotted`           | 4                   |
     | `Dashed`           | 5                   |

5. Add public functions:

   ```c
   ROASTTY_API void roastty_style_default(roastty_style_s*);
   ROASTTY_API bool roastty_style_is_default(const roastty_style_s*);
   ```

   Behavior:
   - `roastty_style_default(NULL)` is a no-op;
   - `roastty_style_default(out)` writes the default style and sets `out->size`;
   - `roastty_style_is_default(NULL)` returns `false`;
   - `roastty_style_is_default(style)` validates that
     `style->size == sizeof(roastty_style_s)` and returns `false` for a size
     mismatch instead of asserting or panicking across the C ABI;
   - `roastty_style_is_default(style)` returns true only when all color tags are
     `NONE`, all boolean flags are false, and `underline == 0`.

   The null/size behavior intentionally hardens the upstream assert-based helper
   for a Rust C ABI boundary. It must be documented in the result.

6. Add tests in `roastty/src/lib.rs` for:
   - enum numeric values;
   - `repr(C)` size, alignment, and every field offset listed in Changes 3;
   - default style writes expected tags, flags, underline, and size;
   - `roastty_style_is_default` accepts the default style;
   - non-default foreground palette, background RGB, underline RGB, and every
     boolean flag make `roastty_style_is_default` false;
   - underline variants produce the expected numeric value;
   - null and size-mismatch behavior.

7. Add C harness coverage in `roastty/tests/abi_harness.c` for:
   - compile/link coverage for the new types and functions;
   - enum numeric values;
   - `sizeof`, `_Alignof`, and key `offsetof` checks;
   - `roastty_style_default`;
   - `roastty_style_is_default` true and false cases;
   - null and size-mismatch behavior.

8. Keep scope narrow:
   - Do not add render state, row iterators, or row-cell iterators.
   - Do not add formatter objects.
   - Do not add style lookup from page storage.
   - Do not add style mutation APIs.
   - Do not add `ghostty_*` symbols or compatibility aliases.

## Verification

Run:

```bash
cargo fmt -- roastty/src/lib.rs
cargo test -p roastty style_c_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

Pass criteria:

- public style C types match the upstream macOS layout with Roastty names;
- default conversion accurately reflects internal `style::Style::default()`;
- non-default color, flag, and underline fields are visible through the C
  struct;
- null and size mismatch behavior is defined and tested;
- C header, Rust ABI tests, C harness, full `roastty` tests, no-Ghostty check,
  and `git diff --check` all pass;
- Codex reviews and approves both the design and completed result.

Partial criteria:

- layout and default-style functions pass, but converting one internal
  non-default style field requires a small internal accessor added in a
  follow-up before render-state style output can use it.

Failure criteria:

- the API exposes `ghostty_*` symbols or compatibility aliases;
- the C layout drifts from upstream without a documented macOS ABI reason;
- `roastty_style_is_default` can panic or assert across the C ABI boundary;
- the implementation expands into render state, formatter objects, page style
  lookup, or style mutation.

## Review

This design must be reviewed with the Codex review skill before implementation.
All real findings must be fixed, and the design must be re-reviewed until Codex
approves it.

After implementation and result recording, the completed result must also be
reviewed with Codex and approved before the result commit.

## Result

**Result:** Pass

Implemented the Roastty style C ABI:

- added `roastty_style_color_tag_e`, `roastty_style_color_value_u`,
  `roastty_style_color_s`, and `roastty_style_s`;
- added `roastty_style_default` and `roastty_style_is_default`;
- added internal conversion from `terminal::style::Style` into the public C
  style struct;
- widened `terminal::color`, `terminal::sgr`, and `terminal::style` visibility
  to `pub(crate)` where needed so the crate-level C ABI can convert internal
  style values without exposing public Rust API;
- added Rust ABI tests for enum values, size/alignment, every required offset,
  default conversion, non-default palette/RGB/flag/underline conversion, null
  handling, and size mismatch handling;
- added C harness coverage for compile/link, enum values, layout, default
  behavior, false cases, null handling, and size mismatch handling.

The implementation intentionally hardens upstream's assert-based
`style_is_default` helper for the Rust C ABI boundary:

- `roastty_style_default(NULL)` is a no-op;
- `roastty_style_is_default(NULL)` returns `false`;
- `roastty_style_is_default` returns `false` for a `size` mismatch instead of
  panicking or asserting across the C boundary.

Verification passed:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/mod.rs roastty/src/terminal/color.rs roastty/src/terminal/sgr.rs roastty/src/terminal/style.rs
cargo test -p roastty style_c_abi
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The full `roastty` test run passed with 1856 Rust tests, the C ABI harness, and
doc tests.

## Conclusion

Roastty now has the public style C struct that upstream render row-cell data
uses for its `style` selector. This removes the main dependency that would have
forced a partial render-state ABI port after Experiment 176.

The next experiment can target render-state C ABI with the style output type
available, while still keeping formatter objects and Kitty graphics separate.
