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

# Experiment 471: grow the Config struct with the terminal/render-behavior group

## Description

Continuing the incremental growth of the aggregating `Config` struct
(Experiments 461–470), this experiment adds the **terminal/render-behavior**
group: `grapheme_width_method`, `osc_color_report_format`, `scroll_to_bottom`,
and `custom_shader_animation` — the remaining already-ported leaf enums /
flag-structs (`GraphemeWidthMethod`, `OscColorReportFormat`, `ScrollToBottom`,
`CustomShaderAnimation`). It adds the four fields and their upstream
`Config`-field defaults to `Config` and its `Default`. The parser and the rest
of upstream `Config` stay deferred.

## Upstream behavior

In `config/Config.zig`, the group's field defaults:

```zig
@"grapheme-width-method": GraphemeWidthMethod = .unicode,
@"osc-color-report-format": OSCColorReportFormat = .@"16-bit",
@"scroll-to-bottom": ScrollToBottom = .default,
@"custom-shader-animation": CustomShaderAnimation = .true,
```

`grapheme-width-method` defaults to `.unicode`; `osc-color-report-format` to
`.16-bit`; `scroll-to-bottom` to `.default` (the struct's own field defaults:
`keystroke = true`, `output = false`); `custom-shader-animation` to `.true`.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
pub(crate) struct Config {
    // ... clipboard (461) … font (470) ...
    /// `grapheme-width-method`.
    pub grapheme_width_method: GraphemeWidthMethod,
    /// `osc-color-report-format`.
    pub osc_color_report_format: OscColorReportFormat,
    /// `scroll-to-bottom`.
    pub scroll_to_bottom: ScrollToBottom,
    /// `custom-shader-animation`.
    pub custom_shader_animation: CustomShaderAnimation,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // ... earlier groups ...
            grapheme_width_method: GraphemeWidthMethod::Unicode,
            osc_color_report_format: OscColorReportFormat::Bits16,
            scroll_to_bottom: ScrollToBottom::default(),
            custom_shader_animation: CustomShaderAnimation::True,
        }
    }
}
```

The defaults are upstream's Config-field defaults: `grapheme-width-method`
`Unicode`, `osc-color-report-format` `Bits16` (the `16-bit` variant),
`scroll-to-bottom` `ScrollToBottom::default()` (the `.default` literal —
`keystroke = true`, `output = false`), `custom-shader-animation` `True`.

## Scope / faithfulness notes

- **Ported (bridged)**: the terminal/render-behavior field group of the
  aggregating `Config` struct (upstream `config.Config`) — the four fields and
  their `Default`.
- **Faithful**: the four fields use the already-ported types
  (`GraphemeWidthMethod`, `OscColorReportFormat`, `ScrollToBottom`,
  `CustomShaderAnimation`); their `Default` values match upstream's Config-field
  defaults (`.unicode`, `.16-bit`, `.default`, `.true`).
- **Faithful adaptation**: the `osc-color-report-format` `.16-bit` literal maps
  to the `Bits16` variant (the non-identifier tag rename from Experiment 444);
  the `scroll-to-bottom` `.default` literal maps to `ScrollToBottom::default()`
  (its own field defaults). The struct continues to grow one coherent field
  group per experiment; the derive set is unchanged.
- **Deferred**: the rest of upstream `Config`'s fields (added group by group in
  later slices), the parser, the `changeConfig` machinery, and the
  conditional-config system. (Consumed by later slices; this experiment grows
  the struct with the terminal/render-behavior group.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add the four fields `grapheme_width_method: GraphemeWidthMethod`,
     `osc_color_report_format: OscColorReportFormat`,
     `scroll_to_bottom: ScrollToBottom`,
     `custom_shader_animation: CustomShaderAnimation` to `Config`, and their
     defaults (`Unicode`, `Bits16`, `ScrollToBottom::default()`, `True`) to the
     `Default` impl.
2. Tests (in `config/mod.rs`):
   - extend the `Config::default()` assertion for the new fields:
     `grapheme_width_method == GraphemeWidthMethod::Unicode`,
     `osc_color_report_format == OscColorReportFormat::Bits16`,
     `scroll_to_bottom == ScrollToBottom::default()`,
     `custom_shader_animation == CustomShaderAnimation::True`; the existing
     group defaults still hold.
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

- `Config` gains the four terminal/render-behavior fields, and
  `Config::default()` sets their upstream defaults (`grapheme-width-method`
  `Unicode`, `osc-color-report-format` `Bits16`, `scroll-to-bottom`
  `ScrollToBottom::default()`, `custom-shader-animation` `True`) while the
  earlier group defaults still hold — a faithful partial of upstream's `Config`;
- the tests pass (the new defaults; the existing defaults), and the existing
  tests still pass;
- the rest of upstream `Config` and the parser stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a default is wrong, a field uses the wrong type, an
unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It verified against the vendored upstream that the four defaults are
correct (`grapheme_width_method = GraphemeWidthMethod::Unicode`,
`Config.zig:507`; `osc_color_report_format = OscColorReportFormat::Bits16`,
mapping upstream `16-bit`, `Config.zig:2920`;
`scroll_to_bottom = ScrollToBottom::default()`, mapping `.default` with
`keystroke = true`, `output = false`, `Config.zig:938` / `:10206`;
`custom_shader_animation = CustomShaderAnimation::True`, `Config.zig:3067`); the
group is acceptable as the remaining already-ported terminal/render behavior
knobs; and the test plan is adequate (assert the four new defaults and keep the
existing groups covered as `Default` grows).

Review artifacts:

- Prompt: `logs/codex-review/20260604-124233-d471-prompt.md` (design)
- Result: `logs/codex-review/20260604-124233-d471-last-message.md` (design)
