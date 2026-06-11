+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 85: Phase F — command palette entry config

## Description

Experiment 84 completed the quick-terminal space and keyboard config fields. The
next upstream config field after the already-ported shell integration block is
`command-palette-entry`.

Upstream declares `command-palette-entry` as `RepeatableCommand = .{}`. It is a
repeatable list of command-palette entries, where each entry has required
`title` and `action` fields and an optional `description` field. Its value
syntax is a comma-separated `key:value` struct parsed with upstream's
comma-aware splitter and Zig quoted-string decoding. The action syntax is the
same typed action syntax used by `keybind`, so this experiment must validate
actions instead of accepting arbitrary raw strings.

This experiment adds the Rust config parser/formatter surface for
`command-palette-entry`, including repeatable append behavior, `clear`, and
empty-value reset to upstream default entries. Runtime command-palette UI
consumption and app C ABI accessors for the command list are out of scope.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Config::command_palette_entry` in upstream declaration order after
    `shell-integration-features` and before `osc-color-report-format`.
  - Add `RepeatableCommand` with upstream semantics:
    - default-initialized configs contain upstream default command-palette
      entries;
    - repeated `command-palette-entry = title:...,action:...` appends entries;
    - `command-palette-entry = clear` clears all entries parsed up to that
      point;
    - `command-palette-entry =` restores upstream defaults;
    - formatter emits one `command-palette-entry = ...` line per entry, or a
      blank entry when the list is empty.
  - Add a `CommandPaletteEntry` value with required `title`, optional
    `description`, required action string, and the parsed action validation
    result needed to prove the action is accepted by Roastty's keybinding action
    parser.
  - Parse entry values with the existing quote-aware `CommaSplitter` and
    `parse_quoted_string`, matching upstream `parseAutoStruct` behavior for
    commas, whitespace, and Zig string escapes.
  - Match upstream duplicate-field behavior: repeated `title`, `description`, or
    `action` fields are allowed, and the last value wins.
  - Add a small crate-visible action validation helper beside the existing
    `parse_config_binding_action` implementation in `roastty/src/lib.rs`, so
    config parsing rejects invalid actions using the same parser that keybind
    config uses.
  - Reject missing required fields, unknown fields, invalid actions, malformed
    quoted strings, malformed comma splitting, and malformed field separators as
    `ConfigSetError::InvalidValue`.
  - Route `command-palette-entry` through `Config::set`, `load_str`,
    `set_cli_args`, clone/equality, diagnostics, and `format_config`.
  - Add focused tests covering defaults, append ordering, `clear`, empty reset,
    formatter output, quoted commas and escapes, invalid values, diagnostics,
    and placement between `shell-integration-features` and
    `osc-color-report-format`.

Out of scope:

- Runtime command-palette UI behavior.
- C ABI command-list exposure.
- Typed command-palette dispatch beyond parse-time action validation.
- Broader formatter reordering of unrelated fields.
- The following upstream fields, including `osc-color-report-format`,
  `vt-kam-allowed`, and custom shader settings.

## Verification

- Run formatting:
  - `cargo fmt`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/85-command-palette-entry-config.md`
- Run targeted tests:
  - `cargo test -p roastty command_palette_entry`
  - `cargo test -p roastty config_format_config`
- Add concrete test cases proving:
  - default configs contain the expected upstream default entries for this
    pinned Ghostty version;
  - appending entries preserves parse order;
  - `clear` empties entries already present, and later entries append from an
    empty list;
  - an empty value restores defaults after `clear`;
  - formatter emits one line per entry and emits a blank entry for an empty
    list;
  - quoted commas and Zig escapes decode in `title`, `description`, and
    `action`;
  - unquoted actions such as `csi:0m`, `text:hello`, and `goto_split:right`
    preserve their full action strings and pass action validation;
  - invalid actions are rejected through the same action parser used by keybind
    config;
  - duplicate fields are accepted with last-value-wins semantics;
  - missing `title`, missing `action`, unknown fields, malformed quoted strings,
    malformed comma splitting, and missing values are diagnosed as invalid
    values;
  - `Config::load_str` records diagnostics for invalid neighboring
    `command-palette-entry` lines while preserving valid parsed entries;
  - clone/equality preserves the command list;
  - default `format_config` places `command-palette-entry` after
    `shell-integration-features` and before `osc-color-report-format`.
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `git status --short` and verify only intended source/docs are present.

