# Experiment 96: Phase F — async backend and auto-update config

## Description

Port the next three upstream parser/formatter fields after `enquiry-response`:

- `async-backend`
- `auto-update`
- `auto-update-channel`

Upstream declares these fields as:

```zig
@"async-backend": AsyncBackend = .auto,
@"auto-update": ?AutoUpdate = null,
@"auto-update-channel": ?build_config.ReleaseChannel = null,
```

This experiment is intentionally parser/formatter-only. It should make Roastty
accept, store, reset, format, clone, and diagnose the config values with
upstream-compatible keywords. It must not implement Linux async backend
selection, Sparkle update checks, macOS update behavior, or the upstream
`finalize()` step that fills a null `auto-update-channel` from
`build_config.release_channel`.

The upstream keyword sets are:

- `async-backend`: `auto`, `epoll`, `io_uring`
- `auto-update`: `off`, `check`, `download`
- `auto-update-channel`: `tip`, `stable`

## Changes

- `roastty/src/config/mod.rs`
  - Add fields to `Config`:
    - `async_backend: AsyncBackend` defaulting to `AsyncBackend::Auto`
    - `auto_update: Option<AutoUpdate>` defaulting to `None`
    - `auto_update_channel: Option<ReleaseChannel>` defaulting to `None`
  - Add `AsyncBackend`, `AutoUpdate`, and `ReleaseChannel` enum types with
    `from_keyword` and `format_entry` implementations.
  - Route `Config::set` keys for the three upstream config names.
  - Format the entries after `enquiry-response`, preserving upstream field
    order:
    - `async-backend` always formats its value
    - unset optional `auto-update` and `auto-update-channel` format as bare void
      lines, matching existing optional-field formatter behavior
  - Add tests for defaults, valid keywords, empty reset to defaults, missing
    values, invalid values, load diagnostics, clone/equality, and formatter
    ordering.
  - Extend enum round-trip/format tests so all new keywords are covered.

No other files should change except documentation and formatter output caused by
these edits.

## Verification

Pass criteria:

1. `cargo test -p roastty async_update_config`
2. `cargo test -p roastty enum_format_entries`
3. `cargo test -p roastty config_format_config`
4. `cargo test -p roastty`
5. `cargo fmt --check`
6. `git diff --check`

The full `cargo test -p roastty` run must pass. The existing ABI harness may
print its known enum-conversion warnings, but no new failures are acceptable.

## Design Review

Codex-native adversarial review ran in fresh context with subagent
`019eb5a3-6ede-7d13-8244-603265836b19`.

Verdict: **APPROVED**

Findings: None.
