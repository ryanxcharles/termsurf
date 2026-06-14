# Experiment 84: Custom format_entry formatter oracle

## Description

Experiment 83 left the formatter inventory with zero dispatch gaps, but CFG-218
still fails because nine rows remain `Audit covered` instead of
`Oracle complete`. Those rows all share the `custom format_entry` inventory
family:

- `background-blur`
- `env`
- `input`
- `mouse-scroll-multiplier`
- `palette`
- `quick-terminal-size`
- `selection-word-chars`
- `undo-timeout`
- `window-decoration`

This experiment will prove the final custom formatter family with a focused Rust
oracle and then promote exactly those nine rows. Passing the experiment should
make every formatter inventory row `Oracle complete`, move CFG-218 to `Pass`,
and leave no formatter audit rows.

## Changes

- `roastty/src/config/mod.rs`
  - Add `custom_format_entry_config_formatter_family_oracle`.
  - Cover direct `format_entry` output for every custom formatter value shape:
    - `BackgroundBlur`: bool keywords, macOS glass keywords, and numeric radius.
    - `RepeatableStringMap` (`env`): empty output and insertion-order
      `KEY=value` output.
    - `RepeatableReadableIo` (`input`): empty output plus `raw:` and `path:`
      tagged output.
    - `MouseScrollMultiplier`: `precision:<f>,discrete:<f>` output.
    - `Palette`: all 256 `index=#rrggbb` output rows.
    - `QuickTerminalSize`: no output when unset, one-value output, and two-value
      output.
    - `SelectionWordChars`: empty/default string output and UTF-8 re-encoding.
    - `Duration` as used by `undo-timeout`: normal duration output and zero
      duration empty output.
    - `WindowDecoration`: every keyword output.
  - Cover representative `Config::set` plus `format_config` output for all nine
    rows in one non-default config.
  - Cover raw-empty reset behavior for the resettable rows: `background-blur`,
    `env`, `input`, `quick-terminal-size`, `selection-word-chars`, and
    `undo-timeout`. For `mouse-scroll-multiplier`, keep the existing
    parser-family behavior that raw empty is a no-op. For `palette`, keep the
    existing parser behavior that empty resets to the default palette.
  - Cover representative formatter order around the nine rows:
    - `selection-word-chars < palette < palette-generate`
    - `mouse-scroll-multiplier < background-opacity < background-blur`
    - `background-blur < unfocused-split-opacity`
    - `env < input < wait-after-command`
    - `window-padding-color < window-decoration < window-title-font-family`
    - `undo-timeout < quick-terminal-position < quick-terminal-size`
    - `quick-terminal-size < gtk-quick-terminal-layer`

- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Add `CUSTOM_FORMAT_ENTRY_ORACLE_TEST`.
  - Promote only rows whose family is exactly `custom format_entry` when that
    oracle is present.
  - Update the CFG-218 owner cascade so Experiment 84 owns the all-complete
    formatter result.
  - Update the complete CFG-218 note to include the oracle count and owner, for
    example
    `Experiment 84 completes formatter coverage: 203 rows Oracle complete; 0 rows are not Oracle complete and 0 rows are formatter gaps.`
    This keeps the generated matrix assertion concrete instead of relying on a
    generic complete message.

- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory. Expected counts:
    - `Canonical formatter rows`: 203
    - `Oracle complete rows`: 203
    - `Audit covered rows`: 0
    - `Gap rows`: 0
    - `Intentional no-output rows`: 1

- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate the matrix. CFG-218 should become `Pass` and cite Experiment 84.

- `issues/0805-roastty-ghostty-parity/README.md`
  - Update the Experiment 84 status after completion.
  - Add a learning recording that the final formatter family was the true
    CFG-218 close condition.

## Verification

- Focused Rust oracle:

  ```bash
  cargo test --manifest-path roastty/Cargo.toml custom_format_entry_config_formatter_family_oracle
  ```

- Existing parser/formatter guards for the nine rows:

  ```bash
  cargo test --manifest-path roastty/Cargo.toml background_blur_config_parser_family_oracle
  cargo test --manifest-path roastty/Cargo.toml env_config_parser_family_oracle
  cargo test --manifest-path roastty/Cargo.toml input_config_parser_family_oracle
  cargo test --manifest-path roastty/Cargo.toml mouse_scroll_multiplier_config_parser_family_oracle
  cargo test --manifest-path roastty/Cargo.toml palette_config_parser_family_oracle
  cargo test --manifest-path roastty/Cargo.toml quick_terminal_size_config_parser_family_oracle
  cargo test --manifest-path roastty/Cargo.toml quick_terminal_size_config_parse_format_reset_and_diagnose
  cargo test --manifest-path roastty/Cargo.toml selection_behavior_config_routes_and_formats
  cargo test --manifest-path roastty/Cargo.toml undo_timeout_config_parse_format_reset_and_diagnose
  cargo test --manifest-path roastty/Cargo.toml config_set_routes_enum_fields
  ```

- Regenerate formatter inventory and matrix:

  ```bash
  PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py \
    --upstream vendor/ghostty/src/config/Config.zig \
    --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig \
    --upstream-formatter vendor/ghostty/src/config/formatter.zig \
    --roastty roastty/src/config/mod.rs \
    --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md \
    --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md \
    --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
  ```

  Expected output:

  ```text
  ghostty_canonical=203
  roastty_formatter_rows=203
  missing_canonical_formatter_rows=0
  extra_formatter_rows=0
  oracle_complete=203
  audit_covered=0
  gap=0
  no_output_rows=1
  ```

