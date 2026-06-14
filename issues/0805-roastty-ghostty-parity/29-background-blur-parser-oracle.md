# Experiment 29: Background Blur Parser Oracle

## Description

CFG-217 still has 34 parser rows that are only `Audit covered`. The next bounded
row is canonical `background-blur`, whose parser path is
`BackgroundBlur::parse_cli`.

Pinned Ghostty models `background-blur` as a tagged union in
`vendor/ghostty/src/config/Config.zig`:

- a missing CLI/config value acts like a boolean flag and sets `.true`;
- normal bool spellings parse first, so `1` is `.true` and `0` is `.false`, not
  numeric radii;
- exact void-union tags `macos-glass-regular` and `macos-glass-clear` are
  accepted;
- anything else is parsed as `std.fmt.parseInt(u8, input, 0)` and stored as
  `.radius`;
- malformed values and over-wide `u8` values are `InvalidValue`;
- formatting emits `false`, `true`, the radius integer, or the exact glass
  keyword.

Roastty already has lower-level `BackgroundBlur::parse_cli` and formatter tests,
plus broader config-route tests. This experiment will add a focused parser
family oracle named for CFG-217 inventory promotion, keep the lower-level parser
test in the verification set, and promote only the canonical row
`background-blur`.

The compatibility alias `background-blur-radius` remains a compatibility-only
parser arm, not a canonical CFG-217 row. Existing compatibility-alias coverage
already tracks that alias separately under CFG-206.

This experiment is limited to parser, formatter, reset/default, diagnostics,
CLI, and clone semantics. Runtime blur rendering and platform-specific blur
effects remain separate parity facets.

CFG-217 must remain `Gap` because other parser helpers are still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused `background_blur_config_parser_family_oracle` test covering:
    - missing/bare flag sets `BackgroundBlur::True`;
    - all upstream bool spellings accepted by `parse_bool`, including `1` and
      `0`, and their formatter output;
    - exact glass keywords and formatter output;
    - base-0 `u8` radius values, including decimal, lowercase/uppercase hex,
      binary/octal, signs, interior underscores, and range boundaries;
    - parser order where `1`/`0` are bools, while real non-bool numbers are
      radii;
    - raw empty option values reset through config dispatch to the default
      `BackgroundBlur::False`;
    - invalid empty direct parser input, invalid keywords, malformed prefixes,
      malformed separators, negative nonzero values, and overflow are
      `InvalidValue`;
    - `load_str` diagnostics preserve earlier valid values while reporting
      invalid later lines;
    - CLI argument parsing reaches the same helper;
    - cloned configs retain parsed background blur values.
  - Keep the existing lower-level
    `background_blur_parse_cli_resolves_bool_glass_and_radius` test in the
    verification set.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Mark only the canonical `background-blur` parser row as `Oracle complete`
    when the background blur oracle test is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 170 `Oracle complete`, 33
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 170 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting background blur parser semantics after the result
    is proven.

## Verification

Pass criteria:

- Focused Roastty tests pass:

```bash
cargo test --manifest-path roastty/Cargo.toml background_blur_config_parser_family_oracle
```

- Existing lower-level and broader config-route tests still pass:

```bash
cargo test --manifest-path roastty/Cargo.toml background_blur_parse_cli_resolves_bool_glass_and_radius
cargo test --manifest-path roastty/Cargo.toml cursor_style_config_keywords_parse_format_and_diagnose
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=170`;
  - `audit_covered=33`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 170 rows are `Oracle complete`;
  - the `background-blur` row is `Oracle complete`;
  - the `background-blur-radius` compatibility arm is not a canonical `PARSE-`
    row;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 29`;
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
cargo test --manifest-path roastty/Cargo.toml background_blur_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml background_blur_parse_cli_resolves_bool_glass_and_radius
cargo test --manifest-path roastty/Cargo.toml cursor_style_config_keywords_parse_format_and_diagnose
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
assert cfg217[11] == 'Experiment 29', cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
background_blur = [row for row in parser_rows if row[1] == '`background-blur`']
assert len(background_blur) == 1, background_blur
assert background_blur[0][4] == 'Oracle complete', background_blur[0]
assert all(row[1] != '`background-blur-radius`' for row in parser_rows)
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 170
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} background_blur={background_blur[0][4]} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/29-background-blur-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Adversarial subagent `019ec405-5f01-7f72-898f-92171384dfee` reviewed the design
and returned `VERDICT: APPROVED`.

- **Optional:** The pass criteria required `py_compile`, which can leave an
  `__pycache__` directory, while cleanup appeared only in the suggested command
  block.
- **Fix:** Added an explicit pass criterion that no `__pycache__` or other
  `py_compile` artifacts remain in the issue folder.

## Result

**Result:** Pass

Implemented the background blur parser oracle and promoted the canonical
`background-blur` row to `Oracle complete`.

Changes made:

- `roastty/src/config/mod.rs`
  - Added `background_blur_config_parser_family_oracle`.
  - Covered bare/missing true, bool-first parsing, exact glass keywords, base-0
    `u8` radii, raw-empty reset, invalid values, diagnostics, CLI parsing,
    formatting, and clone behavior.
  - Verified the shared `parse_uint` helper follows Zig's underscore behavior:
    leading/trailing underscores are rejected after prefix handling, while
    interior underscores, including doubled interior underscores such as `1__0`,
    are skipped.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Added the background blur oracle marker and Experiment 29 ownership.
  - Promotes only canonical `background-blur`, leaving compatibility-only
    `background-blur-radius` outside the canonical parser rows.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerated with 170 `Oracle complete`, 33 `Audit covered`, and 0 `Gap`
    rows.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerated CFG-217 with Experiment 29 as owner and the updated parser
    counts.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Added the background blur learning and updated this experiment to `Pass`.

Verification run:

```bash
cargo test --manifest-path roastty/Cargo.toml background_blur_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml background_blur_parse_cli_resolves_bool_glass_and_radius
cargo test --manifest-path roastty/Cargo.toml cursor_style_config_keywords_parse_format_and_diagnose
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
assert cfg217[11] == 'Experiment 29', cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
background_blur = [row for row in parser_rows if row[1] == '`background-blur`']
assert len(background_blur) == 1, background_blur
assert background_blur[0][4] == 'Oracle complete', background_blur[0]
assert all(row[1] != '`background-blur-radius`' for row in parser_rows)
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 170
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} background_blur={background_blur[0][4]} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/29-background-blur-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

Observed verification output:

- `background_blur_config_parser_family_oracle`: passed.
- `background_blur_parse_cli_resolves_bool_glass_and_radius`: passed.
- `cursor_style_config_keywords_parse_format_and_diagnose`: passed.
- `integer_config_parser_family_oracle`: passed, preserving the existing shared
  integer parser oracle.
- Parser generator:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=170`;
  - `audit_covered=33`;
  - `gap=0`.
- Matrix assertion:
  - `parser_rows=203`;
  - `background_blur=Oracle complete`;
  - `cfg217=Gap`.
- `python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py`:
  passed, and the generated `__pycache__` directory was removed.
- `git diff --check`: passed.

## Conclusion

The canonical `background-blur` parser row now has a durable Tier 1 oracle.
Roastty matches pinned Ghostty's parser boundary for bool-first parsing, exact
glass keywords, base-0 `u8` radii, raw-empty reset through config dispatch,
invalid direct empty input, diagnostics, CLI, and formatting.

This experiment also confirmed the shared integer helper's Zig-compatible
underscore boundary: leading/trailing underscore forms are rejected after prefix
handling, while interior underscores are skipped. CFG-217 remains `Gap` because
33 parser rows are still only `Audit covered`. The next experiment should
continue with another bounded parser row or family from those remaining rows.

## Completion Review

Adversarial subagent `019ec405-5f01-7f72-898f-92171384dfee` reviewed the
completed experiment and initially returned `CHANGES REQUIRED`.

- **Required:** The first result incorrectly changed `parse_uint` and the new
  oracle to reject doubled interior underscores, but pinned Ghostty's
  `std.fmt.parseInt(u8, input, 0)` path accepts interior underscores by skipping
  them.
- **Fix:** Restored `parse_uint` to the Zig-compatible boundary and updated the
  oracle to accept `1__0` as radius `10`.
- **Required:** The diagnostics case claimed invalid later lines preserve an
  earlier valid value, but a later raw-empty reset masked whether the invalid
  line clobbered state.
- **Fix:** Added a separate retained-value diagnostics case with
  `background-blur = 7` followed by invalid `background-blur = nope`, and
  asserted the value remains `BackgroundBlur::Radius(7)`.

The same adversarial subagent re-reviewed only those fixes and returned
`VERDICT: APPROVED`, confirming both Required findings were resolved and no new
Required findings were introduced.
