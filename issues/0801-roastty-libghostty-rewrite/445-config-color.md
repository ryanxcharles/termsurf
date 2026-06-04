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

# Experiment 445: the config Color type and its terminal-RGB conversion (Color, to_terminal_rgb)

## Description

This experiment ports the foundational config `Color` type — an RGB triple — and
its `to_terminal_rgb` conversion to the terminal-native `Rgb`. Upstream's config
`Color` is the leaf color value many config keys hold (`background`,
`foreground`, and, via `BoldColor` / `TerminalColor`, the cursor and selection
colors); its `toTerminalRGB` is the identity conversion to the terminal color
type. Landing it unblocks the later `BoldColor` / `TerminalColor` config types
(which wrap a `Color`). The string parsing (`parseCLI` / `fromHex` / X11 named
colors) and the C extern struct (`cval`) stay deferred.

## Upstream behavior

In `config/Config.zig`:

```zig
pub const Color = struct {
    r: u8,
    g: u8,
    b: u8,

    /// ghostty_config_color_s
    pub const C = extern struct { r: u8, g: u8, b: u8 };
    pub fn cval(self: Color) Color.C { return .{ .r = self.r, .g = self.g, .b = self.b }; }

    /// Convert this to the terminal RGB struct
    pub fn toTerminalRGB(self: Color) terminal.color.RGB {
        return .{ .r = self.r, .g = self.g, .b = self.b };
    }

    pub fn parseCLI(input_: ?[]const u8) !Color { ... x11 named colors, else fromHex ... }
    pub fn clone(self: Color, _: Allocator) error{}!Color { return self; }
    // ... equality, fromHex, formatEntry
};
```

`Color` is a plain `{ r, g, b }` byte triple. `toTerminalRGB` copies the three
channels into the terminal-native `terminal.color.RGB` (a field-for-field
identity). `parseCLI` resolves an X11 named color or a hex string; `cval`
produces the C extern struct for the public API.

## Rust mapping (`roastty/src/config/mod.rs`)

roastty's terminal `Rgb` (`crate::terminal::color::Rgb`) is the conversion
target:

```rust
/// A config color value (upstream `Config.Color`): an RGB byte triple. The
/// string parsing (named colors / hex) and the C extern struct are ported in
/// later slices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    /// Convert to the terminal-native `Rgb` (upstream `Color.toTerminalRGB`): a
    /// field-for-field copy of the three channels.
    pub(crate) fn to_terminal_rgb(self) -> Rgb {
        Rgb::new(self.r, self.g, self.b)
    }
}
```

`to_terminal_rgb` is upstream's `toTerminalRGB`: the three bytes copied into the
terminal `Rgb`. `Color` is `Copy`/`Eq` (a plain value).

## Scope / faithfulness notes

- **Ported (bridged)**: the config `Color` struct (`config/Config.zig`) and its
  `to_terminal_rgb` conversion (upstream `Color.toTerminalRGB`).
- **Faithful**: `Color` is the `{ r, g, b }` byte triple; `to_terminal_rgb`
  copies the three channels into the terminal-native `Rgb` field-for-field —
  exactly upstream's `toTerminalRGB`.
- **Faithful adaptation**: the conversion target is roastty's
  `crate::terminal::color::Rgb` (the analog of upstream `terminal.color.RGB`),
  built via its `Rgb::new(r, g, b)` constructor.