- Assert the generated formatter inventory and CFG-218 status:

  ```bash
  PYTHONDONTWRITEBYTECODE=1 python3 - <<'PY'
  from pathlib import Path

  inventory = Path('issues/0805-roastty-ghostty-parity/config-formatter-inventory.md').read_text()
  matrix = Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text()
  rows = [line for line in inventory.splitlines() if line.startswith('| FORMAT-')]

  expected = {
      'background-blur',
      'env',
      'input',
      'mouse-scroll-multiplier',
      'palette',
      'quick-terminal-size',
      'selection-word-chars',
      'undo-timeout',
      'window-decoration',
  }

  promoted = set()
  for row in rows:
      cells = [cell.strip() for cell in row.strip('|').split('|')]
      option = cells[1].strip('`')
      family = cells[3]
      status = cells[4]
      if family == 'custom format_entry':
          assert status == 'Oracle complete', row
          promoted.add(option)

  assert promoted == expected, promoted
  assert all('| Audit covered |' not in row for row in rows)
  assert all('| Gap |' not in row for row in rows)

  cfg218 = next(line for line in matrix.splitlines() if '| CFG-218 |' in line)
  assert '| Pass |' in cfg218, cfg218
  assert 'Experiment 84' in cfg218, cfg218
  assert '203 rows Oracle complete' in cfg218, cfg218
  PY
  ```

- Hygiene:

  ```bash
  PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_formatter_inventory.py
  rm -rf issues/0805-roastty-ghostty-parity/__pycache__
  cargo fmt --manifest-path roastty/Cargo.toml --check
  prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/84-custom-format-entry-formatter-oracle.md
  git diff --check
  ```

## Design Review

Adversarial reviewer: Codex subagent with fresh context.

Initial verdict: Changes required.

Required finding:

- The design required asserting `203 rows Oracle complete` in CFG-218, but only
  specified updating the owner cascade. The current generator's all-complete
  note was generic, so promotion plus owner update would not satisfy the stated
  assertion.

Fix:

- Added an explicit generator change requiring the complete CFG-218 note to
  include the owner and oracle count, e.g.
  `Experiment 84 completes formatter coverage: 203 rows Oracle complete; 0 rows are not Oracle complete and 0 rows are formatter gaps.`

Final verdict: Approved.

Re-review confirmed the prior finding is resolved and found no remaining
Required findings.

## Result

**Result:** Pass

Experiment 84 added `custom_format_entry_config_formatter_family_oracle` and
promoted the final nine formatter rows to `Oracle complete`:

- `background-blur`
- `env`
- `input`
- `mouse-scroll-multiplier`
- `palette`
- `quick-terminal-size`
- `selection-word-chars`
- `undo-timeout`
- `window-decoration`

The Rust oracle covers direct formatter output, representative `Config::set`
plus `format_config` output, reset/no-op behavior, and local ordering for all
nine rows. The generator now promotes exactly the `custom format_entry` family
when that oracle is present, and the generated CFG-218 matrix row is now `Pass`.

The regenerated formatter inventory reported:

- `ghostty_canonical=203`
- `roastty_formatter_rows=203`
- `missing_canonical_formatter_rows=0`
- `extra_formatter_rows=0`
- `oracle_complete=203`
- `audit_covered=0`
- `gap=0`
- `no_output_rows=1`

Verification run:

- `cargo test --manifest-path roastty/Cargo.toml custom_format_entry_config_formatter_family_oracle`
  passed.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  passed with the counts above.
- The generated formatter inventory and CFG-218 assertion passed.
- `cargo test --manifest-path roastty/Cargo.toml background_blur_config_parser_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml env_config_parser_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml input_config_parser_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml mouse_scroll_multiplier_config_parser_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml palette_config_parser_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml quick_terminal_size_config_parser_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml quick_terminal_size_config_parse_format_reset_and_diagnose`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml selection_behavior_config_routes_and_formats`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml undo_timeout_config_parse_format_reset_and_diagnose`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml config_set_routes_enum_fields`
  passed.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  passed; generated `__pycache__/` was removed.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed.
- `prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/84-custom-format-entry-formatter-oracle.md`
  passed.
- `git diff --check` passed.

## Conclusion

CFG-218 is now closed: every formatter inventory row is `Oracle complete`.
Formatter parity is no longer the blocking Issue 805 config facet. The next
experiment should move to the remaining non-formatter config matrix gaps, such
as diagnostics, validation/finalization, source precedence, reload behavior, or
runtime/UI effects.

## Completion Review

Adversarial reviewer: Codex subagent with fresh context.

Initial verdict: Changes required.

Required findings:

- The palette oracle asserted 256 output rows plus the first and last row, but
  did not prove every middle `index=#rrggbb` row.
- The `selection-word-chars` oracle claimed UTF-8 re-encoding coverage but only
  used ASCII codepoints.

Fixes:

- Updated the palette oracle to enumerate all 256 output rows and compare each
  line against `palette.value[idx]`.
- Updated the `selection-word-chars` oracle to include non-ASCII `\u{e9}` and
  assert the exact UTF-8 output.

Re-verification after the fixes:

- `cargo test --manifest-path roastty/Cargo.toml custom_format_entry_config_formatter_family_oracle`
  passed.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed.
- `prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/84-custom-format-entry-formatter-oracle.md`
  passed.
- `git diff --check` passed.

Final verdict: Approved.

Re-review confirmed both prior findings are resolved and found no new Required
findings.
