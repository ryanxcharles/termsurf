# Experiment 102: Full load pipeline order

## Description

`LOAD-001` is the last incomplete CFG-221 load row. Pinned Ghostty's
`Config.zig::load` applies the complete configuration pipeline in this order:

1. construct defaults;
2. load default config files;
3. load CLI args;
4. load recursively referenced `config-file` entries;
5. finalize derived/validated values.

Experiments 99 through 101 proved each major piece independently, but CFG-221
still lacks an end-to-end oracle that one load entry point executes the pieces
in the pinned Ghostty order. This experiment will add a narrow pipeline helper
and focused tests that prove ordering by making each stage affect later stages
in an observable way.

## Changes

- `roastty/src/config/mod.rs`
  - Add a small internal load-pipeline helper that starts from
    `Config::default()`, then runs:
    - `load_default_files_from_paths`;
    - `set_cli_args_from_base`;
    - `load_recursive_files_from_config`;
    - `finalize_with_report`.
  - Return a report containing the default-file load report, CLI diagnostics,
    recursive load report, and finalization report so tests can assert every
    stage ran.
  - Keep the helper scoped to the config layer and avoid changing external app
    startup behavior unless an existing caller already needs the helper.
  - Add focused tests proving:
    - the pipeline starts from `Config::default()` by asserting an untouched
      field remains at its pinned default value;
    - default files load before CLI args by using a default file value that CLI
      overrides;
    - CLI args load before recursive files by supplying `--config-file` on the
      CLI and proving the recursive file applies afterward;
    - recursive files run after ordinary CLI values by setting the same scalar
      in CLI and the recursive file and asserting the recursive value wins;
    - recursive files load before finalization by using recursive `window-width`
      / `window-height` values below the minimum and asserting finalization
      clamps them to the deterministic minimum;
    - stage reports expose loaded default files, CLI diagnostics, recursive
      loaded files, and finalization output;
    - existing focused default-file, recursive, replay, and finalization tests
      still pass.

- `issues/0805-roastty-ghostty-parity/config_load_inventory.py`
  - Promote `LOAD-001` from `Audit covered` to `Oracle complete` only if the
    end-to-end pipeline test proves the pinned stage order.
  - Update evidence to name the new focused pipeline-order test.

- `issues/0805-roastty-ghostty-parity/config-load-inventory.md`
  - Regenerate the inventory.

- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-221 counts. CFG-221 should become `Pass` only when all 18
    load rows are `Oracle complete`.

- `issues/0805-roastty-ghostty-parity/README.md`
  - Link this experiment as `Designed`.
  - Add a learning if the pipeline helper becomes the reusable config-load entry
    point for future CFG-222 reload work.

## Verification

Pass criteria:

- The new pipeline-order test proves all five pinned stages run in order:
  defaults, default files, CLI args, recursive files, finalization.
- The test is failure-sensitive:
  - if the pipeline did not start from `Config::default()`, an untouched pinned
    default value assertion would fail;
  - if CLI ran before default files, the default file value would override the
    CLI value and fail the assertion;
  - if recursive files ran before CLI, the CLI-provided `config-file` would not
    load and fail the recursive-load assertion;
  - if recursive files did not run after ordinary CLI values, the CLI scalar
    would win instead of the recursive file scalar and fail the same-key
    precedence assertion;
  - if finalization ran before recursive files, the recursive file value would
    not be clamped and the deterministic `window-width` / `window-height`
    minimum assertion would fail.
- The pipeline report proves every stage ran by exposing non-empty or expected
  default-file, CLI, recursive, and finalization artifacts.
- Existing focused guards still pass:

  ```bash
  cargo test --manifest-path roastty/Cargo.toml config_load_pipeline
  cargo test --manifest-path roastty/Cargo.toml config_load_default_files
  cargo test --manifest-path roastty/Cargo.toml config_recursive
  cargo test --manifest-path roastty/Cargo.toml config_replay
  cargo test --manifest-path roastty/Cargo.toml config_finalize
  ```

- The generated load inventory reports:
  - 18 total rows;
  - 18 `Oracle complete` rows;
  - 0 `Audit covered` rows;
  - 0 `Gap` rows;
  - 0 incomplete rows.
- `LOAD-001` is `Oracle complete`.
- CFG-221 becomes `Pass`, points to `config-load-inventory.md`, and records the
  completed counts.
- CFG-217, CFG-218, CFG-219, and CFG-220 remain byte-for-byte unchanged from
  result commit `f2b2a0063` after final Markdown formatting.
- Hygiene passes:

  ```bash
  cargo fmt --manifest-path roastty/Cargo.toml
  PYTHONDONTWRITEBYTECODE=1 python3 \
    issues/0805-roastty-ghostty-parity/config_load_inventory.py \
    --output issues/0805-roastty-ghostty-parity/config-load-inventory.md \
    --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
  prettier --write --prose-wrap always --print-width 80 \
    issues/0805-roastty-ghostty-parity/README.md \
    issues/0805-roastty-ghostty-parity/102-full-load-pipeline-order.md \
    issues/0805-roastty-ghostty-parity/config-load-inventory.md \
    issues/0805-roastty-ghostty-parity/config-matrix.md
  PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
    issues/0805-roastty-ghostty-parity/config_load_inventory.py
  rm -rf issues/0805-roastty-ghostty-parity/__pycache__
  prettier --check \
    issues/0805-roastty-ghostty-parity/README.md \
    issues/0805-roastty-ghostty-parity/102-full-load-pipeline-order.md \
    issues/0805-roastty-ghostty-parity/config-load-inventory.md \
    issues/0805-roastty-ghostty-parity/config-matrix.md
  git diff --check
  ```

## Design Review

Adversarial reviewer: Codex subagent with fresh context.

Initial verdict: Changes required.

Required findings:

- The initial plan claimed to prove defaults but only had concrete artifacts for
  default files, CLI, recursive files, and finalization.
- The initial CLI-before-recursive assertion proved CLI-provided `config-file`
  path discovery but did not prove recursive files beat ordinary CLI values.
- The initial finalization assertion did not name a deterministic field/effect.

Fix:

- Added an untouched pinned default assertion proving the pipeline starts from
  `Config::default()`.
- Added same-key precedence: CLI sets a scalar, the recursive file sets that
  same scalar differently, and the recursive value must win.
- Made the finalization assertion deterministic by using recursive
  `window-width` / `window-height` values below the minimum and requiring
  finalization to clamp them.

Final verdict: Approved.
