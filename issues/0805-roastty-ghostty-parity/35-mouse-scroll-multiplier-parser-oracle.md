# Experiment 35: Mouse Scroll Multiplier Parser Oracle

## Description

CFG-217 still has 28 parser rows that are only `Audit covered`. Canonical
`mouse-scroll-multiplier` is one of the remaining `custom parse_cli` rows.

Pinned Ghostty defines `mouse-scroll-multiplier` as
`MouseScrollMultiplier = .default`, with defaults `precision = 1` and
`discrete = 3`. `MouseScrollMultiplier.parseCLI` requires a value, first tries
auto-struct parsing against the current value, and falls back to parsing the
whole input as a Zig `f64` bare value when auto-struct parsing returns
`InvalidValue`. The bare value sets both `precision` and `discrete`. Auto-struct
field values also use Zig float parsing, preserve unspecified fields from the
current value, accept fields in any order, decode quoted field values as Zig
string literals before field parsing, and treat explicit empty input as a
current/default-preserving no-op because the auto-struct splitter yields no
entries. Unknown fields, bad floats, malformed quoted values, doubled/leading
empty comma entries, and malformed structures are rejected.

Roastty already has lower-level parser and config-routing tests, but the current
parser uses Rust `f64::parse` in this helper. This experiment will add a focused
CFG-217 oracle, fix the helper to use Roastty's existing Zig-compatible float
parser where needed, keep the existing lower-level/routing tests in the
verification set, and promote only canonical `mouse-scroll-multiplier`.

CFG-217 must remain `Gap` because other parser helpers are still audit-only.

## Changes

- `roastty/src/config/mod.rs`
  - Update `MouseScrollMultiplier::parse_cli` to parse bare and field values
    with the same Zig-compatible `f64` grammar used by the shared float parser.
  - Add a focused `mouse_scroll_multiplier_config_parser_family_oracle` test
    covering:
    - defaults and default formatting;
    - bare values setting both `precision` and `discrete`;
    - explicit empty values preserving the current/default value;
    - missing values are `ValueRequired`;
    - single-field updates preserving the other current field;
    - both fields in either order;
    - spaces/tabs trimmed around auto-struct keys and values;
    - quoted auto-struct field values decoded as Zig string literals before
      float parsing;
    - Zig float syntax such as `0x1p4`, `+inf`, `-infinity`, and `nan`;
    - unknown fields, bad floats, malformed quoted values, leading/doubled empty
      comma entries, malformed separators, and malformed Zig float separators;
    - config-file diagnostics preserve an earlier valid value after a later
      invalid value;
    - CLI argument parsing reaches the same helper;
    - formatter output and clone semantics.
  - Keep the existing `mouse_scroll_multiplier_parse_and_format`,
    `mouse_behavior_config_routes_and_formats`, and
    `mouse_behavior_finalize_resolves_and_clamps` tests in the verification set.
- `issues/0805-roastty-ghostty-parity/config_parser_inventory.py`
  - Mark only canonical `mouse-scroll-multiplier` as `Oracle complete` when the
    mouse scroll multiplier oracle test is present.
  - Add mouse scroll multiplier oracle detection to CFG-217 ownership so the
    generated matrix records `Experiment 35` when this oracle is present.
- `issues/0805-roastty-ghostty-parity/config-parser-inventory.md`
  - Regenerate the inventory. Expected status counts: 176 `Oracle complete`, 27
    `Audit covered`, 0 `Gap`.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Keep CFG-217 as `Gap`, but update the note to show 176 parser rows are now
    `Oracle complete`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning documenting `mouse-scroll-multiplier` parser semantics after
    the result is proven.

## Verification

Pass criteria:

- Focused Roastty test passes:

```bash
cargo test --manifest-path roastty/Cargo.toml mouse_scroll_multiplier_config_parser_family_oracle
```

- Existing lower-level, routing, and finalize-boundary tests still pass:

```bash
cargo test --manifest-path roastty/Cargo.toml mouse_scroll_multiplier_parse_and_format
cargo test --manifest-path roastty/Cargo.toml mouse_behavior_config_routes_and_formats
cargo test --manifest-path roastty/Cargo.toml mouse_behavior_finalize_resolves_and_clamps
```

- Parser inventory generator succeeds and reports:
  - `ghostty_canonical=203`;
  - `roastty_parser_rows=203`;
  - `missing_dispatch_rows=0`;
  - `extra_parser_rows=0`;
  - `oracle_complete=176`;
  - `audit_covered=27`;
  - `gap=0`.
