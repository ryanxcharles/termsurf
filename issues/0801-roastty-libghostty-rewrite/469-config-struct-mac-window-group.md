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

# Experiment 469: grow the Config struct with the macOS-window group

## Description

Continuing the incremental growth of the aggregating `Config` struct
(Experiments 461–468), this experiment adds the **macOS-window** group:
`fullscreen`, `macos_non_native_fullscreen`, `macos_titlebar_style`,
`macos_titlebar_proxy_icon`, `macos_window_buttons`, and `macos_hidden` — all
already-ported leaf enums (`Fullscreen`, `NonNativeFullscreen`,
`MacTitlebarStyle`, `MacTitlebarProxyIcon`, `MacWindowButtons`, `MacHidden`,
from Experiments 456–458). It adds the six fields and their upstream
`Config`-field defaults to `Config` and its `Default`. The parser and the rest
of upstream `Config` stay deferred.

## Upstream behavior

In `config/Config.zig`, the macOS-window group's field defaults:

```zig
fullscreen: Fullscreen = .false,
@"macos-non-native-fullscreen": NonNativeFullscreen = .false,
@"macos-titlebar-style": MacTitlebarStyle = .transparent,
@"macos-titlebar-proxy-icon": MacTitlebarProxyIcon = .visible,
@"macos-window-buttons": MacWindowButtons = .visible,
@"macos-hidden": MacHidden = .never,
```

