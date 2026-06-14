# Experiment 64: Optional value formatter oracle

## Description

Experiment 63 promoted the two optional command formatter rows and left 98
formatter rows as `Audit covered`. The next smallest remaining formatter family
is the six-row `optional value` family. These rows all use
`entry_optional(..., |v, f| v.format_entry(f))`, but their inner value
formatters cover different shapes: enums, durations, color lists, themes, and
working directories.

The target rows are:

- `auto-update`;
- `auto-update-channel`;
- `macos-icon-screen-color`;
- `quit-after-last-window-closed-delay`;
- `theme`;
- `working-directory`.

CFG-218 should remain `Gap` because 92 formatter rows will still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `optional_value_config_formatter_family_oracle` test.
  - Cover optional void output for all six rows.
  - Cover enum keyword output for `auto-update` and `auto-update-channel`.
  - Cover comma-joined color-list output for `macos-icon-screen-color`.
  - Cover decomposed duration output for `quit-after-last-window-closed-delay`.
  - Cover single-name and light/dark pair output for `theme`.
  - Cover keyword and path output for `working-directory`.
  - Cover raw-empty reset behavior and representative row order.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Detect `optional_value_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `optional value`.
  - Keep Experiment 64 as the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 111 `Oracle complete` rows and 92
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml optional_value_config_formatter_family_oracle`
  passes.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  still passes.
- Representative existing parser tests for the covered value types still pass:
  - `cargo test --manifest-path roastty/Cargo.toml duration_config_parser_family_oracle`
  - `cargo test --manifest-path roastty/Cargo.toml working_directory_config_parser_family_oracle`
  - `cargo test --manifest-path roastty/Cargo.toml theme_config_parser_family_oracle`
  - `cargo test --manifest-path roastty/Cargo.toml color_list_parse_cli_parses_comma_separated_colors`
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=111`;
  - `audit_covered=92`;
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
  assert 'Experiment 64 inventories formatter coverage: 111 rows Oracle complete; 92 rows are not Oracle complete and 0 rows are formatter gaps.' in cfg218, cfg218

  for option in [
      'auto-update',
      'auto-update-channel',
      'macos-icon-screen-color',
      'quit-after-last-window-closed-delay',
      'theme',
      'working-directory',
  ]:
      row = row_for(option)
      assert 'optional value' in row and 'Oracle complete' in row, row

  for option in ['font-size', 'font-feature', 'window-title-font-family']:
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

The reviewer verified that the README links Experiment 64 as `Designed`, the
experiment has the required sections, the scope is exactly the six
`optional value` formatter rows, the expected 105/98/0 to 111/92/0 count
movement is plausible, and the verification criteria check both promotion and
adjacent unpromoted families.

## Result

**Result:** Pass

Added `optional_value_config_formatter_family_oracle`, detected that oracle in
the formatter inventory generator, and promoted only the six `optional value`
formatter rows.

The new oracle proves:

- optional defaults format as void lines (`key = `) for all six rows;
- `auto-update` and `auto-update-channel` format as enum keywords;
- `macos-icon-screen-color` formats as comma-joined lowercase `#rrggbb` colors;
- `quit-after-last-window-closed-delay` formats as a decomposed duration string;
- `theme` formats as either one name or `light:{name},dark:{name}`;
- `working-directory` formats as a keyword or path;
- raw-empty values reset all six rows back to void output;
- representative row order is stable within formatter order.

The regenerated formatter inventory now reports:

```text
ghostty_canonical=203
roastty_formatter_rows=203
missing_canonical_formatter_rows=0
extra_formatter_rows=0
oracle_complete=111
audit_covered=92
gap=0
no_output_rows=1
```

CFG-218 remains `Gap`, as intended, because 92 formatter rows still need
dedicated non-default formatter oracles.

Verification:

- `cargo test --manifest-path roastty/Cargo.toml optional_value_config_formatter_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml duration_config_parser_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml working_directory_config_parser_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml theme_config_parser_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml color_list_parse_cli_parses_comma_separated_colors`
  passed.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reported the expected 203/203 rows, 111 `Oracle complete`, 92 `Audit covered`,
  and 0 `Gap`.
- Matrix assertion passed: CFG-218 remains `Gap`; all six `optional value`
  formatter rows are `Oracle complete`; representative font and custom
  `format_entry` rows remain `Audit covered`.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.

The reviewer confirmed that exactly the six `optional value` rows are promoted,
the generated inventory reports 203 canonical rows, 203 Roastty rows, 111
`Oracle complete`, 92 `Audit covered`, and 0 formatter gaps, CFG-218 remains
`Gap` and owned by Experiment 64, README status is `Pass`, and changed files are
limited to the requested source, generator, and issue artifacts. The reviewer
also reran the focused optional value formatter oracle, the representative
parser/default tests, `cargo fmt --check`, `git diff --check`, and read-only
matrix/table assertions.

## Conclusion

The optional value formatter rows are now oracle-complete. CFG-218 remains open
with 92 audit-covered formatter rows: 22 font rows and 70 custom `format_entry`
rows.
