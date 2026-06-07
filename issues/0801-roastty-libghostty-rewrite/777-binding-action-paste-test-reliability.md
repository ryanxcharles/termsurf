+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "default"
reasoning = "medium"
+++

# Experiment 777: Binding Action Paste Test Reliability

## Description

Investigate and fix the PTY-backed
`surface_binding_action_write_*_paste_queues_path` tests that prevented
Experiment 776 from completing its broad `surface_binding_action_` verification.

Experiment 776 found that surface key dispatch and binding-action parsing are
likely complete enough to remove stale checklist wording, but the broad
binding-action test filter spent several minutes in the final file-paste tests
and had to be terminated. This experiment focuses on that reliability problem
before attempting another checklist sync.

## Changes

- `roastty/src/lib.rs`
  - Inspect the three PTY-backed paste tests and their shared helpers:
    `surface_binding_action_write_selection_file_paste_queues_path`,
    `surface_binding_action_write_screen_file_paste_queues_path`,
    `surface_binding_action_write_scrollback_file_paste_queues_path`, and
    `surface_snapshot_text_until`.
  - If the investigation finds a real test harness issue, timing bug, cleanup
    gap, or overly slow child command, make the smallest code or test change
    needed to make these tests deterministic.
  - If the tests are already deterministic when run in isolation, record that
    evidence and leave code unchanged.

## Verification

- Run the three previously blocking tests individually, twice each, with output
  enabled and elapsed time recorded:
  - `/usr/bin/time -p cargo test -p roastty surface_binding_action_write_selection_file_paste_queues_path -- --nocapture --test-threads=1`
  - `/usr/bin/time -p cargo test -p roastty surface_binding_action_write_screen_file_paste_queues_path -- --nocapture --test-threads=1`
  - `/usr/bin/time -p cargo test -p roastty surface_binding_action_write_scrollback_file_paste_queues_path -- --nocapture --test-threads=1`
- Run the broader write-action cluster that contains the paste tests and nearby
  write/copy/open false-path coverage:
  - `/usr/bin/time -p cargo test -p roastty surface_binding_action_write_ -- --nocapture --test-threads=1`
- If a code or Rust test change is made, run:
  - `cargo fmt -p roastty`
  - `cargo fmt -p roastty -- --check`
- Run the markdown formatter:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/777-binding-action-paste-test-reliability.md`
- Run:
  - `git diff --check`

The experiment passes if the previously blocking paste tests complete twice in
isolation and in the broader write-action cluster with stable enough timings to
support using them as verification, with any root cause and fix recorded. It is
Partial if the tests remain slow or hanging without a confirmed fix, and Fail
only if the investigation proves the current binding-action path is incorrect
rather than merely unreliable.

## Design Review

Codex reviewed the initial design and found three issues: the file recorded
result-review metadata before implementation, the grouped verification command
was described as only the three paste tests even though it matched a broader
write-action cluster, and the pass criterion claimed deterministic behavior
after only one run.

The design was updated to remove premature result-review metadata, accurately
name the broader grouped filter, and require each previously blocking paste test
to run twice with elapsed timing recorded before calling the tests stable. Codex
reviewed the revision, found no blockers, and approved the Experiment 777 plan
commit.
