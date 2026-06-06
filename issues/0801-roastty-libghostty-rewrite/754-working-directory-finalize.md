+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 754: Working Directory Finalize

## Description

Port the deferred `WorkingDirectory.finalize` behavior from upstream
`config/Config.zig` into Roastty's internal config type. Experiment 489 ported
`WorkingDirectory::parse_cli` and `value`, but explicitly deferred finalization:
expanding a path that starts with `~/` to the current user's home directory.

This experiment keeps the slice internal to `roastty/src/config/mod.rs`. It does
not wire the helper into the top-level C ABI `roastty_config_finalize`, because
that ABI path still does not carry the full typed config struct. It also does
not implement `WorkingDirectory.formatEntry`, full `Config::finalize`, default
working-directory selection, passwd lookup, theme loading, or diagnostics for
home expansion failures.

## Upstream Behavior

In `vendor/ghostty/src/config/Config.zig`, `WorkingDirectory.finalize`:

- returns immediately for `home` and `inherit`;
- returns immediately for `path` values that do not start with `~/`;
- expands only a leading `~/` path using `internal_os.expandHome`;
- leaves the path unchanged if expansion fails or if expansion returns the same
  path;
- otherwise replaces the path with the expanded string.

Roastty already has `os::homedir::expand_home`, a Rust port of the upstream
home-expansion helper. To keep tests deterministic and avoid mutating process
environment, this experiment will add a helper that takes the resolved home
directory as an argument instead of reading `$HOME` directly.

## Changes

- `roastty/src/config/mod.rs`
  - Import `crate::os::homedir::expand_home`.
  - Update the `WorkingDirectory` documentation to say `finalize_with_home` is
    ported while `formatEntry` remains deferred.
  - Add `WorkingDirectory::finalize_with_home(&mut self, home: &OsStr)`.
    - `Home` and `Inherit` remain unchanged.
    - If `home` is empty, treat home resolution as failed and leave the path
      unchanged.
    - `Path` values not beginning with `~/` remain unchanged.
    - `Path("~/...")` is replaced with the expanded home path if the expanded
      path is valid UTF-8.
    - If the expanded path is not valid UTF-8, leave the existing UTF-8 path
      unchanged. The current `WorkingDirectory::Path(String)` representation
      cannot store arbitrary OS bytes; preserving the old path is the safest
      adaptation until a later config representation change.
- Tests in `roastty/src/config/mod.rs`
  - Add focused tests for `finalize_with_home`:
    - `Path("~/projects/app")` plus home `/Users/tester` becomes
      `/Users/tester/projects/app`.
    - `Path("~/")` plus home `/Users/tester` becomes `/Users/tester/`.
    - `Path("~")`, `Path("~other/app")`, `Path("/tmp/app")`, `Home`, and
      `Inherit` are unchanged.
    - An empty home path leaves `Path("~/projects/app")` unchanged.
    - A non-UTF-8 home path leaves the UTF-8 `~/...` path unchanged.

## Verification

- `cargo test -p roastty working_directory -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

The experiment passes if the new helper mirrors upstream's `~/`-only expansion
for the representable UTF-8 cases, preserves non-expanded variants and paths,
and the formatter/check gates pass.

## Design Review

Codex reviewed the first design draft and found two must-fix gaps before the
plan commit: bare `~` needed an unchanged-path test because upstream expands
only `~/`, and the deterministic helper needed explicit empty-home behavior. The
design was updated so bare `~` stays unchanged and an empty home argument is
treated like failed home resolution, leaving the path unchanged.

Codex then approved the corrected design for the plan commit with no blocking
findings. The review confirmed the scope is small and faithful to upstream's
`~/`-only finalization behavior, while staying internal to
`roastty/src/config/mod.rs` until the typed config path is ready.
