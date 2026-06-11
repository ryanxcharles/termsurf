# Experiment 102: Phase F — working-directory finalize

## Description

Add the next upstream config-finalize behavior for `working-directory`.

Upstream `Config.finalize()` computes `probableCliEnvironment()` after theme
loading, then defaults an unset `working-directory` to `inherit` in probable CLI
launches and `home` otherwise. It then finalizes the chosen working directory,
which currently means expanding explicit `~/...` paths. The expensive
default-shell and passwd-home lookup block is guarded out of upstream tests and
is larger than this slice, so this experiment should not add command defaulting
or convert `home` into an absolute path yet.

Roastty already has the `WorkingDirectory` parser/formatter and the explicit
path `~/...` expansion helper. This experiment should connect those pieces to
`Config::finalize()` with deterministic tests for the probable-CLI decision.

This is still a config-internal slice. It should not add default shell
resolution, passwd-home conversion for `WorkingDirectory::Home`, GTK
single-instance runtime defaults, app C ABI exports, live app propagation, link
matcher mutation, or key-remap finalization.

## Changes

- `roastty/src/config/mod.rs`
  - Add a small private finalize context carrying:
    - whether the current launch is a probable CLI environment;
    - the optional home directory used only for explicit `~/...` path expansion.
  - Add a private `probable_cli_environment()` helper matching upstream's
    current rules:
    - return `false` on Windows;
    - on macOS, return `false` if `os::desktop::launched_from_desktop()`;
    - return `true` when `TERM_PROGRAM` is set to a non-empty value;
    - return `true` when process args contain more than the executable path;
    - otherwise return `false`.
  - During `Config::finalize()`, preserve upstream ordering by running
    working-directory finalization after theme loading, font-family inheritance,
    and empty-`term` repair, but before the later scalar tail:
    - if `working_directory` is unset, set it to `Inherit` when the context is a
      probable CLI environment, otherwise `Home`;
    - finalize an explicit `WorkingDirectory::Path("~/...")` against the context
      home directory when one is available;
    - preserve explicitly configured `Home`, `Inherit`, and non-expandable
      paths.
  - Split or refactor the current `finalize_scalars()` helper only as needed to
    make that ordering clear; keep the existing scalar behavior otherwise
    intact.
  - Add deterministic test-only finalize entry points or helpers so tests can
    choose the probable-CLI value and home directory without depending on the
    running test process environment.
  - Add focused tests proving:
    - an unset working directory defaults to `Inherit` for probable CLI;
    - an unset working directory defaults to `Home` outside probable CLI;
    - explicit `Home` and `Inherit` survive finalization;
    - explicit `~/...` paths expand during config finalization;
    - explicit non-expandable paths survive finalization;
    - theme loading still happens before working-directory finalization and
      replayed user config can override a theme-provided working directory.

No default-shell resolution, passwd-home conversion, GTK defaulting, link
matcher mutation, key-remap finalization, app ABI, or runtime propagation should
be implemented in this experiment.

## Verification

Pass criteria:

1. `cargo test -p roastty config_working_directory_finalize`
2. `cargo test -p roastty config_theme_loading`
3. `cargo test -p roastty`
4. `cargo fmt --check`
5. `git diff --check`

The full `cargo test -p roastty` run must pass. The existing ABI harness may
print its known enum-conversion warnings, but no new failures are acceptable.

## Design Review

Codex-native adversarial review ran in fresh context with subagent
`019eb624-0d73-7683-b6ac-420d6226f26c`.

Verdict: **CHANGES REQUIRED**

Findings:

- Required: the initial design placed working-directory finalization after theme
  loading but before the whole scalar tail. Upstream finalizes font-family
  inheritance and repairs an empty `term` before choosing/finalizing
  `working-directory`.

Fix:

- Updated the design to preserve upstream ordering: theme loading first, then
  font-family inheritance and empty-`term` repair, then working-directory
  finalization, then the existing later scalar tail.

Re-review verdict: **APPROVED**

The reviewer confirmed the prior required finding was resolved and found no new
required findings.
