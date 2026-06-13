# Experiment 183: Phase G — key sequence and table audit

## Description

Close the Phase G multi-key/key-table checklist item by proving the current
configured keybinding implementation covers the intended root/table sequence,
leader, chained-action, and file-loaded keybind behavior.

The roadmap item still appears unchecked even though prior experiments have
landed the work in slices: key-table syntax and activation, sequence trie
storage, root and table sequence runtime, `ignore` / `end_key_sequence`, chained
leaves, direct app-key chains, direct app-key surface-control fanout, surface
`all:` / `global:` fanout, and config-file `keybind` loading. The remaining
native keymap and permission-dependent global shortcut work has its own Phase G
roadmap items, so this experiment should not keep the multi-key/key-table item
blocked on those separate concerns.

This is an audit/proof experiment. It should check the roadmap box only if
source inspection and focused tests prove the implemented configured-keybinding
surface is broad enough. It should not claim native keymaps, native global
shortcut registration, or broader Issue 802 completion.

## Changes

- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - After verification, mark it `Pass`, `Partial`, or `Fail`.
  - Check the multi-key/key-table roadmap item only if the source audit and
    focused tests prove configured root/table sequences, active key tables,
    sequence control actions, chained leaves, app-key direct chains, surface
    `all:` / `global:` fanout, and file-loaded keybind entries are wired.
  - Leave trigger-prefix and native-keymap/global-shortcut roadmap items
    unchecked unless a later experiment specifically proves them.

- `issues/0802-libroastty-completion-and-mac-app/183-key-sequence-table-audit.md`
  - Record source evidence, command output, test results, result, conclusion,
    and AI completion review.

- Production code
  - No code change is expected. If the audit finds a real missing behavior,
    record the gap and design a follow-up implementation experiment.

## Verification

Before verification:

- Codex-native adversarial design review approves this experiment.
- Commit the reviewed plan separately from the result.

Source audit:

- Confirm the configured-keybinding trie, chain parent, runtime sequence state,
  active key-table state, and key-sequence action payload exist:

  ```bash
  rg -n "struct ConfigKeybindSet|keybind_chain_parent|active_key_sequence|queued_key_sequence|active_key_tables|ROASTTY_ACTION_KEY_SEQUENCE" \
    roastty/src/lib.rs
  ```

- Confirm the runtime surface path owns key sequence, key table, and
  sequence-control dispatch:

  ```bash
  rg -n "handle_active_key_sequence|start_key_sequence|end_key_sequence|activate_key_table|deactivate_key_table|deactivate_all_key_tables|dispatch_sequence_leaf" \
    roastty/src/lib.rs
  ```

- Confirm config-file `keybind` loading routes through the keybind entry parser
  and storage path:

  ```bash
  rg -n "parse_config_keybind_entry|store_keybind_entry|config_file_keybind" \
    roastty/src/lib.rs
  ```

Focused tests:

- `cargo test -p roastty sequence`
- `cargo test -p roastty key_table`
- `cargo test -p roastty chain`
- `cargo test -p roastty surface_key`
- `cargo test -p roastty app_key`
- `cargo test -p roastty key_sequence`
- `cargo test -p roastty surface_key_all`
- `cargo test -p roastty surface_key_configured_global_all`
- `cargo test -p roastty config_file_keybind_ -- --test-threads=1`
- `cargo test -p roastty config_trigger_ -- --test-threads=1`
- `cargo test -p roastty parse_config_keybind`
- `cargo test -p roastty --test abi_harness`

Regression and hygiene:

- `cargo fmt --check -p roastty`
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/183-key-sequence-table-audit.md issues/0802-libroastty-completion-and-mac-app/README.md`
- `git diff --check`

**Pass** = source audit proves the configured keybinding storage/runtime paths
are wired, all focused tests pass, hygiene checks pass, and the multi-key /
key-table roadmap item can be checked while leaving native/global shortcut items
open.

**Partial** = most configured keybinding behavior is proved, but a specific
sequence/table/chaining/file-load behavior remains unproved or stale. Record the
exact missing proof or implementation gap.

**Fail** = source audit or focused tests contradict the claim that the
multi-key/key-table roadmap item is complete enough to check.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Ptolemy`, fresh context.

