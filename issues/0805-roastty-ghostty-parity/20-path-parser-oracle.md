# Experiment 20: Path Parser Oracle

## Description

CFG-217 still has 132 parser rows that are only `Audit covered`. The next
bounded family is the 3-row path family:

- `background-image`;
- `bell-audio-path`;
- `config-file`.

Pinned Ghostty implements these with `config.path.Path` and
`config.path.RepeatablePath`. At the config-option boundary, optional
single-path fields are parsed as `?Path` plus `Path.parseCLI`: a leading `?` is
an optional-path marker, a quoted leading `?` is preserved as a required literal
path, parsed-empty values such as `?`, `""`, and `?""` are ignored without
overwriting an existing path, and a raw empty value resets the field to its
default `null`. Repeatable paths use `RepeatablePath.parseCLI`: parsed-empty
items are ignored, while a raw empty repeatable value clears the list.

Roastty already has `ConfigFilePath::parse_single`, `RepeatableConfigPath`, and
existing option-level tests for these fields. This experiment will add one
focused family oracle that ties the direct path helper semantics to the required
option dispatch shapes, then promote the 3 path rows to `Oracle complete`.

This experiment is limited to parser and formatter semantics. Path expansion,
relative-base handling, recursive config-file loading, and diagnostics for
missing files remain separate config facets under CFG-221/related rows.

CFG-217 must remain `Gap` because other parser families are still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused path parser family test covering:
    - required single paths, optional single paths, quoted required literal
      paths beginning with `?`, optional quoted paths, parsed-empty optional
      paths, quoted empty required paths, and optional quoted empty paths;
    - `background-image` and `bell-audio-path` as representatives of optional
      single-path fields parsed through the `Path.parseCLI` shape;
    - missing single-path values as `ValueRequired`;
    - raw empty optional single-path values resetting to the field default
      `None` through the surrounding optional dispatch helper;
    - parsed-empty optional single-path values ignored without overwriting an
      existing path;
    - embedded NUL bytes accepted at the parser layer to match Ghostty's
      byte-copying `dupeZ` parser boundary;
    - repeatable `config-file` accumulation for required, optional, quoted
      required-literal, and optional quoted paths;
    - parsed-empty repeatable paths such as `?`, `""`, and `?""` ignored without
      clearing prior entries;
    - raw empty repeatable value clearing the list;
    - missing repeatable values as `ValueRequired`;
    - formatter output for empty, single, optional, and multiple path entries.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Mark path parser rows as `Oracle complete` when the path family oracle test
    is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 74 `Oracle complete`, 129
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 74 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting direct path parser semantics after the result is
    proven.

## Verification

Pass criteria:

- Focused Roastty tests pass:

```bash
cargo test --manifest-path roastty/Cargo.toml path_config_parser_family_oracle
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=74`;
  - `audit_covered=129`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 74 rows are `Oracle complete`;
  - every path row is `Oracle complete`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 20`;
  - CFG-217 evidence points to `config-parser-inventory.md`.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `git diff --check` passes.

## Design Review

Fresh-context adversarial design review found three required issues:

- The initial design required embedded NUL rejection while claiming upstream
  parser parity. Pinned Ghostty's path parser copies bytes into sentinel-backed
  storage and does not report NUL as a parser error, so the plan now requires
  accepting embedded NUL at the parser layer.
- The initial design conflated `Path.parse` with option-level `Path.parseCLI`.
  The plan now scopes single-path option behavior to `?Path` plus
  `Path.parseCLI`, where parsed-empty values do not overwrite existing paths and
  raw empty values reset through `parseIntoField`.
- Because of those mismatches, the initial design did not justify promoting all
  3 path rows to `Oracle complete`. The promotion is now conditional on a path
  oracle that matches `Path.parseCLI`/`RepeatablePath.parseCLI` behavior at the
  config-option boundary.

The corrected design was re-reviewed by the same fresh-context adversarial
subagent and approved with no required findings.

Suggested commands:

```bash
cargo test --manifest-path roastty/Cargo.toml path_config_parser_family_oracle
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
assert cfg217[11] == 'Experiment 20', cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
path_rows = [row for row in parser_rows if row[3] == 'path']
assert len(path_rows) == 3, len(path_rows)
assert all(row[4] == 'Oracle complete' for row in path_rows)
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 74
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} path_oracle={len(path_rows)} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/20-path-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Result

**Result:** Pass

Roastty now has a focused path parser family oracle for the 3 path rows:
`background-image`, `bell-audio-path`, and `config-file`. The implementation now
matches pinned Ghostty's option-boundary path behavior for optional single-path
fields and repeatable paths:

- optional single paths accept required, optional, quoted-required-literal, and
  optional-quoted paths;
- parsed-empty single-path values such as `?`, `""`, and `?""` leave the
  existing value unchanged;
- raw empty single-path values reset the optional field to default `None`;
- embedded NUL bytes are accepted at the parser layer;
- repeatable `config-file` values accumulate, ignore parsed-empty entries, clear
  on raw empty values, and format as repeated config lines.

Focused Roastty verification passed:

```bash
cargo test --manifest-path roastty/Cargo.toml path_config_parser_family_oracle
```

Output summary:

```text
running 1 test
test config::tests::path_config_parser_family_oracle ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4910 filtered out; finished in 0.01s
```

The changed legacy path tests also passed:

```bash
cargo test --manifest-path roastty/Cargo.toml config_file_repeatable_path_parse_cli_matches_upstream
cargo test --manifest-path roastty/Cargo.toml bell_audio_path_parses_single_path_empty_optional_and_nul_values
cargo test --manifest-path roastty/Cargo.toml background_image_config_parse_format_reset_and_diagnose
cargo test --manifest-path roastty/Cargo.toml config_file_accumulates_resets_and_formats
```

The parser inventory generator passed and moved the 3 path rows to
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
oracle_complete=74
audit_covered=129
gap=0
```

The matrix assertion passed:

```text
parser_rows=203 path_oracle=3 cfg217=Gap
```

CFG-217 remains `Gap` because 129 parser rows are still audit-only, but the path
parser family is now oracle-complete.

## Conclusion

Path parser semantics are now proven for the pinned Ghostty target at the
config-option boundary. The main lesson is that path parity must follow
`Path.parseCLI`/`RepeatablePath.parseCLI`, not only the lower-level `Path.parse`
helper: parsed-empty single-path values are no-ops, raw empty values reset
through `parseIntoField`, and parser-layer embedded NUL bytes are accepted.

## Completion Review

Fresh-context adversarial completion review approved the result with no
findings. The reviewer independently verified the focused path oracle, the
changed legacy path tests, Rust fmt check, `git diff --check`, and the generated
matrix/inventory state: 203 parser rows, 3 path rows, 74 `Oracle complete`, 129
`Audit covered`, 0 `Gap` rows, and CFG-217 still `Gap` with owner
`Experiment 20`.