- **Deferred**: the string parsing (`parseCLI` / `fromHex` and the X11
  named-color map), the C extern struct (`cval` / `Color.C`, for the public C
  API), the `formatEntry`, and the `Config` struct that holds `Color`-typed
  keys. (Consumed by later slices — including `BoldColor` / `TerminalColor`,
  which wrap a `Color`; this experiment lands the value type and its terminal
  conversion.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add `pub(crate) struct Color { pub r: u8, pub g: u8, pub b: u8 }` (derive
     `Debug, Clone, Copy, PartialEq, Eq`) and
     `Color::to_terminal_rgb(self) -> Rgb`. Import `Rgb` from
     `crate::terminal::color`.
2. Tests (in `config/mod.rs`):
   - `to_terminal_rgb`:
     `Color { r: 10, g: 20, b: 30 }.to_terminal_rgb() == Rgb::new(10, 20, 30)`;
     a boundary case (`Color { 0, 128, 255 }`); the struct is `Copy`/`Eq` (a
     round-trip and an `assert_ne!` on a differing value).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty config_color
cargo test -p roastty to_terminal_rgb
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Color` is the `{ r, g, b }` byte triple and `to_terminal_rgb` copies the
  three channels into the terminal `Rgb` — faithful to upstream's `Color` and
  `toTerminalRGB`;
- the tests pass (the conversion; the boundary; `Copy`/`Eq`), and the existing
  tests still pass;
- the parsing, the C extern struct, and the `Config` struct stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if `to_terminal_rgb` reorders or drops a channel, the
struct is shaped wrong, an unrelated item changes, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It verified against the vendored upstream: `Color` is exactly three
`u8` channels and `toTerminalRGB` copies them field-for-field into
`terminal.color.RGB` (`Config.zig:5413` / `:5429`); roastty's `Rgb` has the same
three `u8` fields and `Rgb::new(r, g, b)` is the right constructor (`color.rs:1`
/ `:36`); deferring `parseCLI` / `fromHex` / X11 names / `cval` / formatting is
the right boundary (parsing and public C API concerns, while this slice only
needs the foundational value type plus the terminal conversion); `Copy` / `Eq`
is appropriate for a three-byte value type and matches upstream's cheap
clone/equality; and the planned tests (normal + boundary channel values plus
value semantics) are adequate.

Review artifacts:

- Prompt: `logs/codex-review/20260604-105704-d445-prompt.md` (design)
- Result: `logs/codex-review/20260604-105704-d445-last-message.md` (design)

## Result

**Result:** Pass

The config `Color` type and its terminal-RGB conversion are now live.

- `roastty/src/config/mod.rs`:
  `pub(crate) struct Color { pub r: u8, pub g: u8, pub b: u8 }` (upstream
  `Config.Color`) and `Color::to_terminal_rgb(self) -> Rgb`
  (`Rgb::new(self.r, self.g, self.b)`), the field-for-field port of upstream's
  `Color.toTerminalRGB`. Added `use crate::terminal::color::Rgb;`.

Test (in `config/mod.rs`): `config_color_converts_to_terminal_rgb` —
`Color { 10, 20, 30 }.to_terminal_rgb() == Rgb::new(10, 20, 30)`; the boundary
`Color { 0, 128, 255 } == Rgb::new(0, 128, 255)`; a `Copy`/`Eq` round-trip and
an `assert_ne!` on a differing value.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2933 passed, 0 failed (+1, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer + config +
  `lib.rs`/header/`abi_harness.c`) clean; `git diff --check` clean.

## Conclusion

The config layer now carries the foundational `Color` value type and its
terminal-RGB conversion — the first config _value_ type (vs. an enum) in the
module, and the building block the later `BoldColor` / `TerminalColor` config
types wrap. The string parsing (`parseCLI` / `fromHex` / X11 named colors), the
C extern struct (`cval` / `Color.C`), `formatEntry`, and the `Config` struct
stay deferred. With `Color` landed, a
`TerminalColor { Color, CellForeground, CellBackground }` (the renderer's cursor
/ selection color type) and `BoldColor` become natural next slices. The
config-type family remains a clean, gated way to advance the rewrite while the
larger coupled subsystems stay deferred.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed `Color { r, g, b }` faithfully ports upstream's
config RGB value type; `to_terminal_rgb()` is the direct field-for-field
`toTerminalRGB` mapping into roastty's `Rgb`; deferring parsing, formatting, and
the C ABI representation is the right scope; and the test covers normal values,
boundary channel values, and value semantics. No public C ABI/header impact;
nothing needed to change before the result commit.

Review artifacts:

- Prompt: `logs/codex-review/20260604-105859-r445-prompt.md` (result)
- Result: `logs/codex-review/20260604-105859-r445-last-message.md` (result)
