+++
implementer = "codex"
review_design = "codex-adversarial"
+++

# Experiment 90: Phase F — macOS icon config

## Description

Port the pinned upstream macOS icon config group from
`vendor/ghostty/src/config/Config.zig` into `roastty/src/config/mod.rs`.

Upstream defines this macOS icon subgroup after the currently unported
secure-input / AppleScript fields:

- `macos-icon: MacAppIcon = official`
- `macos-custom-icon: ?[:0]const u8 = null`
- `macos-icon-frame: MacAppIconFrame = aluminum`
- `macos-icon-ghost-color: ?Color = null`
- `macos-icon-screen-color: ?ColorList = null`

This experiment is parser/formatter-only. Runtime dock icon selection, custom
icon file loading, styled icon rendering, required-field validation for
`custom-style`, default custom-icon path derivation, app C ABI exposure, and
macOS app integration remain later work.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Config` fields for the five macOS icon options after `macos_hidden` and
    before the font-family group in the current local struct/default region.
  - Initialize defaults to upstream values:
    - `macos_icon = MacAppIcon::Official`
    - `macos_custom_icon = None`
    - `macos_icon_frame = MacAppIconFrame::Aluminum`
    - `macos_icon_ghost_color = None`
    - `macos_icon_screen_color = None`
  - Format the five fields after `macos-hidden` and before `bold-color`, filling
    the current local macOS formatter gap after the existing macOS window
    fields. The intervening upstream secure-input / AppleScript fields remain
    later work, so this placement is local-order-compatible rather than claiming
    those fields are already ported.
  - Route `Config::set` for:
    - `macos-icon` through `set_enum_field`;
    - `macos-custom-icon` through `set_optional_value_field` with the existing
      string parser;
    - `macos-icon-frame` through `set_enum_field`;
    - `macos-icon-ghost-color` through `set_optional_value_field` with
      `Color::parse_cli`;
    - `macos-icon-screen-color` through `set_optional_value_field` with a local
      `ColorList` parse wrapper.
  - Add `MacAppIcon` and `MacAppIconFrame` enums with upstream keywords,
    `from_keyword`, `keyword`, and `format_entry`.
  - Extend default-value, enum-route, optional-string/color, and format-order
    tests.
  - Add focused tests for:
    - all upstream `macos-icon` and `macos-icon-frame` keywords, including
      `custom-style`;
    - default formatter output;
    - empty reset behavior;
    - missing and invalid values;
    - `macos-custom-icon` string parsing/formatting;
    - `macos-icon-ghost-color` named/hex color parsing and formatting;
    - `macos-icon-screen-color` color-list parsing, formatting, empty reset,
      missing/invalid diagnostics, and clone/equality.

- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed` in the experiment index.
  - After implementation, add an operating note describing the parser-only
    status and runtime work left open.

## Verification

Before implementation:

- Codex-native adversarial design review approves the experiment.
- Plan commit exists before source edits begin.

After implementation:

- `cargo fmt`
- `cargo test -p roastty macos_icon`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
- `cargo fmt --check`
- `git diff --check`

Pass criteria:

- The five macOS icon config fields are present in defaults, formatter output,
  `Config::set`, and format-order tests in the current local macOS formatter
  region, while preserving the fact that intervening upstream secure-input /
  AppleScript fields remain unported.
- Enum parsing and formatting matches upstream keywords exactly.
- Optional string/color/color-list parsing uses the existing local config
  semantics and diagnoses missing/invalid values.
- Runtime icon behavior is not claimed or changed by this experiment.

## Design Review

Codex adversarial reviewer `019eb52e-5177-7b42-9961-dac04dbb2236` returned
**Approved** with no required findings. The reviewer raised one optional wording
finding and one nit: the initial plan said the icon options were adjacent after
`macos-hidden`, but upstream has still-unported secure-input / AppleScript
fields between `macos-hidden` and the icon subgroup. The design text was
corrected to make that gap explicit and to describe the planned placement as the
current local macOS formatter region, not a claim that the intervening upstream
fields are already ported.
