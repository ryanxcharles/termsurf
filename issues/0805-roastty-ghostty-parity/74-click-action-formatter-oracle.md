# Experiment 74: Click action formatter oracle

## Description

Experiment 73 promoted six direct color formatter rows and left CFG-218 at 146
`Oracle complete` rows, 57 `Audit covered` rows, and 0 formatter gaps.

The next compact formatter shape is click-action enum output. Pinned Ghostty
uses three small enums for mouse/click behavior:

- `CopyOnSelect`: `false`, `true`, `clipboard`;
- `RightClickAction`: `ignore`, `paste`, `copy`, `copy-or-paste`,
  `context-menu`;
- `MiddleClickAction`: `primary-paste`, `ignore`.

This experiment should promote exactly these three canonical formatter rows:

- `copy-on-select`;
- `right-click-action`;
- `middle-click-action`.

It should not promote other keyword enum-like rows such as `window-theme`,
`fullscreen`, platform enums, quick-terminal enums, or packed-struct/list
formatters.

CFG-218 should remain `Gap` because 54 formatter rows will still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `click_action_config_formatter_family_oracle` test.
  - Cover every upstream keyword for:
    - `CopyOnSelect`: `false`, `true`, `clipboard`;
    - `RightClickAction`: `ignore`, `paste`, `copy`, `copy-or-paste`,
      `context-menu`;
    - `MiddleClickAction`: `primary-paste`, `ignore`.
  - Assert direct enum `format_entry` output.
  - Assert `Config::set` plus `format_config` output for representative
    non-default values.
  - Assert raw-empty reset behavior:
    - `copy-on-select` resets to the platform-resolved default in
      `Config::default()`;
    - `right-click-action` resets to `context-menu`;
    - `middle-click-action` resets to `primary-paste`.
  - Assert representative order around `title-report`, `image-storage-limit`,
    `copy-on-select`, `right-click-action`, `middle-click-action`,
    `click-repeat-interval`, and `config-file`.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Classify exactly the three covered options as `click action`.
  - Detect `click_action_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `click action`.
  - Make Experiment 74 the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 149 `Oracle complete` rows and 54
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo fmt --manifest-path roastty/Cargo.toml` is run after Rust edits.
- `cargo test --manifest-path roastty/Cargo.toml click_action_config_formatter_family_oracle`
  passes and runs at least one test.
- Existing representative click action tests still pass:
  - `cargo test --manifest-path roastty/Cargo.toml enum_format_entries_misc`;
  - `cargo test --manifest-path roastty/Cargo.toml copy_on_select_enabled_unless_false`;
  - `cargo test --manifest-path roastty/Cargo.toml right_click_action_has_the_five_upstream_variants`;
  - `cargo test --manifest-path roastty/Cargo.toml middle_click_action_has_the_two_upstream_variants`;
  - `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=149`;
  - `audit_covered=54`;
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
  assert 'Experiment 74 inventories formatter coverage: 149 rows Oracle complete; 54 rows are not Oracle complete and 0 rows are formatter gaps.' in cfg218, cfg218

  expected_click_action = {
      'copy-on-select',
      'right-click-action',
      'middle-click-action',
  }
  actual_click_action = set()
  evidence_rows = set()
  for line in rows:
      if not line.startswith('| FORMAT-'):
          continue
      cells = [cell.strip() for cell in line.strip('|').split('|')]
      option = cells[1].strip('`')
      family = cells[3]
      evidence = cells[5]
      if family == 'click action':
          actual_click_action.add(option)
      if 'Click action formatter oracle' in evidence:
          evidence_rows.add(option)
  assert actual_click_action == expected_click_action, actual_click_action
  assert evidence_rows == expected_click_action, evidence_rows

  for option in expected_click_action:
      row = row_for(option)
      assert 'click action' in row and 'Oracle complete' in row, row

  for option in ['window-theme', 'fullscreen', 'macos-titlebar-style', 'quick-terminal-position', 'env']:
      row = row_for(option)
      assert 'click action' not in row, row

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

Implemented `click_action_config_formatter_family_oracle` and promoted exactly
the three click action formatter rows:

- `copy-on-select`;
- `right-click-action`;
- `middle-click-action`.

Verification commands:

- `cargo fmt --manifest-path roastty/Cargo.toml` ran after the Rust edits.
- `cargo test --manifest-path roastty/Cargo.toml click_action_config_formatter_family_oracle`
  passed and ran 1 unit test.
- `cargo test --manifest-path roastty/Cargo.toml enum_format_entries_misc`
  passed and ran 1 unit test.
- `cargo test --manifest-path roastty/Cargo.toml copy_on_select_enabled_unless_false`
  passed and ran 1 unit test.
- `cargo test --manifest-path roastty/Cargo.toml right_click_action_has_the_five_upstream_variants`
  passed and ran 1 unit test.
- `cargo test --manifest-path roastty/Cargo.toml middle_click_action_has_the_two_upstream_variants`
  passed and ran 1 unit test.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  passed and ran 1 unit test.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reported:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=149`;
  - `audit_covered=54`;
  - `gap=0`.
- The planned matrix assertion passed.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed.
- `prettier --check --prose-wrap always --print-width 80` passed on the changed
  Markdown files.
- `git diff --check` passed.

## Conclusion

The click action formatter family is now proven for `copy-on-select`,
`right-click-action`, and `middle-click-action`. CFG-218 remains `Gap` because
54 custom formatter rows still need non-default formatter oracles.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.
