+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5"
reasoning = "medium"
+++

# Experiment 749: Config CLI Keybind Triggers

## Description

Add the first real Roastty keybind parsing and storage path behind
`roastty_config_load_cli_args`. Ghostty's C API receives process argv through
`ghostty_init(argc, argv)` and later applies CLI configuration through
`ghostty_config_load_cli_args(config)`. Roastty currently discards argv and
`roastty_config_trigger` only returns static default triggers.

This experiment stores process argv during `roastty_init`, parses simple
root-table `--keybind` CLI options, stores the resulting action-to-trigger
reverse map on `Config`, and makes `roastty_config_trigger` prefer configured
bindings before falling back to static defaults.

This is intentionally not the full keybinding system. It does not implement
config file loading, table keybinds, key sequences, `chain`, `clear`, `unbind`,
global/all prefixes, diagnostics, or custom surface key dispatch.

## Changes

- `roastty/src/lib.rs`
  - Store a process-local, deep-copied byte copy of argv passed to
    `roastty_init`; never borrow caller-owned argv memory.
  - Treat `argc == 0`, null `argv`, and null argv entries as an empty argv list
    for config-loading purposes. A later `roastty_init` call replaces the
    previously captured argv copy.
  - Teach `roastty_config_load_cli_args` to recognize `--keybind=value` and
    `--keybind value`.
  - Parse simple root keybinds in the form `modifier+modifier+key=action`, using
    `+` only on the trigger side and the first `=` as the trigger/action
    separator.
  - Reject empty trigger, empty action, empty trigger components, duplicate
    modifiers, unknown trigger components, missing key, and more than one key
    component.
  - Preserve the action bytes after the first `=`, so values such as
    `ctrl+e=text:=` can be stored even though the action contains `=`.
  - Support lowercase Ghostty modifier spellings: `shift`, `ctrl`, `control`,
    `alt`, `opt`, `option`, `super`, `cmd`, and `command`; uppercase/mixed-case
    modifier spellings are rejected for this experiment.
  - Support one-character Unicode triggers plus a focused physical-key subset
    useful for menu/display bindings: lowercase `key_a`..`key_z`, Ghostty-style
    `KeyA`..`KeyZ`, lowercase `digit_0`..`digit_9`, Ghostty-style
    `Digit0`..`Digit9`, `copy`, `paste`, `escape`, `arrow_up`, `arrow_down`,
    `arrow_left`, `arrow_right`, `home`, `end`, `page_up`, `page_down`, `enter`,
    `tab`, `backspace`, and `insert`.
  - Store bindings as a reverse action-to-trigger map on `Config`; later
    duplicate actions replace earlier triggers, matching Ghostty's menu-trigger
    behavior where the last binding for an action is displayed.
  - Clone stored keybind triggers with `roastty_config_clone`.
  - Make `roastty_config_trigger` return the configured trigger first and then
    the static default trigger when no custom binding exists.
  - Keep malformed or unsupported keybind CLI values ignored without diagnostics
    for this experiment.
- `roastty/tests/abi_harness.c`
  - Add C coverage for `roastty_init(argc, argv)` plus
    `roastty_config_load_cli_args(config)` loading split and equals-form
    `--keybind` values.
  - Assert custom triggers override defaults, clone correctly, and malformed
    keybinds do not affect fallback defaults.
  - Assert null argv, `--keybind` as the final arg, `--keybind` followed by
    another option, `--keybind=`, malformed values mixed with later valid
    values, and multiple `--keybind` options for the same action behave as
    specified.
- Tests in `roastty/src/lib.rs`
  - Cover CLI argv capture and keybind loading.
  - Cover modifier aliases, Unicode triggers, physical triggers, duplicate
    action replacement, clone behavior, null/no-argv false paths, and malformed
    values falling back to defaults.
  - Cover duplicate modifier rejection, unknown component rejection, missing key
    rejection, missing action rejection, action values containing `=`, and later
    `roastty_init` calls replacing previously captured argv.

## Verification

- `cargo test -p roastty config_cli_keybind -- --nocapture --test-threads=1`
- `cargo test -p roastty config_trigger -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness -- --nocapture`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the initial Experiment 749 design and found three blocking
specification gaps: the accepted trigger grammar was underspecified, argv
lifetime and reinitialization behavior was not explicit enough for a C ABI, and
CLI option edge cases needed to be named in the tests.

The design was updated to pin the grammar to `modifier+modifier+key=action`,
define case-sensitivity and rejection behavior, preserve action bytes after the
first `=`, require `roastty_init` to deep-copy argv and replace previous
captures on later calls, and add explicit tests for
dangling/split/empty/malformed `--keybind` options.

Codex re-reviewed the corrected design and approved it for the plan commit. The
review confirmed that the scope is narrow enough to add custom reverse trigger
lookup through CLI-loaded config without implying custom key dispatch, key
tables, sequences, unbinds, or diagnostics.
