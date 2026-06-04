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

# Experiment 472: grow the Config struct with the base-colors group

## Description

Continuing the incremental growth of the aggregating `Config` struct
(Experiments 461–471), this experiment adds the **base-colors** group:
`background` and `foreground` (each `Color`, with concrete default RGB values)
and `theme` (`Option<Theme>`). It is the first group to use the base `Color`
value type (Experiment 445) directly and the `Theme` value type (Experiment 459)
— landing the last two ported config value types into `Config`. It adds the
three fields and their upstream `Config`-field defaults to `Config` and its
`Default`. The parser and the rest of upstream `Config` stay deferred.

## Upstream behavior

In `config/Config.zig`, the base-colors group's field defaults:

```zig
theme: ?Theme = null,
background: Color = .{ .r = 0x28, .g = 0x2C, .b = 0x34 },
foreground: Color = .{ .r = 0xFF, .g = 0xFF, .b = 0xFF },
```

`theme` defaults to `null` (no theme); `background` defaults to the RGB
`{ 0x28, 0x2C, 0x34 }` (a dark slate); `foreground` defaults to
`{ 0xFF, 0xFF, 0xFF }` (white).

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
pub(crate) struct Config {
    // ... clipboard (461) … terminal/render-behavior (471) ...
    /// `background`.
    pub background: Color,
    /// `foreground`.
    pub foreground: Color,
    /// `theme`.
    pub theme: Option<Theme>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // ... earlier groups ...
            background: Color {
                r: 0x28,
                g: 0x2C,
                b: 0x34,
            },
            foreground: Color {
                r: 0xFF,
                g: 0xFF,
                b: 0xFF,
            },
            theme: None,
        }
    }
}
```

The defaults are upstream's Config-field defaults: `background`
`{ 0x28, 0x2C, 0x34 }`, `foreground` `{ 0xFF, 0xFF, 0xFF }`, `theme` `None`
(upstream `null`).

## Scope / faithfulness notes

- **Ported (bridged)**: the base-colors field group of the aggregating `Config`
  struct (upstream `config.Config`) — the three fields and their `Default`.
- **Faithful**: `background` / `foreground` use the already-ported `Color` value
  type (Experiment 445) with the exact upstream default RGB triples; `theme`
  uses `Option<Theme>` (the `Theme` value type from Experiment 459, wrapped in
  `Option`) defaulting to `None` (upstream `null`).
- **Faithful adaptation**: upstream's `?Theme` maps to `Option<Theme>`, `null`
  to `None`; the `Color` literal `{ .r = …, .g = …, .b = … }` maps to the Rust
  `Color { r: …, g: …, b: … }`. `Theme` is `String`-backed and not `Copy`, so
  `Config` continues to derive `Clone`/`PartialEq` (not `Copy`). The struct
  continues to grow one coherent field group per experiment.
- **Deferred**: the rest of upstream `Config`'s fields (added group by group in
  later slices), the parser (including `loadTheme`), the `changeConfig`
  machinery, and the conditional-config system. (Consumed by later slices; this
  experiment grows the struct with the base-colors group.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add the three fields `background: Color`, `foreground: Color`,
     `theme: Option<Theme>` to `Config`, and their defaults
     (`{ 0x28, 0x2C, 0x34 }`, `{ 0xFF, 0xFF, 0xFF }`, `None`) to the `Default`
     impl.
2. Tests (in `config/mod.rs`):
   - extend the `Config::default()` assertion for the new fields:
     `background == Color { r: 0x28, g: 0x2C, b: 0x34 }`,
     `foreground == Color { r: 0xFF, g: 0xFF, b: 0xFF }`, `theme == None`; the
     existing group defaults still hold.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty config_default
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Config` gains the three base-colors fields, and `Config::default()` sets
  their upstream defaults (`background` `{ 0x28, 0x2C, 0x34 }`, `foreground`
  `{ 0xFF, 0xFF, 0xFF }`, `theme` `None`) while the earlier group defaults still
  hold — a faithful partial of upstream's `Config`;
- the tests pass (the new defaults; the existing defaults), and the existing
  tests still pass;
- the rest of upstream `Config` and the parser stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a default RGB is wrong, a field uses the wrong type
(e.g. `theme` not wrapped in `Option`), an unrelated item changes, or any public
C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It verified against the vendored upstream that the defaults are
correct (`theme = None`, mapping `?Theme = null`, `Config.zig:593`;
`background = Color { r: 0x28, g: 0x2C, b: 0x34 }`, `Config.zig:597`;
`foreground = Color { r: 0xFF, g: 0xFF, b: 0xFF }`, `Config.zig:601`);
`Option<Theme>` is the right Rust mapping for `?Theme` and `Color` for the RGB
literals; the base-colors group is coherent (theme plus default
foreground/background colors); and the test plan is adequate (assert these three
defaults and keep the existing groups covered as `Default` grows).

Review artifacts:

- Prompt: `logs/codex-review/20260604-124647-d472-prompt.md` (design)
- Result: `logs/codex-review/20260604-124647-d472-last-message.md` (design)
