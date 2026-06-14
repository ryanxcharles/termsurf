# Experiment 100: Default config template creation

## Description

Experiment 99 identified `LOAD-008` as a structural CFG-221 gap. Pinned Ghostty
creates a template config file when none of the default config candidates are
found:

- on macOS, after both XDG and Application Support candidates are absent, it
  creates the template at the preferred Application Support config path;
- on non-macOS, after the XDG candidates are absent, it creates the template at
  the preferred XDG config path;
- creation errors are logged/warned but do not abort config loading.

Roastty currently loads the same default candidate families but does not record
or create the template file when all default candidates are missing. This
experiment will implement Roastty's equivalent template creation behavior and
promote only `LOAD-008` to `Oracle complete`.

## Changes

- `roastty/src/config/mod.rs`
  - Add an embedded default config template using pinned Ghostty's
    `vendor/ghostty/src/config/config-template`.
  - Add a small helper that creates parent directories, writes the template to
    the selected absolute/owned path, and substitutes the target path into the
    template placeholder.
  - Extend `DefaultConfigLoadReport` with fields that record whether a template
    was created and any nonfatal creation error.
  - Update `Config::load_default_files_from_paths` so that when no default XDG
    or app-support candidate is present:
    - if `preferred_app_support` is present, create the template there;
    - otherwise, if `preferred_xdg` is present, create the template there;
    - otherwise, do nothing because no writable default target is known.
  - Preserve existing duplicate reporting, error continuation, same-path
    app-support deduplication, and load order semantics.
  - Add focused unit tests for:
    - missing XDG plus missing app-support creates a template at preferred
      app-support with the pinned template text and selected path substituted
      into the template;
    - missing XDG with no app-support target creates a template at preferred XDG
      with the pinned template text and selected path substituted into the
      template;
    - any loaded or error default candidate suppresses template creation,
      matching Ghostty's `OptionalFileAction != .not_found` loaded flag;
    - template creation errors are recorded but do not abort loading.

- `issues/0805-roastty-ghostty-parity/config_load_inventory.py`
  - Promote `LOAD-008` from `Gap` to `Oracle complete`.
  - Update evidence to name the new focused Roastty unit tests, including the
    content oracle proving the created file matches the pinned template after
    path substitution.

- `issues/0805-roastty-ghostty-parity/config-load-inventory.md`
  - Regenerate the inventory.

- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-221 counts. CFG-221 must remain `Gap` because `LOAD-001` and
    `LOAD-017` still are not `Oracle complete`.

- `issues/0805-roastty-ghostty-parity/README.md`
  - Link this experiment as `Designed`.
  - Add a learning only if implementation exposes a reusable rule beyond the
    expected template creation behavior.

## Verification

Pass criteria:

- New focused unit tests prove template creation target selection, suppression,
  nonfatal error recording, and created file contents.
- The created-file content oracle proves the generated file equals pinned
  Ghostty's template behavior with the selected target path substituted into the
  template placeholder. LOAD-008 must not be promoted by an empty-file or
  existence-only test.
- Existing default-file load tests still pass, proving no regression to load
  order, duplicate reporting, or error continuation.
- The generated load inventory reports:
  - 18 total rows;
  - 16 `Oracle complete` rows;
  - 1 `Audit covered` row;
  - 1 `Gap` row;
  - 2 incomplete rows.
- `LOAD-008` is `Oracle complete`.
- CFG-221 remains `Gap`, points to `config-load-inventory.md`, and records the
  updated counts.
- CFG-217, CFG-218, CFG-219, and CFG-220 remain byte-for-byte unchanged from
  result commit `55af75479` after final Markdown formatting.
- Hygiene passes:

  ```bash
  cargo fmt --manifest-path roastty/Cargo.toml
  cargo test --manifest-path roastty/Cargo.toml config_load_default_files
  PYTHONDONTWRITEBYTECODE=1 python3 \
    issues/0805-roastty-ghostty-parity/config_load_inventory.py \
    --output issues/0805-roastty-ghostty-parity/config-load-inventory.md \
    --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
  prettier --write --prose-wrap always --print-width 80 \
    issues/0805-roastty-ghostty-parity/README.md \
    issues/0805-roastty-ghostty-parity/100-default-config-template-creation.md \
    issues/0805-roastty-ghostty-parity/config-load-inventory.md \
    issues/0805-roastty-ghostty-parity/config-matrix.md
  PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
    issues/0805-roastty-ghostty-parity/config_load_inventory.py
  rm -rf issues/0805-roastty-ghostty-parity/__pycache__
  prettier --check \
    issues/0805-roastty-ghostty-parity/README.md \
    issues/0805-roastty-ghostty-parity/100-default-config-template-creation.md \
    issues/0805-roastty-ghostty-parity/config-load-inventory.md \
    issues/0805-roastty-ghostty-parity/config-matrix.md
  git diff --check
  ```

## Design Review

Adversarial reviewer: Codex subagent with fresh context.

Initial verdict: Changes required.

Required findings:

- The initial design planned to embed pinned Ghostty's `config-template`, but
  verification only proved target selection, suppression, and nonfatal error
  recording. That could promote `LOAD-008` with an empty or wrong file.
- The inventory evidence needed to name a content oracle, not just creation
  mechanics.

Fix:

- Added pass criteria and test scope requiring the created file to match pinned
  Ghostty's template behavior with the selected target path substituted into the
  template placeholder.
- Added an explicit rule that `LOAD-008` must not be promoted by an empty-file
  or existence-only test.

Final verdict: Approved.
