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

# Experiment 522: the per-field parse dispatch, packed-struct + bool slice (Config::set)

## Description

Extending the aggregate `Config::set` (Experiment 521 wired the enum category),
this experiment adds two more field categories: the **packed-struct** fields
(four — `font-shaping-break`, `scroll-to-bottom`, `shell-integration-features`,
`notify-on-command-finish-action`) over their `parse_cli` (Experiments 517–518),
and the single **bool** field (`background-image-repeat`) over
`parse_bool_field` (Experiment 519). Each gets a small helper that applies the
same empty-string reset the enum slice uses, with the type-appropriate value
semantics.

## Upstream behavior

In `parseIntoField`, after the empty-string reset (`args.zig:326`):

- **Packed struct** (no `parseCLI`): the type-magic path is
  `.@"struct" => parseStruct(Field, alloc, value orelse return error.ValueRequired)`
  (`args.zig:448`). So a missing value is `error.ValueRequired`; a present value
  goes to `parsePackedStruct` (Experiment 517), with a parse failure surfaced as
  `error.InvalidValue`.
- **bool**: `parseBool(value orelse "t")` (`args.zig:416`). So a missing value
  is a bare flag ⇒ `true` (never `ValueRequired`); a present value is
  `parse_bool` or `error.InvalidValue`.

(For both, `value == Some("")` is intercepted by the earlier reset and sets the
field to its default.)

## Rust mapping (`roastty/src/config/mod.rs`)

Two helpers mirror `set_enum_field`, plus five new `Config::set` arms:

```rust
/// Resolve a packed-struct field value (empty-reset + `parseStruct` magic): a
/// set-but-empty value resets to the default; a missing value is `ValueRequired`;
/// otherwise `parse` (the struct's `parse_cli`) or `InvalidValue`.
fn set_packed_field<T>(
    value: Option<&str>,
    default_value: T,
    parse: impl FnOnce(&str) -> Result<T, FlagsParseError>,
) -> Result<T, ConfigSetError> {
    match value {
        Some("") => Ok(default_value),
        None => Err(ConfigSetError::ValueRequired),
        Some(v) => parse(v).map_err(|_| ConfigSetError::InvalidValue),
    }
}

/// Resolve a `bool` field value (empty-reset + `parseBool(value orelse "t")`): a
/// set-but-empty value resets to the default; otherwise `parse_bool_field` (a
/// missing value is a bare flag ⇒ `true`), with `InvalidValue` on a bad value.
fn set_bool_field(value: Option<&str>, default_value: bool) -> Result<bool, ConfigSetError> {
    match value {
        Some("") => Ok(default_value),
        _ => parse_bool_field(value).map_err(|_| ConfigSetError::InvalidValue),
    }
}
```

New `Config::set` arms (added to the existing `match key`):

```rust
"font-shaping-break" => {
    self.font_shaping_break =
        set_packed_field(value, default.font_shaping_break, FontShapingBreak::parse_cli)?
}
"scroll-to-bottom" => {
    self.scroll_to_bottom =
        set_packed_field(value, default.scroll_to_bottom, ScrollToBottom::parse_cli)?
}
"shell-integration-features" => {
    self.shell_integration_features = set_packed_field(
        value,
        default.shell_integration_features,
        ShellIntegrationFeatures::parse_cli,
    )?
}
"notify-on-command-finish-action" => {
    self.notify_on_command_finish_action = set_packed_field(
        value,
        default.notify_on_command_finish_action,
        NotifyOnCommandFinishAction::parse_cli,
    )?
}
"background-image-repeat" => {
    self.bg_image_repeat = set_bool_field(value, default.bg_image_repeat)?
}
```

The packed structs and `bool` are `Copy`, so the reset target `default.<field>`
copies out. The keys are the upstream config keys (same as `format_config`).

## Scope / faithfulness notes

- **Ported (bridged)**: the packed-struct and bool field categories of
  `parseIntoField`, as `set_packed_field` / `set_bool_field` + five
  `Config::set` arms.
- **Faithful**: packed struct — `Some("")` ⇒ reset, `None` ⇒ `ValueRequired`,
  `Some(v)` ⇒ `parse_cli` or `InvalidValue`; bool — `Some("")` ⇒ reset, `None` ⇒
  `true` (bare flag), `Some(v)` ⇒ `parse_bool` or `InvalidValue` — exactly
  upstream's reset + `parseStruct` / `parseBool(value orelse "t")` paths.
- **In-progress (documented)**: the color / font-style categories and the
  deferred `theme` / `background-image-opacity` keys still return `UnknownField`
  pending their experiments.
