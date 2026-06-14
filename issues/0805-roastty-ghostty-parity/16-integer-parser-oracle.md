# Experiment 16: Integer Parser Oracle

## Description

CFG-217 remains open because 164 parser rows are still only `Audit covered`. The
next bounded family is the 10-row integer scalar family. Pinned Ghostty's
generic parser uses `std.fmt.parseInt(Int, value, 0)` for integer fields:
missing values are `ValueRequired`, set-but-empty values reset to the field
default before parsing, base prefixes are accepted in base 0, underscores and
signs follow Zig integer parsing, and invalid or overflowing values become
invalid config values.

Roastty uses five integer helpers across these rows:

- `parse_u32_scalar_field`
- `parse_usize_scalar_field`
- `parse_u64_scalar_field`
- `parse_i16_field`
- `parse_u8_field`

This experiment will prove those direct parser semantics through representative
config fields, then promote all 10 integer scalar parser rows to
`Oracle complete`. CFG-217 must remain `Gap` because other parser families are
still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused integer parser family test covering:
    - base-0 decimal, lowercase and uppercase hex, binary, and octal prefixes
      (`0x`/`0X`, `0b`/`0B`, `0o`/`0O`);
    - accepted `+` on unsigned fields and accepted unsigned `-0`;
    - nonzero negative unsigned values as range failures;
    - signed `i16` positive and negative prefixed values;
    - accepted interior underscores and rejected leading, trailing, and
      prefix-adjacent underscores;
    - rejected bare sign and bare prefixes;
    - missing value as `ValueRequired`;
    - set-but-empty value reset to default;
    - invalid syntax as `InvalidValue`;
    - overflow/range failures as `InvalidValue`;
    - representative fields for every integer helper: `image-storage-limit`
      (`u32`), `scrollback-limit` (`usize`), `linux-cgroup-memory-limit`
      (`Option<u64>`), `window-position-x` (`Option<i16>`), and
      `font-thicken-strength` (`u8`).
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Mark integer scalar parser rows as `Oracle complete` when the integer family
    oracle test is present.
  - Update the generated inventory header so it no longer claims the parser
    inventory is only for Experiment 13.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 49 `Oracle complete`, 154
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 49 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting the integer scalar parser oracle.

## Verification

Pass criteria:

- Focused Roastty tests pass:

```bash
cargo test --manifest-path roastty/Cargo.toml integer_config_parser_family_oracle
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=49`;
  - `audit_covered=154`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 49 rows are `Oracle complete`;
  - every integer scalar row is `Oracle complete`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 evidence points to `config-parser-inventory.md`.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `git diff --check` passes.

Suggested commands:

```bash
cargo test --manifest-path roastty/Cargo.toml integer_config_parser_family_oracle
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
integer_rows = [row for row in parser_rows if row[3] == 'integer scalar']
assert len(integer_rows) == 10, len(integer_rows)
assert all(row[4] == 'Oracle complete' for row in integer_rows)
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 49
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} integer_oracle={len(integer_rows)} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/16-integer-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Result

**Result:** Pass

The integer parser oracle is now implemented in
`integer_config_parser_family_oracle`. It verifies the shared integer parser
semantics through representative `u32`, `usize`, `u64`, `i16`, and `u8` config
fields, including base-0 prefixes, signs, underscores, empty reset, missing
values, invalid syntax, and overflow/range failures.

Focused Roastty verification passed:

```bash
cargo test --manifest-path roastty/Cargo.toml integer_config_parser_family_oracle
```

Output summary:

```text
running 1 test
test config::tests::integer_config_parser_family_oracle ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4906 filtered out; finished in 0.01s
```

The parser inventory generator passed and moved the 10 integer scalar rows to
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
oracle_complete=49
audit_covered=154
gap=0
```

CFG-217 remains `Gap` because 154 parser rows are still audit-only, but there
are still no parser dispatch gaps.

## Conclusion

Integer scalar parser semantics are now proven for the pinned Ghostty target.
The next parser-facet experiment should choose another bounded parser family and
promote only the rows whose accepted/rejected value space has an
upstream-derived oracle.

## Completion Review

Fresh-context adversarial completion review approved the result with no required
findings. The reviewer independently verified the focused Rust test, Rust fmt
check, generator output using `/tmp` destinations, matrix/parser assertions, and
`git diff --check`.

Because the first review response only cited the committed range diff, a focused
re-review explicitly inspected the uncommitted working-tree diff. The re-review
again approved with no findings and confirmed the result stayed within
Experiment 16 scope.

## Design Review

Fresh-context adversarial design review found one required issue and one
optional issue:

- The integer oracle criteria were too broad to prove full Zig base-0 integer
  semantics before promoting all 10 integer rows. The design did not explicitly
  require uppercase prefixes, signed unsigned edge cases, bare signs/prefixes,
  or prefix-adjacent underscore failures.
- The generated parser inventory header still claimed it was generated for
  Experiment 13.

Fixes:

- Added explicit required cases for uppercase `0B`/`0O`/`0X`, accepted `+` and
  unsigned `-0`, nonzero negative unsigned range failure, signed prefixed `i16`,
  bare signs, bare prefixes, and leading/trailing/prefix-adjacent underscores.
- Added the generator header provenance update to the experiment scope.

Re-review approved the fixed design:

```text
VERDICT: APPROVED

Findings: none.
```
