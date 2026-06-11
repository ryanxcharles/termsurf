# Experiment 116: Phase G — key-table syntax storage

## Description

Add the parser/storage foundation for upstream key-table syntax in Roastty's
configured keybinding path.

Upstream Ghostty accepts keybindings of the form `table-name/trigger=action`.
Those bindings are stored in a named key table and are not part of the root
binding set until a runtime action such as `activate_key_table:table-name`
activates that table. Upstream also treats `table-name/` as a table definition /
clear operation, and it avoids mistaking slash keys or slashes in action
parameters for table delimiters.

Roastty currently stores all configured keybindings in one root
`keybind_triggers` vector. This experiment adds named-table parse and storage
only. It does not implement `activate_key_table`, table stacks, one-shot tables,
`deactivate_key_table`, `chain=`, multi-key sequences, or table runtime lookup.

## Changes

- `roastty/src/lib.rs`
  - Add owned named-table storage to `Config` and `App`, alongside the existing
    root `keybind_triggers`.
  - Replace the current `parse_config_keybind` return value with a small parsed
    entry enum so CLI loading can distinguish:
    - root binding;
    - table binding (`table-name/trigger=action`);
    - table clear/definition (`table-name/`).
  - Add table-delimiter detection that mirrors upstream's shape:
    - only scan for `/` before the first `=`;
    - an empty table name is not a table delimiter, so `/=text:foo` remains a
      root slash-key binding;
    - table names containing `+` or `>` are not table delimiters, so
      `ctrl+/=text:foo` and sequence-like slash triggers remain root-trigger
      parse attempts;
    - slashes after `=` are action parameters, not table delimiters.
  - Store table bindings under their table name without adding them to the root
    `keybind_triggers` vector.
  - Clear a table's stored bindings when parsing `table-name/`.
  - Clone table storage through `roastty_config_clone`, `roastty_app_new`, and
    `roastty_app_update_config`, but do not use table storage for runtime
    key-event matching yet.
  - Keep root binding behavior unchanged for config/app/surface lookup,
    `roastty_config_trigger`, default binding fallback, and diagnostics.
- `roastty/tests/abi_harness.c`
  - Add C ABI coverage proving CLI table keybinds parse without diagnostics, do
    not affect root `roastty_config_key_is_binding_handle` /
    `roastty_surface_key_is_binding_handle`, and survive config clone/app copy
    storage without crashing.
- Tests in `roastty/src/lib.rs`
  - Parse and store a table binding without adding it to root lookup.
  - Store multiple bindings in one table and independent bindings in different
    tables.
  - Clear an existing table with `table-name/`.
  - Preserve root slash behavior for `/=text:foo`, `ctrl+/=text:foo`, and
    slashes in action parameters such as `x=text:/hello`.
  - Preserve table slash-key behavior for `mytable//=text:foo`, storing the
    slash-key binding in `mytable` without adding a root binding.
  - Preserve existing malformed keybind diagnostics for invalid root bindings.

## Verification

- Add the unit and ABI-harness coverage above.
- Run:
  - `cargo test -p roastty key_table`
  - `cargo test -p roastty parse_config_keybind`
  - `cargo test -p roastty config_cli_keybind`
  - `cargo test -p roastty surface_key`
  - `cargo test -p roastty --test abi_harness`
  - `cargo test -p roastty -- --test-threads=1`
  - if the known foreground-PID or mouse-reporting races fail, rerun the failing
    test in isolation, then rerun `cargo test -p roastty -- --test-threads=1`
  - `cargo fmt`
  - `cargo fmt --check`
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/116-key-table-syntax-storage.md issues/0802-libroastty-completion-and-mac-app/README.md`

## Design Review

Codex-native adversarial review ran in a fresh-context subagent
(`multi_agent_v1.spawn_agent`, agent `019eb734-c917-7de0-ad41-69a24fcd66f3`).

Initial verdict: **Changes required.** The reviewer found that the first design
did not require coverage for named-table slash-key disambiguation. Upstream has
explicit table syntax cases such as `mytable//=text:foo`, and root slash-key
tests alone would not prove that table slash-key bindings are stored in the
named table rather than parsed as root bindings.

Fix: the verification plan now requires `mytable//=text:foo` coverage, proving
the slash-key binding is stored in `mytable` without adding a root binding.

Final verdict after re-review: **Approved.** The reviewer confirmed the prior
finding was resolved and reported no new required findings.

## Completion Review

Codex-native adversarial review ran in a fresh-context subagent
(`multi_agent_v1.spawn_agent`, agent `019eb741-402c-7352-9145-f6479ef5d1fd`).

Verdict: **Approved.** The reviewer reported no required findings. It confirmed
that table entries are parsed and stored separately from root bindings, runtime
lookup remains scoped to root bindings, clone/app/update propagation is present,
and the Rust plus C ABI tests cover the intended storage and inert-runtime
behavior.

## Result

**Result:** Pass

Roastty now parses upstream-style key-table syntax in the configured keybinding
CLI path and stores named-table bindings separately from the root configured
binding vector. `Config` owns `keybind_tables`, `App` keeps a cloned copy, and
the storage is preserved through `roastty_config_clone`, `roastty_app_new`, and
`roastty_app_update_config`.

Root keybinding behavior remains unchanged. Table bindings are intentionally
inert for `roastty_config_trigger`, `roastty_config_key_is_binding_handle`, app
key lookup, and surface key lookup until a later experiment implements runtime
table activation.

Implemented coverage:

- `foo/a=quit` stores a table binding under `foo` without adding a root binding.
- Multiple named tables store independent bindings.
- `foo/` clears the `foo` table without affecting other tables.
- `/=text:foo`, `ctrl+/=text:foo`, and `x=text:/hello` remain root bindings.
- `mytable//=text:foo` stores a slash-key binding under `mytable`.
- Invalid table triggers use the existing keybind diagnostic path.
- The C ABI harness loads table keybinds from CLI args without diagnostics,
  proves they do not affect root config/surface binding checks, and exercises
  config clone/app/surface copy paths.

Verification run:

- `cargo test -p roastty key_table` — pass
- `cargo test -p roastty parse_config_keybind` — pass
- `cargo test -p roastty config_cli_keybind` — pass
- `cargo test -p roastty surface_key` — pass
- `cargo test -p roastty --test abi_harness` — pass
- `cargo test -p roastty -- --test-threads=1` — pass
  - 4,644 unit tests passed.
  - ABI harness passed.
  - Doc tests passed.
- `cargo fmt --check` — pass
- `git diff --check` — pass
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/116-key-table-syntax-storage.md issues/0802-libroastty-completion-and-mac-app/README.md`
  — pass

## Conclusion

The key-table parser/storage foundation is in place and covered at both Rust
unit-test and C ABI levels. The next key-table experiment can build on this by
adding runtime table activation and lookup semantics without changing the root
binding parser again.
