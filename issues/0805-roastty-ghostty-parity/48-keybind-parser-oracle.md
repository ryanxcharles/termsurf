# Experiment 48: Keybind Parser Oracle

## Description

CFG-217 still has 2 parser rows that are only `Audit covered`. One of those is
`keybind`, the largest remaining parser surface. It controls default keybinding
reset, full clearing, root bindings, key sequences, chained actions, key tables,
table clearing, table/root slash disambiguation, trigger prefixes, action
parsing, file diagnostics, CLI parsing, formatting, and clone/equality behavior.

Pinned Ghostty's `Keybinds.parseCLI` requires a value. Empty input resets the
keybinding set to defaults. The exact value `clear` removes all root bindings
and key tables. Non-empty bindings route through Ghostty's binding parser and
store either root bindings/sequences or table bindings/sequences. A slash before
the first `=` is table syntax only when the table name is non-empty and does not
contain `+` or `>`; this keeps root keys such as `/`, `ctrl+/`, and sequence
triggers ending in `/` valid. `name/` clears or defines a named table.
`chain=...` appends to the most recent root or table binding/sequence, while a
chain with no stored parent, an invalid chained action such as `unbind`, or
table-local `name/chain=...` is invalid.

Roastty already has focused coverage for most of this surface in
`keybind_config_parse_format_reset_load_cli_and_clone`, plus lower-level parser
and runtime keybind tests. This experiment will consolidate the config parser
coverage under the explicit CFG-217 oracle name, extend it where needed for the
remaining upstream parser classes and diagnostics, wire the parser inventory to
recognize the oracle, and promote only the canonical `keybind` row.

CFG-217 must remain `Gap` because `config-default-files` will still be
audit-only after this experiment and belongs with CFG-221 source-precedence /
default-file load semantics.

## Changes

- `roastty/src/config/mod.rs`
  - Rename or wrap the existing keybind parser/config test as
    `keybind_config_parser_family_oracle`.
  - Extend the oracle if needed to cover:
    - missing value returning `ValueRequired`;
    - empty value resetting defaults;
    - `clear` removing root bindings and tables;
    - root direct bindings and root key sequences;
    - chained actions preserving order on root direct bindings and root
      sequences;
    - table direct bindings and table key sequences;
    - table clears with `name/`;
    - table-local `name/chain=...` rejection;
    - chain-without-parent rejection;
    - invalid chained actions such as `chain=unbind`;
    - slash disambiguation for `/`, `ctrl+/`, `shift+/`, sequence slash, and
      table bindings containing slash triggers;
    - trigger prefixes such as `global:`, `all:`, `unconsumed:`, and
      `performable:`;
    - physical key aliases such as `key_a`;
    - config-file diagnostics;
    - CLI parsing;
    - formatting, clone, and equality semantics.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Detect `keybind_config_parser_family_oracle`.
  - Mark only canonical `keybind` as `Oracle complete` when the oracle test is
    present.
  - Add the oracle to CFG-217 ownership so the generated matrix records
    `Experiment 48` when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 202 `Oracle complete`, 1
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 202 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting keybind parser semantics after the result is
    proven.

## Verification

Pass criteria:

- Focused Roastty keybind oracle passes:

```bash
cargo test --manifest-path roastty/Cargo.toml keybind_config_parser_family_oracle
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=202`;
  - `audit_covered=1`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 202 rows are `Oracle complete`;
  - `keybind` is `Oracle complete`;
  - the remaining `Audit covered` set is exactly `config-default-files`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 48`;
  - CFG-217 evidence points to `config-parser-inventory.md`.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run if any Rust file is
  edited.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  passes, and any generated `__pycache__` is removed.
- `git diff --check` passes.

Suggested commands:

```bash
cargo test --manifest-path roastty/Cargo.toml keybind_config_parser_family_oracle
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_parser_inventory.py \
  --upstream vendor/ghostty/src/config/Config.zig \
  --roastty roastty/src/config/mod.rs \
  --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md \
  --output issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
python3 - <<'PY'
from pathlib import Path

rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        cells = [cell.strip() for cell in line.strip('|').split('|')]
        rows.append(cells)

by_option = {row[1].strip('`'): row for row in rows}
audit = {row[1].strip('`') for row in rows if row[4] == 'Audit covered'}

assert len(rows) == 203, len(rows)
assert sum(row[4] == 'Oracle complete' for row in rows) == 202
assert by_option['keybind'][4] == 'Oracle complete'
assert audit == {'config-default-files'}, audit
assert not any(row[4] == 'Gap' for row in rows)

matrix = Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text()
cfg217 = next(line for line in matrix.splitlines() if line.startswith('| CFG-217 '))
cfg217_cells = [cell.strip() for cell in cfg217.strip('|').split('|')]
assert cfg217_cells[4] == 'Gap'
assert cfg217_cells[11] == 'Experiment 48'
assert 'config-parser-inventory.md' in cfg217_cells[6]
PY
cargo fmt --manifest-path roastty/Cargo.toml
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
test -z "$(find issues/0805-roastty-ghostty-parity -name __pycache__ -prune -print)"
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/48-keybind-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

