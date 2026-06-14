# Experiment 34: Window Decoration Parser Oracle

## Description

CFG-217 still has 29 parser rows that are only `Audit covered`. Canonical
`window-decoration` is one of the remaining `custom parse_cli` rows.

Pinned Ghostty defines `window-decoration` as `WindowDecoration = .auto`.
`WindowDecoration.parseCLI` treats a missing value as `.auto`, parses bools
first (`true` to `.auto`, `false` to `.none`), then accepts the exact enum
variant names `auto`, `client`, `server`, and `none`. Empty strings, unknown
values, padded values, and case-changed values are `InvalidValue`.

Roastty already has a lower-level parser test and a broad enum-routing test.
This experiment will add a focused CFG-217 oracle named for inventory promotion,
keep the existing lower-level/routing tests in the verification set, and promote
only canonical `window-decoration`.

CFG-217 must remain `Gap` because other parser helpers are still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `window_decoration_config_parser_family_oracle` test covering:
    - default formatting as `auto`;
    - missing direct parser input returning `Auto`;
    - bool tokens mapping `true`/`1`/`t`/`T` to `Auto`;
    - bool tokens mapping `false`/`0`/`f`/`F` to `None`;
    - exact variant names `auto`, `client`, `server`, and `none`;
    - empty, unknown, whitespace-padded, and case-changed values rejected as
      `InvalidValue`;
    - file config routing and formatting;
    - CLI argument parsing reaches the same helper;
    - diagnostics preserve an earlier valid value after a later invalid value;
    - cloned configs retain the parsed value.
  - Keep the existing `window_decoration_parse_cli_resolves_bool_and_variants`
    and broad enum-routing tests in the verification set.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Mark only canonical `window-decoration` as `Oracle complete` when the window
    decoration oracle test is present.
  - Add window decoration oracle detection to CFG-217 ownership so the generated
    matrix records `Experiment 34` when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 175 `Oracle complete`, 28
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 175 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting `window-decoration` parser semantics after the
    result is proven.

## Verification

Pass criteria:

- Focused Roastty test passes:

```bash
cargo test --manifest-path roastty/Cargo.toml window_decoration_config_parser_family_oracle
```

- Existing lower-level and routing tests still pass:

```bash
cargo test --manifest-path roastty/Cargo.toml window_decoration_parse_cli_resolves_bool_and_variants
cargo test --manifest-path roastty/Cargo.toml enum_config_parser_family_oracle
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=175`;
  - `audit_covered=28`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 175 rows are `Oracle complete`;
  - the `window-decoration` row is `Oracle complete`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 34`;
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
cargo test --manifest-path roastty/Cargo.toml window_decoration_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml window_decoration_parse_cli_resolves_bool_and_variants
cargo test --manifest-path roastty/Cargo.toml enum_config_parser_family_oracle
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
assert cfg217[11] == 'Experiment 34', cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
window_decoration = [row for row in parser_rows if row[1] == '`window-decoration`']
assert len(window_decoration) == 1, window_decoration
assert window_decoration[0][4] == 'Oracle complete', window_decoration[0]
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 175
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} window_decoration={window_decoration[0][4]} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/34-window-decoration-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Fresh-context adversarial subagent review completed before implementation.

**Verdict:** Approved.

No findings.
