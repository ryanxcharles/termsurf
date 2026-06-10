+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 61: Phase F — split visual config surface

## Description

Experiment 60 completed the cursor default group through terminal initialization
and renderer cursor opacity. The next narrow Phase-F slice is the adjacent
split-visual config group from upstream `Config.zig`:

- `unfocused-split-opacity`
- `unfocused-split-fill`
- `split-divider-color`
- `split-preserve-zoom`

Roastty does not yet own the full app split-tree rendering policy; that remains
part of the app/live-render work. This experiment therefore lands the faithful
config surface and helper semantics only: defaults, parsing, formatting,
diagnostics, clone/equality, and focused tests. Runtime split dimming, divider
painting, and zoom-preservation behavior are explicitly out of scope until the
split tree / renderer ownership is present in a later slice.

## Changes

- `roastty/src/config/mod.rs`
  - Add upstream defaults:
    - `unfocused-split-opacity = 0.7`
    - `unfocused-split-fill = null` / unset
    - `split-divider-color = null` / unset
    - `split-preserve-zoom =` no flags set
  - Add `SplitPreserveZoom` as a packed-flag config type with the upstream
    `navigation` flag and `[no-]navigation` formatting/parsing behavior.
  - Route all four fields through `Config::set`, `format_config`, default
    construction, clone/equality, and diagnostics.
  - Preserve upstream declaration/formatter order immediately after
    `background-blur` and before the search color group:
    `unfocused-split-opacity`, `unfocused-split-fill`, `split-divider-color`,
    `split-preserve-zoom`.
  - Keep `unfocused-split-opacity` parse permissive like upstream, then clamp it
    in `Config::finalize` to `[0.15, 1.0]`, matching upstream `Config.finalize`.
  - Reuse the existing `Color` parser/formatter for `unfocused-split-fill` and
    `split-divider-color`, including empty-value reset to `None`.

Out of scope:

- Applying `unfocused-split-opacity` or `unfocused-split-fill` to live renderer
  focus dimming.
- Applying `split-divider-color` to any app split divider drawing.
- Implementing zoom-preservation behavior for split navigation.
- Search colors; they are the next adjacent color group and should get their own
  slice because they feed search highlight rendering.

## Verification

- Run formatting:
  - `cargo fmt -- roastty/src/config/mod.rs`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/61-split-visual-config-surface.md`
- Run targeted tests:
  - `cargo test -p roastty split_visual_config`
  - `cargo test -p roastty split_preserve_zoom`
  - `cargo test -p roastty config_format_config`
- Add concrete test cases proving:
  - defaults match upstream;
  - formatter order places the four fields after `background-blur` and before
    the search color group / next existing fields;
  - `unfocused-split-opacity` round-trips raw in-range and out-of-range floats
    before `Config::finalize`, clamps finalized parsed values below `0.15` and
    above `1.0`, and resets to `0.7` on an empty value;
  - `unfocused-split-fill` and `split-divider-color` accept colors, format as
    hex, reject invalid colors, and reset to blank/`None`;
  - `split-preserve-zoom` accepts `navigation`, `no-navigation`, and standalone
    bool forms, rejects unknown flags, and resets to the empty/default flag set.
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run `git diff --check`.
- Run `git status --short` and verify only intended source/docs are present.

**Pass** = the four split visual config fields are represented faithfully on
`Config`, round-trip through config loading/formatting, preserve upstream order,
and have targeted and full tests passing.

**Partial** = scalar/color fields land but `split-preserve-zoom` exposes a
parser or formatter prerequisite that should be split out.

**Fail** = the fields cannot be represented without broader split-tree or app
runtime ownership.

## Design Review

Codex adversarial reviewer `019eb381-c42b-7823-997c-a6564fa886ab` returned
**Changes required**. The required finding was valid: the first draft deferred
`unfocused-split-opacity` clamping to a future runtime use site, but upstream
parses permissively and clamps during `Config.finalize`. The design now requires
`Config::finalize` to clamp the field to `[0.15, 1.0]` and requires tests for
both raw pre-finalize round-trip behavior and post-finalize clamping.

Re-review returned **Approved**. The reviewer confirmed the required finding is
resolved because the design now explicitly requires permissive parsing plus
`Config::finalize` clamping to `[0.15, 1.0]`, with verification for raw
pre-finalize round-trip behavior and post-finalize clamping.

## Result

**Result:** Pass

Implemented the split visual config surface in `roastty/src/config/mod.rs`:

- added `unfocused_split_opacity`, `unfocused_split_fill`,
  `split_divider_color`, and `split_preserve_zoom` to `Config`;
- added upstream defaults and formatter order immediately after
  `background-blur`;
- routed all four keys through `Config::set` with empty-value resets and
  diagnostics;
- added `SplitPreserveZoom` as a packed flag config type with `navigation` /
  `no-navigation` and standalone bool parsing;
- added `Config::finalize` clamping for `unfocused_split_opacity` to
  `[0.15, 1.0]`;
- added focused tests for defaults, formatter order, raw pre-finalize float
  round-trips, post-finalize clamping, optional color parse/reset/error
  behavior, packed flag parse/reset/error behavior, and clone/equality.

Verification:

- `cargo fmt -- roastty/src/config/mod.rs` passed.
- `cargo test -p roastty split_visual_config` passed.
- `cargo test -p roastty split_preserve_zoom` passed.
- `cargo test -p roastty config_format_config` passed.
- `cargo test -p roastty` passed: 4495 unit tests, ABI harness, and doc-tests.
  The ABI harness still emits the existing enum conversion warnings.
- `cargo fmt --check` passed.
- `git diff --check` passed.

## Conclusion

The Phase-F split visual config keys now exist as a faithful config surface with
the upstream finalize-time opacity clamp. Runtime split dimming, divider
painting, and zoom-preservation behavior remain intentionally out of scope until
the app split tree / renderer ownership is present.

## Completion Review

Codex adversarial reviewer `019eb38c-48ba-7d90-9918-d37834e77a46` returned
**Approved** with no Required, Optional, or Nit findings.

The reviewer independently verified the implementation scope, upstream defaults,
formatter order, parsing/reset behavior, finalize clamp, README `Pass` status,
and Result/Conclusion sections. It also reran:

- `cargo test -p roastty split_visual_config`
- `cargo test -p roastty split_preserve_zoom`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
- `cargo fmt --check`
- `git diff --check`
