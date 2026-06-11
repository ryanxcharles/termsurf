+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 71: Phase F — window padding balance config

## Description

Experiment 70 wired `window-padding-x` and `window-padding-y` into the
aggregating `Config`. The next small upstream field is `window-padding-balance`,
declared in `vendor/ghostty/src/config/Config.zig` as
`WindowPaddingBalance = @import("../renderer/size.zig").PaddingBalance` with
default `.false`.

Upstream `PaddingBalance` has three enum tags:

- `false` — no balancing;
- `true` — balance leftover padding with a top-padding cap;
- `equal` — balance equally on all sides.

This experiment adds the config-level surface only: enum representation,
default, parser/formatter wiring, diagnostics, formatter order, and focused
tests. It intentionally does not apply balancing to renderer geometry; the
runtime behavior belongs with the renderer size/padding implementation.

## Changes

- `roastty/src/config/mod.rs`
  - Add `WindowPaddingBalance::{False, True, Equal}`.
  - Add exact keyword parsing/formatting for `false`, `true`, and `equal`.
  - Add `Config::window_padding_balance` with upstream default `False`.
  - Route `window-padding-balance` through defaults, `Config::set`,
    `format_config`, diagnostics, clone/equality, and formatter-order tests.
  - Preserve the upstream window-padding order:
    - `window-padding-x`
    - `window-padding-y`
    - `window-padding-balance`
    - `window-padding-color`

Out of scope:

- Applying the balance mode to renderer viewport geometry.
- Porting `renderer/size.zig` padding calculations.
- Runtime warnings for unreasonable padding values.
- `keybind` and `key-remap`.

## Verification

- Run formatting:
  - `cargo fmt`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/71-window-padding-balance-config.md`
- Run targeted tests:
  - `cargo test -p roastty window_padding_balance_config`
  - `cargo test -p roastty enum_from_keyword_round_trips`
  - `cargo test -p roastty config_format_config`
- Add concrete test cases proving:
  - the default is `False` and formats as `window-padding-balance = false`;
  - `false`, `true`, and `equal` parse and format exactly;
  - empty values reset to the default;
  - missing values return `ValueRequired`;
  - invalid values return `InvalidValue`;
  - `Config::load_str` records a `ConfigDiagnostic` for an invalid
    `window-padding-balance` line while keeping valid neighboring lines;
  - formatter order places `window-padding-balance` after `window-padding-y` and
    before `window-padding-color`;
  - clone/equality preserves the value.
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `git status --short` and verify only intended source/docs are present.

**Pass** = `window-padding-balance` is faithfully represented on `Config`,
round-trips through config loading/formatting, matches upstream keyword/default
behavior, and has targeted and full tests passing.

**Partial** = the enum lands but a parser/diagnostic/formatter-order edge
requires a follow-up before runtime use.

**Fail** = faithful config representation cannot land without first porting
renderer size balancing.

## Design Review

Codex adversarial reviewer `019eb401-4283-7a72-a77e-ac3cc3e00b23` initially
returned **Changes Required** for one verification issue: the design listed the
targeted command `cargo test -p roastty config_enum_keyword_round_trips`, but no
existing or explicitly planned test name matched that filter, so it could run
zero tests.

The design was updated to use the existing enum test filter
`cargo test -p roastty enum_from_keyword_round_trips`. On re-review, the same
reviewer returned **Approved** with no remaining findings.
