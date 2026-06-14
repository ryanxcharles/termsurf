# Experiment 54: Repeatable path formatter oracle

## Description

Experiment 53 promoted the four window-padding formatter rows and left 132
formatter rows as `Audit covered`. The current formatter inventory reports four
`repeatable path` rows, but one of them is misclassified:
`custom-shader-animation` is an enum-like `CustomShaderAnimation`, not a
`RepeatablePath`.

This experiment will first fix that inventory classifier, then add/promote a
formatter oracle for the actual repeatable path rows:

- `config-file`;
- `custom-shader`;
- `gtk-custom-css`.

`custom-shader-animation` should remain `Audit covered` under a non-repeatable
family until a later enum/custom formatter oracle covers it. CFG-218 should
remain `Gap` because many formatter families still lack non-default formatter
oracles.

## Changes

- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Stop classifying `custom-shader-animation` as `repeatable path` just because
    its Rust identifier contains `custom_shader`.
  - Detect a repeatable path formatter oracle.
  - Promote only formatter rows whose corrected family is `repeatable path`.
  - Keep Experiment 54 as the CFG-218 owner when this oracle is present.
- `roastty/src/config/mod.rs`
  - Add `repeatable_path_config_formatter_family_oracle` or extend the existing
    repeatable-path tests with the same explicit formatter-family scope.
  - Prove empty lists format as a single void line for `config-file`,
    `custom-shader`, and `gtk-custom-css`.
  - Prove required paths format without `?`.
  - Prove optional paths format with a leading `?`.
  - Prove quoted literal `?path` values format as required paths without the
    optional marker.
  - Prove raw-empty reset returns each list to the single void line.
  - Prove representative formatter order across the three repeatable path rows.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 74 `Oracle complete` rows and 129
    `Audit covered` rows.
  - Expected family counts after classifier correction: 3 `repeatable path`
    rows, with `custom-shader-animation` no longer in that family.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml repeatable_path_config_formatter_family_oracle`
  passes, or the equivalent renamed/extended focused formatter test passes.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  still passes.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=74`;
  - `audit_covered=129`;
  - `gap=0`.
- A matrix assertion confirms:
  - CFG-217 remains `Pass`;
  - CFG-218 remains `Gap`;
  - all `boolean`, `integer`, `float`, `string`, `metric modifier`,
    `window padding`, and corrected `repeatable path` formatter rows are
    `Oracle complete`;
  - `custom-shader-animation` is not classified as `repeatable path` and remains
    `Audit covered`;
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

Fixed the formatter inventory classifier so `custom-shader-animation` is no
longer treated as a repeatable path, added
`repeatable_path_config_formatter_family_oracle`, and promoted only formatter
inventory rows whose corrected family is `repeatable path`.

The oracle proves representative repeatable path formatter behavior through
`Config::format_config`:

- empty `config-file`, `custom-shader`, and `gtk-custom-css` lists format as a
  single void line;
- required paths format without `?`;
- optional paths format with a leading `?`;
- quoted literal `?path` values format as required paths without the optional
  marker;
- raw-empty values reset each list back to the single void line;
- the three rows remain in representative formatter order: `config-file`,
  `custom-shader`, `gtk-custom-css`.

The regenerated formatter inventory now reports:

```text
ghostty_canonical=203
roastty_formatter_rows=203
missing_canonical_formatter_rows=0
extra_formatter_rows=0
oracle_complete=74
audit_covered=129
gap=0
no_output_rows=1
```

The corrected family counts include 3 `repeatable path` rows, and
`custom-shader-animation` remains `Audit covered` under a non-repeatable family.

CFG-218 remains `Gap`, as intended, because 129 formatter rows still need
dedicated non-default formatter oracles.

Verification:

- `cargo test --manifest-path roastty/Cargo.toml repeatable_path_config_formatter_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  passed.
- Matrix assertion passed: CFG-217 remains `Pass`; CFG-218 remains `Gap`;
  primitive, metric modifier, window padding, and corrected repeatable path
  formatter families are `Oracle complete`; `custom-shader-animation` is not
  classified as `repeatable path` and remains `Audit covered`; representative
  non-target formatter families remain `Audit covered`.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed.
- `prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/54-repeatable-path-formatter-oracle.md issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/config-formatter-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
  completed.
- `git diff --check` passed.

## Conclusion

The repeatable path formatter family is now oracle-complete after correcting the
`custom-shader-animation` classifier false positive. CFG-218 remains open with
129 audit-covered formatter rows. The next formatter experiments should target
another coherent family such as colors, key remap, key binding, command palette,
or optional values.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.
