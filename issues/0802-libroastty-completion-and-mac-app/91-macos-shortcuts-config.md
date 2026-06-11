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

## Result

**Result:** Pass

Implemented parser/formatter-only support for upstream `macos-shortcuts` in
`roastty/src/config/mod.rs`:

- added `Config::macos_shortcuts` after the macOS icon subgroup;
- defaulted it to upstream `MacShortcuts::Ask`;
- formatted `macos-shortcuts` after `macos-icon-screen-color` and before
  `bold-color`;
- routed `Config::set("macos-shortcuts", ...)` through `set_enum_field`;
- added `MacShortcuts::{Allow,Deny,Ask}` with exact upstream keywords `allow`,
  `deny`, and `ask`;
- extended default audits, enum-route coverage, format-order coverage, enum
  keyword round trips, and a focused parse/format/reset/diagnostic test.

Verification:

- `cargo fmt`
- `cargo test -p roastty macos_shortcuts` — pass
- `cargo test -p roastty config_format_config` — pass
- `cargo test -p roastty surface_mouse_button_reporting_honors_surface_mouse_reporting_gate`
  — pass after the first full-suite run hit that unrelated surface test once
- `cargo test -p roastty` — pass on rerun: 4535 unit tests, C ABI harness pass,
  doc tests pass; the ABI harness still emits the pre-existing 10
  enum-conversion warnings
- `cargo fmt --check`
- `git diff --check`

## Conclusion

The upstream macOS Shortcuts config toggle now exists in roastty's parser and
formatter with the expected default, keywords, reset semantics, diagnostics, and
format-order placement. This remains intentionally parser/formatter-only:
runtime Shortcuts authorization, action dispatch, app C ABI exposure, and macOS
app integration remain later work.

## Completion Review

Codex-native adversarial reviewer `019eb549-271d-78b0-82b9-9296d8c8a9ff`
reviewed the completed experiment with fresh context and returned **Approved**
with no findings.
