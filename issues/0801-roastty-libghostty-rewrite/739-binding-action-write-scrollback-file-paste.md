+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5"
reasoning = "medium"
+++

# Experiment 739: Binding Action Write Scrollback File Paste

## Description

Experiment 738 added `write_scrollback_file:copy` and the internal scrollback
formatter hook needed to format only history above the active screen. The
write-file helper already supports paste actions for selection and screen
targets, so the scrollback target can now share that behavior.

This experiment adds `write_scrollback_file:paste`, including plain/vt/html
formats. It keeps `write_scrollback_file:open`, `write_screen_file:open`, and
`write_selection_file:open` out of scope because those need runtime open-url
plumbing.

## Changes

- `roastty/src/lib.rs`
  - Extend `write_scrollback_file` parsing to accept `paste`, `paste,plain`,
    `paste,vt`, and `paste,html`.
  - Reuse the target-aware write-file helper and paste branch so scrollback
    paste writes `scrollback.txt` / `scrollback.html`, retains the temporary
    directory, and queues exactly the canonical path bytes with no trailing
    newline or NUL.
  - Preserve upstream no-history behavior by returning `false` without creating
    or retaining a temp file when there is no scrollback history.
  - Preserve the readonly gate for paste: return `false` before creating a temp
    file or queueing bytes when the surface is readonly.
  - Preserve queue-failure behavior: return `false` and surface the worker error
    if the queued write fails.
  - Keep rejecting malformed `write_scrollback_file` forms plus `open`.

- `roastty/tests/abi_harness.c`
  - Move valid `write_scrollback_file:paste*` forms from rejected parser
    coverage into valid no-worker / no-callback false-path coverage.
  - Keep `write_scrollback_file:open` and malformed forms rejected.

- Tests in `roastty/src/lib.rs`
  - Cover `write_scrollback_file:paste`, `paste,plain`, `paste,vt`, and
    `paste,html` writing the scrollback history to the expected temp-file
    extension, retaining the directory, and queueing exactly the canonical path
    bytes to the child process.
  - Use a known history-plus-visible-screen fixture and assert written file
    contents include only history above the active screen while excluding
    visible rows.
  - Cover that no-history scrollback paste returns `false` without retaining a
    temp directory.
  - Cover readonly returns `false` before creating/retaining a temp file.
  - Cover queue-failure returns `false`.
  - Keep existing `write_scrollback_file:copy`, `write_screen_file`, and
    `write_selection_file` tests passing.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty write_scrollback_file -- --nocapture --test-threads=1`
- `cargo test -p roastty write_screen_file -- --nocapture --test-threads=1`
- `cargo test -p roastty write_selection_file -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the Experiment 739 design and found no technical blockers. The
review approved the paste-only scrollback scope with open actions deferred, the
scrollback-only formatter behavior, retained temporary file, exact canonical
path bytes with no newline or NUL, readonly gate, no-history false path, and
queue-failure handling.

The review also confirmed the test plan covers parser scope, history-only file
contents, no-history behavior, readonly, queue failure, and regressions for
existing write-file targets. The review required recording `[review.design]`
frontmatter, this review section, and the README tuple before the plan commit;
those records are now present.

## Result

**Result:** Pass

Experiment 739 added `write_scrollback_file:paste` support. The scrollback-file
parser now accepts `paste`, `paste,plain`, `paste,vt`, and `paste,html`
alongside the existing copy forms. Malformed paste forms, unsupported formats,
and `open` remain rejected.

Scrollback paste reuses the target-aware write-file helper and paste branch:
Roastty formats only the history above the active screen, writes
`scrollback.txt` or `scrollback.html` in a retained temporary directory, and
queues exactly the canonical file path bytes to the terminal worker with no
trailing newline or NUL. No-history scrollback paste returns `false` without
retaining a temp file. Readonly mode returns `false` before creating or
retaining a temp file, and worker queue failure returns `false`.

Tests use a known history-plus-visible-screen fixture and assert the written
scrollback file contains `history-red` and `history-two` while excluding
`visible-one`, `visible-two`, and `visible-three`.

Verification passed:

- `cargo fmt -p roastty`
- `cargo test -p roastty write_scrollback_file -- --nocapture --test-threads=1`
  - 4 passed
- `cargo test -p roastty write_screen_file -- --nocapture --test-threads=1`
  - 4 passed
- `cargo test -p roastty write_selection_file -- --nocapture --test-threads=1`
  - 5 passed
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
  - 125 passed
- `cargo test -p roastty --test abi_harness`
  - 1 passed
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Conclusion

All three write-file targets now support copy and paste: selection, screen, and
scrollback. The remaining write-file behavior is the `open` action for each
target, which still needs runtime URL/open action plumbing before a faithful
port.

## Completion Review

Codex reviewed the completed Experiment 739 result and implementation diff. It
found no implementation blockers.

The review confirmed that parser scope, scrollback paste behavior, history-only
formatting, readonly/no-history/queue-failure false paths, temp-file retention,
and the verification record match the plan.
