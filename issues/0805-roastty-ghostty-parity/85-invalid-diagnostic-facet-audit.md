# Experiment 85: Invalid diagnostic facet audit

## Description

CFG-219 is still a broad gap: Roastty has many focused tests that assert
`ConfigDiagnostic` output for invalid config values, but Issue 805 does not yet
have an inventory proving diagnostic parity across every pinned Ghostty
canonical config option.

This experiment will build the diagnostic audit surface before trying to close
CFG-219. The goal is to map every canonical Ghostty option to its current
diagnostic evidence, classify the diagnostic behavior that still needs proof,
and keep CFG-219 as `Gap` unless every row has concrete invalid-value diagnostic
coverage. The expected outcome is a generated inventory and a matrix consistency
guard, not a broad sample-based pass.

## Changes

- `issues/0805-roastty-ghostty-parity/config_diagnostic_inventory.py`
  - Add a bounded inventory generator for CFG-219.
  - Reuse the canonical option list from `config_inventory.py`.
  - Reuse parser-family and parser-path context from
    `config-parser-inventory.md` so diagnostic rows stay aligned with CFG-217.
  - Emit one diagnostic row per canonical option.
  - Classify each row by the diagnostic surface that must be proven:
    - config-file `load_str` line/key/error diagnostics for invalid values;
    - CLI argument position/key/error diagnostics where relevant;
    - missing-value diagnostics where the option accepts bare keys or missing
      CLI values;
    - unknown-field diagnostics for truly noncanonical keys;
    - not-implemented diagnostics for canonical options such as `link`;
    - state-retention behavior after invalid values for stateful or repeatable
      options.
  - Mark rows as `Audit covered` when the diagnostic surface and existing
    evidence are identified.
  - Mark rows as `Oracle complete` only when existing or new evidence proves the
    relevant diagnostic error kind, line/key or CLI position behavior, and
    state-retention behavior where applicable.
  - Mark rows as `Gap` when the generator cannot identify required diagnostic
    evidence for a canonical option.

- `issues/0805-roastty-ghostty-parity/config-diagnostic-inventory.md`
  - Record generated diagnostic rows, counts by diagnostic family, covered rows,
    and gap rows.

- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Update CFG-219 to point at `config-diagnostic-inventory.md`.
  - Keep CFG-219 as `Gap` unless every diagnostic row is `Oracle complete`.
  - Include counts in the CFG-219 note so future experiments can verify progress
    without rereading the full inventory.

- `issues/0805-roastty-ghostty-parity/README.md`
  - Link this experiment as `Designed`.
  - Add a learning only if the audit discovers a reusable diagnostic-proof rule
    or a concrete diagnostic mismatch.

## Verification

Pass criteria:

- The diagnostic inventory generator exits successfully and reports:
  - `ghostty_canonical=203`;
  - `diagnostic_rows=203`;
  - no missing canonical diagnostic rows;
  - no extra diagnostic rows outside the canonical inventory.
- Every generated diagnostic row names:
  - the canonical config option;
  - the parser family/path from the parser inventory;
  - the diagnostic family or behavior that must be proven;
  - current coverage status;
  - evidence artifact or concrete missing evidence.
- A matrix assertion verifies that CFG-219 is internally consistent:
  - if every diagnostic inventory row is `Oracle complete`, CFG-219 may be
    `Pass`;
  - if any diagnostic inventory row is `Audit covered` or `Gap`, CFG-219 remains
    `Gap`;
  - CFG-219 points to `config-diagnostic-inventory.md`;
  - CFG-219 notes the current `Oracle complete`, incomplete, and gap counts.
- The generator must not disturb CFG-217 or CFG-218. Capture both full matrix
  rows before running the generator and assert they are byte-for-byte unchanged
  after generation.
- Any new Rust tests pass with the narrowest relevant `cargo test` filter.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `git diff --check` passes.

Suggested commands:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 - <<'PY'
from pathlib import Path

matrix_text = Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text()
protected_rows = [
    line for line in matrix_text.splitlines()
    if line.startswith('| CFG-217 |') or line.startswith('| CFG-218 |')
]
assert len(protected_rows) == 2, protected_rows
Path('/tmp/issue805-exp85-cfg217-218-before.txt').write_text(
    '\n'.join(protected_rows) + '\n'
)
PY
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_diagnostic_inventory.py \
  --config-inventory issues/0805-roastty-ghostty-parity/config-inventory.md \
  --parser-inventory issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  --roastty roastty/src/config/mod.rs \
  --output issues/0805-roastty-ghostty-parity/config-diagnostic-inventory.md \
  --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
PYTHONDONTWRITEBYTECODE=1 python3 - <<'PY'
from pathlib import Path

protected_before = Path('/tmp/issue805-exp85-cfg217-218-before.txt').read_text()
matrix_text = Path('issues/0805-roastty-ghostty-parity/config-matrix.md').read_text()
protected_after = [
    line for line in matrix_text.splitlines()
    if line.startswith('| CFG-217 |') or line.startswith('| CFG-218 |')
]
assert protected_before == '\n'.join(protected_after) + '\n'

