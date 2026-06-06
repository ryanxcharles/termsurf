+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 756: Config Default File Duplicates

## Description

Add duplicate-candidate reporting to Roastty's internal default config file load
report. Experiment 755 ported the default-file candidate names and ordered
optional loading, but deferred upstream's duplicate warnings because the
internal config module does not have a logging layer. Upstream warns when both a
legacy default file and a preferred default file exist in the same family. This
experiment records that condition in the report so a future logging or C ABI
slice can surface it without re-reading the files.

This stays internal to `roastty/src/config/mod.rs`. It does not add logging, C
ABI behavior, template creation, recursive `config-file` loading, or product UI
for the duplicate warning.

## Upstream Behavior

In `vendor/ghostty/src/config/Config.zig`, `loadDefaultFiles`:

- loads the legacy XDG candidate and the preferred XDG candidate;
- warns if both XDG actions are not `.not_found`;
- on macOS, loads the legacy Application Support candidate and the preferred
  Application Support candidate;
- skips the preferred Application Support candidate when it is the same path as
  the legacy Application Support path;
- warns if both distinct Application Support actions are not `.not_found`.

The warning condition treats both successful loads and non-not-found errors as
"present" for the candidate family. Missing files do not count.

## Changes

- `roastty/src/config/mod.rs`
  - Add duplicate fields to `DefaultConfigLoadReport`:
    - `duplicate_xdg: Option<(PathBuf, PathBuf)>`
    - `duplicate_app_support: Option<(PathBuf, PathBuf)>`
  - Store duplicate tuples in deterministic `(legacy_path, preferred_path)`
    order.
  - Update `Config::load_default_files_from_paths` to track each candidate's
    action status:
    - `Loaded` and `Error` count as present.
    - `NotFound` and absent paths do not count as present.
    - Equal Application Support paths still load once and never count as a
      duplicate pair.
  - Set the duplicate fields when both candidate statuses in a family are
    present.
- Tests in `roastty/src/config/mod.rs`
  - XDG duplicate: both legacy and preferred XDG files exist, both load in
    order, and `duplicate_xdg` contains the two paths.
  - XDG error duplicate: one XDG candidate is a non-not-found error and the
    other loads, and `duplicate_xdg` is still set.
  - XDG missing non-duplicate: only one XDG candidate exists and `duplicate_xdg`
    is `None`.
  - Application Support duplicate: both distinct Application Support files exist
    and `duplicate_app_support` contains the two paths.
  - Application Support error duplicate: one distinct Application Support
    candidate is a non-not-found error and the other loads, and
    `duplicate_app_support` is still set.
  - Equal Application Support paths still load once and `duplicate_app_support`
    is `None`.

## Verification

- `cargo test -p roastty load_default_files -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

The experiment passes if duplicate reporting matches upstream's warning
conditions while preserving Experiment 755's load order, loaded-family flags,
error recording, diagnostics preservation, and Application Support
deduplication.

## Design Review

Codex reviewed the first design draft and found two must-fix gaps before the
plan commit. First, Application Support needed its own error-plus-loaded
duplicate test because it has separate dedupe logic and duplicate reporting.
Second, duplicate tuple order needed to be deterministic. The design was updated
so both duplicate fields store `(legacy_path, preferred_path)` and tests assert
that order, including an Application Support non-not-found error duplicate.

Codex reviewed the updated design and approved it for the plan commit with no
blocking findings. The follow-up review confirmed that the prior gaps were
resolved: duplicate pairs have deterministic `(legacy_path, preferred_path)`
ordering, and the Application Support error-plus-loaded duplicate case is now in
scope. The review also confirmed that the scope remains internal and
report-only, with logging, C ABI exposure, templates, recursive config-file
loading, and UI deferred.

## Result

**Result:** Pass

Implemented duplicate-candidate reporting in `DefaultConfigLoadReport` for XDG
and Application Support default config families. The loader now records each
candidate probe as absent, not found, loaded, or error. Successful loads and
non-not-found errors count as present for duplicate detection, while absent and
not-found candidates do not.

Duplicate tuples are stored in deterministic `(legacy_path, preferred_path)`
order. Equal Application Support paths still load once and do not report a
duplicate.

Verification passed:

- `cargo test -p roastty load_default_files -- --nocapture --test-threads=1` — 6
  passed
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Completion Review

Codex reviewed the completed implementation and found no blocking findings. The
review confirmed that `DefaultConfigCandidateStatus::present()` correctly treats
`Loaded` and `Error` as present while excluding `Absent` and `NotFound`, that
duplicate reporting preserves `(legacy_path, preferred_path)` order, and that
Application Support deduplication is preserved. The review also confirmed that
the tests cover normal XDG and Application Support duplicates, XDG and
Application Support error duplicates, XDG missing non-duplicates, and equal
Application Support path dedupe.

Non-blocking follow-ups from the review: both-candidates-error duplicate
coverage and Application Support missing non-duplicate coverage would add
symmetry, but the current tests cover the important branches and the prior
review requirements.

## Conclusion

Roastty now preserves upstream's duplicate default-config warning condition as
structured internal report data without adding logging or public ABI surface. A
future slice can surface these report fields as warnings where the app/runtime
logging boundary lands.
