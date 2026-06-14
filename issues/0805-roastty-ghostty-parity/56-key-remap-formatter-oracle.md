# Experiment 56: Key remap formatter oracle

## Description

Experiment 55 promoted the two color keyword formatter rows and left 127
formatter rows as `Audit covered`. The next compact formatter family is
`key remap`, currently one row:

- `key-remap`.

Roastty already has broad parser coverage for key remap behavior. This
experiment adds a formatter-family oracle focused specifically on the formatted
`Config::format_config` output for non-empty remaps, empty/reset output, alias
normalization, side normalization, and local formatter order.

CFG-218 should remain `Gap` because many formatter families still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add `key_remap_config_formatter_family_oracle`.
  - Prove the default empty remap set formats as `key-remap = `.
  - Prove normalized remap output for direct modifier names and side-specific
    modifier names.
  - Prove alias normalization such as `control=command` and
    `right_option=left_control`.
  - Prove raw-empty and bare CLI reset behavior returns formatter output to the
    single void line.
  - Prove representative formatter order keeps `key-remap` after the full
    `keybind` formatter output and before `window-padding-x`.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Detect the key-remap formatter oracle.
  - Promote only formatter rows whose family is `key remap`.
  - Keep Experiment 56 as the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 77 `Oracle complete` rows and 126
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml key_remap_config_formatter_family_oracle`
  passes.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  still passes.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=77`;
  - `audit_covered=126`;
  - `gap=0`.
- A matrix assertion confirms:
  - CFG-217 remains `Pass`;
  - CFG-218 remains `Gap`;
  - all previously promoted formatter families remain `Oracle complete`;
  - the `key remap` formatter row is `Oracle complete`;
  - non-target formatter rows are not promoted by this oracle.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passes.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  Markdown files.
- `git diff --check` passes.

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.

## Result

**Result:** Pass

Added `key_remap_config_formatter_family_oracle` and promoted only formatter
inventory rows whose family is `key remap`.

The oracle proves representative key-remap formatter behavior through
`Config::format_config`:

- the default empty remap set formats as `key-remap = `;
- direct modifier remaps and side-specific modifier remaps format as normalized
  side-specific pairs;
- aliases such as `control`, `command`, and `option` normalize to their concrete
  formatter names;
- raw-empty config values and bare CLI `--key-remap` reset the formatter output
  to the single void line;
- `key-remap` remains after the full `keybind` formatter output and before
  `window-padding-x`.

The implementation also confirmed one normalized CLI output order:
`right_ctrl=left_super`, then `right_alt=left_ctrl`, then
`left_ctrl=left_super`.

The regenerated formatter inventory now reports:

```text
ghostty_canonical=203
roastty_formatter_rows=203
missing_canonical_formatter_rows=0
extra_formatter_rows=0
oracle_complete=77
audit_covered=126
gap=0
no_output_rows=1
```

CFG-218 remains `Gap`, as intended, because 126 formatter rows still need
dedicated non-default formatter oracles.

Verification:

- `cargo test --manifest-path roastty/Cargo.toml key_remap_config_formatter_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  passed.
- Matrix assertion passed: CFG-217 remains `Pass`; CFG-218 remains `Gap`; all
  previously promoted formatter families remain `Oracle complete`; the
  `key remap` formatter row is `Oracle complete`; representative non-target
  formatter families remain `Audit covered`.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed.
- `prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/56-key-remap-formatter-oracle.md issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/config-formatter-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
  completed.
- `git diff --check` passed.

## Conclusion

The key-remap formatter row is now oracle-complete. CFG-218 remains open with
126 audit-covered formatter rows. The next compact formatter experiments should
target `keybind`, `command-palette-entry`, or the no-output `link` row.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.
