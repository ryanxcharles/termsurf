# Experiment 59: Keybind formatter oracle

## Description

Experiment 58 promoted the `command-palette-entry` formatter row and left 124
formatter rows as `Audit covered`. The next compact formatter family is
`key binding`, currently one row:

- `keybind`.

Pinned Ghostty formats keybinds by emitting root keybind entries first, then
table entries. Its upstream tests cover single bindings, multiple sequence
bindings, nested sequences, table bindings, and mixed root/table output. Roastty
already has broad keybind parser coverage, but the formatter inventory should
not promote the `key binding` row until a focused formatter-family oracle proves
the non-default formatted output shape directly.

This experiment will add that focused oracle and connect it to the formatter
inventory.

Design review found one expected implementation fix before this row can be
promoted: pinned Ghostty accepts `foo/` as a table clear, but its
`Keybinds.formatEntryDocs` only iterates table bindings and does not emit an
empty `keybind = foo/` line. Roastty currently emits that empty-table line. This
experiment must make table-clear formatting match pinned Ghostty by emitting no
line for cleared/empty tables, and the oracle must prove that exact behavior.

CFG-218 should remain `Gap` because many formatter families still lack
non-default formatter oracles.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `keybind_config_formatter_family_oracle` test.
  - Cover empty/clear output, reset-to-default behavior, direct root bindings,
    chained actions, root key sequences, table keybinds, table clears, slash key
    disambiguation, flag-prefix normalization, exact formatted lines, and the
    local formatter order before `key-remap`.
- `roastty/src/config/keybind.rs`
  - Fix cleared/empty table formatting to match pinned Ghostty by emitting no
    formatter line for empty tables.
- `issues/0805-roastty-ghostty-parity/config_formatter_inventory.py`
  - Detect `keybind_config_formatter_family_oracle`.
  - Promote only formatter rows whose family is `key binding`.
  - Keep Experiment 59 as the CFG-218 owner when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-formatter-inventory.md`
  - Regenerate the formatter inventory.
  - Expected counts after implementation: 80 `Oracle complete` rows and 123
    `Audit covered` rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-218. It should remain `Gap` and report the new promotion
    counts.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml keybind_config_formatter_family_oracle`
  passes.
- `cargo test --manifest-path roastty/Cargo.toml keybind_config_parser_family_oracle`
  still passes.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  still passes.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reports:
  - `ghostty_canonical=203`;
  - `roastty_formatter_rows=203`;
  - `missing_canonical_formatter_rows=0`;
  - `extra_formatter_rows=0`;
  - `oracle_complete=80`;
  - `audit_covered=123`;
  - `gap=0`.
- A matrix assertion confirms:
  - CFG-217 remains `Pass`;
  - CFG-218 remains `Gap`;
  - all previously promoted formatter families remain `Oracle complete`;
  - the `key binding` formatter row is `Oracle complete`;
  - non-target formatter rows are not promoted by this oracle.
- A focused assertion confirms `keybind = foo/` is not emitted after `foo/`
  clears the `foo` key table.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passes.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  Markdown files.
- `git diff --check` passes.

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

Initial verdict: **Changes required**.

Required finding:

- The first design mentioned table-clear coverage without specifying that pinned
  Ghostty emits no formatter line for cleared/empty tables. The reviewer pointed
  to pinned Ghostty's `formatEntryDocs`, which only iterates table bindings, and
  Roastty's current empty-table emission path.

Fix:

- The design now explicitly requires fixing Roastty to emit no `keybind = foo/`
  line for cleared/empty tables and requires a focused assertion for that
  behavior.

Final verdict after re-review: **Approved**.

The reviewer confirmed the prior finding is resolved and found no new required
issues.

## Result

**Result:** Pass

Added a focused keybind formatter oracle and fixed Roastty's cleared-table
formatter behavior to match pinned Ghostty.

Pinned Ghostty accepts `foo/` as a key table clear, but its formatter emits only
actual table bindings; it does not emit an empty `keybind = foo/` line for the
cleared table. Roastty now matches that behavior by skipping cleared/empty
tables during keybind formatting.

The new `keybind_config_formatter_family_oracle` proves:

- default keybinds format and raw-empty reset restores them;
- `clear` formats as a single void `keybind = ` line;
- direct root bindings, chained actions, root key sequences, table bindings, and
  table chains format as exact expected lines;
- cleared tables emit no `keybind = table/` line;
- slash keys are disambiguated from table syntax in root and table bindings;
- keybind flag prefixes normalize away in formatted output;
- `key-remap` follows immediately after the formatted `keybind` rows.

The existing parser-family oracle was updated to expect the same
Ghostty-compatible cleared-table silence.

The regenerated formatter inventory now reports:

```text
ghostty_canonical=203
roastty_formatter_rows=203
missing_canonical_formatter_rows=0
extra_formatter_rows=0
oracle_complete=80
audit_covered=123
gap=0
no_output_rows=1
```

CFG-218 remains `Gap`, as intended, because 123 formatter rows still need
dedicated non-default formatter oracles.

Verification:

- `cargo test --manifest-path roastty/Cargo.toml keybind_config_formatter_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml keybind_config_parser_family_oracle`
  passed.
- `cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle`
  passed.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_formatter_inventory.py --upstream vendor/ghostty/src/config/Config.zig --upstream-formatter-file vendor/ghostty/src/config/formatter_file.zig --upstream-formatter vendor/ghostty/src/config/formatter.zig --roastty roastty/src/config/mod.rs --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md --output issues/0805-roastty-ghostty-parity/config-formatter-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  reported the expected 203/203 rows, 80 `Oracle complete`, 123 `Audit covered`,
  and 0 `Gap`.
- Matrix assertion passed: CFG-217 remains `Pass`; CFG-218 remains `Gap`; all
  previously promoted formatter families remain `Oracle complete`; the
  `key binding` formatter row is `Oracle complete`; representative non-target
  rows remain `Audit covered`.
- A focused assertion confirms `keybind = foo/` is not emitted after `foo/`
  clears the `foo` key table.
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed.
- `prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/59-keybind-formatter-oracle.md issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/config-formatter-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
  completed.
- `git diff --check` passed.

## Conclusion

The `keybind` formatter row is now oracle-complete. The experiment also fixed a
real parity bug: cleared keybind tables are silent when formatted, matching
pinned Ghostty's behavior. CFG-218 remains open with 123 audit-covered formatter
rows left for future experiments.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Initial verdict: **Changes required**.

Required finding:

- The completed experiment was missing the required separate `## Conclusion`
  section.

Nit:

- The result text mentioned a `foo/` cleared-table assertion, but the first
  oracle version only asserted the same behavior with `nav/`.

Fixes:

- Added the separate `## Conclusion` section.
- Added a literal `foo/a=quit` then `foo/` clear case to
  `keybind_config_formatter_family_oracle`, asserting that `keybind = foo/` is
  not emitted.

Final verdict after re-review: **Approved**.

The reviewer confirmed the required finding and nit were resolved and found no
new required issues.
