# Experiment 79: Packed flag formatter oracle

## Description

Experiment 78 promoted the three command-finish notification formatter rows and
left CFG-218 at 164 `Oracle complete` rows, 39 `Audit covered` rows, and 0
formatter gaps.

The next compact formatter family is the remaining packed-flag formatter rows.
Pinned Ghostty uses packed struct flag formatting for these canonical options:

- `app-notifications`: `clipboard-copy`, `config-reload`;
- `bell-features`: `system`, `audio`, `attention`, `title`, `border`;
- `freetype-load-flags`: `hinting`, `force-autohint`, `monochrome`, `autohint`,
  `light`;
- `scroll-to-bottom`: `keystroke`, `output`;
- `shell-integration-features`: `cursor`, `sudo`, `title`, `ssh-env`,
  `ssh-terminfo`, `path`;
- `split-preserve-zoom`: `navigation`.

This experiment should promote exactly those six rows. It should not promote
already-proven packed-flag rows such as `font-shaping-break`,
`font-synthetic-style`, or `notify-on-command-finish-action`, and it should not
promote unrelated custom formatter rows such as `input`, `env`, `palette`, or
`quick-terminal-size`.

CFG-218 should remain `Gap` because 33 formatter rows will still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `packed_flag_config_formatter_family_oracle` test.
  - Assert direct `format_entry` output for default and representative
    non-default states of `AppNotifications`, `BellFeatures`,
    `FreetypeLoadFlags`, `ScrollToBottom`, `ShellIntegrationFeatures`, and
    `SplitPreserveZoom`.
  - Assert `Config::set` plus `format_config` output for representative
    non-default values across all six rows.
  - Assert raw-empty reset behavior for all six rows.
  - Assert representative order around the six rows without promoting unrelated
    adjacent rows:
    - `font-shaping-break`, `freetype-load-flags`, `theme`;
    - `scroll-to-bottom`, `mouse-shift-capture`;
    - `shell-integration`, `shell-integration-features`,
      `command-palette-entry`;
    - `split-divider-color`, `split-preserve-zoom`, `search-foreground`;
    - `bell-features`, `bell-audio-path`, `bell-audio-volume`,
      `app-notifications`.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Classify exactly the six covered options as `packed flag`.
  - Detect `packed_flag_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `packed flag`.
  - Make Experiment 79 the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 170 `Oracle complete` rows and 33
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo fmt --manifest-path roastty/Cargo.toml` is run after Rust edits.
- `cargo test --manifest-path roastty/Cargo.toml packed_flag_config_formatter_family_oracle`
  passes and runs at least one test.
- Existing representative packed-flag tests still pass:
  - `cargo test --manifest-path roastty/Cargo.toml packed_flags_parse_cli`;
  - `cargo test --manifest-path roastty/Cargo.toml packed_flags_parse_cli_shell_notify`;
  - `cargo test --manifest-path roastty/Cargo.toml packed_flags_config_parser_family_oracle`;
  - `cargo test --manifest-path roastty/Cargo.toml config_set_routes_packed_and_bool_fields`;
  - `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=170`;
  - `audit_covered=33`;
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
  assert 'Experiment 79 inventories formatter coverage: 170 rows Oracle complete; 33 rows are not Oracle complete and 0 rows are formatter gaps.' in cfg218, cfg218

  expected_packed_flag = {
      'app-notifications',
      'bell-features',
      'freetype-load-flags',
      'scroll-to-bottom',
      'shell-integration-features',
      'split-preserve-zoom',
  }
  actual_packed_flag = set()
  evidence_rows = set()
  for line in rows:
      if not line.startswith('| FORMAT-'):
          continue
      cells = [cell.strip() for cell in line.strip('|').split('|')]
      option = cells[1].strip('`')
      family = cells[3]
      evidence = cells[5]
      if family == 'packed flag':
          actual_packed_flag.add(option)
      if 'Packed flag formatter oracle' in evidence:
          evidence_rows.add(option)
  assert actual_packed_flag == expected_packed_flag, actual_packed_flag
  assert evidence_rows == expected_packed_flag, evidence_rows

  for option in expected_packed_flag:
      row = row_for(option)
      assert 'packed flag' in row and 'Oracle complete' in row, row

  for option in ['font-shaping-break', 'font-synthetic-style', 'notify-on-command-finish-action', 'input', 'env', 'palette', 'quick-terminal-size']:
      row = row_for(option)
      cells = [cell.strip() for cell in row.strip('|').split('|')]
      assert cells[3] != 'packed flag', row

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

