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

# Experiment 806: Crash Envelope Persistence

## Description

Port the local crash-envelope persistence decision from upstream
`crash/sentry.zig::Transport.sendInternal` into Roastty.

Experiments 804 and 805 added the crash-report directory/listing foundation and
the Sentry envelope parse/serialize foundation. The next useful slice is the
write path for an already serialized envelope: parse it, discard envelopes that
do not contain an event item, create the crash-report directory, and write the
serialized bytes as a local crash report.

This experiment still avoids native Sentry SDK initialization, crash callbacks,
report upload, CLI commands, and frontend flows. It is a persistence foundation
that future SDK transport glue can call.

## Changes

- `roastty/Cargo.toml`
  - Add `uuid` to Roastty only if the default production helper generates report
    filenames in this experiment. Prefer reusing the workspace-locked crate over
    adding a new random/UUID dependency.
- `roastty/src/crash.rs`
  - Add an envelope event check equivalent to upstream `shouldDiscard`: an
    envelope is persistable only if at least one item has `ItemType::Event`.
  - Add a public-in-crate persistence helper that accepts serialized envelope
    bytes, parses them with the existing `Envelope::parse`, discards non-event
    envelopes, creates the crash directory, writes the original serialized bytes
    for event envelopes, and returns whether a report was written.
  - Keep deterministic testing possible by separating the filename-injected
    write path from any production filename generation. Tests should not depend
    on randomness.
  - Use the `.roasttycrash` report extension for newly written reports, while
    keeping existing directory listing behavior extension-agnostic.
  - Add tests for:
    - event envelopes are written to a created crash directory;
    - persisted file bytes exactly match the serialized input bytes;
    - session-only or attachment-only envelopes are discarded before directory
      creation, so they do not create the crash directory and do not create a
      report file;
    - malformed envelopes return the parse error before directory creation, so
      they do not create the crash directory and do not write;
    - filename injection rejects path separators or otherwise cannot escape the
      crash directory;
    - generated production report filenames use the `.roasttycrash` extension.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the crash/Sentry partial rows to mention local
    event-envelope persistence while keeping SDK initialization, crash
    callbacks, upload, CLI commands, and frontend flows open.

## Verification

- Inspect:
  - `vendor/ghostty/src/crash/sentry.zig`
  - `vendor/ghostty/src/crash/sentry_envelope.zig`
  - `vendor/ghostty/src/crash/dir.zig`
- Run:
  - `cargo fmt -p roastty`
  - `cargo test -p roastty crash -- --nocapture --test-threads=1`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/806-crash-envelope-persistence.md`
- Run:
  - `git diff --check`

The experiment passes if Roastty can persist event envelopes locally with tested
discard/error behavior while crash reporting remains partial. It is Partial if
event detection lands but the file write path needs follow-up. It fails if local
persistence cannot be separated cleanly from native Sentry SDK capture.

## Design Review

Codex reviewed the design and found two blocking verification gaps. First, the
discard/error tests did not explicitly require upstream's ordering: parse and
discard non-event envelopes before creating the crash-report directory. The plan
now requires session-only, attachment-only, and malformed-envelope tests to
assert that those paths do not create the crash directory. Second, the report
extension decision was too vague for Roastty's naming constraint. The plan now
requires generated production filenames to use the exact `.roasttycrash`
extension and to test that extension.

Codex re-reviewed the corrected design and approved it with no findings. The
approval confirmed that the no-directory side-effect tests match upstream's
discard-before-`makePath` ordering, the generated report extension is pinned to
`.roasttycrash`, and the scope remains limited to persistence without SDK
initialization, crash callbacks, upload, CLI commands, or frontend work.
