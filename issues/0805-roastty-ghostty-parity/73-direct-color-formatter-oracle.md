# Experiment 73: Direct color formatter oracle

## Description

Experiment 72 promoted the two clipboard access formatter rows and left CFG-218
at 140 `Oracle complete` rows, 63 `Audit covered` rows, and 0 formatter gaps.

The next cohesive formatter shape is direct, non-optional color output. Pinned
Ghostty uses `Color.formatEntry` for `background` and `foreground`, and
`TerminalColor.formatEntry` for the four search color rows. Roastty already has
the low-level `Color` and `TerminalColor` formatter helpers; this experiment
should prove those helpers are wired correctly through `Config::format_config`
for the six canonical direct color rows:

- `background`;
- `foreground`;
- `search-foreground`;
- `search-background`;
- `search-selected-foreground`;
- `search-selected-background`.

This experiment should not promote `palette`, optional color rows, bold color,
window titlebar colors, cursor/selection colors, or any other custom formatter.

CFG-218 should remain `Gap` because 57 formatter rows will still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `direct_color_config_formatter_family_oracle` test.
  - Cover direct `Color.format_entry` lowercase `#rrggbb` output through
    `background` and `foreground`.
  - Cover direct `TerminalColor.format_entry` output through all four search
    color rows:
    - explicit color output as lowercase `#rrggbb`;
    - `cell-foreground`;
    - `cell-background`.
  - Assert raw-empty reset behavior for all six rows.
  - Assert representative order around `theme`, `background`, `foreground`,
    `background-image`, `selection-word-chars`, `palette`, `cursor-color`,
    `split-preserve-zoom`, the four search color rows, and `command`.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Classify exactly the six covered options as `direct color`.
  - Detect `direct_color_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `direct color`.
  - Make Experiment 73 the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 146 `Oracle complete` rows and 57
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo fmt --manifest-path roastty/Cargo.toml` is run after Rust edits.
- `cargo test --manifest-path roastty/Cargo.toml direct_color_config_formatter_family_oracle`
  passes and runs at least one test.
- Existing representative color/parser/formatter tests still pass:
  - `cargo test --manifest-path roastty/Cargo.toml color_format_entry_writes_hex_string`;
  - `cargo test --manifest-path roastty/Cargo.toml terminal_and_bold_color_format_entry`;
  - `cargo test --manifest-path roastty/Cargo.toml color_config_parser_family_oracle`;
  - `cargo test --manifest-path roastty/Cargo.toml search_color_config_defaults_parse_format_and_diagnose`;
  - `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=146`;
  - `audit_covered=57`;
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
  assert 'Experiment 73 inventories formatter coverage: 146 rows Oracle complete; 57 rows are not Oracle complete and 0 rows are formatter gaps.' in cfg218, cfg218

  expected_direct_color = {
      'background',
      'foreground',
      'search-foreground',
      'search-background',
      'search-selected-foreground',
      'search-selected-background',
  }
  actual_direct_color = set()
  evidence_rows = set()
  for line in rows:
      if not line.startswith('| FORMAT-'):
          continue
      cells = [cell.strip() for cell in line.strip('|').split('|')]
      option = cells[1].strip('`')
      family = cells[3]
      evidence = cells[5]
      if family == 'direct color':
          actual_direct_color.add(option)
      if 'Direct color formatter oracle' in evidence:
          evidence_rows.add(option)
  assert actual_direct_color == expected_direct_color, actual_direct_color
  assert evidence_rows == expected_direct_color, evidence_rows

  for option in expected_direct_color:
      row = row_for(option)
      assert 'direct color' in row and 'Oracle complete' in row, row

  for option in ['palette', 'cursor-color', 'selection-foreground', 'window-titlebar-background', 'env']:
      row = row_for(option)
      assert 'direct color' not in row, row

  print('matrix assertions passed')
  PY
  ```

- `cargo fmt --manifest-path roastty/Cargo.toml --check` passes.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  Markdown files after the final generator run.
- `prettier --check --prose-wrap always --print-width 80` passes on changed
  Markdown files.
- `git diff --check` passes.

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.
