# Experiment 70: Font shaping break formatter oracle

## Description

Experiment 69 promoted the two codepoint-map formatter rows and left CFG-218 at
133 `Oracle complete` rows, 70 `Audit covered` rows, and 0 formatter gaps.

The last remaining `font` formatter row is `font-shaping-break`. Pinned Ghostty
defines it as a packed struct with one boolean field, `cursor: bool = true`. The
formatter therefore emits the single `[no-]cursor` flag keyword through the
packed-struct formatter path.

This experiment should promote exactly `font-shaping-break`. It should not
promote the broader `custom format_entry` group, even though many of those rows
also use packed structs or custom formatter helpers.

CFG-218 should remain `Gap` because 69 formatter rows will still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `font_shaping_break_config_formatter_family_oracle` test.
  - Cover default `cursor` output.
  - Cover `no-cursor` output.
  - Cover standalone boolean parsing feeding formatter output.
  - Cover raw-empty reset behavior through `Config::set`.
  - Cover representative `format_config` ordering between `font-thicken`,
    `font-thicken-strength`, `font-shaping-break`, and `alpha-blending`.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Classify exactly `font-shaping-break` as `font shaping break`.
  - Detect `font_shaping_break_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `font shaping break`.
  - Keep Experiment 70 as the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 134 `Oracle complete` rows and 69
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml font_shaping_break_config_formatter_family_oracle`
  passes.
- Existing representative parser/formatter tests for the covered value shape
  still pass:
  - `cargo test --manifest-path roastty/Cargo.toml font_shaping_break_format_entry`;
  - `cargo test --manifest-path roastty/Cargo.toml packed_flags_config_parser_family_oracle`.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  still passes.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=134`;
  - `audit_covered=69`;
  - `gap=0`.
- Run this matrix assertion:

  ```bash
  python3 - <<'PY'
  from pathlib import Path

  matrix = Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text()
  rows = Path('issues/0805-roastty-ghostty-parity/config-formatter-inventory.md').read_text().splitlines()

  def row_for(option: str) -> str:
      for line in rows:
          if not line.startswith('| FORMAT-'):
              continue
          cells = [cell.strip() for cell in line.strip('|').split('|')]
          if len(cells) > 1 and cells[1] == f'`{option}`':
              return line
      raise AssertionError(f'missing row for {option}')

  cfg218 = matrix.split('| CFG-218 |', 1)[1].split('\n', 1)[0]
  assert '| Gap    |' in cfg218 or '| Gap |' in cfg218, cfg218
  assert 'Experiment 70 inventories formatter coverage: 134 rows Oracle complete; 69 rows are not Oracle complete and 0 rows are formatter gaps.' in cfg218, cfg218

  row = row_for('font-shaping-break')
  assert 'font shaping break' in row and 'Oracle complete' in row, row

  for option in ['cursor-style', 'window-theme', 'env', 'input']:
      row = row_for(option)
      assert 'custom format_entry' in row and 'Audit covered' in row, row

  print('matrix assertions passed')
  PY
  ```

- `cargo fmt --manifest-path roastty/Cargo.toml --check` passes.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  Markdown files.
- `git diff --check` passes.

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.

## Result

**Result:** Pass

Implemented `font_shaping_break_config_formatter_family_oracle` and promoted the
single `font shaping break` formatter row in the CFG-218 inventory.

Verification commands:

- `cargo test --manifest-path roastty/Cargo.toml font_shaping_break_config_formatter_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml font_shaping_break_format_entry`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml packed_flags_config_parser_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  passed.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reported:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=134`;
  - `audit_covered=69`;
  - `gap=0`;
  - `no_output_rows=1`.
- The matrix assertion passed.

## Conclusion

The `font-shaping-break` formatter row now has a focused formatter oracle.
CFG-218 remains `Gap`, but its formatter inventory moved from 133
`Oracle complete` / 70 `Audit covered` / 0 gaps to 134 `Oracle complete` / 69
`Audit covered` / 0 gaps. There are no remaining formatter rows classified as
`font`; the remaining incomplete formatter rows are all `custom format_entry`
rows.

## Completion Review

Reviewed by fresh-context Codex adversarial subagents.

Initial verdict: **Changes required**.

Initial findings:

- Required: `config-formatter-inventory.md` was not Prettier-formatted after
  regeneration.
- Required: `config-matrix.md` was not Prettier-formatted after regeneration.

Fix: ran Prettier on the changed Markdown files after the final generator run.

Final verdict: **Approved**.

The re-review confirmed both formatting findings were resolved, no new required
findings were introduced, the Experiment 70 matrix assertion passed,
`cargo fmt --manifest-path roastty/Cargo.toml --check` passed, and
`git diff --check` passed.
