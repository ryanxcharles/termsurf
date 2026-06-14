# Experiment 19: Duration Parser Oracle

## Description

CFG-217 still has 136 parser rows that are only `Audit covered`. The next
bounded family is the 4-row duration family. Pinned Ghostty's
`Duration.parseCLI` accepts a sequence of decimal `number+unit` segments, skips
ASCII whitespace before each segment, accepts trailing whitespace after a
complete segment, treats a bare `0` as unambiguous zero, rejects nonzero bare
numbers, rejects malformed units, and uses saturating arithmetic for overflow.

Roastty already has a ported `Duration::parse_cli` helper and existing field
tests for the four duration options. This experiment will add one focused family
oracle that ties the shared helper semantics to the required and optional config
dispatch shapes, then promote the 4 duration rows to `Oracle complete`.

CFG-217 must remain `Gap` because other parser families are still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused duration parser family test covering:
    - all upstream units: `y`, `w`, `d`, `h`, `m`, `s`, `ms`, `us`, `µs`, and
      `ns`;
    - longest-unit matching (`ms` beats `m`);
    - adjacent segments and whitespace-separated segments;
    - leading whitespace and trailing whitespace after a complete segment;
    - bare `0` as accepted zero;
    - arithmetic/product overflow saturating to `u64::MAX`, such as `600y`;
    - over-wide decimal literals rejected as `InvalidValue`, such as
      `18446744073709551616ns` and `18446744073709551616`;
    - missing value and all-whitespace value as `ValueRequired`;
    - malformed values such as nonzero bare numbers, unit-without-number,
      unknown units, and incomplete trailing segments as `InvalidValue`;
    - required duration field missing value as `ValueRequired`;
    - required duration field set-but-empty reset to the non-empty default;
    - optional duration field missing value as `ValueRequired`;
    - optional duration field set-but-empty reset to default `None`;
    - representative rows:
      - `undo-timeout` for `set_value_field(..., Duration::parse_cli)`;
      - `quit-after-last-window-closed-delay` for
        `set_optional_value_field(..., Duration::parse_cli)`.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Mark duration parser rows as `Oracle complete` when the duration family
    oracle test is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 71 `Oracle complete`, 132
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 71 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting direct duration parser semantics.

## Verification

Pass criteria:

- Focused Roastty tests pass:

```bash
cargo test --manifest-path roastty/Cargo.toml duration_config_parser_family_oracle
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=71`;
  - `audit_covered=132`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 71 rows are `Oracle complete`;
  - every duration row is `Oracle complete`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 19`;
  - CFG-217 evidence points to `config-parser-inventory.md`.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `git diff --check` passes.

Suggested commands:

```bash
cargo test --manifest-path roastty/Cargo.toml duration_config_parser_family_oracle
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
assert cfg217[11] == 'Experiment 19', cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
duration_rows = [row for row in parser_rows if row[3] == 'duration']
assert len(duration_rows) == 4, len(duration_rows)
assert all(row[4] == 'Oracle complete' for row in duration_rows)
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 71
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} duration_oracle={len(duration_rows)} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/19-duration-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Fresh-context adversarial design review found one required issue: the initial
plan only said to cover "saturating overflow", but Ghostty distinguishes
arithmetic/product overflow from over-wide decimal literals. Product overflow
such as `600y` saturates to `u64::MAX`, while an over-wide decimal literal such
as `18446744073709551616ns` is `InvalidValue`.

The design now requires both cases.

## Result

**Result:** Pass

Roastty now has a focused duration parser family oracle that ties the shared
`Duration::parse_cli` helper to both required and optional config dispatch
shapes. It covers the upstream units, longest-unit matching, adjacent and
whitespace-separated segments, trailing whitespace, bare zero, missing values,
malformed values, product-overflow saturation, over-wide decimal literal
rejection, and empty-reset behavior.

Focused Roastty verification passed:

```bash
cargo test --manifest-path roastty/Cargo.toml duration_config_parser_family_oracle
```

Output summary:

```text
running 1 test
test config::tests::duration_config_parser_family_oracle ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4909 filtered out; finished in 0.00s
```

The parser inventory generator passed and moved the 4 duration rows to
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
oracle_complete=71
audit_covered=132
gap=0
```

CFG-217 remains `Gap` because 132 parser rows are still audit-only, but there
are still no parser dispatch gaps.

## Conclusion

Duration parser semantics are now proven for the pinned Ghostty target. The
oracle explicitly records the overflow distinction found during design review:
product overflow saturates, while over-wide decimal literals are invalid.

## Completion Review

Fresh-context adversarial completion review approved the result with no required
findings. The reviewer independently verified the focused duration oracle, Rust
fmt check, generator counts after seeding a temporary matrix copy, matrix/parser
assertions, and `git diff --check`.

The reviewer noted one optional issue: the generator updates an existing matrix
file in place, so a `/tmp` matrix verification path must be seeded from the repo
matrix before running the generator. This experiment leaves that existing
generator contract unchanged; the documented repo command remains the canonical
verification command.
