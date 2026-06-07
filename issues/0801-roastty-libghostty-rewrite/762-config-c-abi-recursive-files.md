+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 762: Config C ABI Recursive Files

## Description

Wire the public `roastty_config_load_recursive_files` C ABI function to the
internal recursive config-file loader added in Experiment 761. The function is
currently an inert stub, while upstream `ghostty_config_load_recursive_files`
delegates to `Config.loadRecursiveFiles`.

This experiment keeps the ABI surface narrow. It does not expose the structured
recursive report directly, implement replay-step behavior, or add new public
diagnostic types. It reuses the existing ABI diagnostics channel established by
Experiments 757 and 761.

## Upstream Behavior

In `vendor/ghostty/src/config/CApi.zig`, `ghostty_config_load_recursive_files`
calls `Config.loadRecursiveFiles` and logs an error if the call itself fails.
Upstream `loadRecursiveFiles` records per-file diagnostics for cycles, missing
required files, non-file paths, and line diagnostics while continuing through
later entries.

Roastty does not yet have upstream's logging/replay boundary. The existing ABI
diagnostics channel stores user-readable messages exposed through
`roastty_config_diagnostics_count` / `roastty_config_get_diagnostic`; this slice
uses that channel for recursive report data.

## Changes

- `roastty/src/lib.rs`
  - Implement `roastty_config_load_recursive_files`:
    - no-op on null config handles;
    - call `config.parsed.load_recursive_files_from_config()`;
    - record line diagnostics from loaded recursive files using the existing
      `push_config_diagnostic` helper;
    - record required missing and other IO errors using the existing file-error
      helper;
    - record relative-path and cycle report entries as ABI diagnostics;
    - call `sync_from_parsed_config` after recursive loading so ABI-visible app
      and surface behavior sees settings loaded by recursive files.
  - Add private helper methods for recursive cycle and relative-path diagnostic
    messages.
- Tests in `roastty/src/lib.rs`
  - recursive ABI loading applies a child config file and syncs
    `confirm-close-surface = always` into app/surface behavior;
  - recursive ABI loading follows a child-appended grandchild in order;
  - recursive line diagnostics and later valid settings are both surfaced;
  - recursive required missing, relative path, and cycle reports become ABI
    diagnostics;
  - null-handle behavior remains a no-op.

## Verification

- `cargo test -p roastty config_c_abi_recursive -- --nocapture --test-threads=1`
- `cargo test -p roastty recursive -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

The experiment passes if the C ABI recursive load function mutates ABI-visible
config state through the internal recursive loader, records recursive report
data through existing diagnostics, preserves null-handle no-op behavior, and
keeps structured report exposure/replay behavior deferred.

## Design Review

Codex reviewed the design and approved it with no blocking findings. The review
confirmed that the scope matches the requested slice: wire the stub to the
Experiment 761 loader, map report data into the existing diagnostics string
channel, sync ABI-visible parsed state using the same pattern as `load_file` /
`load_default_files`, and keep structured report exposure and replay behavior
deferred.
