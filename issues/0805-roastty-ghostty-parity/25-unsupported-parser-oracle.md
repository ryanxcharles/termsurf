# Experiment 25: Unsupported Parser Oracle

## Description

CFG-217 still has 116 parser rows that are only `Audit covered`. The smallest
remaining parser family is the 1-row unsupported family:

- `link`.

Pinned Ghostty declares canonical `link` as `RepeatableLink`, but
`RepeatableLink.parseCLI` always returns `error.NotImplemented`. That makes
`link` a recognized parser path, not an unknown-field path. Ghostty's generic
set-but-empty reset still runs before `parseCLI`, so `link =` resets to the
default link list and succeeds, while bare `link` and non-empty `link = ...`
return the recognized not-implemented parser error.

Experiment 14 added the `link` dispatch and proved that boundary. This
experiment will turn that evidence into an explicit unsupported-family oracle
and promote the one unsupported parser row to `Oracle complete`.

This experiment is limited to parser recognition, reset, diagnostics, and
inventory classification for Ghostty's current not-implemented `link` parser. It
does not implement real link matching, rendering, or click behavior.

CFG-217 must remain `Gap` because other parser families are still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused unsupported parser family oracle test covering:
    - `link` is recognized and returns `ConfigSetError::NotImplemented` for
      missing values;
    - `link` is recognized and returns `ConfigSetError::NotImplemented` for
      non-empty values;
    - the error is distinct from `UnknownField`;
    - raw empty `link =` resets to the default link list and succeeds;
    - `load_str` records a `NotImplemented` diagnostic for invalid `link` lines,
      preserves the default links after `link =`, and still reports truly
      unknown keys as `UnknownField`.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Mark unsupported parser rows as `Oracle complete` when the unsupported
    family oracle test is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 88 `Oracle complete`, 115
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 88 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting unsupported `link` parser-family semantics after
    the result is proven.

## Verification

Pass criteria:

- Focused Roastty tests pass:

```bash
cargo test --manifest-path roastty/Cargo.toml unsupported_config_parser_family_oracle
```

- Existing link-recognition regression test still passes:

```bash
cargo test --manifest-path roastty/Cargo.toml link_config_parser_recognizes_not_implemented_and_empty_reset
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=88`;
  - `audit_covered=115`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 88 rows are `Oracle complete`;
  - the one unsupported row is `Oracle complete`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 25`;
  - CFG-217 evidence points to `config-parser-inventory.md`.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `git diff --check` passes.

Suggested commands:

```bash
cargo test --manifest-path roastty/Cargo.toml unsupported_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml link_config_parser_recognizes_not_implemented_and_empty_reset
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
assert cfg217[11] == 'Experiment 25', cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
unsupported_rows = [row for row in parser_rows if row[3] == 'unsupported']
assert len(unsupported_rows) == 1, len(unsupported_rows)
assert unsupported_rows[0][1] == '`link`', unsupported_rows
assert unsupported_rows[0][4] == 'Oracle complete', unsupported_rows
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 88
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} unsupported_oracle={len(unsupported_rows)} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/25-unsupported-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Fresh-context adversarial design review approved the experiment plan with no
findings.

## Result

**Result:** Pass

Roastty now has a focused unsupported parser family oracle for the one
unsupported parser row, canonical `link`. The oracle proves pinned Ghostty's
current `RepeatableLink.parseCLI` boundary:

- `link` is a recognized config key;
- missing `link` values return `ConfigSetError::NotImplemented`;
- non-empty `link` values return `ConfigSetError::NotImplemented`;
- the recognized not-implemented parser error is distinct from `UnknownField`;
- raw-empty `link =` resets the link list to the default and succeeds;
- load-string diagnostics report `NotImplemented` for invalid `link` lines,
  preserve the default links after `link =`, and still report truly unknown keys
  as `UnknownField`.

Focused Roastty verification passed:

```bash
cargo test --manifest-path roastty/Cargo.toml unsupported_config_parser_family_oracle
```

Output summary:

```text
running 1 test
test config::tests::unsupported_config_parser_family_oracle ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4915 filtered out; finished in 0.00s
```

The existing link-recognition regression test also passed:

```bash
cargo test --manifest-path roastty/Cargo.toml link_config_parser_recognizes_not_implemented_and_empty_reset
```

Output summary:

```text
running 1 test
test config::tests::link_config_parser_recognizes_not_implemented_and_empty_reset ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4915 filtered out; finished in 0.00s
```

The parser inventory generator passed and moved the unsupported `link` row to
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
oracle_complete=88
audit_covered=115
gap=0
```

Matrix assertion output:

```text
parser_rows=203 unsupported_oracle=1 cfg217=Gap
```

## Conclusion

The unsupported parser family is now `Oracle complete`. CFG-217 remains `Gap`
because 115 parser rows are still audit-covered only. The next parser-family
experiment should continue reducing that count with another bounded family.

## Completion Review

Fresh-context adversarial completion review approved the result with no
findings.
