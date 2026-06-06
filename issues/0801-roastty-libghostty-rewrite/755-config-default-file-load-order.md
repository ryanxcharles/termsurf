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

# Experiment 755: Config Default File Load Order

## Description

Port the internal default-config-file load order for Roastty's typed `Config`.
Experiments 531 through 535 added the path resolvers and the
`Config::load_optional_file` building block, but stopped before orchestrating
the default files because the Roastty names were not settled. The current code
already uses concrete Roastty naming in resolver tests:

- XDG subdir/file: `roastty/config`
- macOS bundle id: `com.termsurf.roastty`
- macOS Application Support file: `config`

Upstream Ghostty now has preferred `config.ghostty` files and legacy `config`
files. For Roastty, this experiment uses the direct analogous names:

- preferred XDG: `roastty/config.roastty`
- legacy XDG: `roastty/config`
- preferred Application Support: `config.roastty`
- legacy Application Support: `config`
- bundle id: `com.termsurf.roastty`

This stays internal to `roastty/src/config`. It does not wire the helper into
the public C ABI `roastty_config_load_default_files`, does not create a template
file when no default exists, and does not implement recursive `config-file`
loading.

## Upstream Behavior

In `vendor/ghostty/src/config/Config.zig`, `loadDefaultFiles`:

- loads the legacy XDG file first, then the preferred XDG file;
- considers XDG "loaded" when either candidate is loaded or errors with
  something other than not-found;
- on macOS, loads the legacy Application Support file first, then the preferred
  Application Support file;
- avoids double-loading Application Support when the preferred path resolves to
  the same path as the legacy path;
- considers Application Support "loaded" with the same not-found distinction;
- warns when both legacy and preferred candidates exist;
- writes a template file when neither family is loaded.

Roastty currently lacks a logging/template layer in this internal config module,
so this experiment ports candidate construction, load order, not-found vs
error/loaded accounting, diagnostics collection, and Application Support
deduplication. Warning emission and template creation stay deferred.

## Changes

- `roastty/src/config/loader.rs`
  - Add Roastty default-config constants:
    - `ROASTTY_BUNDLE_ID = "com.termsurf.roastty"`
    - `ROASTTY_XDG_CONFIG_LEGACY = "roastty/config"`
    - `ROASTTY_XDG_CONFIG_PREFERRED = "roastty/config.roastty"`
    - `ROASTTY_APP_CONFIG_LEGACY = "config"`
    - `ROASTTY_APP_CONFIG_PREFERRED = "config.roastty"`
  - Add `default_config_paths_from_home(...)` as a deterministic path builder
    over explicit `xdg_config_home` / `home` values. It returns optional legacy
    and preferred XDG/Application Support paths using the existing resolver
    logic. This pure builder is platform-neutral so tests can verify the macOS
    path family without mutating process environment.
  - Add `default_config_paths()` as the env-reading wrapper. It always returns
    XDG candidates, but returns Application Support candidates only on macOS;
    non-macOS platforms leave that family as `None`.
- `roastty/src/config/mod.rs`
  - Add small report structs for default-file loading:
    - `DefaultConfigPaths`
    - `DefaultConfigFileLoad`
    - `DefaultConfigFileError`
    - `DefaultConfigLoadReport`
  - Add `Config::load_default_files_from_paths(paths)`:
    - load legacy XDG, then preferred XDG;
    - load legacy Application Support, then preferred Application Support;
    - skip the preferred Application Support path when it equals the legacy
      Application Support path;
    - record loaded paths with their per-file diagnostics;
    - record non-not-found IO errors with their path;
    - set `DefaultConfigLoadReport::xdg_loaded` when either XDG candidate loads
      or records a non-not-found error;
    - set `DefaultConfigLoadReport::app_support_loaded` when either Application
      Support candidate loads or records a non-not-found error;
    - keep applying later files after diagnostics/errors.
  - Add `Config::load_default_files()` as the env-reading wrapper that uses
    `loader::default_config_paths()`.
- Tests
  - `loader.rs`: verify the deterministic path builder produces the preferred
    and legacy XDG/Application Support candidates, and returns `None` families
    when home inputs are absent.
  - `config/mod.rs`: verify load order:
    - legacy XDG applies first;
    - preferred XDG overrides legacy XDG;
    - legacy Application Support applies after XDG;
    - preferred Application Support overrides legacy Application Support;
    - equal Application Support paths are loaded only once;
    - missing files are ignored;
    - non-not-found IO errors are recorded, mark their family loaded, and do not
      abort later files;
    - diagnostics from a loaded file are preserved in that file's report.

## Verification

- `cargo test -p roastty default_config -- --nocapture --test-threads=1`
- `cargo test -p roastty load_default_files -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

The experiment passes if the candidate paths use the Roastty names above,
default files load in upstream order with Application Support deduplication,
missing files are ignored, non-not-found errors and per-file diagnostics are
reported without aborting, and all formatter/check gates pass.

## Design Review

Codex reviewed the first design draft and found two blocking ambiguities before
the plan commit. First, the design claimed loaded-family accounting but did not
specify report fields for it. The design was updated to add explicit
`xdg_loaded` and `app_support_loaded` booleans, where loaded files and
non-not-found errors both mark the family loaded. Second, the Application
Support platform behavior was underspecified. The design was updated so the pure
path builder stays platform-neutral for deterministic tests, while the
env-reading `default_config_paths()` wrapper returns Application Support
candidates only on macOS.

Codex then approved the corrected design for the plan commit with no blocking
findings. The review confirmed the scope is faithful to upstream's load order
and appropriately small: candidate construction, ordered optional loading,
loaded/error accounting, per-file diagnostics, and Application Support
deduplication, with C ABI wiring, warnings, templates, recursive `config-file`,
and richer logging deferred.

## Result

**Result:** Pass

Roastty's typed internal config now has default-file candidate construction and
ordered loading. `loader.rs` defines the Roastty preferred and legacy names
(`config.roastty` and `config`) for both XDG and Application Support paths, with
`com.termsurf.roastty` as the bundle id. The deterministic builder returns all
candidate families for tests, while the env-reading wrapper returns Application
Support candidates only on macOS.

`Config::load_default_files_from_paths` loads candidates in upstream order:
legacy XDG, preferred XDG, legacy Application Support, preferred Application
Support. It skips duplicate Application Support paths, records each loaded file
with its diagnostics, records non-not-found IO errors, treats loaded files and
non-not-found errors as loaded-family evidence, ignores missing files, and keeps
processing later candidates after diagnostics or errors.

Verification passed:

- `cargo test -p roastty default_config -- --nocapture --test-threads=1`
- `cargo test -p roastty load_default_files -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Completion Review

Codex reviewed the completed implementation and found no blocking issues. The
review agreed that candidate construction uses the approved Roastty names, the
pure builder and env wrapper have the intended platform behavior, and
`Config::load_default_files_from_paths` preserves upstream order with
Application Support deduplication.

The review also confirmed loaded-family accounting, error handling, continued
loading after errors, and per-file diagnostic preservation. It considered the
verification sufficient. As a non-blocking note, it observed that
`default_config_paths()` platform gating has no direct test, but the wrapper is
small and the pure builder plus load-order tests cover the meaningful behavior
for this internal slice.

## Conclusion

Experiment 755 completes the internal default-config candidate construction and
ordered optional-load orchestration. Template creation, duplicate warnings,
recursive `config-file` loading, and C ABI wiring remain separate future slices.
