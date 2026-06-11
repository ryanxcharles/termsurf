+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 74: Phase F — window tab and titlebar config

## Description

Experiment 73 wired window size and step-resize config. The next upstream fields
before the resize-overlay block are:

- `window-new-tab-position`
- `window-show-tab-bar`
- `window-titlebar-background`
- `window-titlebar-foreground`

Upstream declares `window-new-tab-position` as
`WindowNewTabPosition = .current`, `window-show-tab-bar` as
`WindowShowTabBar = .auto`, and both titlebar colors as optional `Color = null`
in `vendor/ghostty/src/config/Config.zig`.

This experiment adds the config parser/formatter surface only. Runtime tab
insertion behavior, tab bar visibility, and titlebar color application are out
of scope.

## Changes

- `roastty/src/config/mod.rs`
  - Add `WindowNewTabPosition::{Current, End}`.
  - Add `WindowShowTabBar::{Always, Auto, Never}`.
  - Add `Config::window_new_tab_position = Current`.
  - Add `Config::window_show_tab_bar = Auto`.
  - Add `Config::window_titlebar_background: Option<Color> = None`.
  - Add `Config::window_titlebar_foreground: Option<Color> = None`.
  - Route all four keys through defaults, `Config::set`, `format_config`,
    diagnostics, clone/equality, enum keyword tests, and formatter-order tests.
  - Add upstream compatibility behavior for `gtk-tabs-location = hidden`: update
    `window-show-tab-bar` to `Never`; leave other `gtk-tabs-location` values as
    normal unknown-field diagnostics.
  - Preserve upstream order after `window-step-resize`:
    - `window-new-tab-position`
    - `window-show-tab-bar`
    - `window-titlebar-background`
    - `window-titlebar-foreground`

Out of scope:

- Runtime tab placement behavior.
- Runtime tab bar visibility.
- Applying titlebar colors in the macOS/GTK app runtime.
- Resize overlay config.
- Other compatibility handlers not affecting these fields.
- `keybind` and `key-remap`.

## Verification

- Run formatting:
  - `cargo fmt`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/74-window-tab-titlebar-config.md`
- Run targeted tests:
  - `cargo test -p roastty window_tab_titlebar_config`
  - `cargo test -p roastty enum_from_keyword_round_trips`
  - `cargo test -p roastty config_format_config`
- Add concrete test cases proving:
  - enum defaults are `current` and `auto`;
  - enum values parse, format, reset on empty values, return `ValueRequired` on
    missing values, and return `InvalidValue` on unknown values;
  - titlebar colors default/format as empty, parse hex and named X11 colors,
    reset on empty values, return `ValueRequired` on missing values, and return
    `InvalidValue` on bad color values;
  - `gtk-tabs-location = hidden` maps to `window-show-tab-bar = never`, while
    other `gtk-tabs-location` values remain unknown-field diagnostics;
  - `Config::load_str` records diagnostics for invalid neighboring enum/color
    lines while preserving valid values;
  - formatter order matches the upstream sequence around these fields;
  - clone/equality preserves all four values.
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `git status --short` and verify only intended source/docs are present.

**Pass** = the four tab/titlebar fields are represented faithfully on `Config`,
the upstream `gtk-tabs-location = hidden` compatibility behavior is present, the
fields round-trip through config loading/formatting, match upstream defaults and
parser behavior, and have targeted and full tests passing.

**Partial** = some fields land faithfully but a parser, diagnostic, or
formatter-order edge requires a follow-up.

**Fail** = these fields cannot be represented faithfully without first porting
runtime tab or titlebar behavior.

## Design Review

Codex adversarial reviewer `019eb426-a330-7090-94da-56d4102c6535` initially
returned **Changes Required** for one design gap: the first draft omitted
upstream compatibility behavior where `gtk-tabs-location = hidden` maps to
`window-show-tab-bar = never`. The reviewer also suggested explicitly testing a
named X11 color for titlebar color fields.

The design was updated to include the compatibility mapping and tests, while
leaving other `gtk-tabs-location` values as normal unknown-field diagnostics. It
was also updated to require named X11 color coverage for the titlebar color
fields. On re-review, the same reviewer returned **Approved** with no remaining
findings.

## Result

**Result:** Pass

Implemented the four planned config fields in `roastty/src/config/mod.rs`.
`WindowNewTabPosition` and `WindowShowTabBar` now parse and format their
upstream enum keywords, the titlebar color fields use the existing optional
`Color` parser/formatter path, and `gtk-tabs-location = hidden` maps to
`window-show-tab-bar = never`. Other `gtk-tabs-location` values still surface as
unknown fields.

Verification passed:

- `cargo fmt`
- `cargo test -p roastty window_tab_titlebar_config`
- `cargo test -p roastty enum_from_keyword_round_trips`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
  - 4510 unit tests passed
  - ABI harness passed with the existing enum-conversion warnings
  - doc tests passed
- `cargo fmt --check`
- `git diff --check`

## Conclusion

The tab/titlebar config surface now matches the upstream defaults, parser
behavior, optional-color handling, compatibility mapping, and formatter order
needed for this slice. The next experiment can continue with the resize-overlay
fields that follow these entries upstream.

## Completion Review

Codex adversarial reviewer `019eb431-f30d-7f93-94c0-5f930815e27c` returned
**Approved** with no required findings. The reviewer checked the current diff
against the plan commit, verified the four fields, defaults, parser/formatter
support, optional color handling, formatter order, diagnostics, reset behavior,
clone/equality coverage, and `gtk-tabs-location = hidden` compatibility.

The reviewer independently ran:

- `cargo fmt --check`
- `cargo test -p roastty window_tab_titlebar_config`
- `cargo test -p roastty enum_from_keyword_round_trips`
- `cargo test -p roastty config_format_config`
- `git diff --check 8a2f07c719c02 -- ...`
