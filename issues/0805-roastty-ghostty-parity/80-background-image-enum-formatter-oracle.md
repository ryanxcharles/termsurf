# Experiment 80: Background image enum formatter oracle

## Description

Experiment 79 promoted the six remaining packed-flag formatter rows and left
CFG-218 at 170 `Oracle complete` rows, 33 `Audit covered` rows, and 0 formatter
gaps.

The next compact formatter family is the background-image enum pair:

- `background-image-fit`: `contain`, `cover`, `stretch`, `none`;
- `background-image-position`: `top-left`, `top-center`, `top-right`,
  `center-left`, `center-center`, `center-right`, `bottom-left`,
  `bottom-center`, `bottom-right`, `center`.

Roastty already has direct enum `format_entry` coverage for both types, but the
inventory should not promote these rows until there is a named oracle proving
the full config path: `Config::set`, `Config::format_config`, raw-empty resets,
and local formatter ordering. This experiment should promote exactly these two
rows and keep adjacent background-image rows such as `background-image`,
`background-image-opacity`, and `background-image-repeat` governed by their
existing or future proofs.

CFG-218 should remain `Gap` because 31 formatter rows will still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `background_image_enum_config_formatter_family_oracle` test.
  - Assert direct `format_entry` output for every `BackgroundImageFit` and
    `BackgroundImagePosition` keyword.
  - Assert representative `Config::set` plus `format_config` output for both
    rows.
  - Assert raw-empty reset behavior for both rows.
  - Assert representative ordering around `background-image`,
    `background-image-opacity`, `background-image-position`,
    `background-image-fit`, and `background-image-repeat` without promoting the
    adjacent scalar/path/boolean rows.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Classify exactly `background-image-fit` and `background-image-position` as
    `background image enum`.
  - Detect `background_image_enum_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `background image enum`.
  - Make Experiment 80 the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 172 `Oracle complete` rows and 31
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo fmt --manifest-path roastty/Cargo.toml` is run after Rust edits.
- `cargo test --manifest-path roastty/Cargo.toml background_image_enum_config_formatter_family_oracle`
  passes and runs at least one test.
- Existing representative background-image tests still pass:
  - `cargo test --manifest-path roastty/Cargo.toml enum_format_entries_bgimage`;
  - `cargo test --manifest-path roastty/Cargo.toml enum_from_keyword_round_trips_misc`;
  - `cargo test --manifest-path roastty/Cargo.toml background_image_config_parse_format_reset_and_diagnose`;
  - `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=172`;
  - `audit_covered=31`;
  - `gap=0`.
- Run this matrix assertion:

  ```bash
  python3 - <<'PY'
  from pathlib import Path

  inventory = Path('issues/0805-roastty-ghostty-parity/config-formatter-inventory.md').read_text()
  matrix = Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text()

  expected = {
      'background-image-fit',
      'background-image-position',
  }

  promoted = set()
  still_audit = []
  for line in inventory.splitlines():
      if not line.startswith('| FORMAT-'):
          continue
      cells = [cell.strip() for cell in line.strip('|').split('|')]
      option = cells[1].strip('`')
      family = cells[3]
      status = cells[4]
      if family == 'background image enum' and status == 'Oracle complete':
          promoted.add(option)
      elif family == 'background image enum':
          still_audit.append((option, status))

  assert promoted == expected, promoted
  assert not still_audit, still_audit

  for option in ['background-image', 'background-image-opacity', 'background-image-repeat']:
      row = next(
          line for line in inventory.splitlines()
          if line.startswith('| FORMAT-') and f'`{option}`' in line
      )
      assert 'background image enum' not in row, row

  cfg218 = next(line for line in matrix.splitlines() if '| CFG-218 |' in line)
  assert '| Gap |' in cfg218, cfg218
  assert 'Experiment 80 inventories formatter coverage: 172 rows Oracle complete; 31 rows are not Oracle complete and 0 rows are formatter gaps.' in cfg218, cfg218
  PY
  ```

- `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  passes.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passes.
- `prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/80-background-image-enum-formatter-oracle.md`
  passes.

## Design Review

Adversarial reviewer: Codex subagent with fresh context.

Verdict: Approved.

Findings:

- No Required findings.
- Optional discipline note: the reviewer accidentally ran
  `python3 -m py_compile` without `PYTHONDONTWRITEBYTECODE=1`, creating
  `issues/0805-roastty-ghostty-parity/__pycache__/`. The generated cache was
  removed before the plan commit.

## Result

**Result:** Pass

Implemented the background-image enum formatter oracle and promoted exactly the
two planned CFG-218 rows:

- `background-image-fit`;
- `background-image-position`.

The regenerated formatter inventory reports:

- `ghostty_canonical=203`;
- `roastty_formatter_rows=203`;
- `missing_canonical_formatter_rows=0`;
- `extra_formatter_rows=0`;
- `oracle_complete=172`;
- `audit_covered=31`;
- `gap=0`.

The matrix assertion passed and confirmed that adjacent rows `background-image`,
`background-image-opacity`, and `background-image-repeat` were not classified as
`background image enum`.

Verification run:

- `cargo fmt --manifest-path roastty/Cargo.toml`
- `cargo test --manifest-path roastty/Cargo.toml background_image_enum_config_formatter_family_oracle`
  — passed, 1 test.
- `cargo test --manifest-path roastty/Cargo.toml enum_format_entries_bgimage` —
  passed, 1 test.
- `cargo test --manifest-path roastty/Cargo.toml enum_from_keyword_round_trips_misc`
  — passed, matching the existing
  `config::tests::enum_from_keyword_round_trips_misc_fullscreen` test.
- `cargo test --manifest-path roastty/Cargo.toml background_image_config_parse_format_reset_and_diagnose`
  — passed, 1 test.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle` —
  passed, 1 test.
- Formatter inventory regeneration — passed with the expected counts above.
- Matrix assertion — passed.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  — passed, then the generated `__pycache__/` artifact was removed.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` — passed.
- `prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/80-background-image-enum-formatter-oracle.md`
  — passed.
- `git diff --check` — passed.

## Conclusion

Background-image enum formatting is now a durable, non-default CFG-218 oracle
rather than broad audit coverage. The remaining formatter gap is 31
`Audit covered` rows and 0 formatter-dispatch gaps; CFG-218 correctly remains
`Gap` until those remaining rows receive focused formatter oracles.

## Completion Review

Adversarial reviewer: Codex subagent with fresh context.

Verdict: Approved.

Findings:

- No Required findings.

Reviewer verification:

- New and related Cargo tests pass.
- `cargo fmt --check`, `prettier --check`, and `git diff --check` pass.
- Matrix assertion passes.
- Inventory rows show 203 total rows, 172 `Oracle complete` rows, 31
  `Audit covered` rows, and 0 `Gap` rows.
- `background-image-fit` and `background-image-position` are the only
  `background image enum` rows.
- The README marks Experiment 80 as `Pass`, Learnings were updated, and the
  result was still uncommitted during review.

The reviewer did not rerun inventory regeneration or `py_compile` because those
commands can write generated artifacts and the review was read-only.
