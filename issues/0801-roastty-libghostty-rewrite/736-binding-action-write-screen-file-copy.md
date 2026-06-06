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

# Experiment 736: Binding Action Write Screen File Copy

## Description

Experiments 734 and 735 added `write_selection_file:copy` and
`write_selection_file:paste`. Upstream Ghostty uses the same write-screen action
shape for selection, screen, and scrollback targets. Roastty now has enough
temporary-file lifetime, formatter, clipboard, and binding parser foundation to
add the next target incrementally.

This experiment adds `write_screen_file:copy`, including plain/vt/html formats.
It writes the current visible screen to a temporary file and copies the
canonical path to the standard clipboard. `write_screen_file:paste`,
`write_screen_file:open`, `write_scrollback_file`, and
`write_selection_file:open` remain out of scope.

## Changes

- `roastty/src/lib.rs`
  - Generalize the existing selection-file helper around a write-file target:
    selection or screen.
  - For the selection target, preserve the existing active-selection requirement
    and `selection.txt` / `selection.html` filenames.
  - For the screen target, format the active screen with no selection required,
    using unwrap enabled and trim disabled to match the existing write-file
    formatter policy.
  - Name screen files `screen.txt` for plain/vt formats and `screen.html` for
    html.
  - Parse `write_screen_file:copy`, `copy,plain`, `copy,vt`, and `copy,html`.
  - Keep rejecting malformed `write_screen_file` forms plus `paste` and `open`
    until those actions are implemented for the screen target.
  - Dispatch `write_screen_file:copy*` through the same clipboard path as
    `write_selection_file:copy`, retaining the temporary directory on success.

- `roastty/tests/abi_harness.c`
  - Add valid no-callback / no-worker coverage for the new screen-copy forms
    returning `false`.
  - Add malformed `write_screen_file` parser rejection coverage.

- Tests in `roastty/src/lib.rs`
  - Cover parser rejection for empty, malformed, unsupported-action, unsupported
    format, whitespace, and NUL-containing `write_screen_file` forms.
  - Cover `write_screen_file:copy`, `copy,plain`, `copy,vt`, and `copy,html`
    writing a readable temp file with the expected filename extension and
    copying its canonical path as `text/plain` without confirmation.
  - Assert each written file's contents match the existing active-screen
    formatter output for its requested format, using unwrap enabled and trim
    disabled.
  - Cover that screen-file copy does not require an active selection.
  - Cover false paths for null surfaces, detached surfaces, missing workers, and
    missing clipboard callbacks.
  - Keep the existing `write_selection_file` copy/paste tests passing after the
    helper refactor.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty write_screen_file -- --nocapture --test-threads=1`
- `cargo test -p roastty write_selection_file -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the Experiment 736 design and found one real test-plan gap: the
screen-copy tests must assert file contents match the existing active-screen
formatter output for plain, VT, and HTML, not only that a readable file with the
right extension is created. The plan now includes exact content comparisons
against the formatter output with unwrap enabled and trim disabled.

The review also required recording `[review.design]` frontmatter, this review
section, and the README tuple before the plan commit. With those workflow
records added, the review approved the screen-copy-only scope, deferred
paste/open/scrollback behavior, retained-temp-directory model, clipboard path
model, no-active-selection requirement, and existing selection-file regression
coverage.

## Result

**Result:** Pass

Experiment 736 added `write_screen_file:copy` support. The write-file helper now
accepts an explicit target, preserving the existing selection target behavior
while adding a screen target that formats the active screen with no active
selection required.

The parser now accepts `write_screen_file:copy`, `copy,plain`, `copy,vt`, and
`copy,html`. Unsupported `write_screen_file:paste`, `open`, malformed formats,
empty parameters, whitespace variants, and NUL-containing forms remain rejected.

Screen-copy files are written to retained temporary directories as `screen.txt`
for plain/vt output and `screen.html` for html output. On success, Roastty
copies the canonical file path to the standard clipboard as `text/plain` without
confirmation. Existing `write_selection_file:copy` and
`write_selection_file:paste` behavior still uses `selection.txt` /
`selection.html` and passed regression coverage after the helper refactor.

Verification passed:

- `cargo fmt -p roastty`
- `cargo test -p roastty write_screen_file -- --nocapture --test-threads=1`
  - 2 passed
- `cargo test -p roastty write_selection_file -- --nocapture --test-threads=1`
  - 5 passed
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
  - 119 passed
- `cargo test -p roastty --test abi_harness`
  - 1 passed
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Conclusion

Roastty now has the copy side of both selection-file and screen-file binding
targets. The remaining write-file actions are screen paste/open, selection-file
open, and scrollback-file support. The existing target-aware helper can support
those future slices, but open still needs a runtime URL/open integration
decision.

## Completion Review

Codex reviewed the completed Experiment 736 result and implementation diff. It
found no implementation blockers.

The review confirmed that `write_screen_file:copy*` parses only the intended
copy forms, rejects paste/open/malformed inputs, formats the active screen with
unwrap enabled and trim disabled, writes `screen.txt` / `screen.html`, copies
the canonical path as `text/plain` without confirmation, preserves
selection-file behavior through the helper refactor, and records verification
that matches the implemented coverage.
