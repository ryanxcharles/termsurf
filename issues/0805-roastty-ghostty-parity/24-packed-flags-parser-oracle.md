# Experiment 24: Packed Flags Parser Oracle

## Description

CFG-217 still has 125 parser rows that are only `Audit covered`. The next
bounded family is the 9-row packed-flags parser family:

- `app-notifications`;
- `bell-features`;
- `font-shaping-break`;
- `font-synthetic-style`;
- `freetype-load-flags`;
- `notify-on-command-finish-action`;
- `scroll-to-bottom`;
- `shell-integration-features`;
- `split-preserve-zoom`.

Pinned Ghostty parses these options through packed bool structs: a standalone
bool sets every field, while a comma-separated `[no-]flag` list starts from the
struct default and toggles named fields. Flag names are exact field names, with
hyphenated names for upstream quoted fields such as `ssh-env`, `ssh-terminfo`,
`force-autohint`, and `bold-italic`. Unknown names produce `InvalidValue`. The
surrounding config option dispatch maps missing values to `ValueRequired` and
raw empty option values to default resets.

Roastty already has shared `parse_packed_flags` and per-struct `parse_cli`
helpers. This experiment will turn those helpers into a focused family oracle,
then promote the 9 packed-flags rows to `Oracle complete`.

This experiment is limited to parser, formatter, reset, and diagnostic semantics
for direct packed-flags config rows. Runtime effects such as bell,
notifications, shell integration, font shaping, and FreeType backend behavior
remain separate parity facets.

CFG-217 must remain `Gap` because other parser families are still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused packed-flags parser family test covering:
    - standalone bool spellings accepted by upstream `parseBool`, including
      `true`, `false`, `1`, `0`, `t`, `T`, `f`, and `F`;
    - comma-separated flag lists that start from defaults;
    - `no-` prefixes disabling named flags;
    - space/tab trimming around comma parts;
    - exact hyphenated names for quoted upstream fields;
    - duplicate flags where the later token wins;
    - missing values as `ValueRequired`;
    - raw empty option values resetting fields to defaults;
    - invalid direct parser values including unknown flags, snake-case aliases,
      empty strings, empty comma parts, uppercase bool words, newline-padded
      bools, and newline-padded flag names;
    - formatter output in upstream field order;
    - load-string diagnostics preserving valid earlier values while reporting
      invalid later lines.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Mark packed-flags parser rows as `Oracle complete` when the packed-flags
    family oracle test is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 87 `Oracle complete`, 116
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 87 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting packed-flags parser semantics after the result is
    proven.

## Verification

Pass criteria:

- Focused Roastty tests pass:

```bash
cargo test --manifest-path roastty/Cargo.toml packed_flags_config_parser_family_oracle
```

- Existing packed-flags parser regression tests still pass:

```bash
cargo test --manifest-path roastty/Cargo.toml packed_flags_parse_cli
cargo test --manifest-path roastty/Cargo.toml packed_flags_parse_cli_shell_notify
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=87`;
  - `audit_covered=116`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 87 rows are `Oracle complete`;
  - all 9 packed-flags rows are `Oracle complete`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 24`;
  - CFG-217 evidence points to `config-parser-inventory.md`.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `git diff --check` passes.

Suggested commands:

```bash
cargo test --manifest-path roastty/Cargo.toml packed_flags_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml packed_flags_parse_cli
cargo test --manifest-path roastty/Cargo.toml packed_flags_parse_cli_shell_notify
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
assert cfg217[11] == 'Experiment 24', cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
packed_rows = [row for row in parser_rows if row[3] == 'packed flags']
assert len(packed_rows) == 9, len(packed_rows)
assert all(row[4] == 'Oracle complete' for row in packed_rows)
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 87
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} packed_flags_oracle={len(packed_rows)} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/24-packed-flags-parser-oracle.md \
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

Roastty now has a focused packed-flags parser family oracle for the 9 direct
packed-flags parser rows. The oracle proves pinned Ghostty's packed bool struct
parser boundary:

- standalone upstream bool spellings set every flag;
- comma-separated flag lists start from struct defaults;
- `no-` prefixes disable named flags;
- spaces and tabs are trimmed around comma parts;
- hyphenated field names such as `ssh-env`, `ssh-terminfo`, `force-autohint`,
  and `bold-italic` are exact;
- duplicate flags use the later token;
- missing option values are `ValueRequired`;
- raw empty option values reset each packed field to its default;
- unknown flags, snake-case aliases, empty strings, empty comma parts, uppercase
  bool words, newline-padded bools, and newline-padded flag names are rejected;
- formatter output follows upstream field order;
- load-string diagnostics preserve valid earlier values while reporting invalid
  later lines.

Focused Roastty verification passed:

```bash
cargo test --manifest-path roastty/Cargo.toml packed_flags_config_parser_family_oracle
```

Output summary:

```text
running 1 test
test config::tests::packed_flags_config_parser_family_oracle ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4914 filtered out; finished in 0.02s
```

Existing packed-flags regression tests also passed:

```bash
cargo test --manifest-path roastty/Cargo.toml packed_flags_parse_cli
cargo test --manifest-path roastty/Cargo.toml packed_flags_parse_cli_shell_notify
```

Output summaries:

```text
running 2 tests
test config::tests::packed_flags_parse_cli ... ok
test config::tests::packed_flags_parse_cli_shell_notify ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 4913 filtered out; finished in 0.00s
```

```text
running 1 test
test config::tests::packed_flags_parse_cli_shell_notify ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4914 filtered out; finished in 0.00s
```

The parser inventory generator passed and moved all 9 packed-flags rows to
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
oracle_complete=87
audit_covered=116
gap=0
```

Matrix assertion output:

```text
parser_rows=203 packed_flags_oracle=9 cfg217=Gap
```

## Conclusion

The packed-flags parser family is now `Oracle complete`. CFG-217 remains `Gap`
because 116 parser rows are still audit-covered only. The next parser-family
experiment should continue reducing that count with another bounded non-scalar
family.

## Completion Review

Fresh-context adversarial completion review approved the result with no
findings.
