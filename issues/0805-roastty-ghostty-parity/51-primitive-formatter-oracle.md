# Experiment 51: Primitive Formatter Oracle

## Description

Experiment 50 made CFG-218 concrete: all 203 canonical formatter rows are
inventoried, but none are `Oracle complete`. The cheapest next step is the
primitive formatter surface: direct boolean, integer, float, and string rows
that use `EntryFormatter` without option-specific formatting.

This experiment will add one shared primitive formatter oracle and use it to
promote only the formatter inventory rows whose formatter family is `boolean`,
`integer`, `float`, or `string`. CFG-218 should remain `Gap` because optional,
repeatable, color, font, keybind, key remap, command palette, metric modifier,
window padding, no-output, and custom `format_entry` families still need their
own non-default formatter oracles.

Rows currently classified as `font` remain out of scope even when their local
formatter helper is primitive, because the formatter inventory groups those rows
with the broader font formatter surface.

## Changes

- `roastty/src/config/mod.rs`
  - Add `primitive_config_formatter_family_oracle`.
  - Exercise representative direct boolean, integer, float, and string canonical
    options through `Config::format_config`.
  - Prove non-default values format with Ghostty-compatible primitive text:
    `true`/`false`, decimal integers, shortest decimal floats, lowercase `nan`,
    and byte-preserving string output.
  - Prove the primitive rows still appear in `format_config` order relative to
    nearby existing entries where the row order matters.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Detect `primitive_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `boolean`, `integer`, `float`,
    or `string`.
  - Make Experiment 51 the CFG-218 owner when the primitive formatter oracle is
    present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 55 `Oracle complete` rows and 148
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the primitive
    promotion counts.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml primitive_config_formatter_family_oracle`
  passes.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  still passes.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=55`;
  - `audit_covered=148`;
  - `gap=0`.
- A matrix assertion confirms:
  - CFG-217 remains `Pass`;
  - CFG-218 remains `Gap`;
  - CFG-218 points to `config-formatter-inventory.md`;
  - all `boolean`, `integer`, `float`, and `string` formatter rows are
    `Oracle complete`;
  - non-primitive formatter rows are not promoted by this oracle.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passes.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  Markdown files.
- `git diff --check` passes.

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Required findings: none.

Optional finding accepted:

- The reviewer noted that "primitive" could be misread as every direct
  `entry_bool`, `entry_int`, `entry_float`, or `entry_str` call, even for rows
  classified under another family such as `font`. Fixed by clarifying that this
  experiment promotes only rows currently classified as `boolean`, `integer`,
  `float`, or `string`; font-classified primitive-helper rows remain out of
  scope.

Nit noted:

- `prettier --write` is a formatting step rather than a pass/fail check. The
  experiment keeps it as the required Markdown formatting step.

## Result

**Result:** Pass

Added `primitive_config_formatter_family_oracle` and promoted only formatter
inventory rows whose family is `boolean`, `integer`, `float`, or `string`.

The oracle proves representative direct primitive formatter behavior through
`Config::format_config`:

- booleans format as `true` and `false`;
- integers format as decimal text;
- floats format as shortest decimal text and lowercase `nan`;
- strings are emitted as raw byte-preserving text with no escaping or quoting;
- representative primitive rows remain in the expected formatter order relative
  to nearby entries.

The regenerated formatter inventory now reports:

```text
ghostty_canonical=203
roastty_formatter_rows=203
missing_canonical_formatter_rows=0
extra_formatter_rows=0
oracle_complete=55
audit_covered=148
gap=0
no_output_rows=1
```

CFG-218 remains `Gap`, as intended, because 148 non-primitive formatter rows
still need dedicated non-default formatter oracles.

Verification:

- `cargo test --manifest-path roastty/Cargo.toml primitive_config_formatter_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  passed.
- Matrix assertion passed: CFG-217 remains `Pass`; CFG-218 remains `Gap`;
  primitive formatter families are `Oracle complete`; non-primitive formatter
  rows are not promoted.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed.
- `prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/51-primitive-formatter-oracle.md issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/config-formatter-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
  completed.
- `git diff --check` passed.

## Conclusion

The primitive formatter family is now oracle-complete. CFG-218 remains open with
148 audit-covered formatter rows. The next formatter experiments should target
another coherent family, likely optional values, repeatable paths, or metric
modifiers.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.

The reviewer independently verified:

- `primitive_config_formatter_family_oracle` passed.
- `config_default_format_oracle` passed.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed.
- `git diff --check` passed.
- The formatter inventory has 203 rows, 55 `Oracle complete` rows, 148
  `Audit covered` rows, 0 `Gap` rows, and 1 no-output row.
- CFG-217 remains `Pass`.
- CFG-218 remains `Gap`, points to `config-formatter-inventory.md`, and is owned
  by Experiment 51.
- The README marks Experiment 51 `Pass` and records the primitive formatter
  learning.

The reviewer did not run the formatter inventory generator because it writes
files, but inspected the generated output and approved the completed experiment.
