# Experiment 21: Working Directory Parser Oracle

## Description

CFG-217 still has 129 parser rows that are only `Audit covered`. The next
bounded family is the 1-row `working-directory` parser family.

Pinned Ghostty implements `working-directory` as `?WorkingDirectory` plus
`WorkingDirectory.parseCLI`. At the config-option boundary, a raw empty value
resets the optional field to its default `null`; otherwise the child parser
trims ASCII whitespace, rejects missing or empty/all-whitespace values, strips a
single surrounding pair of double quotes, accepts the exact lowercase keywords
`home` and `inherit`, and treats every other resulting byte string as a path.

Roastty already has `WorkingDirectory::parse_cli` and option-level tests. This
experiment will add one focused family oracle that ties those helper semantics
to `working-directory` dispatch, then promote the 1 working-directory row to
`Oracle complete`.

This experiment is limited to parser and formatter semantics. Home expansion,
probable-CLI defaults, command interaction, and finalization behavior remain
separate config facets under CFG-221/related rows.

CFG-217 must remain `Gap` because other parser families are still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused working-directory parser family test covering:
    - exact `home` and `inherit` keyword parsing;
    - case-sensitive keyword behavior, where `Home` and `INHERIT` are paths;
    - ASCII whitespace trimming before keyword/path classification;
    - surrounding double-quote stripping after trimming;
    - quoted keywords becoming keywords;
    - quoted strings with interior edge spaces remaining paths;
    - quoted empty string becoming `Path("")`;
    - ordinary, tilde-prefixed, relative, and embedded-NUL paths;
    - missing values and all-whitespace values as `ValueRequired`;
    - raw empty option value resetting `working-directory` to default `None`;
    - formatter output for `home`, `inherit`, ordinary paths, and empty paths.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Mark working-directory parser rows as `Oracle complete` when the
    working-directory family oracle test is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 75 `Oracle complete`, 128
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 75 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting working-directory parser semantics after the
    result is proven.

## Verification

Pass criteria:

- Focused Roastty tests pass:

```bash
cargo test --manifest-path roastty/Cargo.toml working_directory_config_parser_family_oracle
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=75`;
  - `audit_covered=128`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 75 rows are `Oracle complete`;
  - the single working-directory row is `Oracle complete`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 21`;
  - CFG-217 evidence points to `config-parser-inventory.md`.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `git diff --check` passes.

## Design Review

Fresh-context adversarial design review approved the experiment plan with no
findings.

Suggested commands:

```bash
cargo test --manifest-path roastty/Cargo.toml working_directory_config_parser_family_oracle
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
assert cfg217[11] == 'Experiment 21', cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
working_directory_rows = [row for row in parser_rows if row[3] == 'working directory']
assert len(working_directory_rows) == 1, len(working_directory_rows)
assert working_directory_rows[0][4] == 'Oracle complete'
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 75
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} working_directory_oracle={len(working_directory_rows)} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/21-working-directory-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Result

**Result:** Pass

Roastty now has a focused working-directory parser family oracle for the single
`working-directory` parser row. The oracle proves pinned Ghostty's
option-boundary parser behavior:

- `home` and `inherit` are exact lowercase keywords;
- non-matching values, including differently cased keywords, are paths;
- ASCII whitespace is trimmed before parsing;
- one surrounding pair of double quotes is stripped after trimming;
- quoted keywords become keywords, while quoted strings with interior edge
  spaces remain paths;
- quoted empty strings become `Path("")`;
- ordinary, tilde-prefixed, relative, and embedded-NUL paths are accepted;
- missing and all-whitespace values are `ValueRequired`;
- raw empty option values reset `working-directory` to default `None`;
- formatter output is proven for keywords, ordinary paths, and empty paths.

Focused Roastty verification passed:

```bash
cargo test --manifest-path roastty/Cargo.toml working_directory_config_parser_family_oracle
```

Output summary:

```text
running 1 test
test config::tests::working_directory_config_parser_family_oracle ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4911 filtered out; finished in 0.01s
```

The parser inventory generator passed and moved the 1 working-directory row to
`Oracle complete`:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_parser_inventory.py \
  --upstream vendor/ghostty/src/config/Config.zig \
  --roastty roastty/src/config/mod.rs \
  --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md \
  --output issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
```

Output:

```text
ghostty_canonical=203
roastty_parser_rows=203
missing_canonical_parser_rows=0
missing_dispatch_rows=0
extra_parser_rows=0
compatibility_only_parser_arms=5
noncanonical_noncompat_parser_arms=0
oracle_complete=75
audit_covered=128
gap=0
```

The matrix assertion passed:

```text
parser_rows=203 working_directory_oracle=1 cfg217=Gap
```

CFG-217 remains `Gap` because 128 parser rows are still audit-only, but the
working-directory parser family is now oracle-complete.

## Conclusion

Working-directory parser semantics are now proven for the pinned Ghostty target
at the config-option boundary. The parser row can be separated cleanly from
working-directory finalization: direct parsing handles trimming, quotes,
keywords, path fallback, missing values, raw-empty reset, embedded NULs, and
formatting; home expansion and command/default interaction remain later
configuration facets.

## Completion Review

Fresh-context adversarial completion review approved the result with no
findings. The reviewer independently verified the focused working-directory
oracle, Rust fmt check, `git diff --check`, matrix assertions, and generator
state: 203 parser rows, 75 `Oracle complete`, 128 `Audit covered`, 0 `Gap` rows,
the single working-directory row promoted to `Oracle complete`, and CFG-217
still `Gap` with owner `Experiment 21`.
