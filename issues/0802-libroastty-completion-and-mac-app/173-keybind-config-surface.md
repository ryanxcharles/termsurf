# Experiment 173: Phase F — keybind config surface

## Description

Remove `keybind` from the remaining Phase F public-config tail by moving
keybinding parsing, storage, formatting, clone/equality, and default/reset
semantics into the canonical `roastty::config::Config` object.

Upstream `keybind` is `Config.Keybinds`, not a raw repeatable string field. It
stores a root binding set plus named key tables, tracks the most recent binding
target for `chain=...`, initializes built-in defaults, treats empty `keybind =`
as "restore defaults", treats `keybind = clear` as "remove all keybindings",
supports table definition/clear syntax such as `nav/`, rejects table-prefixed
`chain=...`, and formats the structured bindings back as `keybind = ...`
entries. The existing Roastty app-facing layer already has most of this parser
and runtime storage, but it lives outside `Config`, so `Config::load_str` still
reports `keybind` as `UnknownField` and the app layer has special filtering
logic for config-file keybind lines.

This experiment makes `Config` itself understand `keybind`. Runtime dispatch,
keyboard matching, menu shortcut lookup, global-event-tap installation, and
surface/app action execution must keep their current behavior; this is a
config-surface and ownership cleanup, not a new keybinding-runtime experiment.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Config::keybind` after `working-directory` and before `key-remap`,
    matching upstream declaration order.
  - Add a `Keybinds` config type with root bindings, named tables, and
    chain-target state equivalent to upstream `Config.Keybinds`.
  - Move or share the existing app-layer keybind parser structures from
    `roastty/src/lib.rs` so `Config::set("keybind", ...)`, config-file loading,
    recursive config-file loading, and CLI config args all use the same
    validated parser instead of the app layer separately scanning raw config
    text for `keybind` lines.
  - Preserve the already implemented syntax and diagnostics where they match
    upstream:
    - missing value reports `ValueRequired`;
    - empty value restores default keybindings;
    - `clear` clears root bindings, tables, and chain-target state;
    - direct bindings, sequences, trigger-prefix flags, `chain=...`, and key
      tables parse with existing behavior;
    - table clear `name/` creates or empties that table and resets the chain
      target;
    - table-prefixed `chain=...` is rejected;
    - slash keys such as `/=text:foo`, `ctrl+/=text:foo`, and
      `mytable//=text:foo` keep their existing disambiguation.
  - Format `keybind` entries from structured storage. If all bindings are
    cleared, emit `keybind = `; otherwise emit root bindings and table bindings
    as `keybind = ...` lines, accepting that upstream does not guarantee
    original insertion order for formatted keybinds.
  - Update `Config::default`, `Config::format_config`, `Config::set`,
    clone/equality expectations, and the upstream-order test.
  - Add focused tests covering default formatting, empty reset to defaults,
    `clear`, direct bindings, sequences, chained actions, table bindings, table
    clear, invalid/missing values, load diagnostics, recursive/default-file
    loading, CLI config args, and clone/equality.

- `roastty/src/lib.rs`
  - Replace the app-layer raw `keybind` config-file scanner/filtering path with
    reads from `config::Config::keybind`.
  - Keep the C ABI/runtime behavior unchanged by copying or referencing the
    structured config keybinds into the existing app/surface runtime state where
    `roastty_app_key`, `roastty_surface_key`, `roastty_app_has_global_keybinds`,
    and trigger lookup already expect them.
  - Preserve default-file rollback behavior for `--config-default-files=false`
    by snapshotting/restoring `Config::keybind` with the rest of parsed config
    state, rather than maintaining a parallel `before_default_keybinds`
    snapshot.

- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - After result, mark the remaining-public-config-option Phase F item complete
    if `keybind` is fully owned by `Config` and no `UnknownField` filtering
    remains.
  - After result, add an operating note describing the new ownership boundary:
    `Config` owns keybind parse/format/default/reset state; app/runtime state is
    derived from it.

## Verification

Before implementation:

- Codex-native adversarial design review approves this experiment.
- Commit the reviewed plan separately from the result.

After implementation:

- `cargo test -p roastty keybind_config_parse_format_reset_load_cli_and_clone`
- `cargo test -p roastty config_file_keybind_load_file_overrides_unbinds_and_filters_diagnostics`
- `cargo test -p roastty config_file_keybind_default_files_load_and_rollback_with_cli_disable`
- `cargo test -p roastty config_file_keybind_recursive_load_preserves_chain_order`
- `cargo test -p roastty config_format_config_emits_fields_in_upstream_order`
- `cargo test -p roastty parse_config_keybind`
- `cargo test -p roastty config_cli_keybind`
- `cargo test -p roastty config_trigger`
- `cargo test -p roastty surface_key_configured`
- `cargo test -p roastty app_key`
- `cargo test -p roastty`
- `cargo fmt -p roastty`
- `cargo fmt --check -p roastty`
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/173-keybind-config-surface.md issues/0802-libroastty-completion-and-mac-app/README.md`
- `git diff --check`

**Pass** = `keybind` is a first-class `Config` field with upstream
default/reset/clear/table/chain parse and format semantics, all existing
keybinding runtime tests still pass, config-file keybind `UnknownField`
filtering is gone, and the full roastty suite passes.

**Partial** = `Config` accepts and stores keybinds but some structured
formatting, default-file rollback, CLI/default-file integration, or runtime
state derivation remains in the old parallel path.

**Fail** = moving keybind ownership into `Config` conflicts with existing
runtime dispatch or cannot preserve upstream keybind semantics.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Hooke`, fresh context.

**Verdict:** Approved with no findings.

The reviewer verified that the README links Experiment 173 as `Designed`, the
experiment has the required sections, the plan targets Issue 802's remaining
public config gap, the technical scope matches upstream `Keybinds` semantics for
defaults, reset, clear, tables, chain targets, slash disambiguation, formatting,
clone/equality, and the verification covers focused config tests, existing
keybind file/default/recursive tests, runtime dispatch tests, the full roastty
suite, Rust formatting, markdown formatting, and diff hygiene.
