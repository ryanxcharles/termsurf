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

# Experiment 523: the per-field parse dispatch, color + font-style slice (Config::set)

## Description

Extending `Config::set` (Experiments 521–522 wired the enum, packed-struct, and
bool categories), this experiment adds the **color**, **font-style**, and
**background-blur** field categories — the fields whose types carry a `parseCLI`
(so the dispatch calls it with the value, rather than the type-magic path).
Twelve fields:

- non-optional `Color`: `background`, `foreground` (`Color::parse_cli`).
- `Option<TerminalColor>`: `cursor-color`, `cursor-text`,
  `selection-foreground`, `selection-background` (`TerminalColor::parse_cli`).
- `Option<BoldColor>`: `bold-color` (`BoldColor::parse_cli`).
- `FontStyle`: `font-style`, `font-style-bold`, `font-style-italic`,
  `font-style-bold-italic` (`FontStyle::parse_cli`, Experiment 520).
- `BackgroundBlur`: `background-blur` (`BackgroundBlur::parse_cli`, a
  `&mut self` parser whose missing value yields `.true`, not `ValueRequired`).

After this slice, `Config::set` routes 42 of the 44 fields — all except `theme`
(needs `parseAutoStruct` / `Theme::parse_cli`) and `background-image-opacity`
(float-blocked).

## Upstream behavior

In `parseIntoField`, after the empty-string reset (`args.zig:326`):

- A field **with a `parseCLI`** is parsed by calling it with the value
  (`args.zig:344`). `Color` / `TerminalColor` / `BoldColor` / `FontStyle` all
  have a `parseCLI` (they take `?[]const u8` and return `ValueRequired`
  themselves for a missing value).
- `BackgroundBlur` also has a `parseCLI` — but its missing value sets `.true` (a
  bare-flag-like default, `Config.zig:9676`), **not** `ValueRequired`. Its Rust
  port is a `&mut self` method (`*self = …`), so its arm is written inline
  rather than through the value-returning helper.
- An **optional** field (`?T`) is treated as its child type `T`
  (`args.zig:316`), and the parsed value is stored as `Some(T)`.

So for these fields: `value == Some("")` ⇒ reset to default (the optionals'
default is `None`); otherwise the type's `parse_cli(value)` is called (it
handles `None` ⇒ `ValueRequired` and invalid input ⇒ `InvalidValue`), with the
optional result wrapped in `Some`.

`Color::parse_cli` / `TerminalColor::parse_cli` / `BoldColor::parse_cli` return
`ColorParseError { ValueRequired, Invalid }`; `FontStyle::parse_cli` returns
`FontStyleParseError { ValueRequired }`.

## Rust mapping (`roastty/src/config/mod.rs`)

`From` impls map the leaf errors into `ConfigSetError`, then two generic helpers
(one non-optional, one optional) drive the parse with the shared empty-string
reset:

```rust
impl From<ColorParseError> for ConfigSetError {
    fn from(e: ColorParseError) -> Self {
        match e {
            ColorParseError::ValueRequired => ConfigSetError::ValueRequired,
            ColorParseError::Invalid => ConfigSetError::InvalidValue,
        }
    }
}

impl From<FontStyleParseError> for ConfigSetError {
    fn from(e: FontStyleParseError) -> Self {
        match e {
            FontStyleParseError::ValueRequired => ConfigSetError::ValueRequired,
        }
    }
}

impl From<BackgroundBlurParseError> for ConfigSetError {
    fn from(e: BackgroundBlurParseError) -> Self {
        match e {
            BackgroundBlurParseError::InvalidValue => ConfigSetError::InvalidValue,
        }
    }
}

/// Resolve a field whose type has a `parse_cli(Option<&str>)` (upstream's
/// empty-reset + `parseCLI`): a set-but-empty value resets to the default;
/// otherwise the type's parser (which handles a missing value itself).
fn set_value_field<T, E: Into<ConfigSetError>>(
    value: Option<&str>,
    default_value: T,
    parse: impl FnOnce(Option<&str>) -> Result<T, E>,
) -> Result<T, ConfigSetError> {
    match value {
        Some("") => Ok(default_value),
        _ => Ok(parse(value)?),
    }
}

/// Resolve an `Option<T>` field whose child has a `parse_cli` (upstream's
/// optional-as-child + empty-reset): a set-but-empty value resets to the default
/// (`None`); otherwise the parsed child wrapped in `Some`.
fn set_optional_value_field<T, E: Into<ConfigSetError>>(
    value: Option<&str>,
    default_value: Option<T>,
    parse: impl FnOnce(Option<&str>) -> Result<T, E>,
) -> Result<Option<T>, ConfigSetError> {
    match value {
        Some("") => Ok(default_value),
        _ => Ok(Some(parse(value)?)),
    }
}
```

