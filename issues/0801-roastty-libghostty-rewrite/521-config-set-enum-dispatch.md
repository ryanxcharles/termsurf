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

# Experiment 521: the per-field parse dispatch, enum slice (Config::set + set_enum_field)

## Description

This experiment begins the aggregate **`Config::set(key, value)`** — the inverse
of `Config::format_config` (Experiment 512) and the Rust analog of upstream
`cli.args.parseIntoField` for a `Config`. It establishes the dispatch skeleton
(`ConfigSetError`, the key→field routing, the **set-but-empty reset-to-default**
rule) and wires the **enum** field category (25 fields) via a `set_enum_field`
helper over the `from_keyword` parsers (Experiments 513–516).

`Config::set` is built up by category across experiments. This first slice
handles the enum keys; the other categories (packed structs, bool, colors,
font-style) and the deferred fields (`theme`, needing `parseAutoStruct`;
`background-image-opacity`, float-blocked) are wired in later experiments. Until
then, a non-enum key returns `ConfigSetError::UnknownField` — a documented
in-progress state, not a final behavior.

## Upstream behavior

`parseIntoField` (`cli/args.zig:302`), for a matched field:

1. **Empty-string reset** (`args.zig:326`): if the value is set but empty
   (`v.len == 0`), the field is reset to its default (its `init` for types that
   have one, else the struct default), and parsing returns.
2. Otherwise the field is parsed. For an **enum** field with no `parseCLI`, the
   type-magic path is
   `std.meta.stringToEnum(Field, value orelse return error.ValueRequired)` — a
   missing value is `error.ValueRequired`; a present value is matched by tag
   name, with `error.InvalidValue` when none matches.
3. If no field name matches the key, `parseIntoField` returns
   `error.InvalidField`.

So for an enum field: `value == Some("")` ⇒ reset to default; `value == None` ⇒
`ValueRequired`; `value == Some(v)` ⇒ `from_keyword(v)` or `InvalidValue`.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
/// An error from `Config::set` (upstream `parseIntoField`'s
/// `error.{InvalidField,InvalidValue,ValueRequired}`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfigSetError {
    UnknownField,
    InvalidValue,
    ValueRequired,
}

/// Resolve an enum field value (upstream's empty-reset + `stringToEnum` magic):
/// a set-but-empty value resets to the default; a missing value is
/// `ValueRequired`; otherwise `parse` (from_keyword) or `InvalidValue`.
fn set_enum_field<T: Copy>(
    value: Option<&str>,
    default_value: T,
    parse: impl FnOnce(&str) -> Option<T>,
) -> Result<T, ConfigSetError> {
    match value {
        Some("") => Ok(default_value),
        None => Err(ConfigSetError::ValueRequired),
        Some(v) => parse(v).ok_or(ConfigSetError::InvalidValue),
    }
}

