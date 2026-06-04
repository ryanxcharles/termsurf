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

# Experiment 476: the config TerminalColor CLI parser (TerminalColor::parse_cli)

## Description

With the config `Color` fully bridged (`from_hex` / `parse_cli` / `format_buf`,
Experiments 473–475), this experiment ports the next value type that builds
directly on it: `TerminalColor.parseCLI`. A `TerminalColor` is either the two
cell-relative sentinels (`cell-foreground` / `cell-background`) or an explicit
`Color`; its parser checks the two sentinel keywords and otherwise delegates to
`Color::parse_cli`. The `formatEntry` side (which depends on the not-yet-ported
config `EntryFormatter`) stays deferred.

## Upstream behavior

In `config/Config.zig`, `TerminalColor.parseCLI`:

```zig
pub const TerminalColor = union(enum) {
    color: Color,
    @"cell-foreground",
    @"cell-background",

    pub fn parseCLI(input_: ?[]const u8) !TerminalColor {
        const input = input_ orelse return error.ValueRequired;
        if (std.mem.eql(u8, input, "cell-foreground")) return .@"cell-foreground";
        if (std.mem.eql(u8, input, "cell-background")) return .@"cell-background";
        return .{ .color = try Color.parseCLI(input) };
    }
    // ...
};
```

A missing value is `error.ValueRequired`. The raw (un-trimmed) input is compared
exactly (`std.mem.eql`) against `cell-foreground` and `cell-background`; a match
yields that sentinel. Otherwise the input is handed to `Color.parseCLI` (which
does its own whitespace trim, X11 name lookup, and hex fallback) and wrapped in
`.color`. A value that is neither sentinel nor a valid color propagates
`Color.parseCLI`'s `error.InvalidValue`. Upstream's tests: `"#4e2a84"` →
`color {78,42,132}`; `"black"` → `color {0,0,0}`; `"cell-foreground"` /
`"cell-background"` → the sentinels; `"a"` → `error.InvalidValue`.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
impl TerminalColor {
    /// Parse a config terminal-color value (upstream `TerminalColor.parseCLI`):
    /// the keywords `cell-foreground` / `cell-background` yield the cell
    /// sentinels (exact match on the raw input); anything else delegates to
    /// [`Color::parse_cli`]. A missing value is `ColorParseError::ValueRequired`.
    pub(crate) fn parse_cli(input: Option<&str>) -> Result<TerminalColor, ColorParseError> {
        let input = input.ok_or(ColorParseError::ValueRequired)?;
        if input == "cell-foreground" {
            return Ok(TerminalColor::CellForeground);
        }
        if input == "cell-background" {
            return Ok(TerminalColor::CellBackground);
        }
        Ok(TerminalColor::Color(Color::parse_cli(Some(input))?))
    }
}
```

`parse_cli` mirrors upstream: the `ValueRequired` guard, the two exact sentinel
keyword checks on the raw input (no trim — upstream uses `std.mem.eql`), and the
delegation to `Color::parse_cli` for everything else (which is where the trim,
X11 lookup, and hex fallback live). The error type is the shared
`ColorParseError` (`ValueRequired` here, `Invalid` propagated from
`Color::parse_cli`).

## Scope / faithfulness notes

- **Ported (bridged)**: the config `TerminalColor` CLI parser
  (`TerminalColor::parse_cli`, upstream `TerminalColor.parseCLI`).
- **Faithful**: the `ValueRequired` guard on a missing value; the exact
  (un-trimmed) `cell-foreground` / `cell-background` keyword checks before the
  color path; the delegation to `Color::parse_cli` for the explicit-color case,
  including the propagated `Invalid` — exactly upstream's `parseCLI`.
- **Faithful adaptation**: `?[]const u8` maps to `Option<&str>`;
  `std.mem.eql(u8, input, "...")` maps to `input == "..."`; the shared
  `ColorParseError` carries both `ValueRequired` and `Invalid`.
- **Faithful re-use**: the explicit-color path reuses the already-ported
  `Color::parse_cli` (Experiment 474), so the trim / X11 lookup / hex behavior
  is shared, not duplicated.
- **Deferred**: `TerminalColor.formatEntry` (delegates to `Color.formatEntry` /
  writes the sentinel `@tagName`; depends on the not-yet-ported config
  `EntryFormatter`), and the broader config parser/formatter (`loadCli` /
  per-field dispatch / file loading). (Consumed by later slices; this experiment
  lands the value parser.) `TerminalColor::to_terminal_rgb` is already ported.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add
     `TerminalColor::parse_cli(input: Option<&str>) -> Result<TerminalColor, ColorParseError>`.
2. Tests (in `config/mod.rs`):
   - mirror upstream's `parseCLI` test: `parse_cli(Some("#4e2a84"))` →
     `Color(Color{78,42,132})`; `parse_cli(Some("black"))` →
     `Color(Color{0,0,0})`; `parse_cli(Some("cell-foreground"))` →
     `CellForeground`; `parse_cli(Some("cell-background"))` → `CellBackground`;
     `parse_cli(Some("a"))` → `Err(Invalid)`; plus a missing value (`None` →
     `Err(ValueRequired)`); and a whitespace-padded sentinel
     (`parse_cli(Some(" cell-foreground"))` → `Err(Invalid)`) confirming the
     sentinel match is exact/un-trimmed and falls through to `Color::parse_cli`
     (folded in from the design review).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty terminal_color_parse_cli
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `TerminalColor::parse_cli` returns the cell sentinels for the exact
  `cell-foreground` / `cell-background` keywords and otherwise delegates to
  `Color::parse_cli`, returning `ColorParseError::ValueRequired` on a missing
  value — faithful to upstream's `parseCLI`;
- the tests pass (the upstream cases; the missing-value error), and the existing
  tests still pass;
- `TerminalColor.formatEntry` and the broader config parser/formatter stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a sentinel keyword or a color is parsed wrong, a
missing value does not error, the sentinel check is incorrectly trimmed or
loosened, an unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with one
**Low** finding and no Required/Recommended findings. It verified against the
vendored upstream (`Config.zig:5554`): `None` → `ValueRequired` matches
`input_ orelse error.ValueRequired`; the exact un-trimmed sentinel checks for
`cell-foreground` / `cell-background` match upstream (`:5556`); delegating all
other inputs to `Color::parse_cli(Some(input))` correctly preserves named
colors, hex parsing, and `Invalid` propagation; the planned success/error cases
are adequate (`:5579`); and deferring `formatEntry` is the right scope (it
depends on the config formatter abstraction).

- **Low (folded in):** add a test for the raw sentinel subtlety — a
  whitespace-padded `" cell-foreground"` must not match the sentinel and should
  fall through to `Color::parse_cli`, producing `Err(ColorParseError::Invalid)`.
  Added to the test plan above.

Review artifacts:

- Prompt: `logs/codex-review/20260604-130737-d476-prompt.md` (design)
- Result: `logs/codex-review/20260604-130737-d476-last-message.md` (design)
