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

# Experiment 540: the keyboard layout

## Description

Continuing the input-layer port (Experiment 539 ported `input/mouse.zig`), this
experiment ports upstream `input/keyboard.zig` — the **keyboard layout** type —
into a new `roastty::input::keyboard` module. `Layout` is a small enum that
distinguishes the few keyboard layouts roastty needs in order to pick a sensible
default for `macos-option-as-alt`: it maps an Apple keyboard-layout ID to a
`Layout` (`map_apple_id`) and derives the default `OptionAsAlt` for a layout
(`detect_option_as_alt`). The file is self-contained apart from `OptionAsAlt`,
which roastty already has at `input::key_mods::OptionAsAlt`.

## Upstream behavior

`input/keyboard.zig`:

```zig
const OptionAsAlt = @import("config.zig").OptionAsAlt;

/// Keyboard layouts. Not heavily used; we only need to distinguish a few layouts
/// for nice-to-have features like the default for "macos-option-as-alt".
pub const Layout = enum {
    unknown,          // unmapped layout; make no assumptions
    us_standard,
    us_international,

    /// Map an Apple keyboard layout ID (from Carbon's
    /// TIKeyboardLayoutGetInputSourceProperty) to a value in this enum, or null if
    /// unrecognized (so callers can detect that scenario).
    pub fn mapAppleId(id: []const u8) ?Layout {
        if (std.mem.eql(u8, id, "com.apple.keylayout.US")) return .us_standard
        else if (std.mem.eql(u8, id, "com.apple.keylayout.USInternational")) return .us_international;
        return null;
    }

    /// The default macos-option-as-alt value for this layout. On US layouts the option
    /// key is typically wanted as alt (option-B should be alt-B, not "∫"); on unknown
    /// layouts we make no assumption.
    pub fn detectOptionAsAlt(self: Layout) OptionAsAlt {
        return switch (self) {
            .us_standard, .us_international => .true,
            .unknown => .false,
        };
    }
};
```

- `mapAppleId` recognizes exactly two Apple layout IDs (`com.apple.keylayout.US`
  ⇒ `us_standard`, `com.apple.keylayout.USInternational` ⇒ `us_international`)
  and returns `null` for anything else (including layouts that would map to
  `unknown`) so callers can tell a recognized-but-unmapped ID from an
  unrecognized one.
- `detectOptionAsAlt` returns `.true` for the US layouts (treat option as alt)
  and `.false` for `unknown`.

## Rust mapping (`roastty/src/input/keyboard.rs`)

A plain enum plus the two methods; the Apple-ID string compares become a `match`
on `&str`, and `OptionAsAlt` is reused from `input::key_mods`:

```rust
//! Keyboard layout (port of upstream `input/keyboard`).

use crate::input::key_mods::OptionAsAlt;

/// Keyboard layouts. These aren't heavily used; roastty only needs to distinguish a few
/// layouts for nice-to-have features such as the default for `macos-option-as-alt`
/// (upstream `input.keyboard.Layout`).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Layout {
    /// Unknown, unmapped layout; make no assumptions about the keyboard layout.
    #[default]
    Unknown,
    UsStandard,
    UsInternational,
}

impl Layout {
    /// Map an Apple keyboard-layout ID (from Carbon's
    /// `TIKeyboardLayoutGetInputSourceProperty`) to a `Layout`, or `None` if the ID is
    /// unrecognized — so callers can detect that scenario.
    pub(crate) fn map_apple_id(id: &str) -> Option<Layout> {
        match id {
            "com.apple.keylayout.US" => Some(Layout::UsStandard),
            "com.apple.keylayout.USInternational" => Some(Layout::UsInternational),
            _ => None,
        }
    }

    /// The default `macos-option-as-alt` value for this layout. On US layouts the option
    /// key is typically wanted as alt (option-B ⇒ alt-B, not "∫"); on an unknown layout
    /// make no assumption.
    pub(crate) fn detect_option_as_alt(self) -> OptionAsAlt {
        match self {
            Layout::UsStandard | Layout::UsInternational => OptionAsAlt::True,
            Layout::Unknown => OptionAsAlt::False,
        }
    }
}
```

`#[default] Unknown` matches upstream's "unknown ⇒ make no assumptions" zero
state (the same convenience as `input::mouse::Momentum`'s `#[default] None`).
The upstream import of `OptionAsAlt` from `config.zig` is satisfied by roastty's
existing `input::key_mods::OptionAsAlt` (same type, different home).

## Scope / faithfulness notes

- **Ported (bridged)**: `input.keyboard.Layout` (`unknown` / `us_standard` /
  `us_international`) with `map_apple_id` and `detect_option_as_alt`, into
  `input::keyboard`.
- **Faithful**: the exact variant set; `map_apple_id` recognizing exactly the
  two Apple IDs (and `None` otherwise); `detect_option_as_alt` ⇒ `True` for US
  layouts, `False` for `Unknown`.
- **Faithful adaptation**: `std.mem.eql` string compares → a `match` on `&str`;
  the `config.zig` `OptionAsAlt` import → roastty's
  `input::key_mods::OptionAsAlt`; the natural `unknown` zero-state →
  `#[default] Unknown`.
- **Deferred**: the macOS Carbon call (`TIKeyboardLayoutGetInputSourceProperty`)
  that produces the layout ID at runtime, and wiring `detect_option_as_alt` into
  config defaulting — both are consumer/frontend glue.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/input/keyboard.rs` (new): `Layout` + `map_apple_id` +
   `detect_option_as_alt`.
2. `roastty/src/input/mod.rs`: add `pub(crate) mod keyboard;`.
3. Tests (in `keyboard.rs`):
   - **map_apple_id**: `"com.apple.keylayout.US"` ⇒ `Some(UsStandard)`;
     `"com.apple.keylayout.USInternational"` ⇒ `Some(UsInternational)`; an
     unrecognized ID (`"com.apple.keylayout.German"`) and `""` ⇒ `None`.
   - **detect_option_as_alt**: `UsStandard` ⇒ `True`; `UsInternational` ⇒
     `True`; `Unknown` ⇒ `False`.
   - **default**: `Layout::default() == Layout::Unknown`.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty keyboard
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/input/keyboard.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `input::keyboard::Layout` has the exact upstream variant set, `map_apple_id`
  recognizes exactly the two Apple IDs (else `None`), and `detect_option_as_alt`
  returns `True` for the US layouts and `False` for `Unknown` — faithful to
  `input/keyboard.zig`;
- the tests pass (map / detect / default), and the existing tests still pass;
- the Carbon runtime call and config-defaulting wiring stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the variant set, the ID mapping, or the
option-as-alt defaulting diverges from upstream, an unrelated item changes, or
any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. The design matches upstream `keyboard.zig`: `Layout` has exactly
`unknown` / `us_standard` / `us_international`; `map_apple_id` recognizes only
`com.apple.keylayout.US` and `com.apple.keylayout.USInternational`, returning
`None` for everything else; and `detect_option_as_alt` maps both US layouts to
`True` and `Unknown` to `False`. Reusing roastty's existing
`input::key_mods::OptionAsAlt` is the right adaptation (`True` / `False`
correspond directly to upstream `.true` / `.false`), and keeping the Carbon
detection / config-default wiring deferred is properly scoped.
`#[default] Unknown` is acceptable as a Rust convenience for the natural "make
no assumptions" state, even though upstream declares no explicit default.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d540-prompt.md` (design)
- Result: `logs/codex-review/20260604-d540-last-message.md` (design)
