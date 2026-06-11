# Experiment 125: Phase G — app-key surface control actions

## Description

Make `roastty_app_key` handle the remaining configured direct actions that
upstream classifies as surface-scoped keybinding actions but Roastty currently
marks unsupported in the app-key path: `activate_key_table`,
`activate_key_table_once`, `deactivate_key_table`, `deactivate_all_key_tables`,
and `end_key_sequence`.

Experiment 123 wired direct app-key chains for app-scoped and ordinary
surface-scoped actions, but deliberately left key-table and sequence-control
actions out of scope. Upstream `App.keyEvent` does not own key-table or sequence
state; it rejects sequence leaders, accepts focused non-global app-scoped leaves
only, and uses `performAllChainedAction` for `global:` leaves so surface-scoped
actions run on all surfaces. Roastty already has that app-key shape and already
fans out ordinary global surface actions to live surfaces. This experiment
removes the remaining unsupported-action carve-out for direct key-table and
sequence-control actions so global app-key captures can control each live
surface's existing key-table/sequence state.

This is not an app-key sequence experiment. Upstream explicitly rejects
`global:` and `all:` trigger sequences during parsing, and app-level key
handling ignores sequence leaders. Roastty should keep that behavior.

## Changes

- `roastty/src/lib.rs`
  - Classify `ParsedBindingAction::ActivateKeyTable`, `ActivateKeyTableOnce`,
    `DeactivateKeyTable`, `DeactivateAllKeyTables`, and `EndKeySequence` as
    app-key surface-scoped actions instead of unsupported actions.
  - Keep focused non-global app-key behavior unchanged: any surface-scoped
    action in a focused non-global app-key leaf returns `false` so the surface
    path remains responsible for it.
  - Keep global app-key behavior unchanged apart from the new action coverage:
    global leaves consume and fan out surface-scoped actions to all live app
    surfaces.
  - Preserve parser/storage behavior for sequence leaders: `global:x>y=...` and
    `all:x>y=...` remain invalid, and app-key matching continues to use direct
    leaf bindings only.
- `roastty/src/lib.rs` tests
  - Replace the negative "ignores key-table actions for now" coverage with
    positive global app-key tests for activating key tables on live surfaces,
    including `activate_key_table_once`.
  - Add coverage that global app-key deactivation actions pop one table or all
    tables on each live surface.
  - Replace the negative "ignores sequence-control actions for now" coverage
    with positive global `end_key_sequence` coverage that ends/flushed existing
    live-surface sequence state without inventing app-owned sequence state.
  - Keep or add explicit coverage that focused non-global key-table and
    sequence-control app-key leaves return `false` without dispatching.
  - Keep existing coverage that app-key ignores sequence leader bindings.

Out of scope:

- Native keymaps and keyboard-layout reload.
- Native global shortcut registration.
- App-owned key-table or key-sequence state.
- Supporting `global:` or `all:` trigger sequences.
- The remaining `crash` binding action.
- Broader `all:` routing and full upstream default binding table completion.

## Verification

- Run formatting:
  - `cargo fmt -- roastty/src/lib.rs`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/125-app-key-surface-control-actions.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Run targeted tests:
  - `cargo test -p roastty app_key`
  - `cargo test -p roastty key_table`
  - `cargo test -p roastty key_sequence`
- Run full Roastty tests:
  - `cargo test -p roastty -- --test-threads=1`
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run the same Prettier command with `--check`.

**Pass** = global `roastty_app_key` directly fans out key-table and
sequence-control actions to live surfaces, focused non-global app-key routing
still rejects surface-scoped leaves, global/all trigger sequences remain
unsupported, and targeted plus full tests pass.

**Partial** = key-table app-key actions work but sequence-control forwarding or
focused non-global rejection needs a follow-up.

**Fail** = app-key surface-control forwarding requires a larger app/surface
state redesign.

## Design Review

**Reviewer:** Codex-native adversarial reviewer, fresh context
(`multi_agent_v1.spawn_agent`, agent `019eb812-2438-72d0-a37e-f36c3addb734`)

**Verdict:** Approved

**Findings:** None.

**Notes:** The reviewer confirmed that the scope matches upstream
`App.keyEvent`, that key-table and `end_key_sequence` actions are surface-scoped
upstream, that `global:` / `all:` trigger sequences remain invalid, and that the
plan is small enough for one experiment.
