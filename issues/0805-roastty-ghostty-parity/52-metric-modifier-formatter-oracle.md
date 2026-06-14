# Experiment 52: Metric Modifier Formatter Oracle

## Description

Experiment 51 promoted the primitive formatter families but left 148 formatter
rows as `Audit covered`. The next small family is `metric modifier`: 12 non-font
`adjust-*` formatter rows that all use
`entry_optional(..., format_metric_modifier)`.

This experiment will add one shared metric modifier formatter oracle and promote
only the rows currently classified as `metric modifier` in
`config-formatter-inventory.md`. Font-classified metric modifier rows such as
`adjust-font-baseline` remain out of scope because the inventory groups them
with the broader font formatter surface.

CFG-218 should remain `Gap` because many formatter families still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add `metric_modifier_config_formatter_family_oracle`.
  - Exercise representative non-font metric modifier rows through
    `Config::format_config`.
  - Prove absolute modifiers format as decimal strings.
  - Prove percent modifiers format as `(stored_value - 1) * 100` plus `%`,
    including clamped negative percentages, infinity, and `nan%`.
  - Prove raw-empty optional values format as the void line.
  - Prove representative metric modifier rows remain in formatter order.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Detect `metric_modifier_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `metric modifier`.
  - Keep Experiment 52 as the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 67 `Oracle complete` rows and 136
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml metric_modifier_config_formatter_family_oracle`
  passes.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  still passes.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=67`;
  - `audit_covered=136`;
  - `gap=0`.
- A matrix assertion confirms:
  - CFG-217 remains `Pass`;
  - CFG-218 remains `Gap`;
  - all `boolean`, `integer`, `float`, `string`, and `metric modifier` formatter
    rows are `Oracle complete`;
  - font-classified metric-helper rows are not promoted by this oracle.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passes.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  Markdown files.
- `git diff --check` passes.

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.

The reviewer verified that the README links Experiment 52 as `Designed`, the
experiment has the required sections, the 67/136 expected counts are consistent
with 55 existing oracle rows plus 12 metric modifier rows, font-classified
metric helper rows remain out of scope, CFG-218 remains `Gap`, and the
verification criteria are concrete.

## Result

**Result:** Pass

Added `metric_modifier_config_formatter_family_oracle` and promoted only
formatter inventory rows whose family is `metric modifier`.

The oracle proves representative metric modifier formatter behavior through
`Config::format_config`:

- absolute modifiers format as decimal strings;
- percent modifiers format as `(stored_value - 1) * 100` plus `%`;
- clamped negative percentages format at the clamped stored value;
- infinity and `nan` percentage values format as `inf%` and `nan%`;
- raw-empty optional metric modifier values format as the void line;
- representative metric modifier rows remain in the expected formatter order.

The regenerated formatter inventory now reports:

```text
ghostty_canonical=203
roastty_formatter_rows=203
missing_canonical_formatter_rows=0
extra_formatter_rows=0
oracle_complete=67
audit_covered=136
gap=0
no_output_rows=1
```

CFG-218 remains `Gap`, as intended, because 136 formatter rows still need
dedicated non-default formatter oracles.

Verification:

- `cargo test --manifest-path roastty/Cargo.toml metric_modifier_config_formatter_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  passed.
- Matrix assertion passed: CFG-217 remains `Pass`; CFG-218 remains `Gap`;
  primitive and metric modifier formatter families are `Oracle complete`;
  non-target formatter rows are not promoted, including font-classified metric
  helper rows.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed.
- `prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/52-metric-modifier-formatter-oracle.md issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/config-formatter-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
  completed.
- `git diff --check` passed.

## Conclusion

The metric modifier formatter family is now oracle-complete. CFG-218 remains
open with 136 audit-covered formatter rows. The next formatter experiments
should target another coherent family such as window padding, repeatable paths,
or optional values.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.
