# Experiment 81: GTK enum formatter oracle

## Description

Experiment 80 promoted the two background-image enum formatter rows and left
CFG-218 at 172 `Oracle complete` rows, 31 `Audit covered` rows, and 0 formatter
gaps.

The next compact formatter family is the GTK enum cluster:

- `gtk-single-instance`: `false`, `true`, `detect`;
- `gtk-tabs-location`: `top`, `bottom`;
- `gtk-toolbar-style`: `flat`, `raised`, `raised-border`;
- `gtk-titlebar-style`: `native`, `tabs`.

Roastty already has parser coverage for the GTK chrome cluster and direct
`from_keyword` checks for these enum types. This experiment should promote only
the four enum formatter rows by proving direct enum formatter output, the
`Config::set` to `Config::format_config` path, raw-empty resets, compatibility
inputs that normalize to these formatter rows, and local formatter ordering. It
should not promote adjacent GTK booleans such as `gtk-titlebar`,
`gtk-titlebar-hide-when-maximized`, `gtk-wide-tabs`, or repeatable path rows
such as `gtk-custom-css`.

CFG-218 should remain `Gap` because 27 formatter rows will still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `gtk_enum_config_formatter_family_oracle` test.
  - Assert direct `format_entry` output for every `GtkSingleInstance`,
    `GtkTabsLocation`, `GtkToolbarStyle`, and `GtkTitlebarStyle` keyword.
  - Assert representative `Config::set` plus `format_config` output for all four
    rows.
  - Assert compatibility behavior that feeds the same formatter rows:
    - `gtk-single-instance = desktop` formats as `gtk-single-instance = detect`;
    - `adw-toolbar-style = flat` formats as `gtk-toolbar-style = flat`;
    - `gtk-tabs-location = hidden` hides the tab bar without changing the
      formatted `gtk-tabs-location` row.
  - Assert raw-empty reset behavior for all four rows.
  - Assert representative ordering around `gtk-opengl-debug`,
    `gtk-single-instance`, `gtk-titlebar`, `gtk-tabs-location`,
    `gtk-titlebar-hide-when-maximized`, `gtk-toolbar-style`,
    `gtk-titlebar-style`, `gtk-wide-tabs`, and `gtk-custom-css` without
    promoting adjacent scalar or repeatable-path rows.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Classify exactly the four covered options as `gtk enum`.
  - Detect `gtk_enum_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `gtk enum`.
  - Make Experiment 81 the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 176 `Oracle complete` rows and 27
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo fmt --manifest-path roastty/Cargo.toml` is run after Rust edits.
- `cargo test --manifest-path roastty/Cargo.toml gtk_enum_config_formatter_family_oracle`
  passes and runs at least one test.
- Existing representative GTK tests still pass:
  - `cargo test --manifest-path roastty/Cargo.toml gtk_chrome_config_parse_format_compat_reset_and_diagnose`;
  - `cargo test --manifest-path roastty/Cargo.toml config_compatibility_alias_semantics`;
  - `cargo test --manifest-path roastty/Cargo.toml config_gtk_single_instance_finalize`;
  - `cargo test --manifest-path roastty/Cargo.toml enum_from_keyword_round_trips_misc`;
  - `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=176`;
  - `audit_covered=27`;
  - `gap=0`.
- Run this matrix assertion:

  ```bash
  python3 - <<'PY'
  from pathlib import Path

  inventory = Path('issues/0805-roastty-ghostty-parity/config-formatter-inventory.md').read_text()
  matrix = Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text()

  expected = {
      'gtk-single-instance',
      'gtk-tabs-location',
      'gtk-toolbar-style',
      'gtk-titlebar-style',
  }

  promoted = set()
  still_audit = []
  for line in inventory.splitlines():
      if not line.startswith('| FORMAT-'):
          continue
      cells = [cell.strip() for cell in line.strip('|').split('|')]
      option = cells[1].strip('`')
      family = cells[3]
      status = cells[4]
      if family == 'gtk enum' and status == 'Oracle complete':
          promoted.add(option)
      elif family == 'gtk enum':
          still_audit.append((option, status))

  assert promoted == expected, promoted
  assert not still_audit, still_audit

  for option in [
      'gtk-opengl-debug',
      'gtk-titlebar',
      'gtk-titlebar-hide-when-maximized',
      'gtk-wide-tabs',
      'gtk-custom-css',
  ]:
      row = next(
          line for line in inventory.splitlines()
          if line.startswith('| FORMAT-') and f'`{option}`' in line
      )
      assert 'gtk enum' not in row, row

  cfg218 = next(line for line in matrix.splitlines() if '| CFG-218 |' in line)
  assert '| Gap |' in cfg218, cfg218
  assert 'Experiment 81 inventories formatter coverage: 176 rows Oracle complete; 27 rows are not Oracle complete and 0 rows are formatter gaps.' in cfg218, cfg218
  PY
  ```

- `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  passes; remove any generated `__pycache__/` artifact afterward.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passes.
- `prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/81-gtk-enum-formatter-oracle.md`
  passes.
- `git diff --check` passes.

## Design Review

Adversarial reviewer: Codex subagent with fresh context.

Verdict: Approved.

Findings:

- No Required findings.

Reviewer evidence:

- The README links Experiment 81 as `Designed`.
- The experiment has Description, Changes, and Verification sections.
- The GTK compatibility plan matches Roastty and upstream behavior:
  - `gtk-single-instance = desktop` maps to `detect`;
  - `gtk-tabs-location = hidden` hides the tab bar without changing the enum;
  - `adw-toolbar-style` aliases `gtk-toolbar-style`.
- Verification includes a focused Rust test, existing GTK/parser coverage,
  inventory regeneration with expected counts, a non-vacuous matrix assertion,
  `py_compile`, `cargo fmt --check`, Prettier, and `git diff --check`.
