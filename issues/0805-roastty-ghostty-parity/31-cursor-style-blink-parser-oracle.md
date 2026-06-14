# Experiment 31: Cursor Style Blink Parser Oracle

## Description

CFG-217 still has 32 parser rows that are only `Audit covered`. The next bounded
row is canonical `cursor-style-blink`, whose dispatch path is
`set_optional_value_field(value, default.cursor_style_blink, parse_bool_field)`.

Pinned Ghostty defines `cursor-style-blink` as `?bool = null`. Its documented
values are blank, `true`, and `false`. Through Ghostty's generic optional-field
parsing, the direct parser boundary is:

- raw empty config values reset the optional field to the default `null`;
- a missing/bare value reaches the bool child parser and sets `true`;
- bool spellings accepted by `cli.args.parseBool` set `Some(true)` or
  `Some(false)`;
- invalid values, uppercase words, whitespace-padded values, and numeric values
  outside `0`/`1` are `InvalidValue`;
- formatting emits a blank entry when unset, or `true`/`false` when set.

Roastty already has a narrow cursor-style-blink test, but CFG-217 needs a
focused parser-family oracle named for inventory promotion and covering the full
optional-bool dispatch boundary. This experiment will add that oracle, keep the
existing cursor-style-blink test in the verification set, and promote only
canonical `cursor-style-blink`.

This experiment is limited to parser, formatter, reset/default, diagnostics,
CLI, and clone semantics. Cursor rendering and blink runtime behavior remain
separate parity facets.

CFG-217 must remain `Gap` because other parser helpers are still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `cursor_style_blink_config_parser_family_oracle` test
    covering:
    - default/unset formatting as a blank entry;
    - bare/missing values setting `Some(true)`;
    - accepted true spellings: `1`, `t`, `T`, and `true`;
    - accepted false spellings: `0`, `f`, `F`, and `false`;
    - raw empty option values resetting to `None`;
    - invalid values, uppercase words, whitespace-padded values, and numeric
      values outside `0`/`1` reporting `InvalidValue`;
    - `load_str` diagnostics preserving earlier valid values while reporting
      invalid later lines;
    - CLI argument parsing reaching the same helper;
    - cloned configs retaining parsed values.
  - Keep the existing
    `cursor_style_blink_accepts_unset_true_false_and_diagnoses` test in the
    verification set.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Mark only the canonical `cursor-style-blink` parser row as `Oracle complete`
    when the cursor-style-blink oracle test is present.
  - Add cursor-style-blink oracle detection to CFG-217 ownership so the
    generated matrix records `Experiment 31` when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 172 `Oracle complete`, 31
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 172 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting cursor-style-blink optional-bool parser semantics
    after the result is proven.

## Verification

Pass criteria:

- Focused Roastty tests pass:

```bash
cargo test --manifest-path roastty/Cargo.toml cursor_style_blink_config_parser_family_oracle
```

- Existing cursor style test still passes:

```bash
cargo test --manifest-path roastty/Cargo.toml cursor_style_blink_accepts_unset_true_false_and_diagnoses
cargo test --manifest-path roastty/Cargo.toml boolean_config_parser_family_oracle
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=172`;
  - `audit_covered=31`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 172 rows are `Oracle complete`;
  - the `cursor-style-blink` row is `Oracle complete`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 31`;
  - CFG-217 evidence points to `config-parser-inventory.md`.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  passes.
- No `__pycache__` or other `py_compile` artifacts remain in the issue folder.
- `git diff --check` passes.

Suggested commands:

```bash
cargo test --manifest-path roastty/Cargo.toml cursor_style_blink_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml cursor_style_blink_accepts_unset_true_false_and_diagnoses
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
assert cfg217[11] == 'Experiment 31', cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
cursor_blink = [row for row in parser_rows if row[1] == '`cursor-style-blink`']
assert len(cursor_blink) == 1, cursor_blink
assert cursor_blink[0][4] == 'Oracle complete', cursor_blink[0]
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 172
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} cursor_style_blink={cursor_blink[0][4]} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/31-cursor-style-blink-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Adversarial subagent `019ec41a-99c2-7503-8d43-499277004bed` reviewed the design
and returned `VERDICT: APPROVED` with no findings.

The reviewer verified the optional-bool semantics against pinned Ghostty:
`cursor-style-blink` is `?bool = null`, optional fields parse as their child
type, empty set values reset to the default, bare bool values parse as
`parseBool(value orelse "t")`, and accepted bool spellings are exactly `1`, `t`,
`T`, `true`, `0`, `f`, `F`, and `false`.
