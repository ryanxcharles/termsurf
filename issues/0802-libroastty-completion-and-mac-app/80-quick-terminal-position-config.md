+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 80: Phase F — quick terminal position config

## Description

Experiment 79 wired `undo-timeout`. The next unported upstream config field is:

- `quick-terminal-position`

Upstream declares this as `QuickTerminalPosition = .top` in
`vendor/ghostty/src/config/Config.zig`. The enum variants are `top`, `bottom`,
`left`, `right`, and `center`. It controls the position of Ghostty's quick
terminal window; changing it on macOS requires a full app restart.

This experiment adds the config parser/formatter surface only. Quick-terminal
runtime behavior, toggle actions, layout calculation, restart requirements, and
the following `quick-terminal-size` struct are out of scope.

## Changes

- `roastty/src/config/mod.rs`
  - Add a `QuickTerminalPosition` enum with the upstream variants:
    - `top`
    - `bottom`
    - `left`
    - `right`
    - `center`
  - Add `Config::quick_terminal_position` with upstream default `Top`.
  - Route `quick-terminal-position` through `Config::set`, config loading
    diagnostics, clone/equality, and formatting.
  - Preserve the current local formatter convention by inserting the key after
    `undo-timeout`, the previous upstream field in this local app-lifecycle
    block.

Out of scope:

- Runtime quick-terminal creation, positioning, restart behavior, and toggle
  actions.
- `quick-terminal-size`, the next upstream field after
  `quick-terminal-position`.
- GTK-only quick-terminal layer/namespace fields.
- Any broader formatter reordering of already-ported keys.

## Verification

- Run formatting:
  - `cargo fmt`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/80-quick-terminal-position-config.md`
- Run targeted tests:
  - `cargo test -p roastty quick_terminal_position_config`
  - `cargo test -p roastty config_format_config`
- Add concrete test cases proving:
  - the default formats as `quick-terminal-position = top`;
  - all five upstream keywords parse and format;
  - an empty value resets the field to `top`;
  - a missing value returns `ValueRequired`;
  - invalid values return `InvalidValue`;
  - `Config::load_str` records diagnostics for invalid neighboring
    `quick-terminal-position` lines while preserving valid parsed values;
  - formatter order includes the key immediately after `undo-timeout`;
  - clone/equality preserves the enum value.
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `git status --short` and verify only intended source/docs are present.

**Pass** = the quick-terminal-position field is represented faithfully on
`Config`, round-trips through config loading/formatting, matches the upstream
default and enum keyword behavior, and has targeted and full tests passing.

**Partial** = the parser/formatter field lands, but enum coverage, diagnostics,
or formatter-order coverage requires a follow-up.

**Fail** = the key cannot be represented faithfully without first implementing
quick-terminal runtime behavior.

## Design Review

Codex adversarial reviewer `019eb46d-5224-77e0-80c0-45c621c996c3` returned
**Approved** with no required findings. The reviewer confirmed that the README
links Experiment 80 as `Designed`, the experiment has the required sections, the
scope is narrow and limited to `quick-terminal-position`, the plan matches
upstream default and enum tags, the formatter placement after `undo-timeout` is
explicit and matches the current local ordering, and the verification plan
includes the required formatting, targeted test, full test, hygiene, and status
checks.