**Verdict:** Approved.

Findings: None. The reviewer confirmed the README links Experiment 183 as
`Designed`, the experiment has the required sections, the scope matches the
first remaining Phase G item, and the design explicitly limits itself to
configured key sequence/table proof while leaving trigger-prefix and native /
global shortcut items open. Required hygiene checks and separate plan/result
commit gates are present.

## Result

**Result:** Pass.

This audit found the configured multi-key/key-table surface wired broadly enough
to close the first remaining Phase G checklist item.

Source inspection confirmed:

- `roastty/src/lib.rs` defines key sequence action payloads, keybind trie
  storage, `keybind_chain_parent`, runtime `active_key_sequence` /
  `queued_key_sequence`, and `active_key_tables`.
- The runtime path owns `start_key_sequence`, `handle_active_key_sequence`,
  `dispatch_sequence_leaf`, `end_key_sequence`, `activate_key_table`,
  `deactivate_key_table`, and `deactivate_all_key_tables`.
- Config-file `keybind` loading routes through `parse_config_keybind_entry` and
  `store_keybind_entry`, with focused `config_file_keybind_*` coverage.

Focused verification passed:

- `cargo test -p roastty sequence` — 52 passed.
- `cargo test -p roastty key_table` — 25 passed.
- `cargo test -p roastty chain` — 26 passed.
- `cargo test -p roastty key_sequence` — 15 passed.
- `cargo test -p roastty surface_key` — 99 passed.
- `cargo test -p roastty app_key` — 31 passed.
- `cargo test -p roastty surface_key_all` — 6 passed.
- `cargo test -p roastty surface_key_configured_global_all` — 2 passed.
- `cargo test -p roastty config_file_keybind_ -- --test-threads=1` — 4 passed.
- `cargo test -p roastty config_trigger_ -- --test-threads=1` — 12 passed.
- `cargo test -p roastty parse_config_keybind` — 23 passed.
- `cargo test -p roastty --test abi_harness` — 1 passed, with the existing
  enum-conversion warnings and `[unknown](scope): message`.

Regression and hygiene checks passed:

- `cargo fmt --check -p roastty`
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/183-key-sequence-table-audit.md issues/0802-libroastty-completion-and-mac-app/README.md`
- `git diff --check`

No production code changed. The roadmap checkbox for configured multi-key
sequences/chords, leader/key tables, chains, app-key dispatch, surface fanout,
and file-loaded keybinds is now justified. Trigger-prefix final proof and native
keymap/global shortcut work remain separate open Phase G items.

## Completion Review

**Reviewer:** Codex-native adversarial review subagent `Galileo the 2nd`, fresh
context.

**Verdict:** Approved.

Findings: None.

The reviewer independently verified that only the experiment file and issue
README are modified, the result commit had not been made, the diff from plan
commit `716f3e096252` is documentation-only, the source `rg` probes find the
claimed key sequence/table storage, runtime dispatch, and config-file keybind
paths, and the hygiene checks pass.

The reviewer also reran all focused tests listed in this experiment and
confirmed the recorded counts, including the ABI harness with the existing
enum-conversion warnings and `[unknown](scope): message`.

The reviewer approved the result as staying inside the audit/proof scope,
checking only the configured multi-key/key-table roadmap item, and not claiming
trigger-prefix finality, native keymaps, native global shortcuts, or overall
Issue 802 completion.

## Conclusion

Configured key sequences and key tables are complete enough for the Phase G
roadmap item: the trie/storage layer, root and active-table sequence runtime,
sequence-control actions, chained leaves, direct app-key chains, surface `all:`
/ `global:` fanout, file-loaded keybind entries, and ABI harness all have
focused evidence.

The next required Issue 802 work is not more sequence/table implementation. It
is the remaining Phase G trigger-prefix finality audit and native
keymap/global-shortcut audit, plus the optional debug overlay if still desired.
