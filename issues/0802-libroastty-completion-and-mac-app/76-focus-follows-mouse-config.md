+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 76: Phase F — focus follows mouse config

## Description

Experiment 75 wired the resize-overlay config block. The next upstream field is
`focus-follows-mouse`.

Upstream declares `focus-follows-mouse` as `bool = false` in
`vendor/ghostty/src/config/Config.zig`, immediately after
`resize-overlay-duration` and before the clipboard access fields.

This experiment adds the config parser/formatter surface only. Runtime split
focus behavior driven by mouse movement is out of scope.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Config::focus_follows_mouse = false`.
  - Route `focus-follows-mouse` through defaults, `Config::set`,
    `format_config`, diagnostics, clone/equality, and formatter-order tests.
  - Preserve upstream order after `resize-overlay-duration` and before
    `clipboard-read`.

Out of scope:

- Runtime split focus behavior.
- Mouse movement focus dispatch.
- Clipboard fields; the existing clipboard config group should remain unchanged.
- `title-report`.
- `keybind` and `key-remap`.

## Verification

- Run formatting:
  - `cargo fmt`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/76-focus-follows-mouse-config.md`
- Run targeted tests:
  - `cargo test -p roastty focus_follows_mouse_config`
  - `cargo test -p roastty config_format_config`
- Add concrete test cases proving:
  - the default is `false`;
  - `true`, `false`, bare values, and empty reset follow the existing bool field
    parser semantics;
  - missing values return the bool bare-flag behavior, and invalid values return
    `InvalidValue`;
  - `Config::load_str` records diagnostics for invalid neighboring lines while
    preserving valid values;
  - formatter order matches the upstream sequence around this field;
  - clone/equality preserves the value.
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `git status --short` and verify only intended source/docs are present.

**Pass** = `focus-follows-mouse` is represented faithfully on `Config`,
round-trips through config loading/formatting, matches upstream default and bool
parser behavior, and has targeted and full tests passing.

**Partial** = the field lands faithfully but a parser, diagnostic, or
formatter-order edge requires a follow-up.

**Fail** = this field cannot be represented faithfully without first porting
runtime mouse-focus behavior.

## Design Review

Codex adversarial reviewer `019eb43f-8491-7490-a6cb-b7d22436c052` returned
**Approved** with no required findings. The reviewer confirmed that the README
links Experiment 76 as `Designed`, the experiment is design-only with the
required sections, runtime focus behavior stays out of scope, and the plan
matches upstream `focus-follows-mouse: bool = false` ordering after
`resize-overlay-duration` and before `clipboard-read`. The reviewer also
confirmed the verification plan covers defaults, bool parser/reset semantics,
diagnostics, formatter order, clone/equality, targeted tests, full
`cargo test -p roastty`, `cargo fmt --check`, `git diff --check`, and status
inspection.

## Result

**Result:** Pass

Implemented `focus-follows-mouse` in `roastty/src/config/mod.rs` as a bool
config field with upstream default `false`. The field now routes through
`Config::set`, formats in upstream order after `resize-overlay-duration` and
before `clipboard-read`, and has focused coverage for bool parsing, bare values,
empty reset, invalid diagnostics, and clone/equality.

Verification passed:

- `cargo fmt`
- `cargo test -p roastty focus_follows_mouse_config`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
  - 4513 unit tests passed
  - ABI harness passed with the existing enum-conversion warnings
  - doc tests passed
- `cargo fmt --check`
- `git diff --check`

## Conclusion

The `focus-follows-mouse` config surface now matches the upstream default,
parser behavior, diagnostics, formatter order, and clone/equality expectations.
The next config fields in upstream order are the clipboard access/behavior
fields, which are already represented in Roastty, so the next experiment should
inspect the following unported field after the clipboard group.

## Completion Review

Codex adversarial reviewer `019eb444-b79d-7f62-87d7-a5a667a05ad2` returned
**Approved** with no required findings. The reviewer confirmed that the
implementation is scoped to the `focus-follows-mouse` config surface only,
matches the upstream default and order, uses the existing bool parsing/reset
semantics, updates diagnostics and clone/equality coverage, and that the docs
and README status match the current uncommitted state.

The reviewer independently verified targeted tests, full
`cargo test -p roastty`, `cargo fmt --check`, `git diff --check`, and the
Prettier check.
