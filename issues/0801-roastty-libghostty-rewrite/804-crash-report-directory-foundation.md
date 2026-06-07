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

# Experiment 804: Crash Report Directory Foundation

## Description

Port the local crash-report directory/listing foundation from upstream
`crash/dir.zig` and the list-only shape of `cli/crash_report.zig`.

Issue 801 still marks `sentry` crash reporting as not started and the supporting
subsystems row leaves Sentry-style crash reporting open. A full crash-reporting
port requires native Sentry SDK initialization, crash callbacks, envelope
transport, and CLI/frontend integration. This experiment should not attempt that
large step. It should add the smaller local report inventory foundation that
future Sentry capture and CLI listing can share.

## Changes

- `roastty/src/crash.rs`
  - Add `Report` and `CrashDir` types.
  - Add `default_dir_path()` for Roastty's local crash-report directory. On
    macOS this should resolve under
    `~/Library/Application Support/com.termsurf.roastty/crash` when `$HOME` is
    available, matching Roastty's existing Application Support bundle-id
    convention. If `$HOME` is unavailable, it should fall back to a scoped
    temporary directory such as `${TMPDIR}/roastty/crash`, not the raw system
    temporary directory.
  - Add `CrashDir::new(path)` for tests and future callers.
  - Add `CrashDir::reports()` that tolerates a missing directory by returning an
    empty list, filters to regular files, captures file name and modification
    time, and sorts newest-first to match the upstream `crash-report` listing
    behavior.
  - Add tests for missing directories, filtering non-files, newest-first
    ordering, basename-only report names, bundle-id Application Support default
    path construction, and scoped temporary fallback construction.
- `roastty/src/lib.rs`
  - Export the internal `crash` module.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the supporting subsystem and `sentry`
    dependency rows from not-started/undifferentiated wording to partial wording
    that names local crash-report directory/listing support while keeping Sentry
    SDK initialization, envelope capture/persistence, CLI commands, and
    frontend/report upload flows open.

## Verification

- Inspect:
  - `vendor/ghostty/src/crash/dir.zig`
  - `vendor/ghostty/src/cli/crash_report.zig`
  - `vendor/ghostty/src/crash/sentry.zig`
  - `vendor/ghostty/src/crash/sentry_envelope.zig`
- Run:
  - `cargo fmt -p roastty`
  - `cargo test -p roastty crash -- --nocapture --test-threads=1`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/804-crash-report-directory-foundation.md`
- Run:
  - `git diff --check`

The experiment passes if Roastty has tested local crash-report directory/listing
mechanics and the checklist rows remain partial. It is Partial if the directory
type exists but sorting or default path behavior needs follow-up. It fails if
local listing cannot be usefully separated from Sentry SDK capture.

## Design Review

Codex reviewed the design and found two blocking path issues. First, the
original macOS default used `~/Library/Application Support/roastty/crash`, but
existing Roastty config loading uses the bundle ID `com.termsurf.roastty`.
Second, the original `$HOME` fallback said "system temporary directory", which
could accidentally list unrelated files from `/tmp` if implemented literally.
The design now uses `~/Library/Application Support/com.termsurf.roastty/crash`
with `$HOME`, falls back to a scoped `${TMPDIR}/roastty/crash`, and requires
tests for both path forms. Codex re-reviewed the corrected design and approved
it with no blocking findings. The approval confirmed that the path scope is
defensible, the listing verification covers the key mechanics, and the
README/result wording must stay partial without implying Sentry crate
initialization or crash capture.

## Result

**Result:** Pass

Roastty now has a `crash` module with local crash-report directory/listing
support:

- `CrashDir::new(path)` for explicit report directories.
- `CrashDir::default()` and `default_dir_path()` for the default local crash
  report directory.
- `$HOME` default path:
  `~/Library/Application Support/com.termsurf.roastty/crash`.
- `$HOME`-unavailable fallback: `${TMPDIR}/roastty/crash`.
- `CrashDir::reports()` returns an empty list for missing directories, filters
  to regular files, returns basename-only report names, captures modification
  time, and sorts newest-first with a deterministic name tiebreaker.

This does not implement Sentry SDK initialization, crash callbacks, envelope
capture/persistence, report upload, CLI commands, or frontend flows.

Verification:

- Inspected:
  - `vendor/ghostty/src/crash/dir.zig`
  - `vendor/ghostty/src/cli/crash_report.zig`
  - `vendor/ghostty/src/crash/sentry.zig`
  - `vendor/ghostty/src/crash/sentry_envelope.zig`
- `cargo fmt -p roastty` — passed
- `cargo test -p roastty crash -- --nocapture --test-threads=1` — 5 passed
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/804-crash-report-directory-foundation.md`
  — passed
- `git diff --check` — passed

## Conclusion

Crash reporting can move from not started to a partial local report foundation.
The next crash-reporting work should parse and persist Sentry envelopes, then
wire native SDK capture and CLI/frontend report flows around this directory
inventory.

## Completion Review

Codex reviewed the staged result and found no blocking findings. The review
approved `CrashDir` because it uses the bundle-id Application Support path when
`$HOME` is non-empty, uses a scoped temp fallback otherwise, handles missing
directories, filters non-files, returns basenames, captures mtimes, and sorts
newest-first with a deterministic name tiebreaker. The review also approved the
tests and docs because they cover the key risks for this slice while keeping
Sentry SDK initialization, crash callbacks/capture, envelope persistence,
upload, CLI commands, and frontend flows explicitly missing.