New `Config::set` arms (added before the `_ =>` catch-all):

```rust
"background" => self.background = set_value_field(value, default.background, Color::parse_cli)?,
"foreground" => self.foreground = set_value_field(value, default.foreground, Color::parse_cli)?,
"cursor-color" => {
    self.cursor_color =
        set_optional_value_field(value, default.cursor_color, TerminalColor::parse_cli)?
}
"cursor-text" => {
    self.cursor_text =
        set_optional_value_field(value, default.cursor_text, TerminalColor::parse_cli)?
}
"selection-foreground" => {
    self.selection_foreground =
        set_optional_value_field(value, default.selection_foreground, TerminalColor::parse_cli)?
}
"selection-background" => {
    self.selection_background =
        set_optional_value_field(value, default.selection_background, TerminalColor::parse_cli)?
}
"bold-color" => {
    self.bold_color = set_optional_value_field(value, default.bold_color, BoldColor::parse_cli)?
}
"font-style" => self.font_style = set_value_field(value, default.font_style, FontStyle::parse_cli)?,
"font-style-bold" => {
    self.font_style_bold = set_value_field(value, default.font_style_bold, FontStyle::parse_cli)?
}
"font-style-italic" => {
    self.font_style_italic = set_value_field(value, default.font_style_italic, FontStyle::parse_cli)?
}
"font-style-bold-italic" => {
    self.font_style_bold_italic =
        set_value_field(value, default.font_style_bold_italic, FontStyle::parse_cli)?
}
// `BackgroundBlur::parse_cli` is `&mut self` (it overwrites `self` in place), so its
// arm is inline: `Some("")` resets to the default; otherwise parse in place (a
// missing value sets `.true`, the bare-flag default).
"background-blur" => {
    if value == Some("") {
        self.background_blur = default.background_blur;
    } else {
        self.background_blur.parse_cli(value)?;
    }
}
```

`Color` / `TerminalColor` / `BoldColor` (and their options) are `Copy`, so the
reset target copies out; `FontStyle` is not `Copy`, so `default.font_style`
_moves_ out of the once-built `default` (a partial move, fine — only one arm
runs per call). The keys are the upstream config keys (same as `format_config`).

## Scope / faithfulness notes

- **Ported (bridged)**: the color and font-style field categories of
  `parseIntoField`, as the `From` impls + `set_value_field` /
  `set_optional_value_field` + twelve `Config::set` arms (one inline for
  `background-blur`).
- **Faithful**: `Some("")` ⇒ reset (optionals to `None`); otherwise the type's
  `parse_cli` (handling `None` ⇒ `ValueRequired`, invalid ⇒ `InvalidValue`),
  with optional results wrapped in `Some` — exactly upstream's reset +
  `parseCLI` + optional-as-child paths. The `ColorParseError` /
  `FontStyleParseError` ⇒ `ConfigSetError` mapping preserves `ValueRequired` /
  `InvalidValue`.
- **In-progress (documented)**: `theme` and `background-image-opacity` still
  return `UnknownField` pending their experiments.
- **Deferred**: `theme` (`parseAutoStruct`) and `background-image-opacity`
  (float-blocked); the int type-magic; the `loadCli` / file loader.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add the three `From` impls, `set_value_field`,
   `set_optional_value_field`, and the twelve new `Config::set` arms (one inline
   for `background-blur`).
2. Tests (in `config/mod.rs`): `background` / `foreground` parse a hex / named
   color; the optional colors parse a value (`cursor-color`, incl.
   `cell-foreground`) and reset to `None` on `Some("")`; `font-style*` parse
   `default` / `false` / a named style; `None` ⇒ `ValueRequired`; an invalid
   color ⇒ `InvalidValue`; routing verified via `format_config`.
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

