+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "default"
reasoning = "medium"

[review.result]
agent = "codex"
model = "default"
reasoning = "medium"
+++

# Experiment 791: Datastruct Utilities Checklist Sync

## Description

The Issue 801 supporting-subsystems checklist still leaves `CircBuf`,
`IntrusiveLinkedList`, and other utility datastructures unchecked "as needed."
The current Roastty tree already contains the named utilities and several
adjacent terminal collection helpers under `roastty/src/terminal/`:
`circ_buf.rs`, `intrusive_linked_list.rs`, `array_list_collection.rs`,
`cache_table.rs`, `lru.rs`, and `segmented_pool.rs`.

This experiment verifies those existing utility datastructures and updates the
checklist row to complete for the current supporting datastruct set. It does not
claim that unrelated future utility types will never be needed.

## Changes

- `issues/0801-roastty-libghostty-rewrite/README.md`
  - Change the `CircBuf` / `IntrusiveLinkedList` datastruct row from unchecked
    "as needed" to checked with scoped wording for the implemented terminal
    utility collections.
  - Add the Experiment 791 index entry.
- `issues/0801-roastty-libghostty-rewrite/791-datastruct-utilities-checklist-sync.md`
  - Record the verification evidence and review result.

## Verification

- Inspect current utility datastructure modules:
  - `roastty/src/terminal/circ_buf.rs`
  - `roastty/src/terminal/intrusive_linked_list.rs`
  - `roastty/src/terminal/array_list_collection.rs`
  - `roastty/src/terminal/cache_table.rs`
  - `roastty/src/terminal/lru.rs`
  - `roastty/src/terminal/segmented_pool.rs`
- Run focused datastructure tests:
  - `cargo test -p roastty circ_buf -- --nocapture --test-threads=1`
  - `cargo test -p roastty intrusive_linked_list -- --nocapture --test-threads=1`
  - `cargo test -p roastty array_list_collection -- --nocapture --test-threads=1`
  - `cargo test -p roastty cache_table -- --nocapture --test-threads=1`
  - `cargo test -p roastty lru -- --nocapture --test-threads=1`
  - `cargo test -p roastty segmented_pool -- --nocapture --test-threads=1`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/791-datastruct-utilities-checklist-sync.md`
- Run:
  - `git diff --check`

The experiment passes if the named datastructure modules exist, focused tests
pass, and the README row is checked with wording scoped to the current utility
collection set. It is Partial if only `CircBuf` or only `IntrusiveLinkedList`
verifies. It fails if the original unchecked row is still accurate.

## Design Review

Codex reviewed the design and found no blocking findings. The review approved
the docs-only scope, checked row limited to the current terminal utility
collection set, explicit future-helper caveat, and non-empty focused test
filters.

## Result

**Result:** Pass

The named terminal utility datastructure modules exist and the focused tests
passed:

- `cargo test -p roastty circ_buf -- --nocapture --test-threads=1`: 19 passed
- `cargo test -p roastty intrusive_linked_list -- --nocapture --test-threads=1`:
  5 passed
- `cargo test -p roastty array_list_collection -- --nocapture --test-threads=1`:
  3 passed
- `cargo test -p roastty cache_table -- --nocapture --test-threads=1`: 4 passed
- `cargo test -p roastty lru -- --nocapture --test-threads=1`: 10 passed
- `cargo test -p roastty segmented_pool -- --nocapture --test-threads=1`: 5
  passed

Formatting and diff hygiene checks passed:

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/791-datastruct-utilities-checklist-sync.md`
- `git diff --check`

The README row now marks the current `CircBuf`, `IntrusiveLinkedList`, and
adjacent terminal utility collection set complete while explicitly leaving
future incidental helpers as-needed.

## Conclusion

The unchecked datastruct-utilities row was stale. Roastty already carries the
current supporting utility collection set needed by the terminal core:
`circ_buf`, `intrusive_linked_list`, `array_list_collection`, `cache_table`,
`lru`, and `segmented_pool`, all with focused passing tests. Future incidental
helpers are still allowed, but the checklist no longer needs to track this
implemented set as open work.

## Completion Review

Codex reviewed the completed experiment and found no blocking findings. The
review approved the checked row scoped to the current terminal utility
collection set, the future-helper caveat, the recorded verification counts plus
Prettier and `git diff --check`, and the README status/provenance.
