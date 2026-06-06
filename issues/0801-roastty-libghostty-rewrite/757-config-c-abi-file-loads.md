+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 757: Config C ABI File Loads

## Description

Wire Roastty's C ABI config file-loading entry points into the internal Rust
config loader. Experiments 755 and 756 made default config file discovery,
ordered loading, diagnostics, errors, and duplicate reporting real inside
`roastty/src/config`. The public ABI still exposes `roastty_config_load_file`
and `roastty_config_load_default_files`, but both functions are stubs.

This experiment makes those ABI calls perform real loads while keeping the
surface narrow. It does not expose the duplicate report fields through the C ABI
yet, does not add logging callbacks, does not implement recursive `config-file`,
and does not convert every `roastty_config_get` key to read from the parsed
config. It only proves the ABI load path by syncing parsed state that the
wrapper already owns: `confirm_close_surface`.

## Upstream Behavior

In `vendor/ghostty/src/config/CApi.zig`:

- `ghostty_config_load_file` takes a null-terminated path, calls
  `Config.loadFile`, and logs an error if loading fails.
- `ghostty_config_load_default_files` calls `Config.loadDefaultFiles` and logs
  an error if loading fails.
- both functions return `void`.

Roastty does not have the upstream logging boundary yet. Existing ABI
diagnostics are stored as user-readable `CString`s and exposed through
`roastty_config_diagnostics_count` / `roastty_config_get_diagnostic`, so file IO
errors and config parse diagnostics should flow into that channel for this
slice.

## Changes

- `roastty/src/lib.rs`
  - Add a parsed `config::Config` field to the ABI `Config` wrapper.
  - Initialize and clone the parsed config alongside existing ABI wrapper state.
  - Add a small sync helper that copies parsed `confirm_close_surface` into the
    wrapper's existing `confirm_close_surface` field after config file loads.
  - Implement `roastty_config_load_file`:
    - no-op on a null config handle or null path pointer;
    - read the null-terminated C path with `CStr` and convert it losslessly to a
      Rust path with Unix `OsStrExt::from_bytes` / `Path`;
    - load the file through `config::Config::load_file`;
    - record any returned per-line diagnostics in ABI diagnostics;
    - record open/read errors in ABI diagnostics;
    - sync `confirm_close_surface` on successful file reads, even when line
      diagnostics were produced.
  - Implement `roastty_config_load_default_files`:
    - no-op on a null config handle;
    - call `config::Config::load_default_files`;
    - record default-file IO errors in ABI diagnostics;
    - record loaded-file line diagnostics in ABI diagnostics, including the file
      path and line number;
    - keep duplicate report warning surfacing deferred.
- Tests in `roastty/src/lib.rs`
  - `roastty_config_load_file` applies `confirm-close-surface = false`, then an
    app or surface created from that config observes no close confirmation.
  - A loaded file with an invalid key records a diagnostic but still applies
    later valid settings and syncs the wrapper state.
  - A missing explicit file records one diagnostic through the existing ABI
    diagnostic accessors.
  - Loading a file, then cloning the ABI config, preserves both the synced
    `confirm_close_surface` state and any file-load diagnostics in the clone.
  - `roastty_config_load_default_files` is exercised with a test-only
    environment lock that points `XDG_CONFIG_HOME` and `HOME` at a temporary
    directory, so it never touches the user's real home config. The test writes
    one default candidate, loads it through the public ABI function, and asserts
    the synced wrapper state.

## Verification

- `cargo test -p roastty config_load_file -- --nocapture --test-threads=1`
- `cargo test -p roastty config_c_abi -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

The experiment passes if the public C ABI file-load functions mutate ABI-visible
config state, preserve existing null-handle/no-crash behavior, report IO and
line diagnostics through the existing diagnostics accessors, and avoid exposing
deferred duplicate/logging/recursive behavior.

## Design Review

Codex reviewed the first design draft and found two blockers. First, C path byte
handling was underspecified: upstream accepts raw path bytes, so Roastty must
avoid lossy string conversion. The plan now requires lossless Unix path
conversion via `OsStrExt::from_bytes`. Second, the plan said to clone the parsed
config but did not verify clone behavior. The test plan now requires cloning an
ABI config after file loading and asserting that both synced
`confirm_close_surface` state and file-load diagnostics survive in the clone.

Codex reviewed the updated design and approved it for the plan commit with no
blocking findings. The follow-up review confirmed that the path conversion and
clone-verification blockers were resolved and that the slice remains narrow:
real C ABI file/default loading, diagnostics through the existing ABI
diagnostics channel, and syncing only `confirm_close_surface`, with duplicate
warnings, logging callbacks, recursive `config-file` loading, and broader getter
conversion deferred.

## Result

**Result:** Pass

Implemented the public C ABI file load functions in `roastty/src/lib.rs`.
`roastty_config_load_file` now reads null-terminated C paths losslessly as Unix
path bytes, loads the file through the typed Rust config loader, records file IO
errors and per-line config diagnostics through the existing ABI diagnostics
channel, and syncs parsed `confirm_close_surface` into the runtime-facing ABI
wrapper state after successful reads.

`roastty_config_load_default_files` now calls the internal default-file loader,
records loaded-file diagnostics and non-not-found default-file errors, and syncs
the wrapper state. Duplicate warning exposure remains deferred.

Verification passed:

- `cargo test -p roastty config_load_file -- --nocapture --test-threads=1`
- `cargo test -p roastty config_c_abi -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Completion Review

Codex reviewed the completed implementation and found no blocking findings. The
review confirmed that explicit file loads use lossless Unix path conversion,
diagnostics flow through the existing ABI diagnostics channel,
`confirm_close_surface` syncs after successful reads, default-file reports are
recorded without exposing duplicate warnings, clone handling includes the parsed
config and diagnostics, null-handle behavior is preserved, and the default-file
test uses a temporary environment.

Non-blocking follow-ups from the review: add a Unix-only non-UTF-8 path test for
`roastty_config_load_file`, add a C ABI harness check for the load functions,
and add a C ABI-level default-file diagnostic/error test when the diagnostics
surface gets broader coverage.

## Conclusion

The C ABI config load functions are no longer inert stubs. They now connect the
public `roastty_config_load_file` and `roastty_config_load_default_files` entry
points to the internal Rust config loader while preserving the intentionally
narrow ABI surface for this slice.