matrix_rows = []
for line in matrix_text.splitlines():
    if line.startswith('| CFG-'):
        matrix_rows.append([cell.strip() for cell in line.strip('|').split('|')])

cfg217 = next(row for row in matrix_rows if row[0] == 'CFG-217')
cfg218 = next(row for row in matrix_rows if row[0] == 'CFG-218')
cfg219 = next(row for row in matrix_rows if row[0] == 'CFG-219')
assert cfg217[4] == 'Pass', cfg217
assert cfg218[4] == 'Pass', cfg218
assert 'config-diagnostic-inventory.md' in cfg219[6], cfg219

diagnostic_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-diagnostic-inventory.md').read_text().splitlines():
    if line.startswith('| DIAG-'):
        diagnostic_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(diagnostic_rows) == 203, len(diagnostic_rows)
incomplete = [row for row in diagnostic_rows if row[5] != 'Oracle complete']
oracle_complete = sum(row[5] == 'Oracle complete' for row in diagnostic_rows)
gap_count = sum(row[5] == 'Gap' for row in diagnostic_rows)
assert (not incomplete and cfg219[4] == 'Pass') or (incomplete and cfg219[4] == 'Gap')
assert f'{oracle_complete} rows Oracle complete' in cfg219[12], cfg219
assert f'{len(incomplete)} rows are not Oracle complete' in cfg219[12], cfg219
assert f'{gap_count} rows are diagnostic gaps' in cfg219[12], cfg219
print(f'diagnostic_rows={len(diagnostic_rows)} incomplete={len(incomplete)} cfg219={cfg219[4]}')
PY
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_diagnostic_inventory.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/85-invalid-diagnostic-facet-audit.md
prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/85-invalid-diagnostic-facet-audit.md
git diff --check
```

## Design Review

Adversarial reviewer: Codex subagent with fresh context.

Initial verdict: Changes required.

Required findings:

- CFG-217 and CFG-218 protection was not concrete enough because the draft only
  asserted those rows stayed `Pass`; it would not catch evidence, owner, guard
  command, or note churn.
- The verification required CFG-219 to note `Oracle complete`, incomplete, and
  gap counts, but the suggested assertion did not check that note text.

Nit:

- The pass criteria required `prettier --write`, but the suggested command only
  ran `prettier --check`.

Fixes:

- Added a byte-for-byte before/after assertion for the full CFG-217 and CFG-218
  matrix rows.
- Added explicit CFG-219 note assertions for `Oracle complete`, incomplete, and
  diagnostic gap counts.
- Added `prettier --write --prose-wrap always --print-width 80` before the
  Prettier check command.

Final verdict: Approved.

Re-review confirmed all prior findings are resolved.

## Result

**Result:** Pass

The diagnostic inventory generator now emits one row for each of the 203 pinned
Ghostty canonical config options and updates CFG-219 from the generated counts.
The generated inventory found 122 rows with existing diagnostic-specific oracle
evidence, 81 rows that are only audit-covered, and 0 rows that are structural
diagnostic gaps.

CFG-219 remains `Gap`, as intended, because not every diagnostic row is
`Oracle complete`. CFG-217 and CFG-218 were captured before generation and
asserted byte-for-byte unchanged after generation.

Verification output:

```text
ghostty_canonical=203
diagnostic_rows=203
missing_canonical_diagnostic_rows=0
extra_diagnostic_rows=0
oracle_complete=122
audit_covered=81
gap=0
diagnostic_rows=203 incomplete=81 cfg219=Gap
```

Additional checks passed:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile issues/0805-roastty-ghostty-parity/config_diagnostic_inventory.py
prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/85-invalid-diagnostic-facet-audit.md issues/0805-roastty-ghostty-parity/config-diagnostic-inventory.md
git diff --check
```

## Conclusion

Experiment 85 creates the CFG-219 diagnostic audit surface without overclaiming
full diagnostic parity. Future diagnostic experiments can now choose bounded
families from `config-diagnostic-inventory.md`, promote rows only when explicit
`ConfigDiagnostic` behavior is proven, and let CFG-219 move to `Pass` only when
all 203 diagnostic rows are `Oracle complete`.

## Completion Review

Adversarial reviewer: Codex subagent with fresh context.

Final verdict: Approved.

Findings: None.

The reviewer confirmed that the result commit had not been made, the inventory
has 203 diagnostic rows matching 203 canonical options, counts are 122
`Oracle complete`, 81 `Audit covered`, and 0 `Gap`, CFG-217 and CFG-218 are
unchanged, CFG-219 remains `Gap` with the generated inventory and count note,
the README marks Experiment 85 `Pass`, the learning is recorded, and the
experiment verification snippets use the correct diagnostic status column.
