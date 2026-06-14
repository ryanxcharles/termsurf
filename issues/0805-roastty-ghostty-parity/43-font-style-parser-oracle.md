# Experiment 43: Font Style Parser Oracle

## Description

CFG-217 still has 14 parser rows that are only `Audit covered`. Four of those
rows are the same pinned Ghostty `FontStyle.parseCLI` surface: `font-style`,
`font-style-bold`, `font-style-italic`, and `font-style-bold-italic`.

Pinned Ghostty's `FontStyle.parseCLI` requires a value, maps exact `default` to
the default variant, maps exact `false` to the disabled-style variant, and maps
every other supplied value to a named style without validation or trimming. That
includes an explicitly empty parser input, which becomes an empty style name at
the direct parser boundary; the `Config::set` empty-reset behavior is a separate
required-field dispatch concern. Formatting emits `default`, `false`, or the
stored name. `nameValue` returns `null` for `default` and `false` and returns
the stored style name only for named styles.

Roastty already has focused `FontStyle` parser/formatter coverage. This
experiment will make that coverage an explicit CFG-217 oracle, extend it where
needed for config-field dispatch across all four canonical `font-style*` fields,
wire the parser inventory to recognize the oracle, and promote only those four
rows.

CFG-217 must remain `Gap` because other parser helpers are still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Rename or wrap the existing `FontStyle` regression as
    `font_style_config_parser_family_oracle`.
  - Extend the oracle if needed to cover:
    - direct missing value returning `ValueRequired`;
    - direct exact `default` and `false` tokens;
    - direct arbitrary named styles, including empty strings, uppercase tokens,
      whitespace-padded names, and punctuation;
    - formatter output for `default`, `false`, and named styles;
    - `enabled()` and `name_value()` helper semantics;
    - config dispatch for all four `font-style*` fields;
    - set-but-empty config reset to the default value;
    - missing field diagnostics;
    - CLI parsing, formatting, and clone semantics.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Detect `font_style_config_parser_family_oracle`.
  - Mark only canonical `font-style`, `font-style-bold`, `font-style-italic`,
    and `font-style-bold-italic` as `Oracle complete` when the oracle test is
    present.
  - Add the oracle to CFG-217 ownership so the generated matrix records
    `Experiment 43` when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 193 `Oracle complete`, 10
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 193 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting `FontStyle` parser semantics after the result is
    proven.

## Verification

Pass criteria:

- Focused Roastty font-style oracle passes:

```bash
cargo test --manifest-path roastty/Cargo.toml font_style_config_parser_family_oracle
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=193`;
  - `audit_covered=10`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 193 rows are `Oracle complete`;
  - the four canonical `font-style*` rows are `Oracle complete`;
  - the remaining `Audit covered` set is exactly the pre-existing audit-only set
    minus those four rows;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 43`;
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
cargo test --manifest-path roastty/Cargo.toml font_style_config_parser_family_oracle
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
assert sum(row[4] == 'Oracle complete' for row in rows) == 193
assert sum(row[4] == 'Audit covered' for row in rows) == 10
assert not [row for row in rows if row[4] == 'Gap']
expected_audit = {
    '`clipboard-codepoint-map`',
    '`config-default-files`',
    '`font-codepoint-map`',
    '`font-variation`',
    '`font-variation-bold`',
    '`font-variation-bold-italic`',
    '`font-variation-italic`',
    '`key-remap`',
    '`keybind`',
    '`theme`',
}
actual_audit = {row[1] for row in rows if row[4] == 'Audit covered'}
assert actual_audit == expected_audit, sorted(actual_audit ^ expected_audit)
for option in {
    '`font-style`',
    '`font-style-bold`',
    '`font-style-italic`',
    '`font-style-bold-italic`',
}:
    row = next(row for row in rows if row[1] == option)
    assert row[4] == 'Oracle complete', row

cfg217 = None
for line in Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text().splitlines():
    if line.startswith('| CFG-217 |'):
        cfg217 = [cell.strip() for cell in line.strip('|').split('|')]
        break
assert cfg217 is not None
assert cfg217[4] == 'Gap', cfg217
assert cfg217[11] == 'Experiment 43', cfg217
assert '193 parser rows Oracle complete' in cfg217[12], cfg217
print('font_style_rows=4 oracle_complete=193 cfg217=Gap')
PY
cargo fmt --manifest-path roastty/Cargo.toml
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/43-font-style-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Reviewed by an adversarial Codex subagent with fresh context.

**Verdict:** Approved.

Findings: none.
