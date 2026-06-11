# Experiment 134: Phase G — file keybind loading

## Description

Route `keybind` entries loaded from config files into the same app-facing
keybind parser and storage path already used by CLI `--keybind` entries.
Experiment 133 made the hosted macOS unit-test runner deterministic and exposed
the remaining concrete failure: `TemporaryConfig("keybind=...")` reports
`UnknownField`, so menu shortcuts and `Config.keyboardShortcut(for:)` keep using
defaults instead of configured file values.

This experiment should fix file-loaded keybinds without changing copied Swift
app behavior. The Rust ABI layer should own the bridge because `config::Config`
still intentionally rejects not-yet-ported non-field keys as `UnknownField`,
while `libroastty` already owns configured keybind storage, chaining, trigger
lookup, and app propagation.

## Changes

- Inspect and update the C-facing config load path in `roastty/src/lib.rs`:
  - `roastty_config_load_file`
  - `roastty_config_load_default_files`
  - `roastty_config_load_recursive_files`
  - existing CLI keybind handling in `roastty_config_load_cli_args`
- Reuse the existing `parse_config_keybind_entry` and
  `Config::store_keybind_entry` path for `keybind` file lines, including direct
  bindings, `unbind`, table entries, sequences, and `chain=` continuation lines.
- Reuse the repo's config-line parser instead of hand-splitting config text. If
  visibility is needed, expose only a narrow `pub(crate)` parser wrapper from
  `roastty/src/config/mod.rs` rather than making the whole loader module public.
- Preserve file order and diagnostic line numbers. `chain=` file entries must
  attach to the prior stored keybind entry from the same load sequence, and
  invalid file keybinds must report an actionable diagnostic instead of a
  generic `UnknownField`.
- Prevent duplicate diagnostics for valid `keybind` lines. The parsed config
  loader may still report `UnknownField` for `keybind`; filter only those
  diagnostics after successfully handling the keybind line in the C-facing
  bridge. Do not suppress unrelated unknown-field diagnostics.
- Keep Swift app sources and tests unchanged unless the Rust fix proves the
  existing tests assert the wrong behavior.
- Update this experiment's result, Issue 802 operating notes, and the Issue 802
  roadmap/checklist after verification.

## Verification

Pass criteria:

- `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/134-file-keybind-loading.md issues/0802-libroastty-completion-and-mac-app/README.md`
- `cargo fmt`
- `cargo fmt --check`
- `cargo build -p roastty`
- Focused Rust tests for file-loaded keybind behavior, covering at minimum:
  - `keybind=super+h=goto_split:left` loaded from a config file overrides the
    default menu shortcut trigger.
  - `keybind = super+d=unbind` loaded from a config file suppresses the default
    `new_split:right` shortcut.
  - file-loaded `chain=` preserves ordering and diagnostics through
    `roastty_config_load_file`.
  - default-file and recursive-file loads route valid `keybind` entries through
    the same storage path, including duplicate-diagnostic filtering for valid
    keybinds.
  - recursive-file load preserves load order for chained keybinds across
    parent/child files where the existing config-file semantics load that
    sequence.
  - non-keybind unknown fields still produce config diagnostics.
- `cargo test -p roastty config_file_keybind_ -- --test-threads=1`
- `cargo test -p roastty config_trigger_ -- --test-threads=1`
- `cargo test -p roastty -- --test-threads=1`
- `cd roastty && macos/build.nu --action test --only-testing RoasttyTests/ConfigTests`
- `cd roastty && macos/build.nu --action test --only-testing RoasttyTests/MenuShortcutManagerTests`
- `cd roastty && macos/build.nu --action test`
- `git diff --check`

The focused macOS `ConfigTests` and `MenuShortcutManagerTests` commands should
pass the six keybind assertions that remained after Experiment 133. The full
non-UI macOS test gate should either pass or fail with the next concrete
post-keybind gap, and the result must record the exact test counts and any
remaining assertions.

## Design Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, `Lagrange`)

**Verdict:** Approved after fixes

The initial design review returned **Changes Required** with two verification
findings:

- The pass criteria omitted the full `cargo test -p roastty -- --test-threads=1`
  ABI gate required for C-facing config behavior changes.
- The focused Rust test requirements covered only `roastty_config_load_file`,
  even though the design scopes default-file and recursive-file loads too.

Both findings were fixed by adding the full Rust test gate and focused coverage
requirements for default-file loading, recursive-file loading, duplicate
diagnostic filtering, recursive chain ordering, and preservation of non-keybind
unknown diagnostics. The re-review approved the design with no remaining
required findings.