- Matrix assertion verifies:
  - `config-parser-inventory.md` has 203 `PARSE-` rows;
  - exactly 176 rows are `Oracle complete`;
  - the `mouse-scroll-multiplier` row is `Oracle complete`;
  - no row is `Gap`;
  - CFG-217 remains `Gap`;
  - CFG-217 owner is `Experiment 35`;
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
cargo test --manifest-path roastty/Cargo.toml mouse_scroll_multiplier_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml mouse_scroll_multiplier_parse_and_format
cargo test --manifest-path roastty/Cargo.toml mouse_behavior_config_routes_and_formats
cargo test --manifest-path roastty/Cargo.toml mouse_behavior_finalize_resolves_and_clamps
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
assert cfg217[11] == 'Experiment 35', cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
mouse_scroll = [row for row in parser_rows if row[1] == '`mouse-scroll-multiplier`']
assert len(mouse_scroll) == 1, mouse_scroll
assert mouse_scroll[0][4] == 'Oracle complete', mouse_scroll[0]
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 176
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} mouse_scroll_multiplier={mouse_scroll[0][4]} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
python3 -m py_compile issues/0805-roastty-ghostty-parity/config_parser_inventory.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/35-mouse-scroll-multiplier-parser-oracle.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-parser-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Fresh-context adversarial subagent review completed before implementation.

**Initial verdict:** Changes required.

Required findings and fixes:

- Empty value semantics were incomplete. The plan now distinguishes missing
  values as `ValueRequired` from explicit empty values as upstream auto-struct
  no-ops preserving the current/default value.
- Quoted auto-struct field values were missing from the oracle scope. The plan
  now requires valid quoted numeric field values and malformed quoted field
  rejection.

**Re-review verdict:** Approved.

No required findings remain.

## Result

**Result:** Pass

Implemented the focused `mouse_scroll_multiplier_config_parser_family_oracle`
test, fixed `MouseScrollMultiplier::parse_cli` to use Roastty's Zig-compatible
float parser and upstream-compatible auto-struct empty/quoted value behavior,
and promoted only canonical `mouse-scroll-multiplier` in the CFG-217 parser
inventory. The generated inventory now reports:

- `ghostty_canonical=203`
- `roastty_parser_rows=203`
- `missing_dispatch_rows=0`
- `extra_parser_rows=0`
- `oracle_complete=176`
- `audit_covered=27`
- `gap=0`

The matrix assertion verified that `mouse-scroll-multiplier` is now
`Oracle complete`, no parser row is `Gap`, and CFG-217 still remains `Gap` with
owner `Experiment 35`.

Verification commands run:

```bash
cargo test --manifest-path roastty/Cargo.toml mouse_scroll_multiplier_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml mouse_scroll_multiplier_parse_and_format
cargo test --manifest-path roastty/Cargo.toml mouse_behavior_config_routes_and_formats
cargo test --manifest-path roastty/Cargo.toml mouse_behavior_finalize_resolves_and_clamps
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
assert cfg217[11] == 'Experiment 35', cfg217

parser_rows = []
for line in Path('issues/0805-roastty-ghostty-parity/config-parser-inventory.md').read_text().splitlines():
    if line.startswith('| PARSE-'):
        parser_rows.append([cell.strip() for cell in line.strip('|').split('|')])
assert len(parser_rows) == 203, len(parser_rows)
mouse_scroll = [row for row in parser_rows if row[1] == '`mouse-scroll-multiplier`']
assert len(mouse_scroll) == 1, mouse_scroll
assert mouse_scroll[0][4] == 'Oracle complete', mouse_scroll[0]
assert sum(row[4] == 'Oracle complete' for row in parser_rows) == 176
assert all(row[4] != 'Gap' for row in parser_rows)
print(f'parser_rows={len(parser_rows)} mouse_scroll_multiplier={mouse_scroll[0][4]} cfg217={cfg217[4]}')
PY
cargo fmt --manifest-path roastty/Cargo.toml
```

## Conclusion

`mouse-scroll-multiplier` matches the pinned Ghostty direct parser boundary for
the covered `MouseScrollMultiplier` semantics: defaults, default formatting,
bare values, auto-struct field updates, explicit empty no-op values, missing
values, quoted auto-struct values, Zig float syntax, invalid structures and
floats, diagnostics, CLI parsing, formatter output, clone semantics, and the
parser/finalization boundary. CFG-217 remains open because 27 parser rows are
still only `Audit covered`.

## Completion Review

Fresh-context adversarial subagent review completed after implementation and
verification.

**Initial verdict:** Changes required.

Required finding and fix:

- Invalid auto-struct values could partially mutate the multiplier before
  returning `InvalidValue`. The parser now applies auto-struct field updates to
  a temporary initialized from the current value and assigns it back only after
  the whole input succeeds. The oracle now asserts `precision:9,foo:1` preserves
  the prior valid value.

**Re-review verdict:** Approved.

No required findings remain.
