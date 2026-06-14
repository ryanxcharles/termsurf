# Experiment 61: Optional color formatter oracle

## Description

Experiment 60 promoted the 11 optional scalar formatter rows and left 112
formatter rows as `Audit covered`. The next coherent slice is optional color
formatting: optional rows whose value type formats through `Color`,
`TerminalColor`, or `BoldColor`.

Pinned Ghostty formats colors as lowercase `#rrggbb`, `TerminalColor` sentinel
values as `cell-foreground` or `cell-background`, and `BoldColor.bright` as
`bright`. Optional `null` values format as void lines (`key = `). Roastty
already has broad color parser coverage, but CFG-218 should only promote these
formatter rows after a formatter-specific oracle proves their non-default output
and reset behavior directly.

The target rows are:

- `bold-color`;
- `cursor-color`;
- `cursor-text`;
- `macos-icon-ghost-color`;
- `selection-background`;
- `selection-foreground`;
- `split-divider-color`;
- `unfocused-split-fill`;
- `window-titlebar-background`;
- `window-titlebar-foreground`.

`macos-icon-screen-color` is intentionally excluded because it formats a color
list, not a single optional color.

CFG-218 should remain `Gap` because 102 formatter rows will still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `optional_color_config_formatter_family_oracle` test.
  - Cover optional void output, lowercase hex color output, named-color
    normalization, `TerminalColor` sentinel output, `BoldColor.bright`,
    raw-empty reset behavior, and representative row order.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Classify only the 10 optional single-color rows as `optional color`.
  - Detect `optional_color_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `optional color`.
  - Keep Experiment 61 as the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 101 `Oracle complete` rows and 102
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml optional_color_config_formatter_family_oracle`
  passes.
- `cargo test --manifest-path roastty/Cargo.toml color_config_parser_family_oracle`
  still passes.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  still passes.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=101`;
  - `audit_covered=102`;
  - `gap=0`.
- A matrix assertion confirms:
  - CFG-217 remains `Pass`;
  - CFG-218 remains `Gap`;
  - all previously promoted formatter families remain `Oracle complete`;
  - the 10 optional color formatter rows are `Oracle complete`;
  - `macos-icon-screen-color`, optional custom non-color rows, and font rows are
    not promoted by this oracle. Run the assertion as:

  ```bash
  python3 - <<'PY'
  from pathlib import Path

  matrix = Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text()
  rows = Path('issues/0805-roastty-ghostty-parity/config-formatter-inventory.md').read_text().splitlines()

  def row_for(option: str) -> str:
      needle = f'| `{option}` |'
      for line in rows:
          if needle in line:
              return line
      raise AssertionError(f'missing row for {option}')

  cfg217 = matrix.split('| CFG-217 |', 1)[1].split('\n', 1)[0]
  cfg218 = matrix.split('| CFG-218 |', 1)[1].split('\n', 1)[0]
  assert '| Pass   |' in cfg217 or '| Pass |' in cfg217, cfg217
  assert '| Gap    |' in cfg218 or '| Gap |' in cfg218, cfg218
  assert 'Experiment 61 inventories formatter coverage: 101 rows Oracle complete; 102 rows are not Oracle complete and 0 rows are formatter gaps.' in cfg218, cfg218

  for option in [
      'bold-color',
      'cursor-color',
      'cursor-text',
      'macos-icon-ghost-color',
      'selection-background',
      'selection-foreground',
      'split-divider-color',
      'unfocused-split-fill',
      'window-titlebar-background',
      'window-titlebar-foreground',
  ]:
      row = row_for(option)
      assert 'optional color' in row and 'Oracle complete' in row, row

  for option in ['macos-icon-screen-color', 'command', 'theme', 'working-directory']:
      row = row_for(option)
      assert 'Audit covered' in row, row

  for option in ['font-size', 'font-feature', 'window-title-font-family']:
      row = row_for(option)
      assert 'font' in row and 'Audit covered' in row, row

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

Required findings: none.

Optional finding:

- The matrix assertion was described but not directly runnable.

Fix:

- Added the exact inline Python matrix assertion snippet to the verification
  section.
