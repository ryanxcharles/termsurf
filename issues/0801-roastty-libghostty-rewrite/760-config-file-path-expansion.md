+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 760: Config File Path Expansion

## Description

Add path expansion for stored `config-file` entries. Experiment 759 added typed
storage and parsing for `config-file`, but recursive loading still cannot be
implemented faithfully because upstream expects every `config-file` path to be
absolute by the time `loadRecursiveFiles` iterates it.

This experiment expands only the stored `config-file` values. It does not open
recursive files, detect cycles, check file types, implement
`roastty_config_load_recursive_files`, expose C ABI config-file values, or add
path expansion for unrelated future path fields.

## Upstream Behavior

In `vendor/ghostty/src/config/Config.zig`:

- after loading a config file, `loadReader` calls `expandPaths(dirname(path))`;
- after loading CLI args, `loadCliArgs` calls `expandPaths(cwd)`;
- `expandPaths` expands `RepeatablePath` and `Path` fields:
  - already-absolute paths are left alone;
  - `~/` expands through the platform home directory helper;
  - other relative paths resolve against the supplied absolute base directory;
  - file-not-found during relative resolution still resolves to `base/path`;
  - expansion errors blank the path and record a diagnostic.

Roastty currently has only one path field in this category: `config-file`.

## Changes

- `roastty/src/config/mod.rs`
  - Add `ConfigFilePath::expand_from_base(base)` and
    `RepeatableConfigPath::expand_from_base(base)` helpers.
  - For relative paths, first canonicalize an existing target relative to the
    absolute base directory. This matches upstream's `dir.realpath(path)` step
    and keeps `child`, `./child`, and symlinked paths from remaining distinct.
  - If the relative target is missing, still resolve to `base/path`, matching
    upstream's FileNotFound fallback.
  - Expand `~/` through Roastty's existing home expansion helper.
  - Leave absolute paths unchanged.
  - On expansion errors other than missing-file fallback, match upstream by
    blanking the path to required empty. Diagnostics for those errors remain
    deferred until Roastty has a general non-line config diagnostic type; this
    experiment pins the state mutation so recursive loading will skip the entry.
  - Before `Config::load_file(path)` reads a file, canonicalize the target path
    and use the canonical parent directory as the expansion base. This preserves
    Experiment 757's relative C ABI path behavior while ensuring stored
    recursive paths become absolute.
  - After `Config::load_file(path)` succeeds, expand stored `config-file`
    entries relative to the canonical loaded file's parent directory.
  - Add a test-only `Config::set_cli_args_from_base(args, base)` helper and make
    `Config::set_cli_args(args)` call it with `std::env::current_dir()`, so CLI
    `config-file` entries expand relative to cwd in production and to a stable
    temp dir in tests.
- Tests in `roastty/src/config/mod.rs`
  - file-loaded `config-file = child` expands relative to the containing config
    file's directory;
  - file-loaded `config-file = ./child` canonicalizes when `child` exists;
  - CLI `--config-file=child` expands relative to the provided base;
  - optional `?child` preserves optional status while expanding;
  - absolute paths remain unchanged;
  - `~/child` expands through a controlled `HOME`;
  - missing relative targets still resolve to `base/child`;
  - a non-missing expansion error blanks the path to required empty;
  - `Config::load_file` called with a relative file path still expands nested
    `config-file` entries to absolute paths;
  - expanded entries still format as `config-file = /absolute` or
    `config-file = ?/absolute`.

## Verification

- `cargo test -p roastty config_file -- --nocapture --test-threads=1`
- `cargo test -p roastty config_path -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

The experiment passes if stored `config-file` paths are absolute after file and
CLI loading, optional status is preserved, home/relative/absolute cases match
upstream semantics, and recursive file opening remains deferred.

## Design Review

Codex reviewed the first design draft and found three blockers. First, existing
relative targets must canonicalize before the missing-file fallback so future
cycle detection sees stable paths. Second, expansion errors must define the path
state; the design now matches upstream by blanking the path to required empty
while deferring diagnostic exposure. Third, `Config::load_file` needed explicit
relative caller-path behavior; the design now canonicalizes the loaded config
path and uses its canonical parent directory as the expansion base.

Codex reviewed the updated design and approved it for implementation with no
remaining blocking findings. The follow-up review confirmed that canonicalizing
existing relative targets, preserving missing-file fallback behavior, blanking
non-missing expansion errors to required empty, and canonicalizing loaded config
paths before deriving the expansion base are sufficient for this slice.
