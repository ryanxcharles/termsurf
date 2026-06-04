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

# Experiment 448: the scroll-to-bottom config type (ScrollToBottom)

## Description

This experiment ports the `scroll-to-bottom` config type — `ScrollToBottom`, a
two-flag struct (`keystroke`, `output`) controlling when the viewport snaps to
the bottom. Its intrinsic field defaults (`keystroke = true`, `output = false`)
are meaningful (the `Config` field adopts them via `.default`), so the `Default`
impl is hand-written, like `FontShapingBreak` (Experiment 437). The renderer's
`DerivedConfig` reads its `output` flag
(`scroll_to_bottom_on_output = config.@"scroll-to-bottom".output`); the consumer
is plain field access, so this slice lands the value type with its defaults, and
the renderer wiring stays deferred.

## Upstream behavior

In `config/Config.zig`, the type and its `Config` field (default `.default`):

```zig
@"scroll-to-bottom": ScrollToBottom = .default,

pub const ScrollToBottom = packed struct {
    keystroke: bool = true,
    output: bool = false,

    pub const default: ScrollToBottom = .{};
};
```

In `renderer/generic.zig`'s `DerivedConfig.init`, the renderer reads the
`output` flag:

```zig
.scroll_to_bottom_on_output = config.@"scroll-to-bottom".output,
```

`ScrollToBottom` has two independent flags: `keystroke` (scroll to bottom on a
keystroke, default `true`) and `output` (scroll to bottom on new output, default
`false`). The `.default` constant is the struct with its field defaults. The
renderer consumes the `output` flag for its scroll-to-bottom-on-output feature.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
/// The `scroll-to-bottom` config (upstream `ScrollToBottom`): when the viewport
/// snaps to the bottom. `keystroke` (default `true`) snaps on a keystroke;
/// `output` (default `false`) snaps on new output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScrollToBottom {
    /// Scroll to the bottom on a keystroke.
    pub keystroke: bool,
    /// Scroll to the bottom on new output.
    pub output: bool,
}

impl Default for ScrollToBottom {
    /// Upstream's field defaults `keystroke = true`, `output = false`.
    fn default() -> Self {
        Self {
            keystroke: true,
            output: false,
        }
    }
}
```

The hand-written `Default` matches upstream's field defaults
(`keystroke = true`, `output = false`); a derived `Default` would make
`keystroke` `false`. The two flags are independent `bool`s. The renderer reads
`output` directly (no method).

## Scope / faithfulness notes

- **Ported (bridged)**: the `ScrollToBottom` config type (`config/Config.zig`),
  with its intrinsic field defaults.
- **Faithful**: the struct has the two upstream flags (`keystroke`, `output`);
  the `Default` is `keystroke = true`, `output = false` (upstream's field
  defaults, the `.default` constant).
- **Faithful adaptation**: upstream is a `packed struct` (bit-packed storage);
  in Rust it is a plain value struct (no ABI involved — internal config), so a
  derived layout is fine. The `Default` is hand-written because Rust's derived
  `Default` for `bool` is `false`, not upstream's `keystroke = true`. No method
  is extracted — the renderer's consumer is plain `.output` field access.
- **Deferred**: the string parsing, the `formatEntry`, the `Config` struct that
  holds the `scroll-to-bottom` key, and the renderer's `DerivedConfig` wiring
  (`scroll_to_bottom_on_output = config.@"scroll-to-bottom".output`) and the
  keystroke consumer. (Consumed by a later slice; this experiment lands the
  value type and its defaults.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add
     `pub(crate) struct ScrollToBottom { pub keystroke: bool, pub output: bool }`
     (derive `Debug, Clone, Copy, PartialEq, Eq`) and a hand-written
     `impl Default` (`keystroke: true`, `output: false`).
2. Tests (in `config/mod.rs`):
   - `ScrollToBottom::default()` has `keystroke == true`, `output == false`; a
     `{ keystroke: false, output: true }` value differs from the default and
     round-trips `Copy`/`Eq`; the two flags are independent (a value differing
     only in `output` is `!=`).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty scroll_to_bottom
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `ScrollToBottom` has the two upstream flags and the `Default` is
  `keystroke = true`, `output = false` — faithful to upstream's type and field
  defaults;
- the tests pass (the default; the independent flags; `Copy`/`Eq`), and the
  existing tests still pass;
- the parsing, the `Config` struct, and the renderer wiring stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a flag is missing/extra, the `Default` is wrong
(e.g. `keystroke = false`), an unrelated item changes, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It verified against the vendored upstream:
`ScrollToBottom { keystroke, output }` matches the two-field packed struct
(`Config.zig:10206`); the hand-written `Default` is required and exact
(`keystroke = true`, `output = false`, matching `.default = .{}` with the field
defaults, `Config.zig:10207`); the `Config`-field default is correctly
documented as `.default` (`Config.zig:938`); not extracting a method is the
right call (the renderer consumer directly reads `.output`, `generic.zig:646`);
and the tests cover the non-derived default, value semantics, and flag
independence.

Review artifacts:

- Prompt: `logs/codex-review/20260604-110920-d448-prompt.md` (design)
- Result: `logs/codex-review/20260604-110920-d448-last-message.md` (design)
