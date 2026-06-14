# Experiment 14: Link Parser Recognition

## Description

Experiment 13 found the first concrete non-default parser gap: upstream Ghostty
has canonical `link: RepeatableLink`, while Roastty has no `link` arm in
`Config::set_from_source`. Pinned Ghostty's `RepeatableLink.parseCLI` returns
`error.NotImplemented`, so setting `link` is a recognized-field parser failure,
not an unknown-field failure.

This experiment will close that dispatch gap without pretending the full `link`
parser exists. Roastty should recognize `link`, preserve Ghostty's generic
empty-value reset behavior, and return an explicit not-implemented config error
for bare or non-empty values. That error must be distinct from `UnknownField`.
The parser inventory should then report 203 canonical dispatch paths, 0 dispatch
gaps, and 203 rows still below `Oracle complete`, keeping CFG-217 as `Gap`.

## Changes

- `roastty/src/config/mod.rs`
  - Add a `ConfigSetError` variant for upstream `error.NotImplemented` or an
    equivalently named recognized-but-unsupported parser error.
  - Add a `Config::set_from_source` arm for canonical `link` that:
    - resets the link list to the default when the value is set but empty
      (`link =` / `--link=`), matching Ghostty's generic reset-before-parse
      branch;
    - returns the new not-implemented error for missing values (`link` /
      `--link`) and non-empty values, matching Ghostty's
      `RepeatableLink.parseCLI` path.
  - Add focused tests proving:
    - `cfg.set("link", Some(...))` returns the new recognized parser error, not
      `UnknownField`;
    - `cfg.set("link", None)` follows the same upstream not-implemented path;
    - `cfg.set("link", Some(""))` succeeds and restores the default link list;
    - `load_str` records a diagnostic for `link` with the new error;
    - a truly unknown key still returns `UnknownField`.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Classify the `link` dispatch arm as a custom unsupported parser path.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the parser inventory. Expected counts: 203 parser rows, 0
    dispatch gaps, 203 incomplete rows, and CFG-217 still `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Update CFG-217 evidence/notes to say the `link` dispatch gap is closed but
    parser-family oracles are still incomplete.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning for the `link` not-implemented semantics if the tests confirm
    the behavior.

## Verification

Pass criteria:

- Focused Roastty tests pass:

```bash
cargo test --manifest-path roastty/Cargo.toml link_config_parser_recognizes_not_implemented_and_empty_reset
```

- The parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_canonical_parser_rows=0`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=0`;
  - `audit_covered=203`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - no parser row has status `Gap`;
  - all parser rows remain below `Oracle complete`;
  - CFG-217 remains `Gap`;
  - CFG-217 evidence points to `config-parser-inventory.md`.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run.
- `prettier --write --prose-wrap always --print-width 80` is run on changed
  markdown files.
- `git diff --check` passes.

Suggested commands:

```bash
cargo test --manifest-path roastty/Cargo.toml link_config_parser_recognizes_not_implemented_and_empty_reset
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
    if not line.startswith('| CFG-'):
        continue
    matrix_rows.append([cell.strip() for cell in line.strip('|').split('|')])
cfg217 = next(row for row in matrix_rows if row[0] == 'CFG-217')
assert cfg217[4] == 'Gap', cfg217
assert 'config-parser-inventory.md' in cfg217[6], cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if not line.startswith('| PARSE-'):
        continue
    parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
assert all(row[4] != 'Gap' for row in parser_rows)
assert all(row[4] != 'Oracle complete' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/14-link-parser-recognition.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Fresh-context adversarial design review found one required issue:

- The draft made `link` unconditionally return not implemented, but Ghostty's
  generic parser resets set-but-empty values before calling `parseCLI`.

Fix:

- Updated the design and tests to treat `link =` / `--link=` as a successful
  default reset, while bare and non-empty `link` values return the
  not-implemented parser error.

Re-review approved the fixed design:

```text
VERDICT: APPROVED

Findings: none.
```

## Result

**Result:** Pass

Roastty now recognizes canonical `link` in `Config::set_from_source` without
claiming the full upstream parser exists:

- `link =` / `--link=` resets the link list to the default and succeeds,
  matching Ghostty's generic set-but-empty reset before `parseCLI`.
- Bare `link` / `--link` and non-empty `link` values return
  `ConfigSetError::NotImplemented`, distinct from `UnknownField`.
- Unknown keys still return `UnknownField`.
- The parser inventory now reports 203 canonical parser rows, 0 missing dispatch
  rows, 203 audit-covered rows, and 0 gap rows.
- CFG-217 remains `Gap` because no parser row is `Oracle complete`.

Verification:

```bash
cargo test --manifest-path roastty/Cargo.toml link_config_parser_recognizes_not_implemented_and_empty_reset
```

Output:

```text
running 1 test
test config::tests::link_config_parser_recognizes_not_implemented_and_empty_reset ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 4904 filtered out; finished in 0.00s
```

Parser inventory command:

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
oracle_complete=0
audit_covered=203
gap=0
```

Matrix assertion output:

```text
parser_rows=203 cfg217=Gap
```

## Conclusion

The first concrete parser dispatch gap is closed. The next CFG-217 work should
raise parser families from `Audit covered` to `Oracle complete` with
upstream-derived accepted-value and rejection/reset oracles.

## Completion Review

Fresh-context adversarial completion review approved the result:

```text
VERDICT: APPROVED

Findings: none.
```

The reviewer independently ran the focused Rust test, regenerated the parser
inventory to temporary files, verified the matrix assertion, checked
`git diff --check`, checked
`cargo fmt --manifest-path roastty/Cargo.toml -- --check`, confirmed the result
was uncommitted before review, and verified the upstream empty-reset and
`RepeatableLink.parseCLI` `NotImplemented` evidence.
