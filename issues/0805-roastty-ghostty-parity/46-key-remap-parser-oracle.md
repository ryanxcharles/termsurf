# Experiment 46: Key Remap Parser Oracle

## Description

CFG-217 still has 4 parser rows that are only `Audit covered`. One of those is
`key-remap`, which maps modifier keys before keybind matching and terminal input
encoding.

Pinned Ghostty's `RemapSet.parseCLI` treats a missing value as an empty value,
and an empty value resets the remap set. Non-empty input must contain the first
`=` assignment separator. The left side is a source modifier and the right side
is a target modifier. Supported modifier names include canonical names and
Ghostty aliases such as `control`, `command`, `cmd`, `opt`, and sided aliases
such as `right_option`. An unsided source expands into both left and right
source mappings. An unsided target defaults to the left side. `finalize` sorts
mappings so right-sided source mappings are considered before overlapping
generic expansions, which lets a later `right_ctrl=...` mapping override the
right half of an earlier `ctrl=...` expansion. Formatting emits one blank
`key-remap = ` entry for an empty set, or one formatted `from=to` entry per
stored mapping.

Roastty already has focused `RemapSet` tests for most of those semantics and
config-level tests for routing, formatting, reset, invalid values, and finalize.
This experiment will turn that coverage into the explicit CFG-217 parser oracle
for `key-remap`, extend it where needed for CLI/config-file diagnostics and
clone/equality behavior, wire the parser inventory to recognize the oracle, and
promote only the canonical `key-remap` row.

CFG-217 must remain `Gap` because `config-default-files`, `keybind`, and `theme`
will still be audit-only after this experiment.

## Changes

- `roastty/src/config/mod.rs`
  - Rename or wrap the config-level key-remap regression as
    `key_remap_config_parser_family_oracle`.
  - Extend the oracle if needed to cover:
    - config dispatch accepting missing values as an empty reset, matching
      pinned Ghostty `parseCLI`;
    - direct empty resets;
    - first-`=` splitting;
    - unsided source expansion to left and right mappings;
    - sided source mappings affecting only that side;
    - unsided target defaulting to the left side;
    - sided target preservation;
    - canonical and alias modifier names;
    - invalid missing-assignment and invalid-modifier diagnostics;
    - finalize ordering where right-sided mappings override generic expansions;
    - formatting of both empty and non-empty sets;
    - config-file diagnostics;
    - CLI parsing;
    - clone/equality semantics.
- `roastty/src/input/key_mods.rs`
  - Reuse existing `RemapSet` parser/finalize tests if they already prove the
    direct helper surface. Add focused cases only if the config-level oracle
    needs a missing facet.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Detect `key_remap_config_parser_family_oracle`.
  - Mark only canonical `key-remap` as `Oracle complete` when the oracle test is
    present.
  - Add the oracle to CFG-217 ownership so the generated matrix records
    `Experiment 46` when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 200 `Oracle complete`, 3
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 200 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting key-remap parser semantics after the result is
    proven.

## Verification

Pass criteria:

- Focused Roastty key-remap oracle passes:

```bash
cargo test --manifest-path roastty/Cargo.toml key_remap_config_parser_family_oracle
```

- Existing direct `RemapSet` tests still pass:

```bash
cargo test --manifest-path roastty/Cargo.toml key_remap_set
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=200`;
  - `audit_covered=3`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 200 rows are `Oracle complete`;
  - `key-remap` is `Oracle complete`;
  - the remaining `Audit covered` set is exactly `config-default-files`,
    `keybind`, and `theme`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 46`;
  - CFG-217 evidence points to `config-parser-inventory.md`.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run if any Rust file is
  edited.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  passes without leaving `__pycache__` or other generated artifacts.
- `git diff --check` passes.

Suggested commands:

```bash
cargo test --manifest-path roastty/Cargo.toml key_remap_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml key_remap_set
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
        cells = [cell.strip() for cell in line.strip('|').split('|')]
        rows.append(cells)

by_option = {row[1].strip('`'): row for row in rows}
audit = {row[1].strip('`') for row in rows if row[4] == 'Audit covered'}

assert len(rows) == 203, len(rows)
assert sum(row[4] == 'Oracle complete' for row in rows) == 200
assert by_option['key-remap'][4] == 'Oracle complete'
assert audit == {'config-default-files', 'keybind', 'theme'}, audit
assert not any(row[4] == 'Gap' for row in rows)