Initial verdict: **Changes required**.

Required findings:

- The planned `shell-integration`, `shell-integration-features`,
  `working-directory` order check was false because `working-directory` formats
  earlier than the shell integration rows.
- The planned `app-notifications`, `quit-after-last-window-closed` order check
  was false because `quit-after-last-window-closed` formats earlier than
  `app-notifications`.

Fixes:

- Replaced the shell integration order check with `shell-integration`,
  `shell-integration-features`, `command-palette-entry`.
- Replaced the app notification order check with `bell-features`,
  `bell-audio-path`, `bell-audio-volume`, `app-notifications`.

Final verdict after re-review: **Approved**.

## Result

**Result:** Pass

Experiment 79 promoted exactly the six planned packed-flag formatter rows:
`app-notifications`, `bell-features`, `freetype-load-flags`, `scroll-to-bottom`,
`shell-integration-features`, and `split-preserve-zoom`.

Implementation:

- Added `packed_flag_config_formatter_family_oracle` in
  `roastty/src/config/mod.rs`.
- Classified exactly those six rows as the `packed flag` formatter family in
  `config_formatter_inventory.py`.
- Regenerated `config-formatter-inventory.md` and `config-matrix.md`.

Verification completed:

- `cargo fmt --manifest-path roastty/Cargo.toml`
- `cargo test --manifest-path roastty/Cargo.toml packed_flag_config_formatter_family_oracle`
  passed with 1 test.
- Representative existing tests passed:
  - `cargo test --manifest-path roastty/Cargo.toml packed_flags_parse_cli`
    passed with 2 tests because the filter also matches
    `packed_flags_parse_cli_shell_notify`.
  - `cargo test --manifest-path roastty/Cargo.toml packed_flags_parse_cli_shell_notify`
  - `cargo test --manifest-path roastty/Cargo.toml packed_flags_config_parser_family_oracle`
  - `cargo test --manifest-path roastty/Cargo.toml config_set_routes_packed_and_bool_fields`
  - `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
- The formatter inventory generator reported:
  - `ghostty_canonical=203`
  - `roastty_formatter_rows=203`
  - `missing_canonical_formatter_rows=0`
  - `extra_formatter_rows=0`
  - `oracle_complete=170`
  - `audit_covered=33`
  - `gap=0`
  - `no_output_rows=1`
- The matrix assertion passed after tightening the excluded-row check to inspect
  the family column rather than searching the whole row text. This matters
  because `notify-on-command-finish-action` is correctly owned by the command
  notification family while its evidence text legitimately mentions packed flag
  output.
- The matrix assertion verified:
  - CFG-218 remains `Gap`.
  - The CFG-218 count text is now 170 Oracle complete rows, 33 not Oracle
    complete rows, and 0 formatter gaps.
  - Exactly the six planned rows have family `packed flag`.
  - Exactly the six planned rows cite `Packed flag formatter oracle` evidence.
  - `font-shaping-break`, `font-synthetic-style`,
    `notify-on-command-finish-action`, `input`, `env`, `palette`, and
    `quick-terminal-size` were not promoted as `packed flag`.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed.
- `prettier --write --prose-wrap always --print-width 80` was run on changed
  Markdown files after the generator run.
- `prettier --check --prose-wrap always --print-width 80` passed on changed
  Markdown files.
- `git diff --check` passed.

## Conclusion

The remaining packed-flag formatter rows are now independently guarded. CFG-218
remains open because 33 formatter rows still need non-default formatter oracles,
but the packed-flag family has no remaining formatter evidence gap.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.
