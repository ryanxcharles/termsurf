# Experiment 57: Link no-output formatter oracle

## Description

Experiment 56 promoted the `key-remap` formatter row and left 126 formatter rows
as `Audit covered`. The next compact formatter family is the intentional
`no-output` row:

- `link`.

Pinned Ghostty declares canonical `link: RepeatableLink`, but
`RepeatableLink.parseCLI` currently returns `error.NotImplemented` and
`RepeatableLink.formatEntry` intentionally emits no output because `link` cannot
currently be set. Roastty already models `link` as recognized-but-unsupported
for parsing. This experiment will add a formatter oracle proving that Roastty
does not emit any `link = ...` config line while preserving the recognized
parser behavior around empty reset and not-implemented non-empty values.

CFG-218 should remain `Gap` because many formatter families still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add `link_no_output_config_formatter_oracle`.
  - Prove default `Config::format_config` emits no `link = ` line.
  - Prove non-empty `link` values remain recognized but not implemented and do
    not introduce formatter output.
  - Prove raw-empty `link =` resets the default link list and still emits no
    formatter output.
  - Prove the existing `link-url` formatter row still emits normally, so the
    oracle distinguishes `link` from adjacent link-related rows.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Detect the link no-output formatter oracle.
  - Promote only formatter rows whose family is `no-output`.
  - Keep Experiment 57 as the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 78 `Oracle complete` rows and 125
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml link_no_output_config_formatter_oracle`
  passes.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  still passes.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=78`;
  - `audit_covered=125`;
  - `gap=0`;
  - `no_output_rows=1`.
- A matrix assertion confirms:
  - CFG-217 remains `Pass`;
  - CFG-218 remains `Gap`;
  - all previously promoted formatter families remain `Oracle complete`;
  - the `no-output` formatter row is `Oracle complete`;
  - `link-url` remains outside this oracle and is not promoted by accident.
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

Added `link_no_output_config_formatter_oracle` and promoted only formatter
inventory rows whose family is `no-output`.

The oracle proves the intentional no-output formatter behavior for canonical
`link` through `Config::format_config`:

- default config emits no `link = ` formatter line;
- non-empty `link` values remain recognized but not implemented and do not
  introduce formatter output;
- raw-empty `link =` resets the default link list and still emits no formatter
  output;
- adjacent `link-url` formatter output still emits normally, proving the oracle
  distinguishes canonical no-output `link` from link-related formatter rows.

The regenerated formatter inventory now reports:

```text
ghostty_canonical=203
roastty_formatter_rows=203
missing_canonical_formatter_rows=0
extra_formatter_rows=0
oracle_complete=78
audit_covered=125
gap=0
no_output_rows=1
```

CFG-218 remains `Gap`, as intended, because 125 formatter rows still need
dedicated non-default formatter oracles.

Verification:

- `cargo test --manifest-path roastty/Cargo.toml link_no_output_config_formatter_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  passed.
- Matrix assertion passed: CFG-217 remains `Pass`; CFG-218 remains `Gap`; all
  previously promoted formatter families remain `Oracle complete`; the
  `no-output` formatter row is `Oracle complete`; `link-url` remains a separate
  formatter row; representative non-target formatter families remain
  `Audit covered`.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed.
- `prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/57-link-no-output-formatter-oracle.md issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/config-formatter-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
  completed.
- `git diff --check` passed.

## Conclusion

The canonical no-output `link` formatter row is now oracle-complete. CFG-218
remains open with 125 audit-covered formatter rows. The next compact formatter
experiments should target `keybind` or `command-palette-entry`.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.
