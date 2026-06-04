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

# Experiment 467: grow the Config struct with the optional-colors group

## Description

Continuing the incremental growth of the aggregating `Config` struct
(Experiments 461–466), this experiment adds the **optional-colors** group:
`cursor_color`, `cursor_text`, `selection_foreground`, `selection_background`
(each `Option<TerminalColor>`), and `bold_color` (`Option<BoldColor>`). These
reuse the already-ported color value types (`TerminalColor` from Experiment 446,
`BoldColor` from Experiment 447) — the first time the `Config` aggregate uses
the color value types it was built toward. All five default to `None` (upstream
`null`). The parser and the rest of upstream `Config` stay deferred.

## Upstream behavior

In `config/Config.zig`, the optional-colors group's field defaults (all `null`):

```zig
@"selection-foreground": ?TerminalColor = null,
@"selection-background": ?TerminalColor = null,
@"cursor-color": ?TerminalColor = null,
@"cursor-text": ?TerminalColor = null,
@"bold-color": ?BoldColor = null,
```

Each is an optional color that defaults to `null` (unset): when `null`, the
consumer uses a fallback (e.g. the cursor falls back to the inverse cell color,
the selection to the theme's selection colors). The renderer's `DerivedConfig`
reads these (`cursor_color`, `cursor_text`, `selection_*`, `bold_color`) and
resolves each via `toTerminalRGB` / `toTerminal` when present.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
pub(crate) struct Config {
    // ... clipboard (461) … background-image (466) ...
    /// `cursor-color`.
    pub cursor_color: Option<TerminalColor>,
    /// `cursor-text`.
    pub cursor_text: Option<TerminalColor>,
    /// `selection-foreground`.
    pub selection_foreground: Option<TerminalColor>,
    /// `selection-background`.
    pub selection_background: Option<TerminalColor>,
    /// `bold-color`.
    pub bold_color: Option<BoldColor>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // ... earlier groups ...
            cursor_color: None,
            cursor_text: None,
            selection_foreground: None,
            selection_background: None,
            bold_color: None,
        }
    }
}
```

The defaults are upstream's Config-field defaults: all five are `None` (upstream
`null`). The fields use the already-ported `TerminalColor` / `BoldColor` value
types, wrapped in `Option` (upstream's `?`).

## Scope / faithfulness notes

- **Ported (bridged)**: the optional-colors field group of the aggregating
  `Config` struct (upstream `config.Config`) — the five fields and their
  `Default`.
- **Faithful**: the four `?TerminalColor` fields and the one `?BoldColor` field
  use the already-ported value types wrapped in `Option`; all five `Default`
  values are `None` (upstream `null`).
- **Faithful adaptation**: upstream's `?T` maps to `Option<T>`; the `null`
  default maps to `None`. `Option<TerminalColor>` / `Option<BoldColor>` are
  `Clone`/`PartialEq` (the value types are, and `TerminalColor` is `Copy` so its
  `Option` is too; `BoldColor` is `Copy`). The struct continues to grow one
  coherent field group per experiment; the derive set is unchanged.
- **Deferred**: the rest of upstream `Config`'s fields (added group by group in
  later slices), the parser, the `changeConfig` machinery, the
  conditional-config system, and the renderer `DerivedConfig` resolution of
  these optional colors (`toTerminalRGB` / `toTerminal` when present, the `None`
  fallback). (Consumed by later slices; this experiment grows the struct with
  the optional-colors group.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add the five fields `cursor_color: Option<TerminalColor>`,
     `cursor_text: Option<TerminalColor>`,
     `selection_foreground: Option<TerminalColor>`,
     `selection_background: Option<TerminalColor>`,
     `bold_color: Option<BoldColor>` to `Config`, and their defaults (all
     `None`) to the `Default` impl.
2. Tests (in `config/mod.rs`):
   - extend the `Config::default()` assertion for the new fields: all five are
     `None` (`cursor_color`, `cursor_text`, `selection_foreground`,
     `selection_background`, `bold_color`); the existing group defaults still
     hold.
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

- `Config` gains the five optional-colors fields, and `Config::default()` sets
  them all to `None` (upstream `null`) while the earlier group defaults still
  hold — a faithful partial of upstream's `Config`;
- the tests pass (the new `None` defaults; the existing defaults), and the
  existing tests still pass;
- the rest of upstream `Config`, the parser, and the renderer resolution stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a default is not `None`, a field uses the wrong type
(e.g. not wrapped in `Option`), an unrelated item changes, or any public C
API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It verified against the vendored upstream: all five fields are
optional and default to `null`, so `Option<...> = None` is the right Rust
mapping (`selection_foreground` `Config.zig:707`; `selection_background` `:708`;
`cursor_color` `:851`; `cursor_text` `:902`; `bold_color` `:3709`); the types
are correct (the four cursor/selection fields `Option<TerminalColor>`,
bold-color `Option<BoldColor>`); reusing the already-ported `TerminalColor` /
`BoldColor` value types is the right boundary (renderer/`DerivedConfig`
resolution belongs in a later slice); the optional-colors group is coherent; and
the test plan is adequate (assert all five new defaults are `None` and keep the
existing `Config` defaults covered).

Review artifacts:

- Prompt: `logs/codex-review/20260604-122706-d467-prompt.md` (design)
- Result: `logs/codex-review/20260604-122706-d467-last-message.md` (design)
