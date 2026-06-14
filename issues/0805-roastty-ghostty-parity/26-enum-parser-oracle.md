# Experiment 26: Enum Parser Oracle

## Description

CFG-217 still has 115 parser rows that are only `Audit covered`. The next
bounded shared parser family is the 52-row enum family.

Most enum-family rows dispatch through one of two helpers:

- `set_enum_field(value, default, Enum::from_keyword)` for required enum fields;
- `set_optional_enum_field(value, default, Enum::from_keyword)` for optional
  enum fields.

Three canonical enum rows also have pinned Ghostty compatibility branches before
or inside the enum parse path:

- `macos-dock-drop-behavior = window` maps to `new-window`;
- `gtk-single-instance = desktop` maps to `detect`;
- `gtk-tabs-location = hidden` sets `window-show-tab-bar = never` and leaves
  `gtk-tabs-location` unchanged.

Pinned Ghostty's enum parser boundary is the Zig type-magic enum shape:
`std.meta.stringToEnum` accepts exact enum tag strings only, missing values are
`ValueRequired`, invalid values are `InvalidValue`, and raw empty option values
reset to the field default before child parsing. Optional enum fields use the
same child enum parser but wrap successful values in `Some(...)`. The
compatibility branches above are part of the enum-family parser surface because
the canonical keys are still classified as enum rows.

Roastty already has focused `from_keyword` tests that round-trip all enum types
used by the config rows and reject representative invalid spellings. This
experiment will add the missing config-option boundary oracle for required and
optional enum fields, keep the existing keyword tests as part of the
verification surface, include the three compatibility enum branches, and promote
the 52 enum rows to `Oracle complete`.

This experiment is limited to parser, formatter, reset, compatibility, and
diagnostic semantics for enum config rows. Runtime effects of the enum choices
remain separate parity facets.

CFG-217 must remain `Gap` because other parser families are still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused enum parser family oracle test covering:
    - required enum dispatch accepts exact keywords and updates formatted config
      output;
    - optional enum dispatch accepts exact keywords and stores `Some(...)`;
    - missing values are `ValueRequired` for required and optional enum fields;
    - raw empty values reset required and optional fields to their defaults;
    - invalid values, bool-like numeric strings, uppercase strings, snake-case
      aliases, and whitespace-padded values are rejected as `InvalidValue`;
    - compatibility values for `macos-dock-drop-behavior`,
      `gtk-single-instance`, and `gtk-tabs-location`, including the
      `gtk-tabs-location = hidden` side-effect on `window-show-tab-bar`;
    - load-string diagnostics preserve valid earlier enum values while reporting
      invalid later lines.
  - Keep the existing enum keyword and formatter tests in the verification set
    so each concrete enum type remains covered by exact keyword round-trips.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Mark enum parser rows as `Oracle complete` when the enum family oracle test
    is present, with evidence that includes both direct enum helper behavior and
    the three pinned compatibility enum branches.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 140 `Oracle complete`, 63
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 140 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting enum parser semantics after the result is proven.

## Verification

Pass criteria:

- Focused Roastty tests pass:

```bash
cargo test --manifest-path roastty/Cargo.toml enum_config_parser_family_oracle
```

- Existing enum keyword and formatter regression tests still pass:

```bash
cargo test --manifest-path roastty/Cargo.toml enum_from_keyword_round_trips
cargo test --manifest-path roastty/Cargo.toml enum_format_entries
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=140`;
  - `audit_covered=63`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 140 rows are `Oracle complete`;
  - all 52 enum rows are `Oracle complete`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 26`;
  - CFG-217 evidence points to `config-parser-inventory.md`.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `git diff --check` passes.

Suggested commands:

```bash
cargo test --manifest-path roastty/Cargo.toml enum_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml enum_from_keyword_round_trips
cargo test --manifest-path roastty/Cargo.toml enum_format_entries
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
assert cfg217[11] == 'Experiment 26', cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
enum_rows = [row for row in parser_rows if row[3] == 'enum']
assert len(enum_rows) == 52, len(enum_rows)
assert all(row[4] == 'Oracle complete' for row in enum_rows)
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 140
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} enum_oracle={len(enum_rows)} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/26-enum-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Fresh-context adversarial design review found one required issue:

- The draft overclaimed that every enum-family row is a plain
  `Enum::from_keyword` helper path. Three canonical enum rows have pinned
  Ghostty compatibility behavior: `macos-dock-drop-behavior = window`,
  `gtk-single-instance = desktop`, and `gtk-tabs-location = hidden`.

Fix:

- Updated the design to classify most rows as direct enum helpers, explicitly
  list the three compatibility branches, require the oracle to cover those
  branches and the `gtk-tabs-location = hidden` side effect, and update the
  generator evidence language.

Re-review approved the fixed design:

```text
VERDICT: APPROVED

No Required findings remain.
```

## Result

**Result:** Pass

Roastty now has a focused enum parser family oracle for the 52 enum parser rows.
The oracle proves the shared required and optional enum option boundary plus the
pinned compatibility branches:

- required enum dispatch accepts exact keywords and updates formatted config
  output;
- optional enum dispatch accepts exact keywords and stores `Some(...)`;
- missing values are `ValueRequired` for required and optional enum fields;
- raw empty values reset required and optional fields to their defaults;
- invalid values, bool-like numeric strings, uppercase strings, snake-case
  aliases, and whitespace-padded values are rejected as `InvalidValue`;
- `macos-dock-drop-behavior = window` maps to `new-window`;
- `gtk-single-instance = desktop` maps to `detect`;
- `gtk-tabs-location = hidden` sets `window-show-tab-bar = never` while leaving
  `gtk-tabs-location` unchanged;
- load-string diagnostics preserve valid earlier enum values while reporting
  invalid later lines.

Focused Roastty verification passed:

```bash
cargo test --manifest-path roastty/Cargo.toml enum_config_parser_family_oracle
```

Output summary:

```text
running 1 test
test config::tests::enum_config_parser_family_oracle ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4916 filtered out; finished in 0.01s
```

Existing enum keyword and formatter regression tests also passed:

```bash
cargo test --manifest-path roastty/Cargo.toml enum_from_keyword_round_trips
cargo test --manifest-path roastty/Cargo.toml enum_format_entries
```

Output summaries:

```text
running 4 tests
test config::tests::enum_from_keyword_round_trips_shell_notify ... ok
test config::tests::enum_from_keyword_round_trips_and_rejects_unknown ... ok
test config::tests::enum_from_keyword_round_trips_mac_bgimage_shader ... ok
test config::tests::enum_from_keyword_round_trips_misc_fullscreen ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 4913 filtered out; finished in 0.00s
```

```text
running 7 tests
test config::tests::enum_format_entries_misc ... ok
test config::tests::enum_format_entries ... ok
test config::tests::enum_format_entries_shader_mouse ... ok
test config::tests::enum_format_entries_2 ... ok
test config::tests::enum_format_entries_mac ... ok
test config::tests::enum_format_entries_fullscreen ... ok
test config::tests::enum_format_entries_bgimage ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 4910 filtered out; finished in 0.00s
```

The parser inventory generator passed and moved all 52 enum rows to
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
oracle_complete=140
audit_covered=63
gap=0
```

Matrix assertion output:

```text
parser_rows=203 enum_oracle=52 cfg217=Gap
```

## Conclusion

The enum parser family is now `Oracle complete`. CFG-217 remains `Gap` because
63 parser rows are still audit-covered only. The next parser-family experiment
should continue reducing that count with another bounded family.

## Completion Review

Fresh-context adversarial completion review approved the result with no
findings.