impl Config {
    /// Set one config field from a `key = value` pair (upstream
    /// `cli.args.parseIntoField` for `Config`). Built up by category across
    /// experiments; this slice routes the enum fields. A non-enum key currently
    /// returns `UnknownField` (wired in later experiments).
    pub(crate) fn set(&mut self, key: &str, value: Option<&str>) -> Result<(), ConfigSetError> {
        let default = Config::default();
        match key {
            "copy-on-select" => {
                self.copy_on_select =
                    set_enum_field(value, default.copy_on_select, CopyOnSelect::from_keyword)?
            }
            "clipboard-read" => {
                self.clipboard_read =
                    set_enum_field(value, default.clipboard_read, ClipboardAccess::from_keyword)?
            }
            // … the remaining 23 enum fields, each:
            //   "<key>" => self.<field> = set_enum_field(value, default.<field>, <Enum>::from_keyword)?,
            _ => return Err(ConfigSetError::UnknownField),
        }
        Ok(())
    }
}
```

The 25 enum fields and keys (each
`self.<field> = set_enum_field(value, default.<field>, <Enum>::from_keyword)?`):

| key                           | field                         | enum                      |
| ----------------------------- | ----------------------------- | ------------------------- |
| `copy-on-select`              | `copy_on_select`              | `CopyOnSelect`            |
| `clipboard-read`              | `clipboard_read`              | `ClipboardAccess`         |
| `clipboard-write`             | `clipboard_write`             | `ClipboardAccess`         |
| `mouse-shift-capture`         | `mouse_shift_capture`         | `MouseShiftCapture`       |
| `right-click-action`          | `right_click_action`          | `RightClickAction`        |
| `middle-click-action`         | `middle_click_action`         | `MiddleClickAction`       |
| `shell-integration`           | `shell_integration`           | `ShellIntegration`        |
| `notify-on-command-finish`    | `notify_on_command_finish`    | `NotifyOnCommandFinish`   |
| `window-colorspace`           | `window_colorspace`           | `WindowColorspace`        |
| `alpha-blending`              | `alpha_blending`              | `AlphaBlending`           |
| `window-padding-color`        | `window_padding_color`        | `WindowPaddingColor`      |
| `background-image-position`   | `bg_image_position`           | `BackgroundImagePosition` |
| `background-image-fit`        | `bg_image_fit`                | `BackgroundImageFit`      |
| `confirm-close-surface`       | `confirm_close_surface`       | `ConfirmCloseSurface`     |
| `link-previews`               | `link_previews`               | `LinkPreviews`            |
| `window-subtitle`             | `window_subtitle`             | `WindowSubtitle`          |
| `fullscreen`                  | `fullscreen`                  | `Fullscreen`              |
| `macos-non-native-fullscreen` | `macos_non_native_fullscreen` | `NonNativeFullscreen`     |
| `macos-titlebar-style`        | `macos_titlebar_style`        | `MacTitlebarStyle`        |
| `macos-titlebar-proxy-icon`   | `macos_titlebar_proxy_icon`   | `MacTitlebarProxyIcon`    |
| `macos-window-buttons`        | `macos_window_buttons`        | `MacWindowButtons`        |
| `macos-hidden`                | `macos_hidden`                | `MacHidden`               |
| `grapheme-width-method`       | `grapheme_width_method`       | `GraphemeWidthMethod`     |
| `osc-color-report-format`     | `osc_color_report_format`     | `OscColorReportFormat`    |
| `custom-shader-animation`     | `custom_shader_animation`     | `CustomShaderAnimation`   |

`default` is `Config::default()` (built once); each enum field is `Copy`, so the
reset target `default.<field>` copies out. The keys are the upstream config keys
(same as `format_config`).

## Scope / faithfulness notes

- **Ported (bridged)**: `Config::set` skeleton, `ConfigSetError`, the
  empty-string reset, and the enum-field category (via `set_enum_field` +
  `from_keyword`).
- **Faithful**: for an enum field, `Some("")` ⇒ reset to default, `None` ⇒
  `ValueRequired`, `Some(v)` ⇒ `from_keyword` or `InvalidValue` — exactly
  upstream's reset + `stringToEnum` path. A truly unknown key ⇒ `UnknownField`
  (upstream `InvalidField`).
- **In-progress (documented)**: non-enum _known_ keys currently ⇒ `UnknownField`
  pending their category experiments — an explicit incremental state, not a
  final behavior.
- **Deferred**: the packed-struct / bool / color / font-style field categories;
  `theme` (`parseAutoStruct`) and `background-image-opacity` (float-blocked);
  the int type-magic; the `loadCli` / file loader.
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`: add `ConfigSetError`, `set_enum_field`, and
   `Config::set` with the 25 enum arms.
2. Tests (in `config/mod.rs`): a sample of enum keys parse
   (`set("fullscreen", Some("non-native"))` ⇒ field set; round-trip vs
   `from_keyword`); `Some("")` resets to the default; `None` ⇒
   `Err(ValueRequired)`; an invalid value ⇒ `Err(InvalidValue)`; an unknown key
   ⇒ `Err(UnknownField)`.
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