**Verdict:** Approved.

The reviewer reported no findings. It confirmed the README links Experiment 48
as `Designed`, the experiment has the required sections, and the plan matches
the CFG-217 parser-facet scope without claiming runtime keybinding parity.

## Result

**Result:** Pass.

Implemented the keybind parser oracle and promoted only the canonical `keybind`
parser row.

Changes made:

- Renamed and expanded the config-level keybind test to
  `keybind_config_parser_family_oracle`.
- Covered missing value, empty default reset, `clear`, root direct bindings,
  root key sequences, chained actions, table direct bindings, table sequences,
  table clears, invalid root/table chains, invalid chained actions, slash
  disambiguation, trigger prefixes, physical key aliases, config-file
  diagnostics, CLI parsing, formatting, equality, and clone behavior.
- Added `keybind_config_parser_family_oracle` detection to
  `config_parser_inventory.py`.
- Regenerated `config-parser-inventory.md` and `config-matrix.md`.
- Added the README learning that this oracle proves parser-surface behavior but
  not runtime shortcut dispatch.

Verification performed:

```bash
cargo test --manifest-path roastty/Cargo.toml keybind_config_parser_family_oracle
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_parser_inventory.py \
  --upstream vendor/ghostty/src/config/Config.zig \
  --roastty roastty/src/config/mod.rs \
  --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md \
  --output issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
python3 - <<'PY'
from pathlib import Path

rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        cells = [cell.strip() for cell in line.strip('|').split('|')]
        rows.append(cells)

by_option = {row[1].strip('`'): row for row in rows}
audit = {row[1].strip('`') for row in rows if row[4] == 'Audit covered'}

assert len(rows) == 203, len(rows)
assert sum(row[4] == 'Oracle complete' for row in rows) == 202
assert by_option['keybind'][4] == 'Oracle complete'
assert audit == {'config-default-files'}, audit
assert not any(row[4] == 'Gap' for row in rows)

matrix = Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text()
cfg217 = next(line for line in matrix.splitlines() if line.startswith('| CFG-217 '))
cfg217_cells = [cell.strip() for cell in cfg217.strip('|').split('|')]
assert cfg217_cells[4] == 'Gap'
assert cfg217_cells[11] == 'Experiment 48'
assert 'config-parser-inventory.md' in cfg217_cells[6]
PY
cargo fmt --manifest-path roastty/Cargo.toml
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
test -z "$(find issues/0805-roastty-ghostty-parity -name __pycache__ -prune -print)"
git diff --check
```

The parser inventory reports:

```text
ghostty_canonical=203
roastty_parser_rows=203
missing_canonical_parser_rows=0
missing_dispatch_rows=0
extra_parser_rows=0
compatibility_only_parser_arms=5
noncanonical_noncompat_parser_arms=0
oracle_complete=202
audit_covered=1
gap=0
```

The first focused run exposed that the oracle expected prefix formatting that
did not match pinned Ghostty. The formatter and oracle now match pinned Ghostty:
prefix flags affect stored parser/runtime metadata, but formatted keybind lines
print only the trigger key/sequence and action.

## Conclusion

`keybind` parser parity is now Oracle complete for CFG-217. The remaining
audit-only parser row is `config-default-files`, so CFG-217 correctly remains
`Gap` at 202 Oracle complete rows, 1 Audit covered row, and 0 dispatch gaps.

## Completion Review

Reviewed by fresh-context Codex adversarial subagents.

Initial completion review verdict: Changes required.

Findings:

- Required: the oracle asserted non-Ghostty keybind formatter semantics for
  trigger prefixes. Fixed by changing Roastty keybind formatting to match pinned
  Ghostty: `global:`, `all:`, `unconsumed:`, and `performable:` remain parser
  metadata but are not emitted in formatted config lines.
- Required: the oracle parsed table bindings and table sequences, then cleared
  the table before asserting formatted table entries. Fixed by asserting table
  direct/sequence entries and their chained actions before separately asserting
  the `nav/` table-clear output.

Verification after fixes:

```bash
cargo test --manifest-path roastty/Cargo.toml keybind_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_parser_inventory.py \
  --upstream vendor/ghostty/src/config/Config.zig \
  --roastty roastty/src/config/mod.rs \
  --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md \
  --output issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
cargo fmt --manifest-path roastty/Cargo.toml --check
git diff --check
```

Re-review verdict: Approved.

The re-review confirmed both required findings were resolved, reran the focused
keybind oracle, the default format oracle, `cargo fmt --check`,
`git diff --check`, and the read-only matrix assertion, and reported no new
Required findings.
