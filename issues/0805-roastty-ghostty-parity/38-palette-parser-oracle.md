# Experiment 38: Palette Parser Oracle

## Description

CFG-217 still has 24 parser rows that are only `Audit covered`. Canonical
`palette` is one of the remaining `custom parse_cli` rows.

Pinned Ghostty defines `palette` as `Config.Palette`. `Palette.parseCLI`
requires a value, requires the first `=`, trims ASCII space and tab around the
key only, parses the key with Zig's base-0 `u8` integer grammar, parses the
color suffix with `Color.parseCLI`, and mutates the palette entry plus mask only
after both key and color parsing succeed. The surrounding `Config` optional
field dispatch treats an explicit empty config value as reset-to-default before
calling the child parser, while a missing value remains required.

Roastty already has lower-level palette parser, key parser, formatter,
config-routing, replay, diagnostics, and clone tests. This experiment will make
that coverage an explicit CFG-217 oracle, extend it where needed for the direct
parser boundary, wire the parser inventory to recognize the oracle, and promote
only canonical `palette`.

CFG-217 must remain `Gap` because other parser helpers are still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Rename or wrap the existing palette parser/config regression coverage as
    `palette_config_parser_family_oracle` so the inventory generator can detect
    it as the CFG-217 oracle for `Palette::parse_cli`.
  - Extend the oracle if needed to cover:
    - missing direct parser values;
    - missing `=` and first-`=` splitting;
    - ASCII space/tab key trimming;
    - Zig base-0 `u8` key syntax, including decimal, binary, octal, hexadecimal,
      uppercase prefixes, leading `+`, `-0`, interior underscores, malformed
      underscores, bare prefixes, overflow, and negative nonzero values;
    - color suffix parsing through `Color::parse_cli`, including accepted named
      and hex colors plus rejected invalid colors;
    - failed key or color parses leaving prior palette value and mask unchanged;
    - repeated assignments updating multiple indices and mask bits;
    - config empty value resetting to the default palette through surrounding
      dispatch;
    - config-file diagnostics preserving earlier valid values;
    - formatter output for all 256 entries;
    - replay behavior and clone semantics.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Detect `palette_config_parser_family_oracle`.
  - Mark only canonical `palette` as `Oracle complete` when the oracle test is
    present.
  - Add palette oracle detection to CFG-217 ownership so the generated matrix
    records `Experiment 38` when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 180 `Oracle complete`, 23
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 180 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting palette parser semantics after the result is
    proven.

## Verification

Pass criteria:

- Focused Roastty palette-family oracle passes:

```bash
cargo test --manifest-path roastty/Cargo.toml palette_config_parser_family_oracle
```

- Existing lower-level palette tests still pass:

```bash
cargo test --manifest-path roastty/Cargo.toml palette_parse_cli_sets_indices_and_mask
cargo test --manifest-path roastty/Cargo.toml palette_parse_cli_key_matches_zig_parse_int
cargo test --manifest-path roastty/Cargo.toml palette_format_entry_writes_all_256
cargo test --manifest-path roastty/Cargo.toml palette_config_replay_preserves_cli_and_file_values
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=180`;
  - `audit_covered=23`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 180 rows are `Oracle complete`;
  - `palette` is `Oracle complete`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 38`;
  - CFG-217 evidence points to `config-parser-inventory.md`.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  passes.
- No `__pycache__` or other `py_compile` artifacts remain in the issue folder.
- `git diff --check` passes.

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.

The reviewer confirmed the README links Experiment 38 as `Designed`, the
experiment has the required sections, the scope is limited to the canonical
`palette` parser row and CFG-217 inventory ownership, the design matches
upstream `Palette.parseCLI`, and the verification list includes the required
tests, generator assertions, formatting, py_compile cleanup, prettier, and
`git diff --check`.

Suggested commands:

```bash
cargo test --manifest-path roastty/Cargo.toml palette_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml palette_parse_cli_sets_indices_and_mask
cargo test --manifest-path roastty/Cargo.toml palette_parse_cli_key_matches_zig_parse_int
cargo test --manifest-path roastty/Cargo.toml palette_format_entry_writes_all_256
cargo test --manifest-path roastty/Cargo.toml palette_config_replay_preserves_cli_and_file_values
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
        rows.append([cell.strip() for cell in line.strip('|').split('|')])

