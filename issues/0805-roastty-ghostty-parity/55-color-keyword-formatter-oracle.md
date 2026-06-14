# Experiment 55: Color keyword formatter oracle

## Description

Experiment 54 promoted the corrected repeatable path formatter rows and left 129
formatter rows as `Audit covered`. The next smallest formatter family is
`color`, currently two rows:

- `osc-color-report-format`;
- `window-colorspace`.

Despite the family name, these rows are not arbitrary RGB color formatters. They
are keyword/enum formatter rows associated with color behavior. This experiment
will prove the exact keyword output for both rows and promote only the rows
currently classified as `color`.

CFG-218 should remain `Gap` because many formatter families still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add `color_keyword_config_formatter_family_oracle`.
  - Prove `osc-color-report-format` formats `none`, `8-bit`, and `16-bit`.
  - Prove `window-colorspace` formats `srgb` and `display-p3`.
  - Prove raw-empty values reset both rows to their defaults and then format
    those defaults.
  - Prove representative formatter order between `window-colorspace` and
    `osc-color-report-format`.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Detect the color keyword formatter oracle.
  - Promote only formatter rows whose family is `color`.
  - Keep Experiment 55 as the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 76 `Oracle complete` rows and 127
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml color_keyword_config_formatter_family_oracle`
  passes.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  still passes.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=76`;
  - `audit_covered=127`;
  - `gap=0`.
- A matrix assertion confirms:
  - CFG-217 remains `Pass`;
  - CFG-218 remains `Gap`;
  - all previously promoted formatter families remain `Oracle complete`;
  - all `color` formatter rows are `Oracle complete`;
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

Added `color_keyword_config_formatter_family_oracle` and promoted only formatter
inventory rows whose family is `color`.

The oracle proves representative color keyword formatter behavior through
`Config::format_config`:

- `osc-color-report-format` formats `none`, `8-bit`, and `16-bit`;
- `window-colorspace` formats `srgb` and `display-p3`;
- raw-empty values reset both rows to their defaults and then format those
  defaults;
- the two rows remain in the actual formatter order: `window-colorspace`, then
  `osc-color-report-format`.

The implementation discovered that the original design's explicit order
expectation was backwards. `Config::format_config` emits `window-colorspace`
before `osc-color-report-format`, so the oracle was corrected to prove the
actual local formatter order.

The regenerated formatter inventory now reports:

```text
ghostty_canonical=203
roastty_formatter_rows=203
missing_canonical_formatter_rows=0
extra_formatter_rows=0
oracle_complete=76
audit_covered=127
gap=0
no_output_rows=1
```

CFG-218 remains `Gap`, as intended, because 127 formatter rows still need
dedicated non-default formatter oracles.

Verification:

- `cargo test --manifest-path roastty/Cargo.toml color_keyword_config_formatter_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  passed.
- Matrix assertion passed: CFG-217 remains `Pass`; CFG-218 remains `Gap`; all
  previously promoted formatter families remain `Oracle complete`; all `color`
  formatter rows are `Oracle complete`; representative non-target formatter
  families remain `Audit covered`.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed.
- `prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/55-color-keyword-formatter-oracle.md issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/config-formatter-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
  completed.
- `git diff --check` passed.

## Conclusion

The color keyword formatter family is now oracle-complete. CFG-218 remains open
with 127 audit-covered formatter rows. The next small formatter experiments
should target another compact family such as key remap, key binding, command
palette, or no-output `link`.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.
