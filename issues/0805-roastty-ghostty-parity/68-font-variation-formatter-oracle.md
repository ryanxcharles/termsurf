# Experiment 68: Font variation formatter oracle

## Description

Experiment 67 promoted the style-shaped font formatter rows and left 76
formatter rows as `Audit covered`. The next compact font slice is the four rows
backed by `RepeatableFontVariation`.

These rows share the same formatter behavior: an empty list emits one void line,
and a populated list emits one `axis=value` line per item in insertion order,
with float values formatted by their canonical display text and `nan`
lowercased.

The target rows are:

- `font-variation`;
- `font-variation-bold`;
- `font-variation-italic`;
- `font-variation-bold-italic`.

CFG-218 should remain `Gap` because 72 formatter rows will still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `font_variation_config_formatter_family_oracle` test.
  - Cover empty-list void output for each target row.
  - Cover multiple formatted `axis=value` lines in insertion order.
  - Cover decimal, negative, hexadecimal-float-normalized, infinity, and `nan`
    output.
  - Cover raw-empty reset behavior.
  - Cover representative row order across the target rows.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Classify exactly the four target rows as `font variation`.
  - Detect `font_variation_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `font variation`.
  - Keep Experiment 68 as the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 131 `Oracle complete` rows and 72
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml font_variation_config_formatter_family_oracle`
  passes.
- Existing representative parser/formatter tests for the covered value shape
  still pass:
  - `cargo test --manifest-path roastty/Cargo.toml font_variation_config_parser_family_oracle`
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  still passes.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=131`;
  - `audit_covered=72`;
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
  assert 'Experiment 68 inventories formatter coverage: 131 rows Oracle complete; 72 rows are not Oracle complete and 0 rows are formatter gaps.' in cfg218, cfg218

  for option in [
      'font-variation',
      'font-variation-bold',
      'font-variation-italic',
      'font-variation-bold-italic',
  ]:
      row = row_for(option)
      assert 'font variation' in row and 'Oracle complete' in row, row

  for option in ['font-codepoint-map', 'font-shaping-break']:
      row = row_for(option)
      assert 'font' in row and 'Audit covered' in row, row

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

The reviewer verified that the README links Experiment 68 as `Designed`, the
experiment has the required sections, scope is limited to the four
`RepeatableFontVariation` rows, adjacent `font-codepoint-map` and
`font-shaping-break` rows remain explicitly guarded as `Audit covered`, the
expected 127/76/0 to 131/72/0 count movement is plausible, pinned Ghostty
behavior matches the design, and referenced existing filters resolve in
`roastty/src/config/mod.rs`.

## Result

**Result:** Pass

Implemented `font_variation_config_formatter_family_oracle` and promoted the
four `font variation` formatter rows in the CFG-218 inventory.

Verification commands:

- `cargo test --manifest-path roastty/Cargo.toml font_variation_config_formatter_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml font_variation_config_parser_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  passed.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reported:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=131`;
  - `audit_covered=72`;
  - `gap=0`;
  - `no_output_rows=1`.
- The matrix assertion passed.

## Conclusion

The four `font-variation*` formatter rows now have a focused formatter oracle.
CFG-218 remains `Gap`, but its formatter inventory moved from 127
`Oracle complete` / 76 `Audit covered` / 0 gaps to 131 `Oracle complete` / 72
`Audit covered` / 0 gaps. The remaining font-classified formatter rows are
`font-codepoint-map` and `font-shaping-break`; the other 70 incomplete formatter
rows remain `custom format_entry` rows.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.