assert len(rows) == 203, len(rows)
assert sum(row[4] == 'Oracle complete' for row in rows) == 180
assert sum(row[4] == 'Audit covered' for row in rows) == 23
assert not [row for row in rows if row[4] == 'Gap']
row = next(row for row in rows if row[1] == '`palette`')
assert row[4] == 'Oracle complete', row

cfg217 = None
for line in Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text().splitlines():
    if line.startswith('| CFG-217 |'):
        cfg217 = [cell.strip() for cell in line.strip('|').split('|')]
        break
assert cfg217 is not None
assert cfg217[4] == 'Gap', cfg217
assert cfg217[11] == 'Experiment 38', cfg217
assert '180 parser rows Oracle complete' in cfg217[12], cfg217
print('palette_oracle_rows=1 oracle_complete=180 cfg217=Gap')
PY
cargo fmt --manifest-path roastty/Cargo.toml
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/38-palette-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Result

**Result:** Pass

Roastty now has a focused palette parser family oracle for the canonical
`palette` row.

Implementation notes:

- Renamed and extended the existing config-routing regression as
  `palette_config_parser_family_oracle`.
- Added direct parser coverage for required values, missing `=`, first-`=`
  splitting, named color parsing, failed color/key parse atomicity, and overflow
  behavior.
- Taught `config_parser_inventory.py` to detect the palette oracle, promote only
  `palette`, and make CFG-217's owner `Experiment 38` when this oracle is
  present.

Verification commands run:

```bash
cargo test --manifest-path roastty/Cargo.toml palette_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml palette_parse_cli_sets_indices_and_mask
cargo test --manifest-path roastty/Cargo.toml palette_parse_cli_key_matches_zig_parse_int
cargo test --manifest-path roastty/Cargo.toml palette_format_entry_writes_all_256
cargo test --manifest-path roastty/Cargo.toml palette_config_replay_preserves_cli_and_file_values
```

Results:

```text
test config::tests::palette_config_parser_family_oracle ... ok
test config::tests::palette_parse_cli_sets_indices_and_mask ... ok
test config::tests::palette_parse_cli_key_matches_zig_parse_int ... ok
test config::tests::palette_format_entry_writes_all_256 ... ok
test config::tests::palette_config_replay_preserves_cli_and_file_values ... ok
```

The `palette_config_parser_family_oracle` filter also matched the existing
`command_palette_config_parser_family_oracle`; both tests passed in that run.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_parser_inventory.py \
  --upstream vendor/ghostty/src/config/Config.zig \
  --roastty roastty/src/config/mod.rs \
  --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md \
  --output issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
```

Result:

```text
ghostty_canonical=203
roastty_parser_rows=203
missing_canonical_parser_rows=0
missing_dispatch_rows=0
extra_parser_rows=0
compatibility_only_parser_arms=5
noncanonical_noncompat_parser_arms=0
oracle_complete=180
audit_covered=23
gap=0
```

The matrix assertion passed and printed:

```text
palette_oracle_rows=1 oracle_complete=180 cfg217=Gap
```

Additional hygiene checks passed:

```bash
cargo fmt --manifest-path roastty/Cargo.toml
python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
```

## Conclusion

The `palette` parser row is now oracle-complete for CFG-217. The important
upstream boundary is that `Palette.parseCLI` splits on the first `=`, trims only
the key with ASCII space/tab, parses a Zig base-0 `u8` key and `Color.parseCLI`
color suffix, and mutates the palette only after both parse successfully.
CFG-217 remains `Gap` because 23 parser rows are still only audit-covered.

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

Findings: none.

The reviewer independently verified the palette-focused tests, Rust format
check, prettier check, `git diff --check`, generated inventory counts, `palette`
row promotion, CFG-217's `Gap` status and Experiment 38 ownership, and that the
result commit had not yet been made.
