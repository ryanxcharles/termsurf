# Experiment 82: macOS enum formatter oracle

## Description

Experiment 81 promoted the four GTK enum formatter rows and left CFG-218 at 176
`Oracle complete` rows, 27 `Audit covered` rows, and 0 formatter gaps.

The next compact formatter family is the macOS enum cluster:

- `macos-non-native-fullscreen`: `false`, `true`, `visible-menu`,
  `padded-notch`;
- `macos-window-buttons`: `visible`, `hidden`;
- `macos-titlebar-style`: `native`, `transparent`, `tabs`, `hidden`;
- `macos-titlebar-proxy-icon`: `visible`, `hidden`;
- `macos-dock-drop-behavior`: `new-tab`, `new-window`;
- `macos-hidden`: `never`, `always`;
- `macos-icon`: `official`, `blueprint`, `chalkboard`, `microchip`, `glass`,
  `holographic`, `paper`, `retro`, `xray`, `custom`, `custom-style`;
- `macos-icon-frame`: `aluminum`, `beige`, `plastic`, `chrome`;
- `macos-shortcuts`: `allow`, `deny`, `ask`.

Roastty already has direct enum `format_entry` coverage for several of these
types and parser coverage for the macOS icon, tail, shortcuts, and dock-drop
compatibility behaviors. This experiment should promote only the nine macOS enum
formatter rows by proving direct enum formatter output, `Config::set` plus
`Config::format_config`, raw-empty resets, the
`macos-dock-drop-behavior = window` compatibility input, and local formatter
ordering. It should not promote adjacent macOS scalar/optional rows such as
`macos-option-as-alt`, `macos-window-shadow`, `macos-custom-icon`,
`macos-icon-ghost-color`, or `macos-icon-screen-color`.

CFG-218 should remain `Gap` because 18 formatter rows will still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `macos_enum_config_formatter_family_oracle` test.
  - Assert direct `format_entry` output for every `NonNativeFullscreen`,
    `MacWindowButtons`, `MacTitlebarStyle`, `MacTitlebarProxyIcon`,
    `MacOSDockDropBehavior`, `MacHidden`, `MacAppIcon`, `MacAppIconFrame`, and
    `MacShortcuts` keyword.
  - Assert representative `Config::set` plus `format_config` output for all nine
    rows.
  - Assert `macos-dock-drop-behavior = window` formats as
    `macos-dock-drop-behavior = new-window`.
  - Assert raw-empty reset behavior for all nine rows.
  - Assert representative ordering around the macOS formatter block:
    - `app-notifications`, `macos-non-native-fullscreen`,
      `macos-window-buttons`, `macos-titlebar-style`,
      `macos-titlebar-proxy-icon`, `macos-dock-drop-behavior`,
      `macos-option-as-alt`, `macos-window-shadow`, `macos-hidden`,
      `macos-auto-secure-input`;
    - `macos-icon`, `macos-custom-icon`, `macos-icon-frame`,
      `macos-icon-ghost-color`, `macos-icon-screen-color`, `macos-shortcuts`,
      `linux-cgroup`.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Classify exactly the nine covered options as `macos enum`.
  - Detect `macos_enum_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `macos enum`.
  - Make Experiment 82 the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 185 `Oracle complete` rows and 18
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo fmt --manifest-path roastty/Cargo.toml` is run after Rust edits.
- `cargo test --manifest-path roastty/Cargo.toml macos_enum_config_formatter_family_oracle`
  passes and runs at least one test.
- Existing representative macOS tests still pass:
  - `cargo test --manifest-path roastty/Cargo.toml enum_format_entries_mac`;
  - `cargo test --manifest-path roastty/Cargo.toml enum_format_entries_fullscreen`;
  - `cargo test --manifest-path roastty/Cargo.toml enum_from_keyword_round_trips_mac_bgimage_shader`;
  - `cargo test --manifest-path roastty/Cargo.toml enum_from_keyword_round_trips_misc_fullscreen`;
  - `cargo test --manifest-path roastty/Cargo.toml macos_icon_config_parse_format_reset_and_diagnose`;
  - `cargo test --manifest-path roastty/Cargo.toml macos_tail_config_parse_format_reset_and_diagnose`;
  - `cargo test --manifest-path roastty/Cargo.toml macos_shortcuts_config_parse_format_reset_and_diagnose`;
  - `cargo test --manifest-path roastty/Cargo.toml config_compatibility_alias_semantics`;
  - `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=185`;
  - `audit_covered=18`;
  - `gap=0`.
- Run this matrix assertion:

  ```bash
  python3 - <<'PY'
  from pathlib import Path

  inventory = Path('issues/0805-roastty-ghostty-parity/config-formatter-inventory.md').read_text()
  matrix = Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text()

  expected = {
      'macos-non-native-fullscreen',
      'macos-window-buttons',
      'macos-titlebar-style',
      'macos-titlebar-proxy-icon',
      'macos-dock-drop-behavior',
      'macos-hidden',
      'macos-icon',
      'macos-icon-frame',
      'macos-shortcuts',
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
      if family == 'macos enum' and status == 'Oracle complete':
          promoted.add(option)
      elif family == 'macos enum':
          still_audit.append((option, status))

  assert promoted == expected, promoted
  assert not still_audit, still_audit

  for option in [
      'macos-option-as-alt',
      'macos-window-shadow',
      'macos-auto-secure-input',
      'macos-custom-icon',
      'macos-icon-ghost-color',
      'macos-icon-screen-color',
      'linux-cgroup',
  ]:
      row = next(
          line for line in inventory.splitlines()
          if line.startswith('| FORMAT-') and f'`{option}`' in line
      )
      assert 'macos enum' not in row, row

  cfg218 = next(line for line in matrix.splitlines() if '| CFG-218 |' in line)
  assert '| Gap |' in cfg218, cfg218
  assert 'Experiment 82 inventories formatter coverage: 185 rows Oracle complete; 18 rows are not Oracle complete and 0 rows are formatter gaps.' in cfg218, cfg218
  PY
  ```