**Pass** = `command-palette-entry` is represented faithfully on `Config`,
round-trips through config loading/formatting, matches upstream default list
semantics and parser behavior for this slice, and has targeted and full tests
passing.

**Partial** = the field lands for explicit custom entries, but upstream default
entries or `clear` / empty-reset semantics require a follow-up.

**Fail** = `command-palette-entry` cannot be represented faithfully without
first implementing runtime command-palette behavior or the app C ABI command
list.

## Design Review

Codex adversarial reviewer `019eb4bc-9290-77d3-a742-ca954948fac1` returned
**Changes Required** with two required findings:

- The original design treated `action` as an unvalidated raw string even though
  upstream parses it as a typed `input.Command.Action`. Accepted: this design
  now requires parse-time action validation through the same parser used by
  keybind config.
- The original design rejected duplicate struct fields, but upstream
  `parseAutoStruct` allows duplicates and later fields overwrite earlier ones.
  Accepted: this design now requires last-value-wins duplicate-field behavior.

Codex adversarial reviewer `019eb4bf-3d12-72b2-a172-483d406da28a` re-reviewed
the fixes and returned **Approved** with no remaining findings. The reviewer
confirmed the action-validation and duplicate-field findings were resolved and
that the issue README links Experiment 85 as `Designed`.

## Result

**Result:** Pass

Implemented `command-palette-entry` in `roastty/src/config/mod.rs` as a
`RepeatableCommand` containing `CommandPaletteEntry` values with `title`,
`description`, and `action` strings. `Config::default` now initializes the
pinned upstream command-palette defaults: 88 entries sorted in upstream title
order.

The parser now matches the important upstream `RepeatableCommand` behavior for
this slice:

- repeated `command-palette-entry` lines append entries;
- `command-palette-entry = clear` clears all entries seen so far;
- empty and missing values restore the upstream defaults;
- duplicate `title`, `description`, or `action` fields are accepted with
  last-value-wins semantics;
- field splitting respects quoted commas via the existing upstream-style
  `CommaSplitter`;
- quoted values decode Zig string escapes via `parse_quoted_string`;
- actions are validated and canonicalized through Roastty's existing config
  keybinding action parser before the entry is accepted;
- formatter output emits one `command-palette-entry = ...` line per entry, or a
  blank line for an empty list.

The implementation also added `crate::canonical_config_binding_action` in
`roastty/src/lib.rs` so config parsing can validate and canonicalize action
syntax without duplicating the app-layer keybinding action parser.

The plan said missing values should diagnose as invalid, but upstream
`RepeatableCommand.parseCLI` maps a missing input to an empty string and then
restores defaults. The implementation follows upstream and tests both `Some("")`
and `None` as default restoration.

Verification passed:

- `cargo fmt`
- `cargo test -p roastty command_palette_entry`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
  - 4523 unit tests passed
  - ABI harness passed with the existing 10 enum-conversion warnings
  - doc tests passed
- `cargo fmt --check`
- `git diff --check`

## Conclusion

`command-palette-entry` now has a faithful parser/formatter config surface for
the pinned Ghostty default list, repeatable semantics, `clear`, default restore,
duplicate-field handling, quoted values, and action validation. Runtime
command-palette UI consumption and C ABI command-list exposure remain later
work. The next upstream config field after this slice is
`osc-color-report-format`, which is already present locally, so the next config
gap audit should continue after that field.

## Completion Review

Codex adversarial reviewer `019eb4ca-3d82-7d32-938a-4941c978982f` returned
**Changes Required** with two required findings:

- The first implementation validated `action` but stored the accepted raw input,
  so shorthand values such as `copy_to_clipboard` would format back differently
  from upstream's parsed typed action formatter. Accepted: `roastty/src/lib.rs`
  now exposes `canonical_config_binding_action`, and
  `CommandPaletteEntry::parse_auto_struct` stores the canonical action string.
- The first tests checked default count, first entry, last entry, one middle
  entry, and parseability, but did not prove the full pinned upstream default
  list. Accepted: `command_palette_entry_config_parse_format_reset_and_diagnose`
  now includes an independent 88-entry ordered snapshot assertion.

Codex adversarial reviewer `019eb4db-14fa-7962-b6db-5b9739b18569` re-reviewed
the fixes and returned **Approved** with no remaining findings. The reviewer
verified the canonical action storage fix, the full ordered default-list
snapshot, `cargo test -p roastty command_palette_entry`,
`cargo test -p roastty config_format_config`, `cargo fmt --check`, and
`git diff --check`.
