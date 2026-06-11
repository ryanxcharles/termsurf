+++
implementer = "codex"
review_design = "codex-adversarial"
+++

# Experiment 91: Phase F — macOS Shortcuts config

## Description

Port the pinned upstream `macos-shortcuts` config option from
`vendor/ghostty/src/config/Config.zig` into `roastty/src/config/mod.rs`.

Upstream defines this immediately after the macOS icon subgroup:

- `macos-shortcuts: MacShortcuts = ask`

This experiment is parser/formatter-only. Runtime Shortcuts authorization,
Shortcuts action dispatch, AppleScript/Shortcuts integration, app C ABI
exposure, and macOS app integration remain later work.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Config::macos_shortcuts: MacShortcuts` after the macOS icon subgroup
    and before the current local font-family group.
  - Initialize the default to upstream's `MacShortcuts::Ask`.
  - Format `macos-shortcuts` after `macos-icon-screen-color` and before
    `bold-color`, preserving the current local macOS formatter region.
  - Route `Config::set("macos-shortcuts", ...)` through `set_enum_field`.
  - Add `MacShortcuts` enum variants and exact upstream keywords:
    - `allow`
    - `deny`
    - `ask`
  - Extend default-value, enum-route, format-order, and enum keyword round-trip
    tests.
  - Add a focused test for parse/format/reset/diagnostics and clone/equality.

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
- `cargo test -p roastty macos_shortcuts`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
- `cargo fmt --check`
- `git diff --check`

Pass criteria:

- `macos-shortcuts` is present in defaults, formatter output, `Config::set`, and
  format-order tests in the current local macOS formatter region.
- Enum parsing and formatting matches upstream keywords exactly.
- Empty values reset to default, missing values return `ValueRequired`, and
  invalid values return `InvalidValue`.
- Runtime Shortcuts behavior is not claimed or changed by this experiment.

## Design Review

Codex-native adversarial reviewer `019eb53d-3e83-7083-b7e5-8a0927198351`
reviewed the design with fresh context and returned **Approved** with no
findings.
