+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 75: Phase F — resize overlay config

## Description

Experiment 74 wired the tab/titlebar config fields immediately before the
resize-overlay block. The next upstream fields are:

- `resize-overlay`
- `resize-overlay-position`
- `resize-overlay-duration`

Upstream declares `resize-overlay` as `ResizeOverlay = .@"after-first"`,
`resize-overlay-position` as `ResizeOverlayPosition = .center`, and
`resize-overlay-duration` as
`Duration = .{ .duration = 750 * std.time.ns_per_ms }` in
`vendor/ghostty/src/config/Config.zig`.

This experiment adds the config parser/formatter surface only. Runtime resize
overlay rendering and timing behavior are out of scope.

## Changes

- `roastty/src/config/mod.rs`
  - Add `ResizeOverlay::{Always, Never, AfterFirst}`.
  - Add
    `ResizeOverlayPosition::{Center, TopLeft, TopCenter, TopRight, BottomLeft, BottomCenter, BottomRight}`.
  - Add `Config::resize_overlay = AfterFirst`.
  - Add `Config::resize_overlay_position = Center`.
  - Add
    `Config::resize_overlay_duration = Duration { duration: 750 * NS_PER_MS }`.
  - Route all three keys through defaults, `Config::set`, `format_config`,
    diagnostics, clone/equality, enum keyword tests, and formatter-order tests.
  - Preserve upstream order after the titlebar color fields:
    - `resize-overlay`
    - `resize-overlay-position`
    - `resize-overlay-duration`

Out of scope:

- Runtime resize overlay rendering.
- Runtime resize overlay positioning.
- Runtime resize overlay timers.
- `focus-follows-mouse`.
- Clipboard fields.
- `keybind` and `key-remap`.

## Verification

- Run formatting:
  - `cargo fmt`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/75-resize-overlay-config.md`
- Run targeted tests:
  - `cargo test -p roastty resize_overlay_config`
  - `cargo test -p roastty enum_from_keyword_round_trips`
  - `cargo test -p roastty config_format_config`
- Add concrete test cases proving:
  - enum defaults are `after-first` and `center`;
  - enum values parse, format, reset on empty values, return `ValueRequired` on
    missing values, and return `InvalidValue` on unknown values;
  - `resize-overlay-duration` defaults/formats as `750ms`, parses composite
    durations such as `1s 250ms`, resets on empty values, returns
    `ValueRequired` on missing values, and returns `InvalidValue` on invalid
    duration values;
  - `Config::load_str` records diagnostics for invalid neighboring enum/duration
    lines while preserving valid values;
  - formatter order matches the upstream sequence around these fields;
  - clone/equality preserves all three values.
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `git status --short` and verify only intended source/docs are present.

**Pass** = the three resize-overlay fields are represented faithfully on
`Config`, round-trip through config loading/formatting, match upstream defaults
and parser behavior, and have targeted and full tests passing.

**Partial** = some fields land faithfully but a parser, diagnostic, or
formatter-order edge requires a follow-up.

**Fail** = these fields cannot be represented faithfully without first porting
runtime resize overlay behavior.

## Design Review

Codex adversarial reviewer `019eb435-06d3-7fb3-a4d5-3f0d11425387` returned
**Approved** with no required findings. The reviewer confirmed that the README
links Experiment 75 as `Designed`, the design has the required sections, the
scope is limited to the parser/formatter config surface, and the planned fields,
defaults, enum variants, formatter order, reset/error behavior, diagnostics,
clone/equality coverage, and tests match upstream and the surrounding Rust
config patterns.

## Result

**Result:** Pass

Implemented the three planned resize overlay config fields in
`roastty/src/config/mod.rs`. `ResizeOverlay` and `ResizeOverlayPosition` now
parse and format their upstream enum keywords, `resize-overlay-duration` uses
the existing `Duration` parser/formatter path, and all three fields preserve the
upstream defaults and formatter order.

Verification passed:

- `cargo fmt`
- `cargo test -p roastty resize_overlay_config`
- `cargo test -p roastty enum_from_keyword_round_trips`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
  - 4512 unit tests passed
  - ABI harness passed with the existing enum-conversion warnings
  - doc tests passed
- `cargo fmt --check`
- `git diff --check`

## Conclusion

The resize-overlay config surface now matches the upstream defaults, parser
behavior, duration handling, formatter order, diagnostics, and clone/equality
requirements for this slice. The next experiment can continue with
`focus-follows-mouse`, the field that follows the resize-overlay block upstream.

## Completion Review

Codex adversarial reviewer `019eb43c-7257-71d0-ae28-910ca1dbfcd8` returned
**Approved** with no required findings. The reviewer confirmed that the
implementation matches the approved scope and upstream defaults/keywords,
`Duration` uses the existing parser/formatter path, `Config::set`
reset/error/diagnostic paths are covered, formatter order is correct, docs and
README status match the current Pass state, focused tests plus
`cargo fmt --check` and `git diff --check` pass, and HEAD remains at the plan
commit with only uncommitted result changes.