- the twelve new arms route via `set_value_field` / `set_optional_value_field` /
  the inline `background-blur` arm with the faithful reset / `ValueRequired` /
  `InvalidValue` / bare-flag (`background-blur` ⇒ `.true`) semantics and `Some`
  wrapping for optionals;
- the tests pass (color parse, optional reset, font-style, missing, invalid),
  and the existing tests still pass;
- `theme` and `background-image-opacity` stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a field's semantics diverge from upstream (esp. an
optional not wrapping in `Some`, or a wrong error mapping), a key is mis-mapped,
an unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex's first review raised one **Required** finding: `background-blur` (a
`Config` field emitted by `format_config`, with a custom upstream `parseCLI`)
was missing from the slice. It was added as a twelfth (inline) arm — roastty's
`BackgroundBlur::parse_cli` is `&mut self` and a missing value sets `.true` (a
bare-flag default, not `ValueRequired`) — together with
`From<BackgroundBlurParseError> for ConfigSetError`
(`InvalidValue -> InvalidValue`), and the counts were updated (12 arms; 42 of 44
fields routed).

Codex re-reviewed the revised design and **approved** it with **no findings**:
the `background-blur` arm is faithful — upstream resets `Some("")` before
`parseCLI`, then calls the mutating `parseCLI(value)` for non-empty cases
(`args.zig:326`/`:351`), and `BackgroundBlur.parseCLI` treats `None` as `.true`
then tries bool / void tags / radius (`Config.zig:9676`), so
`Some("") -> default` else `self.background_blur.parse_cli(value)?` is the right
shape; `BackgroundBlurParseError::InvalidValue -> InvalidValue` is correct (no
`ValueRequired` path since `None` is meaningful). The other 11 arms remain
correct, and the 12-arm / 42-of-44 count is consistent with `theme` and
`background-image-opacity` deferred.

Review artifacts:

- Prompt: `logs/codex-review/20260604-182512-d523-prompt.md` (design, first pass
  — Required: add `background-blur`)
- Prompt: `logs/codex-review/20260604-182735-d523-prompt.md` (design, revised)
- Result: `logs/codex-review/20260604-182735-d523-last-message.md` (design,
  revised)

## Result

**Result:** Pass

The three `From` impls, `set_value_field`, `set_optional_value_field`, and the
twelve new `Config::set` arms (one inline for `background-blur`) were added.
Non-optional `Color` / `FontStyle` parse via their `parse_cli` after the
empty-reset; optional colors parse the child and wrap in `Some` (reset to `None`
on `Some("")`); `background-blur` parses in place (a missing value ⇒ `.true`).
The error mappings preserve `ValueRequired` / `InvalidValue`. The helpers use
`.map_err(Into::into)` (the `Into<ConfigSetError>` bound). The new test
`config_set_routes_color_and_fontstyle_fields` covers direct colors,
terminal-color keywords, optional reset-to-`None`, font-style variants,
`background-blur` bool/bare-flag/radius/error, and the color missing/invalid
errors — routing verified via `format_config`. `Config::set` now routes 42 of 44
fields.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3009 passed, 0 failed (one new test; no regressions).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + lib.rs/header/abi_harness.c)
  clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no
findings**: the implementation matches the revised design and upstream behavior
— empty values reset before parsing, optional color fields parse the child and
wrap in `Some`, empty optionals reset to `None`, and missing values are left to
each parser; the `background-blur` inline arm is faithful (`Some("")` resets,
`None` becomes true via its parser, valid values parse in place, invalid →
`InvalidValue`); the error mappings are correct; the test covers the important
cases and gates are clean. "Approved with no findings."

Review artifacts:

- Prompt: `logs/codex-review/20260604-183117-r523-prompt.md` (result)
- Result: `logs/codex-review/20260604-183117-r523-last-message.md` (result)

## Conclusion

`Config::set` now routes 42 of the 44 fields — every field except `theme`
(awaiting `parseAutoStruct` / `Theme::parse_cli`) and the float-blocked
`background-image-opacity`. The remaining loader work is: `Theme::parse_cli`
(the `parseAutoStruct` light/dark-pair parser) + its `Config::set` arm; then the
`loadCli` / config-file loader that splits `key = value` lines and drives
`Config::set`. (The int type-magic has no current raw-int `Config` field;
`background-image-opacity` stays float-blocked.)
