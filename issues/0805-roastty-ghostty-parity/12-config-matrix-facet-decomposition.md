# Experiment 12: Config Matrix Facet Decomposition

## Description

The config matrix currently has 203 canonical option rows marked `Gap`. That was
appropriate after Experiment 6, when the rows represented broad option parity
and only name inventory had been proven. Experiments 8 through 11 have since
proven several global config facets:

- every pinned Ghostty canonical option name is represented in Roastty;
- every pinned Ghostty default config line formats identically after app-name
  normalization;
- every pinned Ghostty default config line is accepted by Roastty's per-line
  parser.

Keeping every canonical option row as a broad `Gap` now obscures what is proven
and what remains. Marking those rows `Pass` without changing their meaning would
be worse because it would overclaim full parser, diagnostic, precedence, reload,
runtime, and UI parity.

This experiment decomposes the matrix semantics: canonical option rows should
record the proven inventory/coverage facet only, while remaining unproven
configuration behavior is tracked by explicit facet rows. This makes later
experiments able to close the right gaps without hand-waving over per-option
runtime or diagnostic behavior.

## Changes

- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Rename/reword the 203 canonical option rows from broad option-parity rows to
    precise canonical option inventory rows.
  - Mark those rows `Pass` only for represented canonical option coverage,
    backed by the Experiment 6 inventory guard and the later default
    formatter/parser guards.
  - Keep compatibility alias rows and existing default formatter/parser rows.
  - Add explicit `Gap` rows for unproven config facets, at minimum:
    - non-default parser semantics across all canonical options;
    - non-default formatter behavior and order where relevant;
    - invalid-value diagnostics across all canonical options;
    - validation and finalization behavior;
    - config source precedence and repeated-file load semantics;
    - config reload behavior;
    - runtime/UI effects for options that affect app behavior.
- `issues/0805-roastty-ghostty-parity/config-inventory.md`
  - Update the classification notes so represented canonical option rows are no
    longer described as behavior gaps.
  - State that behavior gaps moved into explicit facet rows.
- `issues/0805-roastty-ghostty-parity/config_inventory.py`
  - Update the matrix row generator so future regeneration preserves the new
    inventory-row semantics instead of recreating stale broad `Gap` rows.
  - Keep the helper bounded to source-name inventory; do not make it generate
    false behavior claims.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning explaining why config rows are decomposed into inventory rows
    and behavior facet rows.
  - Update the Experiment 12 status after the result is known.

## Verification

Pass criteria:

- The config matrix still lists every one of the 203 pinned Ghostty canonical
  config options.
- Canonical option rows are `Pass` only for inventory/representation coverage,
  not for full behavior parity.
- Every remaining unproven config area is represented by an explicit `Gap` row
  rather than hidden in notes.
- The matrix still has zero missing canonical options and zero Roastty-only
  options according to the inventory helper.
- `python3 issues/0805-roastty-ghostty-parity/config_inventory.py --upstream vendor/ghostty/src/config/Config.zig --roastty roastty/src/config/mod.rs --output issues/0805-roastty-ghostty-parity/config-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  runs successfully and emits the current inventory counts.
- A small matrix-count check confirms:
  - 203 canonical inventory rows are `Pass`;
  - the compatibility alias rows remain `Pass`;
  - existing default formatter/parser rows remain `Pass`;
  - the new behavior facet rows remain `Gap`.
- The count/status check must assert row categories and fail nonzero on
  mismatch; printing raw totals is not sufficient.
- `prettier --write --prose-wrap always --print-width 80` has been run on the
  changed issue markdown files.
- `git diff --check` passes.

Suggested commands:

```bash
python3 issues/0805-roastty-ghostty-parity/config_inventory.py \
  --upstream vendor/ghostty/src/config/Config.zig \
  --roastty roastty/src/config/mod.rs \
  --output issues/0805-roastty-ghostty-parity/config-inventory.md \
  --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
python3 - <<'PY'
from pathlib import Path
rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text().splitlines():
    if not line.startswith('| CFG-'):
        continue
    cells = [cell.strip() for cell in line.strip('|').split('|')]
    rows.append(cells)

canonical_inventory = [
    row for row in rows
    if row[1].startswith('canonical option `')
]
assert len(canonical_inventory) == 203, len(canonical_inventory)
assert all(row[4] == 'Pass' for row in canonical_inventory)

aliases = [row for row in rows if row[1].startswith('compatibility alias `')]
assert len(aliases) == 8, len(aliases)
assert all(row[4] == 'Pass' for row in aliases)

required_pass = {
    'Baseline user config file path',
    'Default config format oracle, full surface',
    'Default `keybind` format and contents',
    'Default `command-palette-entry` format',
    'Default config parser oracle',
}
seen = {row[1]: row[4] for row in rows}
for name in required_pass:
    assert seen.get(name) == 'Pass', (name, seen.get(name))

required_gaps = {
    'Non-default parser semantics',
    'Non-default formatter behavior and order',
    'Invalid-value diagnostics',
    'Validation and finalization behavior',
    'Config source precedence and repeated-file load semantics',
    'Config reload behavior',
    'Runtime and UI effects',
}
for name in required_gaps:
    assert seen.get(name) == 'Gap', (name, seen.get(name))

print(f'rows={len(rows)} canonical_inventory={len(canonical_inventory)} aliases={len(aliases)} gaps={sum(row[4] == "Gap" for row in rows)}')
PY
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/12-config-matrix-facet-decomposition.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md \
  issues/0805-roastty-ghostty-parity/config-inventory.md
git diff --check
```

## Design Review

Fresh-context adversarial design review initially found two required issues:

- The proposed facet rows missed non-default formatter/order coverage and
  validation/finalization coverage, both required by Issue 805.
- The proposed matrix-count check printed totals instead of asserting row
  categories and statuses.

Fixes:

- Added explicit `Gap` facets for non-default formatter behavior/order and
  validation/finalization behavior.
- Replaced the loose count snippet with an assertion-based matrix check that
  verifies canonical inventory row count/status, alias count/status, required
  pass rows, and required gap rows.

Re-review approved the design:

```text
VERDICT: APPROVED

Findings: none Required.
```
