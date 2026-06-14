# Experiment 18: String Parser Oracle

## Description

CFG-217 still has 145 parser rows that are only `Audit covered`. The next
bounded family is the 9-row string scalar family. Pinned Ghostty's generic
config parser uses the type-magic `[]const u8` / `[:0]const u8` branches in
`parseIntoField`: a missing value is `ValueRequired`, otherwise the byte slice
is copied exactly. Optional string fields are parsed through the child string
parser, so a bare key is still `ValueRequired`; the surrounding empty-value
dispatch resets the field to its default.

Roastty's direct string helper already preserves ordinary text and distinguishes
missing values from explicit empty strings, but it currently rejects embedded
NUL bytes. Ghostty's byte-slice copy does not reject embedded NULs at this
layer, including for sentinel slices, where the sentinel is appended after the
copied slice. This experiment will align the direct string parser with that
upstream shape and prove the representative required and optional string-field
behavior.

When complete, all 9 string scalar parser rows should move to `Oracle complete`.
CFG-217 must remain `Gap` because other parser families are still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Update `parse_string_field` to copy any provided `&str` exactly, including
    embedded NUL bytes.
  - Add a focused string parser family test covering:
    - direct helper missing value as `ValueRequired`;
    - direct helper explicit empty string as an accepted empty string;
    - direct helper embedded NUL preservation;
    - required string field missing value as `ValueRequired`;
    - required string field value preservation, including Unicode and embedded
      NUL bytes;
    - required string field set-but-empty reset to the default;
    - optional string field missing value as `ValueRequired`;
    - optional string field value preservation, including embedded NUL bytes;
    - optional string field set-but-empty reset to the default `None`.
  - Use representative fields for both helper shapes:
    - `term` for `set_value_field(..., parse_string_field)`, because its
      non-empty default proves that `key =` takes the empty-reset branch instead
      of parsing an empty string;
    - `parse_string_field(Some(""))` directly for explicit empty-string
      preservation;
    - `title` or `class` for
      `set_optional_value_field(..., parse_string_field)`.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Mark string scalar parser rows as `Oracle complete` when the string family
    oracle test is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 67 `Oracle complete`, 136
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 67 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting direct string parser semantics and the embedded
    NUL correction.

## Verification

Pass criteria:

- Focused Roastty tests pass:

```bash
cargo test --manifest-path roastty/Cargo.toml string_config_parser_family_oracle
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=67`;
  - `audit_covered=136`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 67 rows are `Oracle complete`;
  - every string scalar row is `Oracle complete`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 18`;
  - CFG-217 evidence points to `config-parser-inventory.md`.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `git diff --check` passes.

Suggested commands:

```bash
cargo test --manifest-path roastty/Cargo.toml string_config_parser_family_oracle
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
assert cfg217[11] == 'Experiment 18', cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
string_rows = [row for row in parser_rows if row[3] == 'string']
assert len(string_rows) == 9, len(string_rows)
assert all(row[4] == 'Oracle complete' for row in string_rows)
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 67
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} string_oracle={len(string_rows)} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/18-string-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Fresh-context adversarial design review found one required issue: the initial
plan allowed `enquiry-response` as the representative required string field for
empty-reset behavior, but that field's default is already empty, so it would not
distinguish parsing an explicit empty string from taking the empty-reset branch.

The design now requires `term` for the required string-field reset assertion
because its default is non-empty, and keeps explicit empty-string preservation
at the direct `parse_string_field(Some(""))` helper level.

## Result

**Result:** Pass

Roastty's direct string parser now copies any provided `&str` exactly, including
embedded NUL bytes, matching Ghostty's byte-slice copy behavior at this parser
layer. The focused oracle proves missing values, explicit empty strings, NUL
preservation, required string fields, optional string fields, and empty-reset
dispatch using `term` and `title`.

Focused Roastty verification passed:

```bash
cargo test --manifest-path roastty/Cargo.toml string_config_parser_family_oracle
```

Output summary:

```text
running 1 test
test config::tests::string_config_parser_family_oracle ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4908 filtered out; finished in 0.00s
```

The parser inventory generator passed and moved the 9 string scalar rows to
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
oracle_complete=67
audit_covered=136
gap=0
```

CFG-217 remains `Gap` because 136 parser rows are still audit-only, but there
are still no parser dispatch gaps.

## Conclusion

String scalar parser semantics are now proven for the pinned Ghostty target. The
experiment also found and fixed a real parser parity issue: embedded NUL bytes
were incorrectly rejected by Roastty's direct string helper.

## Completion Review

Fresh-context adversarial completion review initially found one required issue:
existing tests for string rows still expected embedded NUL values to be
rejected, which contradicted the new oracle. The stale tests and load-string
diagnostic expectations were updated so embedded NUL string values are accepted
and preserved consistently across the existing config parser test coverage.

Focused re-review approved the completed result with no remaining findings. The
reviewer independently verified the focused string oracle, the broader
`config_parse_format_reset_and_diagnose` subset, generator/matrix assertions,
Rust fmt check, and `git diff --check`.