`fullscreen` defaults to `.false`; `macos-non-native-fullscreen` to `.false`;
`macos-titlebar-style` to `.transparent`; `macos-titlebar-proxy-icon` to
`.visible`; `macos-window-buttons` to `.visible`; `macos-hidden` to `.never`.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
pub(crate) struct Config {
    // ... clipboard (461) … surface-policy (468) ...
    /// `fullscreen`.
    pub fullscreen: Fullscreen,
    /// `macos-non-native-fullscreen`.
    pub macos_non_native_fullscreen: NonNativeFullscreen,
    /// `macos-titlebar-style`.
    pub macos_titlebar_style: MacTitlebarStyle,
    /// `macos-titlebar-proxy-icon`.
    pub macos_titlebar_proxy_icon: MacTitlebarProxyIcon,
    /// `macos-window-buttons`.
    pub macos_window_buttons: MacWindowButtons,
    /// `macos-hidden`.
    pub macos_hidden: MacHidden,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // ... earlier groups ...
            fullscreen: Fullscreen::False,
            macos_non_native_fullscreen: NonNativeFullscreen::False,
            macos_titlebar_style: MacTitlebarStyle::Transparent,
            macos_titlebar_proxy_icon: MacTitlebarProxyIcon::Visible,
            macos_window_buttons: MacWindowButtons::Visible,
            macos_hidden: MacHidden::Never,
        }
    }
}
```

The defaults are upstream's Config-field defaults: `fullscreen` `False`,
`macos-non-native-fullscreen` `False`, `macos-titlebar-style` `Transparent`,
`macos-titlebar-proxy-icon` `Visible`, `macos-window-buttons` `Visible`,
`macos-hidden` `Never`.

## Scope / faithfulness notes

- **Ported (bridged)**: the macOS-window field group of the aggregating `Config`
  struct (upstream `config.Config`) — the six fields and their `Default`.
- **Faithful**: the six fields use the already-ported types (`Fullscreen`,
  `NonNativeFullscreen`, `MacTitlebarStyle`, `MacTitlebarProxyIcon`,
  `MacWindowButtons`, `MacHidden`); their `Default` values match upstream's
  Config-field defaults (`.false`, `.false`, `.transparent`, `.visible`,
  `.visible`, `.never`).
- **Faithful adaptation**: the struct continues to grow one coherent field group
  per experiment. The derive set (`Clone`/`PartialEq`) is unchanged. roastty is
  macOS-only, so these macOS-window fields are directly relevant.
- **Deferred**: the rest of upstream `Config`'s fields (added group by group in
  later slices), the parser, the `changeConfig` machinery, and the
  conditional-config system. (Consumed by later slices; this experiment grows
  the struct with the macOS-window group.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add the six fields `fullscreen: Fullscreen`,
     `macos_non_native_fullscreen: NonNativeFullscreen`,
     `macos_titlebar_style: MacTitlebarStyle`,
     `macos_titlebar_proxy_icon: MacTitlebarProxyIcon`,
     `macos_window_buttons: MacWindowButtons`, `macos_hidden: MacHidden` to
     `Config`, and their defaults (`False`, `False`, `Transparent`, `Visible`,
     `Visible`, `Never`) to the `Default` impl.
2. Tests (in `config/mod.rs`):
   - extend the `Config::default()` assertion for the new fields:
     `fullscreen == Fullscreen::False`,
     `macos_non_native_fullscreen == NonNativeFullscreen::False`,
     `macos_titlebar_style == MacTitlebarStyle::Transparent`,
     `macos_titlebar_proxy_icon == MacTitlebarProxyIcon::Visible`,
     `macos_window_buttons == MacWindowButtons::Visible`,
     `macos_hidden == MacHidden::Never`; the existing group defaults still hold.
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

- `Config` gains the six macOS-window fields, and `Config::default()` sets their
  upstream defaults while the earlier group defaults still hold — a faithful
  partial of upstream's `Config`;
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
findings**. It verified against the vendored upstream that the six defaults are
correct (`fullscreen = Fullscreen::False`, `Config.zig:1469`;
`macos_non_native_fullscreen = NonNativeFullscreen::False`, `:3198`;
`macos_titlebar_style = MacTitlebarStyle::Transparent`, `:3261`;
`macos_titlebar_proxy_icon = MacTitlebarProxyIcon::Visible`, `:3282`;
`macos_window_buttons = MacWindowButtons::Visible`, `:3219`;
`macos_hidden = MacHidden::Never`, `:3358`); the macOS-window group is coherent
and the field names are faithful Rust mappings of the config keys; and the test
plan is adequate (assert these six defaults and keep the existing groups covered
as `Default` grows).

Review artifacts:

- Prompt: `logs/codex-review/20260604-123415-d469-prompt.md` (design)
- Result: `logs/codex-review/20260604-123415-d469-last-message.md` (design)

## Result

**Result:** Pass

The `Config` struct now carries the macOS-window field group.

- `roastty/src/config/mod.rs`: `Config` gains `fullscreen: Fullscreen`,
  `macos_non_native_fullscreen: NonNativeFullscreen`,
  `macos_titlebar_style: MacTitlebarStyle`,
  `macos_titlebar_proxy_icon: MacTitlebarProxyIcon`,
  `macos_window_buttons: MacWindowButtons`, and `macos_hidden: MacHidden`;
  `Config::default()` sets their upstream Config-field defaults —
  `Fullscreen::False`, `NonNativeFullscreen::False`,
  `MacTitlebarStyle::Transparent`, `MacTitlebarProxyIcon::Visible`,
  `MacWindowButtons::Visible`, `MacHidden::Never`.

Test (in `config/mod.rs`): `config_default_clipboard_group` extended to assert
the six new macOS-window defaults alongside the eight prior groups' defaults;
the modified-config inequality and the `Clone`/`PartialEq` round-trip remain.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2952 passed, 0 failed (no regressions; the existing
  `config_default` test was extended).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer + config +
  `lib.rs`/header/`abi_harness.c`) clean; `git diff --check` clean.

## Conclusion

The aggregating `Config` struct now holds nine field groups — clipboard (461),
mouse/click (462), shell-integration (463), notification (464),
renderer-appearance (465), background-image (466), optional-colors (467),
surface-policy (468), and macOS-window — thirty-two fields total. The
macOS-window group consumes the macOS titlebar / window / fullscreen enums
ported in Experiments 456–458, directly relevant to roastty's macOS-only target.
The parser, the `changeConfig` machinery, the conditional-config system, and the
remaining upstream `Config` fields stay deferred.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed all six macOS-window fields were added with
faithful upstream defaults; the Rust field names correctly map the upstream
config keys; extending the existing `Config::default()` test is adequate and
keeps all prior groups covered; and the deferred parser / `changeConfig` /
conditional-config work remains properly scoped. No public C ABI/header impact;
nothing needed to change before the result commit.

Review artifacts:

- Prompt: `logs/codex-review/20260604-123608-r469-prompt.md` (result)
- Result: `logs/codex-review/20260604-123608-r469-last-message.md` (result)