- `Config::set` routes the 25 enum keys via `set_enum_field` with the faithful
  reset / `ValueRequired` / `InvalidValue` semantics, and an unknown key ⇒
  `UnknownField`;
- the tests pass (parse, reset, missing, invalid, unknown), and the existing
  tests still pass;
- the other field categories and the deferred fields stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if an enum field's semantics diverge from upstream, a
key is mis-mapped, an unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with one
**Low** finding (folded into this experiment's tests): make the enum-dispatch
test table-driven over all 25 key arms ("set each key to a non-default valid
value, then reset with `Some("")`") rather than a sample, to cheaply catch
key/field/enum copy-paste mistakes in what is mostly routing.

Codex found **no faithfulness issues**: the enum semantics are right — upstream
resets `Some("")` before any parser/type-magic runs (`args.zig:326`), then the
enum branch does `stringToEnum(Field, value orelse return error.ValueRequired)`
(`args.zig:442`), so `Some("") -> default`, `None -> ValueRequired`, and
`Some(v) -> from_keyword(v) or InvalidValue` is faithful. The incremental
category-by-category dispatch framing is acceptable while internal/in-progress
and documented (returning `UnknownField` for non-enum known keys is not final
behavior). The 25 key/field/enum mappings line up with the current Rust `Config`
fields and upstream keys (including `bg_image_position`/`bg_image_fit`, both
clipboard fields, and correctly excluding `window-decoration`, which has custom
parse behavior rather than plain `stringToEnum`). Building `Config::default()`
once per `set()` call is the right reset source for this slice.

Review artifacts:

- Prompt: `logs/codex-review/20260604-181358-d521-prompt.md` (design)
- Result: `logs/codex-review/20260604-181358-d521-last-message.md` (design)

## Result

**Result:** Pass

`ConfigSetError`, `set_enum_field`, and `Config::set` (with the 25 enum arms)
were added. For an enum field, `Some("")` resets to the default (from
`Config::default()`), `None` is `ValueRequired`, and `Some(v)` is
`from_keyword(v)` or `InvalidValue`; an unknown key is `UnknownField`. The new
table-driven test `config_set_routes_enum_fields` (folding the design-review Low
finding) exercises all 25 enum keys through `Config::set` and verifies routing
via `format_config`, plus the missing-value, invalid-value, unknown-key, and
reset-to-default cases.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3007 passed, 0 failed (one new test; no regressions).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + lib.rs/header/abi_harness.c)
  clean; `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no
findings**: the implementation matches the enum-slice semantics — `Some("")`
resets from `Config::default()`, `None` gives `ValueRequired`, invalid enum
values give `InvalidValue`, and unmatched keys give `UnknownField`, with the
empty-value reset faithfully ordered before the `stringToEnum` branch; the
folded table-driven test is adequate (every enum key exercised through `set`
then verified via `format_config`, plus missing/invalid/unknown/reset), and the
incremental `UnknownField` for non-enum known keys remains documented and
deferred. "Approved with no findings."

Review artifacts:

- Prompt: `logs/codex-review/20260604-181803-r521-prompt.md` (result)
- Result: `logs/codex-review/20260604-181803-r521-last-message.md` (result)

## Conclusion

`Config::set` now exists and routes the 25 enum fields faithfully, with the
empty-string reset and `ValueRequired` / `InvalidValue` / `UnknownField`
semantics established. The next slices extend the dispatch by category: the
packed-struct + bool fields (`set` arms over `parse_cli` / `parse_bool_field`),
then the color + font-style fields, then `theme` (after `parseAutoStruct` /
`Theme::parse_cli`) and the float-blocked `background-image-opacity`. Then the
`loadCli` / file loader drives `Config::set` over parsed CLI args / config-file
lines.
