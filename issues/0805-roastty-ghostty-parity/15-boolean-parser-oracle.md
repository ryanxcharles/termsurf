# Experiment 15: Boolean Parser Oracle

## Description

CFG-217 remains open because parser rows are only `Audit covered`. The largest
simple parser family is the 40-row boolean family. Ghostty's generic boolean
parser accepts exactly `1`, `t`, `T`, and `true` as true; `0`, `f`, `F`, and
`false` as false; treats a bare missing value as `true`; rejects other values;
and resets set-but-empty values to the field default before parsing.

This experiment will make that shared oracle durable for the ordinary
`set_bool_field` dispatch rows and update the parser inventory accordingly.
`config-default-files` is deliberately excluded from `Oracle complete` in this
experiment: pinned Ghostty's direct parser treats it as a normal boolean, while
the effective default-file load switch is controlled later by load-order replay
semantics. That belongs to CFG-221, not this parser-family slice.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused boolean parser family test covering:
    - all upstream true spellings;
    - all upstream false spellings;
    - bare missing value as `true`;
    - set-but-empty value as reset to default;
    - invalid values as `InvalidValue`;
    - a representative false-default boolean (`maximize`) and true-default
      boolean (`link-url`).
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Mark ordinary boolean parser rows as `Oracle complete` when they use
    `set_bool_field`, the boolean family oracle test is present, and the option
    is not `config-default-files`.
  - Leave `config-default-files` as `Audit covered` with a note pointing to
    CFG-221 because its direct parser and effective load-order semantics must be
    proven together.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 39 `Oracle complete`, 164
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 39 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting the boolean parser oracle and why
    `config-default-files` remains a CFG-221/load-order item.

## Verification

Pass criteria:

- Focused Roastty tests pass:

```bash
cargo test --manifest-path roastty/Cargo.toml boolean_config_parser_family_oracle
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=39`;
  - `audit_covered=164`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 39 rows are `Oracle complete`;
  - every boolean row except `config-default-files` is `Oracle complete`;
  - `config-default-files` remains `Audit covered`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 evidence points to `config-parser-inventory.md`.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `git diff --check` passes.

Suggested commands:

```bash
cargo test --manifest-path roastty/Cargo.toml boolean_config_parser_family_oracle
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_parser_inventory.py \
  --upstream vendor/ghostty/src/config/Config.zig \
  --roastty roastty/src/config/mod.rs \
  --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md \
  --output issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
python3 - <<'PY'
from pathlib import Path

matrix_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text().splitlines():
    if line.startswith('| CFG-'):
        matrix_rows.append([cell.strip() for cell in line.strip('|').split('|')])
cfg217 = next(row for row in matrix_rows if row[0] == 'CFG-217')
assert cfg217[4] == 'Gap', cfg217
assert 'config-parser-inventory.md' in cfg217[6], cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
boolean_rows = [row for row in parser_rows if row[3] == 'boolean']
assert len(boolean_rows) == 40, len(boolean_rows)
ordinary_boolean_rows = [row for row in boolean_rows if row[1] != '`config-default-files`']
config_default_files = next(row for row in boolean_rows if row[1] == '`config-default-files`')
assert len(ordinary_boolean_rows) == 39, len(ordinary_boolean_rows)
assert all(row[4] == 'Oracle complete' for row in ordinary_boolean_rows)
assert config_default_files[4] == 'Audit covered', config_default_files
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 39
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} boolean_oracle={len(ordinary_boolean_rows)} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/15-boolean-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Fresh-context adversarial design review found one required issue:

- The draft overgeneralized `config-default-files` by treating Roastty's current
  file-source ignored behavior as the Ghostty parser oracle. Pinned Ghostty
  directly parses `config-default-files` as a normal boolean; its effective
  default-file behavior is controlled later by load-order replay.

Fix:

- Scoped this experiment to the 39 ordinary boolean rows and left
  `config-default-files` as `Audit covered` for CFG-221/load-order work.

Re-review approved the fixed design:

```text
VERDICT: APPROVED

Findings: none.
```