matrix = Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text()
cfg217 = next(line for line in matrix.splitlines() if line.startswith('| CFG-217 '))
cfg217_cells = [cell.strip() for cell in cfg217.strip('|').split('|')]
assert cfg217_cells[4] == 'Gap'
assert cfg217_cells[11] == 'Experiment 46'
assert 'config-parser-inventory.md' in cfg217_cells[6]
PY
cargo fmt --manifest-path roastty/Cargo.toml
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py
test -z "$(find issues/0805-roastty-ghostty-parity -name __pycache__ -prune -print)"
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/46-key-remap-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

**Verdict:** Approved.

Findings:

- Optional: the suggested `prettier` command omitted generated markdown files
  listed in Changes. Fixed by including `config-parser-inventory.md` and
  `config-matrix.md`.
- Nit: the suggested `find ... __pycache__` command reported artifacts but did
  not fail automatically. Fixed by wrapping it in a `test -z` assertion.

## Result

**Result:** Pass.

Implemented the key-remap parser oracle and promoted only the canonical
`key-remap` parser row.

Changes made:

- Renamed and expanded the config-level key-remap test to
  `key_remap_config_parser_family_oracle`.
- Covered pinned Ghostty's missing-value reset behavior for `key-remap`, direct
  empty reset behavior, invalid non-empty values, config-file diagnostics, CLI
  parsing, formatting, finalize ordering, alias modifiers, and clone equality.
- Added `key_remap_config_parser_family_oracle` detection to
  `config_parser_inventory.py`.
- Regenerated `config-parser-inventory.md` and `config-matrix.md`.
- Added the README learning that `key-remap` is a missing-value reset special
  case.

Verification performed:

```bash
cargo test --manifest-path roastty/Cargo.toml key_remap_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml key_remap_set
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
        cells = [cell.strip() for cell in line.strip('|').split('|')]
        rows.append(cells)

by_option = {row[1].strip('`'): row for row in rows}
audit = {row[1].strip('`') for row in rows if row[4] == 'Audit covered'}

assert len(rows) == 203, len(rows)
assert sum(row[4] == 'Oracle complete' for row in rows) == 200
assert by_option['key-remap'][4] == 'Oracle complete'
assert audit == {'config-default-files', 'keybind', 'theme'}, audit
assert not any(row[4] == 'Gap' for row in rows)

matrix = Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text()
cfg217 = next(line for line in matrix.splitlines() if line.startswith('| CFG-217 '))
cfg217_cells = [cell.strip() for cell in cfg217.strip('|').split('|')]
assert cfg217_cells[4] == 'Gap'
assert cfg217_cells[11] == 'Experiment 46'
assert 'config-parser-inventory.md' in cfg217_cells[6]
PY
cargo fmt --manifest-path roastty/Cargo.toml --check
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
test -z "$(find issues/0805-roastty-ghostty-parity -name __pycache__ -prune -print)"
git diff --check
```

The parser inventory reports:

```text
ghostty_canonical=203
roastty_parser_rows=203
missing_canonical_parser_rows=0
missing_dispatch_rows=0
extra_parser_rows=0
compatibility_only_parser_arms=5
noncanonical_noncompat_parser_arms=0
oracle_complete=200
audit_covered=3
gap=0
```

`cargo test --manifest-path roastty/Cargo.toml key_remap_set` passed 9 tests.
`cargo test --manifest-path roastty/Cargo.toml key_remap_config_parser_family_oracle`
passed the focused config oracle. The first run of the config oracle exposed
that finalized left-side mapping order is insertion-preserving after right-side
mappings are sorted first; the oracle was corrected to that pinned behavior.

## Conclusion

`key-remap` parser parity is now Oracle complete for CFG-217. The remaining
audit-only parser rows are `config-default-files`, `keybind`, and `theme`, so
CFG-217 correctly remains `Gap` at 200 Oracle complete rows, 3 Audit covered
rows, and 0 dispatch gaps.

## Completion Review

Reviewed by fresh-context Codex adversarial subagents.

Initial completion review verdict: Changes required.

Findings:

- Required: the recorded matrix assertion used an exact `| Gap |` substring and
  failed against padded Markdown table cells. Fixed by parsing the CFG-217 row
  into stripped cells and comparing the status, owner, and evidence cells.
- Optional: bare `--key-remap` reset behavior was documented but not directly
  asserted by the oracle. Fixed by adding a `set_cli_args(["--key-remap"])`
  assertion that clears the remap set.
- The reviewer accidentally created
  `issues/0805-roastty-ghostty-parity/__pycache__/`. Removed the artifact and
  verified no `__pycache__` remains.

Re-review verdict: Approved.

The re-review confirmed the assertion fix, the bare CLI reset assertion, and the
absence of `__pycache__`; it also reran the focused config oracle and
`git diff --check` successfully.