- `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  passes; remove any generated `__pycache__/` artifact afterward.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passes.
- `prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/82-macos-enum-formatter-oracle.md`
  passes.
- `git diff --check` passes.

## Design Review

Adversarial reviewer: Codex subagent with fresh context.

Verdict: Approved.

Findings:

- No Required findings.

Reviewer evidence:

- The README links Experiment 82 as `Designed`.
- The experiment has Description, Changes, and Verification sections.
- The scope is narrowly limited to the nine remaining macOS direct enum
  formatter rows.
- The `macos-dock-drop-behavior = window` compatibility case is faithful to
  upstream `compatMacOSDockDropBehavior`.
- The expected count transition is consistent with the current inventory: 176
  `Oracle complete` plus 9 promoted rows gives 185, leaving 18 `Audit covered`
  rows and 0 formatter gaps.
- Verification includes focused Rust tests, existing regression tests,
  regenerated inventory and matrix checks, non-vacuous matrix assertions,
  `py_compile`, `cargo fmt --check`, Prettier, and `git diff --check`.

## Result

**Result:** Pass

Implemented the macOS enum formatter oracle and promoted exactly the nine
planned CFG-218 rows:

- `macos-non-native-fullscreen`;
- `macos-window-buttons`;
- `macos-titlebar-style`;
- `macos-titlebar-proxy-icon`;
- `macos-dock-drop-behavior`;
- `macos-hidden`;
- `macos-icon`;
- `macos-icon-frame`;
- `macos-shortcuts`.

The regenerated formatter inventory reports:

- `ghostty_canonical=203`;
- `roastty_formatter_rows=203`;
- `missing_canonical_formatter_rows=0`;
- `extra_formatter_rows=0`;
- `oracle_complete=185`;
- `audit_covered=18`;
- `gap=0`.

The matrix assertion passed and confirmed that adjacent rows
`macos-option-as-alt`, `macos-window-shadow`, `macos-auto-secure-input`,
`macos-custom-icon`, `macos-icon-ghost-color`, `macos-icon-screen-color`, and
`linux-cgroup` were not classified as `macos enum`.

Verification run:

- `cargo fmt --manifest-path roastty/Cargo.toml`
- `cargo test --manifest-path roastty/Cargo.toml macos_enum_config_formatter_family_oracle`
  — passed, 1 test.
- `cargo test --manifest-path roastty/Cargo.toml enum_format_entries_mac` —
  passed, 1 test.
- `cargo test --manifest-path roastty/Cargo.toml enum_format_entries_fullscreen`
  — passed, 1 test.
- `cargo test --manifest-path roastty/Cargo.toml enum_from_keyword_round_trips_mac_bgimage_shader`
  — passed, 1 test.
- `cargo test --manifest-path roastty/Cargo.toml enum_from_keyword_round_trips_misc_fullscreen`
  — passed, 1 test.
- `cargo test --manifest-path roastty/Cargo.toml macos_icon_config_parse_format_reset_and_diagnose`
  — passed, 1 test.
- `cargo test --manifest-path roastty/Cargo.toml macos_tail_config_parse_format_reset_and_diagnose`
  — passed, 1 test.
- `cargo test --manifest-path roastty/Cargo.toml macos_shortcuts_config_parse_format_reset_and_diagnose`
  — passed, 1 test.
- `cargo test --manifest-path roastty/Cargo.toml config_compatibility_alias_semantics`
  — passed, 1 test.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle` —
  passed, 1 test.
- Formatter inventory regeneration — passed with the expected counts above.
- Matrix assertion — passed.
- `PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  — passed, then the generated `__pycache__/` artifact was removed.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` — passed.
- `prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/82-macos-enum-formatter-oracle.md`
  — passed.
- `git diff --check` — passed.

## Conclusion

macOS enum formatting is now a durable CFG-218 oracle covering direct enum
output, config-level output, raw-empty resets, the dock-drop compatibility shim,
and local ordering across the adjacent macOS formatter block. The remaining
formatter gap is 18 `Audit covered` rows and 0 formatter-dispatch gaps; CFG-218
correctly remains `Gap` until those remaining rows receive focused formatter
oracles.

## Completion Review

Adversarial reviewer: Codex subagent with fresh context.

Verdict: Approved.

Findings:

- No Required findings.

Reviewer verification:

- The diff only adds the planned Rust oracle test plus inventory, matrix, and
  README updates.
- The test covers direct enum formatter output, `Config::set` plus
  `format_config`, dock-drop `window` compatibility input, raw-empty resets, and
  local ordering for the nine macOS enum rows.
- The inventory classifies exactly those nine rows as `macos enum`; adjacent
  macOS/scalar rows remain outside the family.
- CFG-218 remains `Gap` with 185 `Oracle complete` rows, 18 rows not complete,
  and 0 formatter gaps.
- The README marks Experiment 82 as `Pass` and adds a relevant Learning.
- `git status --short` showed the result was still uncommitted during review.
- `cargo test --manifest-path roastty/Cargo.toml macos_enum_config_formatter_family_oracle`
  passed.
- `cargo fmt --manifest-path roastty/Cargo.toml --check`, Prettier, and
  `git diff --check` passed.

The reviewer did not rerun `py_compile` because it can create `__pycache__`
artifacts and the review was read-only.