- **Deferred**: the color + font-style `Config::set` arms; `theme`
  (`parseAutoStruct`) and `background-image-opacity` (float-blocked); the int
  type-magic; the `loadCli` / file loader.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add `set_packed_field` and `set_bool_field`, and
   the five new `Config::set` arms.
2. Tests (in `config/mod.rs`): each packed-struct key parses a `[no-]flag` value
   and a standalone bool (routed correctly, verified via `format_config`);
   `background-image-repeat` set to `false` / a bare flag (`None` ⇒ `true`);
   `Some("")` resets each; `None` ⇒ `ValueRequired` for a packed struct (but
   `true` for the bool); an invalid value ⇒ `InvalidValue`.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty config_set
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- the five new arms route via `set_packed_field` / `set_bool_field` with the
  faithful reset / `ValueRequired` / bare-flag-`true` / `InvalidValue`
  semantics;
- the tests pass (parse, standalone bool, bare flag, reset, missing, invalid),
  and the existing tests still pass;
- the remaining categories and deferred fields stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a field's semantics diverge from upstream (esp.
bool's bare-flag-`true` vs packed-struct's `ValueRequired` on a missing value),
a key is mis-mapped, an unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the packed-struct path is faithful — `parseIntoField`
reaches
`.@"struct" => parseStruct(Field, alloc, value orelse return error.ValueRequired)`
for structs without a custom `parseCLI`, and `parseStruct` dispatches packed
structs to `parsePackedStruct` (`args.zig:448`/`:517`), so packed fields get
`None -> ValueRequired` while `parsePackedStruct` failures surface as
`InvalidValue`. The bool asymmetry is correct: `parseBool(value orelse "t")`
gives `None -> true` (not `ValueRequired`, `args.zig:416`). For both categories,
`Some("") -> default` is handled by the earlier empty-reset block before the
type magic (`args.zig:326`). The five field mappings are right (the four packed
structs and the single raw bool field are upstream config fields —
`Config.zig:374`/`:698`/ `:938`/`:1232`/`:2858`), and mapping `FlagsParseError`
/ bool `InvalidValue` into `ConfigSetError::InvalidValue` is faithful.

Review artifacts:

- Prompt: `logs/codex-review/20260604-181957-d522-prompt.md` (design)
- Result: `logs/codex-review/20260604-181957-d522-last-message.md` (design)

## Result

**Result:** Pass

`set_packed_field`, `set_bool_field`, and five new `Config::set` arms were
added. Packed-struct fields are `Some("")` ⇒ reset, `None` ⇒ `ValueRequired`,
`Some(v)` ⇒ `parse_cli` or `InvalidValue`; the bool field is `Some("")` ⇒ reset,
`None` ⇒ `true` (bare flag), `Some(v)` ⇒ `parse_bool` or `InvalidValue`. The new
test `config_set_routes_packed_and_bool_fields` exercises each packed key
(`[no-]flag` list / standalone bool), the bool field (explicit value + bare
flag), the packed-vs-bool missing-value asymmetry, the invalid-value error, and
a reset.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3008 passed, 0 failed (one new test; no regressions).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + lib.rs/header/abi_harness.c)
  clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no
findings**: the implementation preserves the key asymmetry — packed structs
require a value (upstream `parseStruct(..., value orelse ValueRequired)`) while
bool fields use `parseBool(value orelse "t")` (missing value ⇒ bare-flag `true`)
— and the `Some("")` reset-before-parse is correctly applied to both; the five
new routes match the approved slice; the tests cover packed flag lists,
standalone bools, unknown values, the packed/bool missing-value cases, and
reset-to-default; gates are clean and the remaining loader categories stayed
deferred. "Approved with no findings."

Review artifacts:

- Prompt: `logs/codex-review/20260604-182227-r522-prompt.md` (result)
- Result: `logs/codex-review/20260604-182227-r522-last-message.md` (result)

## Conclusion

`Config::set` now routes 30 of the 43 fields (25 enums + 4 packed structs + 1
bool). The next slices add the **color** fields (`background` / `foreground` via
`Color::parse_cli`; `cursor-color` / `cursor-text` / `selection-foreground` /
`selection-background` via `Option<TerminalColor>`; `bold-color` via
`Option<BoldColor>`) and the **font-style** fields (`font-style*` via
`FontStyle::parse_cli`, Experiment 520), then `theme` (after `parseAutoStruct` /
`Theme::parse_cli`) and the float-blocked `background-image-opacity`. Then the
`loadCli` / file loader drives `Config::set`.
