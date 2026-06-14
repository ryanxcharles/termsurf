# Experiment 77: Quick terminal enum formatter oracle

## Description

Experiment 76 promoted the three resize overlay formatter rows and left CFG-218
at 156 `Oracle complete` rows, 47 `Audit covered` rows, and 0 formatter gaps.

The next compact formatter family is the simple quick-terminal enum subset.
Pinned Ghostty uses enum keyword formatting for these adjacent options:

- `quick-terminal-position`: `top`, `bottom`, `left`, `right`, `center`;
- `gtk-quick-terminal-layer`: `overlay`, `top`, `bottom`, `background`;
- `quick-terminal-screen`: `main`, `mouse`, `macos-menu-bar`;
- `quick-terminal-space-behavior`: `remain`, `move`;
- `quick-terminal-keyboard-interactivity`: `none`, `on-demand`, `exclusive`.

This experiment should promote exactly those five rows. It should not promote
`quick-terminal-size`, `gtk-quick-terminal-namespace`,
`quick-terminal-animation-duration`, `quick-terminal-autohide`, or unrelated
quick-terminal, GTK, platform, or enum-like rows.

CFG-218 should remain `Gap` because 42 formatter rows will still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `quick_terminal_enum_config_formatter_family_oracle` test.
  - Cover every upstream keyword for `QuickTerminalPosition`,
    `QuickTerminalLayer`, `QuickTerminalScreen`, `QuickTerminalSpaceBehavior`,
    and `QuickTerminalKeyboardInteractivity`.
  - Assert direct enum `format_entry` output.
  - Assert `Config::set` plus `format_config` output for representative
    non-default values across all five rows.
  - Assert raw-empty reset behavior for all five rows.
  - Assert representative order around `undo-timeout`,
    `quick-terminal-position`, `quick-terminal-size`,
    `gtk-quick-terminal-layer`, `gtk-quick-terminal-namespace`,
    `quick-terminal-screen`, `quick-terminal-animation-duration`,
    `quick-terminal-autohide`, `quick-terminal-space-behavior`,
    `quick-terminal-keyboard-interactivity`, and `shell-integration`.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Classify exactly the five covered options as `quick terminal enum`.
  - Detect `quick_terminal_enum_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `quick terminal enum`.
  - Make Experiment 77 the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 161 `Oracle complete` rows and 42
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo fmt --manifest-path roastty/Cargo.toml` is run after Rust edits.
- `cargo test --manifest-path roastty/Cargo.toml quick_terminal_enum_config_formatter_family_oracle`
  passes and runs at least one test.
- Existing representative quick-terminal enum tests still pass:
  - `cargo test --manifest-path roastty/Cargo.toml quick_terminal_position_config_parse_format_reset_and_diagnose`;
  - `cargo test --manifest-path roastty/Cargo.toml gtk_quick_terminal_config_parse_format_reset_and_diagnose`;
  - `cargo test --manifest-path roastty/Cargo.toml quick_terminal_screen_animation_config_parse_format_reset_and_diagnose`;
  - `cargo test --manifest-path roastty/Cargo.toml quick_terminal_space_keyboard_config_parse_format_reset_and_diagnose`;
  - `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=161`;
  - `audit_covered=42`;
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
  assert 'Experiment 77 inventories formatter coverage: 161 rows Oracle complete; 42 rows are not Oracle complete and 0 rows are formatter gaps.' in cfg218, cfg218

  expected_quick_terminal_enum = {
      'quick-terminal-position',
      'gtk-quick-terminal-layer',
      'quick-terminal-screen',
      'quick-terminal-space-behavior',
      'quick-terminal-keyboard-interactivity',
  }
  actual_quick_terminal_enum = set()
  evidence_rows = set()
  for line in rows:
      if not line.startswith('| FORMAT-'):
          continue
      cells = [cell.strip() for cell in line.strip('|').split('|')]
      option = cells[1].strip('`')
      family = cells[3]
      evidence = cells[5]
      if family == 'quick terminal enum':
          actual_quick_terminal_enum.add(option)
      if 'Quick terminal enum formatter oracle' in evidence:
          evidence_rows.add(option)
  assert actual_quick_terminal_enum == expected_quick_terminal_enum, actual_quick_terminal_enum
  assert evidence_rows == expected_quick_terminal_enum, evidence_rows

  for option in expected_quick_terminal_enum:
      row = row_for(option)
      assert 'quick terminal enum' in row and 'Oracle complete' in row, row

  for option in ['quick-terminal-size', 'gtk-quick-terminal-namespace', 'quick-terminal-animation-duration', 'quick-terminal-autohide', 'gtk-titlebar-style']:
      row = row_for(option)
      assert 'quick terminal enum' not in row, row

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

## Result

**Result:** Pass

Experiment 77 promoted exactly the five planned quick-terminal enum formatter
rows: `quick-terminal-position`, `gtk-quick-terminal-layer`,
`quick-terminal-screen`, `quick-terminal-space-behavior`, and
`quick-terminal-keyboard-interactivity`.

Implementation:

- Added `quick_terminal_enum_config_formatter_family_oracle` in
  `roastty/src/config/mod.rs`.
- Classified exactly those five rows as the `quick terminal enum` formatter
  family in `config_formatter_inventory.py`.
- Regenerated `config-formatter-inventory.md` and `config-matrix.md`.

Verification completed:

- `cargo fmt --manifest-path roastty/Cargo.toml`
- `cargo test --manifest-path roastty/Cargo.toml quick_terminal_enum_config_formatter_family_oracle`
  passed with 1 test.
- Representative existing tests passed:
  - `cargo test --manifest-path roastty/Cargo.toml quick_terminal_position_config_parse_format_reset_and_diagnose`
  - `cargo test --manifest-path roastty/Cargo.toml gtk_quick_terminal_config_parse_format_reset_and_diagnose`
  - `cargo test --manifest-path roastty/Cargo.toml quick_terminal_screen_animation_config_parse_format_reset_and_diagnose`
  - `cargo test --manifest-path roastty/Cargo.toml quick_terminal_space_keyboard_config_parse_format_reset_and_diagnose`
  - `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
- The formatter inventory generator reported:
  - `ghostty_canonical=203`
  - `roastty_formatter_rows=203`
  - `missing_canonical_formatter_rows=0`
  - `extra_formatter_rows=0`
  - `oracle_complete=161`
  - `audit_covered=42`
  - `gap=0`
  - `no_output_rows=1`
- The matrix assertion passed and verified:
  - CFG-218 remains `Gap`.
  - The CFG-218 count text is now 161 Oracle complete rows, 42 not Oracle
    complete rows, and 0 formatter gaps.
  - Exactly the five planned rows have family `quick terminal enum`.
  - Exactly the five planned rows cite `Quick terminal enum formatter oracle`
    evidence.
  - `quick-terminal-size`, `gtk-quick-terminal-namespace`,
    `quick-terminal-animation-duration`, `quick-terminal-autohide`, and
    `gtk-titlebar-style` were not promoted as `quick terminal enum`.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed.
- `prettier --write --prose-wrap always --print-width 80` was run on changed
  Markdown files after the generator run.
- `prettier --check --prose-wrap always --print-width 80` passed on changed
  Markdown files.
- `git diff --check` passed.

## Conclusion

The simple quick-terminal enum formatter slice is now independently guarded.
CFG-218 remains open because 42 formatter rows still need non-default formatter
oracles, but the quick-terminal enum family has no remaining formatter evidence
gap.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.
