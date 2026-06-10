+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 66: Phase F — scrollbar config

## Description

Experiment 65 added the scalar launch/runtime config fields through
`scrollback-limit`. The next upstream config field is:

- `scrollbar`

Upstream declares `scrollbar: Scrollbar = .system` in
`vendor/ghostty/src/config/Config.zig`, with `Scrollbar` containing two values:

- `system`
- `never`

This experiment ports that config surface only: the enum, the `Config` field,
defaults, parsing/reset behavior, formatting, diagnostics, and focused tests.
Runtime scrollbar UI behavior is intentionally out of scope because roastty does
not yet have the live app scrollbar pass wired into this config path.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Scrollbar` as a clone/copy/equatable enum:
    - `System`
    - `Never`
  - Add `Scrollbar::keyword`, `Scrollbar::from_keyword`, and
    `Scrollbar::format_entry`.
  - Add `Config::scrollbar: Scrollbar = Scrollbar::System`.
  - Route `scrollbar` through defaults, `Config::set`, `format_config`,
    clone/equality, and diagnostics.
  - Preserve local formatter order after `scrollback-limit` and before
    `link-previews`, leaving the still-unported upstream `link` / `link-url`
    fields for later experiments.

Out of scope:

- Runtime scrollbar display behavior.
- `link`, `link-url`, `maximize`, and later window config fields.
- Applying `scrollback-limit` or scrollbar state to terminal allocation or app
  UI.

## Verification

- Run formatting:
  - `cargo fmt -- roastty/src/config/mod.rs`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/66-scrollbar-config.md`
- Run targeted tests:
  - `cargo test -p roastty scrollbar_config`
  - `cargo test -p roastty config_format_config`
- Add concrete test cases proving:
  - the default is `system`;
  - both upstream keywords parse and format;
  - empty values reset to `system`;
  - missing values return `ValueRequired`;
  - invalid values return `InvalidValue`;
  - `Config::load_str` records `ConfigDiagnostic` line/key/error entries for an
    invalid `scrollbar` line while keeping valid neighboring lines;
  - formatter order places `scrollbar` after `scrollback-limit` and before
    `link-previews`;
  - clone/equality preserves the value.
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `git status --short` and verify only intended source/docs are present.

**Pass** = `scrollbar` is represented faithfully on `Config`, round-trips
through config loading/formatting, matches upstream enum parser behavior, and
has targeted and full tests passing.

**Partial** = the enum lands but a parser, diagnostic, or formatter-order edge
needs a follow-up before runtime use.

**Fail** = `scrollbar` cannot be represented faithfully without first porting
broader scrollback or app UI infrastructure.

## Design Review

Codex adversarial reviewer `019eb3c5-1380-7260-8a8c-1216817093d5` returned
**Approved** with no findings.

The reviewer verified that the README links Exp66 as `Designed`, the scope is a
narrow config-surface slice, deferring `link` / `link-url` and runtime UI
behavior is acceptable, and the plan matches upstream `Scrollbar` values and
default.
