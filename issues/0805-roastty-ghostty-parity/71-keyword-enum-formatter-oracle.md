# Experiment 71: Keyword enum formatter oracle

## Description

Experiment 70 promoted the final font-specific formatter row and left CFG-218 at
134 `Oracle complete` rows, 69 `Audit covered` rows, and 0 formatter gaps.

The next smallest formatter shape is direct keyword enum formatting. Pinned
Ghostty's generic enum formatter writes the enum tag keyword for non-default
values, preserving punctuation such as `linear-corrected` and `block_hollow`.
Roastty implements this shape through each enum's `format_entry` method.

This experiment should promote exactly four direct keyword enum rows:

- `alpha-blending`;
- `cursor-style`;
- `mouse-shift-capture`;
- `scrollbar`.

These rows are a deliberately small slice of the remaining `custom format_entry`
group. The experiment should not promote other keyword-like rows such as
`window-theme`, `fullscreen`, `right-click-action`, macOS/GTK enums, or packed
struct/list formatters; those require their own focused oracles.

CFG-218 should remain `Gap` because 65 formatter rows will still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `keyword_enum_config_formatter_family_oracle` test.
  - Cover every upstream keyword for:
    - `AlphaBlending`: `native`, `linear`, `linear-corrected`;
    - `CursorStyle`: `block`, `bar`, `underline`, `block_hollow`;
    - `MouseShiftCapture`: `false`, `true`, `always`, `never`;
    - `Scrollbar`: `system`, `never`.
  - Assert each enum's direct `format_entry` output.
  - Assert representative `Config::set` plus `format_config` output for
    non-default values and raw-empty reset behavior.
  - Assert representative ordering across nearby formatter rows:
    `font-shaping-break`, `alpha-blending`, `cursor-style`, `scroll-to-bottom`,
    `mouse-shift-capture`, and `scrollbar`.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Classify exactly the four covered options as `keyword enum`.
  - Detect `keyword_enum_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `keyword enum`.
  - Make Experiment 71 the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 138 `Oracle complete` rows and 65
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml keyword_enum_config_formatter_family_oracle`
  passes and runs at least one test.
- Existing focused parser/formatter tests for the covered value shapes still
  pass:
  - `cargo test --manifest-path roastty/Cargo.toml enum_format_entries`;
  - `cargo test --manifest-path roastty/Cargo.toml enum_format_entries_mac`;
  - `cargo test --manifest-path roastty/Cargo.toml cursor_style_config_keywords_parse_format_and_diagnose`;
  - `cargo test --manifest-path roastty/Cargo.toml scrollbar_config_parse_format_reset_and_diagnose`.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  still passes.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=138`;
  - `audit_covered=65`;
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
  assert 'Experiment 71 inventories formatter coverage: 138 rows Oracle complete; 65 rows are not Oracle complete and 0 rows are formatter gaps.' in cfg218, cfg218

  for option in ['alpha-blending', 'cursor-style', 'mouse-shift-capture', 'scrollbar']:
      row = row_for(option)
      assert 'keyword enum' in row and 'Oracle complete' in row, row

  for option in ['window-theme', 'env', 'input', 'scroll-to-bottom']:
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
