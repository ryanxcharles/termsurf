# Experiment 63: Optional command formatter oracle

## Description

Experiment 62 promoted the two optional single-path formatter rows and left 100
formatter rows as `Audit covered`. The next compact optional slice is optional
command formatting: `command` and `initial-command`.

Pinned Ghostty formats `Command.shell` as the shell command string and
`Command.direct` as `direct:` plus single-space-joined argv items. Optional
`null` values format as void lines (`key = `). Roastty already has parser
coverage for shell/direct prefixes, trimming, direct argument splitting, empty
optional reset behavior, diagnostics, and string conversion. This experiment
will add a formatter-specific oracle for the two optional command rows before
promoting them in CFG-218.

The target rows are:

- `command`;
- `initial-command`.

CFG-218 should remain `Gap` because 98 formatter rows will still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `optional_command_config_formatter_family_oracle` test.
  - Cover optional void output, shell command output, explicit `shell:` prefix
    normalization, direct command output, direct empty payload output, raw-empty
    reset behavior, and representative row order.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Classify only `command` and `initial-command` as `optional command`.
  - Detect `optional_command_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `optional command`.
  - Keep Experiment 63 as the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 105 `Oracle complete` rows and 98
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml optional_command_config_formatter_family_oracle`
  passes.
- `cargo test --manifest-path roastty/Cargo.toml command_config_parser_family_oracle`
  still passes.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  still passes.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=105`;
  - `audit_covered=98`;
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

  cfg217 = matrix.split('| CFG-217 |', 1)[1].split('\n', 1)[0]
  cfg218 = matrix.split('| CFG-218 |', 1)[1].split('\n', 1)[0]
  assert '| Pass   |' in cfg217 or '| Pass |' in cfg217, cfg217
  assert '| Gap    |' in cfg218 or '| Gap |' in cfg218, cfg218
  assert 'Experiment 63 inventories formatter coverage: 105 rows Oracle complete; 98 rows are not Oracle complete and 0 rows are formatter gaps.' in cfg218, cfg218

  for option in ['command', 'initial-command']:
      row = row_for(option)
      assert 'optional command' in row and 'Oracle complete' in row, row

  for option in ['theme', 'working-directory', 'macos-icon-screen-color', 'quit-after-last-window-closed-delay']:
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

Findings: none.

The reviewer verified the README link, required sections, limited two-row scope,
plausible 105/98/0 count movement, and verification criteria.

## Result

**Result:** Pass

Added `optional_command_config_formatter_family_oracle`, split the two optional
command rows into an `optional command` formatter family, and promoted only that
family.

The new oracle proves:

- optional command defaults format as void lines (`key = `);
- shell commands format as the shell command string;
- explicit `shell:` prefixes normalize away in formatted output;
- direct commands format as `direct:` plus single-space-joined argv items;
- direct empty payloads format as `direct:`;
- raw-empty values reset both optional command rows back to void output;
- representative row order is stable within upstream declaration order.

The regenerated formatter inventory now reports:

```text
ghostty_canonical=203
roastty_formatter_rows=203
missing_canonical_formatter_rows=0
extra_formatter_rows=0
oracle_complete=105
audit_covered=98
gap=0
no_output_rows=1
```

CFG-218 remains `Gap`, as intended, because 98 formatter rows still need
dedicated non-default formatter oracles.

Verification:

- `cargo test --manifest-path roastty/Cargo.toml optional_command_config_formatter_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml command_config_parser_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  passed.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reported the expected 203/203 rows, 105 `Oracle complete`, 98 `Audit covered`,
  and 0 `Gap`.
- Matrix assertion passed: CFG-217 remains `Pass`; CFG-218 remains `Gap`; all
  previously promoted formatter families remain `Oracle complete`; the two
  optional command formatter rows are `Oracle complete`; optional custom
  non-command rows and font rows remain `Audit covered`.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed.
- `prettier --write --prose-wrap always --print-width 80` completed on the
  changed Markdown files.
- `git diff --check` passed.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.

The reviewer independently reran the focused optional command formatter oracle,
the existing command parser oracle, the default formatter oracle,
`cargo fmt --check`, `git diff --check`, `prettier --check`, and the matrix
assertion. The reviewer also confirmed that only `command` and `initial-command`
are promoted as `optional command`, that the formatter matrix reports 105
`Oracle complete` rows and 98 `Audit covered` rows, and that the result commit
had not been made before review.

## Conclusion

The optional command formatter rows are now oracle-complete. The remaining
optional custom `format_entry` rows are now theme, working-directory,
color-list, duration, and enum-like values. CFG-218 remains open with 98
audit-covered formatter rows.
