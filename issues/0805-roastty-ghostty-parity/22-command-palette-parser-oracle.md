# Experiment 22: Command Palette Parser Oracle

## Description

CFG-217 still has 128 parser rows that are only `Audit covered`. The next
bounded family is the 1-row command-palette parser family:

- `command-palette-entry`.

Pinned Ghostty implements this option as `RepeatableCommand.parseCLI`. At the
config-option boundary, a missing or raw empty value restores the default
command list, exact `clear` empties the list, and all other values are parsed as
an auto-struct `input.Command` with `title`, optional `description`, and
required `action` fields. Quoted values may contain commas, duplicate fields are
accepted with the last value winning, unknown fields and invalid actions are
invalid, and formatter output emits repeated `command-palette-entry` lines.

Roastty already has `RepeatableCommand`, `CommandPaletteEntry`, and broad
option-level tests for default entries, parse, format, reset, diagnostics, and
cloning. This experiment will add one focused family oracle that ties those
semantics to the parser-facet inventory, then promote the 1 command-palette row
to `Oracle complete`.

This experiment is limited to parser, reset, formatter, and diagnostic
semantics. Runtime command-palette UI behavior, command execution, and app menu
integration remain separate feature/runtime facets.

CFG-217 must remain `Gap` because other parser families are still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Add a focused command-palette parser family test covering:
    - default list presence and default actions being canonical;
    - exact `clear` clearing all entries;
    - raw empty and missing values restoring the default list;
    - appending valid entries after clear;
    - required `title` and `action` fields;
    - optional `description`;
    - whitespace around keys and values;
    - quoted values containing commas;
    - duplicate fields with last value winning;
    - action canonicalization, such as `copy_to_clipboard` becoming
      `copy_to_clipboard:mixed`;
    - quoted string escape decoding for text payloads;
    - formatter output for empty, single, described, and multiple entries;
    - invalid values: title-only, action-only, unknown action, unknown field,
      unterminated quote, and invalid escape.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Mark command-palette parser rows as `Oracle complete` when the
    command-palette family oracle test is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 76 `Oracle complete`, 127
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 76 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting command-palette parser semantics after the result
    is proven.

## Verification

Pass criteria:

- Focused Roastty tests pass:

```bash
cargo test --manifest-path roastty/Cargo.toml command_palette_config_parser_family_oracle
```

- Existing command-palette option-level regression test still passes:

```bash
cargo test --manifest-path roastty/Cargo.toml command_palette_entry_config_parse_format_reset_and_diagnose
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=76`;
  - `audit_covered=127`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 76 rows are `Oracle complete`;
  - the single command-palette row is `Oracle complete`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 22`;
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
cargo test --manifest-path roastty/Cargo.toml command_palette_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml command_palette_entry_config_parse_format_reset_and_diagnose
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
assert cfg217[11] == 'Experiment 22', cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
command_palette_rows = [row for row in parser_rows if row[3] == 'command palette']
assert len(command_palette_rows) == 1, len(command_palette_rows)
assert command_palette_rows[0][4] == 'Oracle complete'
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 76
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} command_palette_oracle={len(command_palette_rows)} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/22-command-palette-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Result

**Result:** Pass

Roastty now has a focused command-palette parser family oracle for the single
`command-palette-entry` parser row. The oracle proves pinned Ghostty's
`RepeatableCommand.parseCLI` boundary:

- the default command list is present and stores canonical actions;
- exact `clear` empties the list;
- raw empty and missing values restore the default list;
- valid entries append after clear;
- `title` and `action` are required, while `description` is optional;
- whitespace around auto-struct keys and values is accepted;
- quoted values can contain commas;
- duplicate fields are accepted with the last value winning;
- action aliases are canonicalized, such as `copy_to_clipboard` becoming
  `copy_to_clipboard:mixed`;
- quoted string escapes are decoded before action parsing;
- formatter output is proven for empty, single, described, and multiple entries;
- invalid title-only, action-only, unknown-action, unknown-field, unterminated
  quote, and invalid-escape values are rejected.

Focused Roastty verification passed:

```bash
cargo test --manifest-path roastty/Cargo.toml command_palette_config_parser_family_oracle
```

Output summary:

```text
running 1 test
test config::tests::command_palette_config_parser_family_oracle ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4912 filtered out; finished in 0.01s
```

The existing command-palette option-level regression test also passed:

```bash
cargo test --manifest-path roastty/Cargo.toml command_palette_entry_config_parse_format_reset_and_diagnose
```

Output summary:

```text
running 1 test
test config::tests::command_palette_entry_config_parse_format_reset_and_diagnose ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4912 filtered out; finished in 0.00s
```

The parser inventory generator passed and moved the 1 command-palette row to
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
oracle_complete=76
audit_covered=127
gap=0
```

The matrix assertion passed:

```text
parser_rows=203 command_palette_oracle=1 cfg217=Gap
```

CFG-217 remains `Gap` because 127 parser rows are still audit-only, but the
command-palette parser family is now oracle-complete.

## Conclusion

Command-palette parser semantics are now proven for the pinned Ghostty target at
the direct parser boundary. The key shape is repeatable auto-struct parsing:
missing/raw-empty restores defaults, exact `clear` empties entries, valid
auto-struct commands append, and the formatter emits one `command-palette-entry`
line per stored command.

## Completion Review

Fresh-context adversarial completion review approved the result with no
findings. The reviewer independently verified the focused command-palette
oracle, the existing command-palette regression test, Rust fmt check,
`git diff --check`, and the matrix assertion: 203 parser rows, exactly 1
command-palette row, 76 `Oracle complete`, CFG-217 still `Gap`, and owner
`Experiment 22`.
