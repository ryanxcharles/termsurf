+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 228: Port Renderer `is_symbol` Predicate

## Description

Port `isSymbol` from upstream `renderer/cell.zig` into the `renderer::cell`
module (started in Experiment 227). `isSymbol` classifies "symbol-like"
codepoints, which the deferred `constraintWidth` uses to decide whether a glyph
may extend to two cells.

Upstream `isSymbol` is `symbols.get(cp)`, a build-time-generated 3-stage lookup
table. That table is **not** an opaque blob: its membership is defined exactly
in `vendor/ghostty/src/build/uucode_config.zig` (`computeIsSymbol`):

```
is_symbol = general_category == .other_private_use
    or block == .arrows
    or block == .dingbats
    or block == .emoticons
    or block == .miscellaneous_symbols
    or block == .enclosed_alphanumerics
    or block == .enclosed_alphanumeric_supplement
    or block == .miscellaneous_symbols_and_pictographs
    or block == .transport_and_map_symbols;
```

A Unicode block is a fixed contiguous range, and the Private-Use general
category (`Co`) is three fixed ranges, so `is_symbol` is faithfully expressible
as an **11-range membership check** — no generated table is needed, and the
result is byte-for-byte identical to the upstream table for every codepoint.
(Note: this is narrower than Unicode's general symbol categories — e.g. `+`
(U+002B, `Sm`) is not a symbol here — so the ranges must match the block
definition exactly, not a general "is symbol" notion.)

This fits the risk-based sizing rule: one coherent surface (one predicate in an
existing module), predictable tests (block edges), one mechanism, localized
failure.

### Ranges to port

Private-Use general category (`Co`):

- `0xE000..=0xF8FF` (BMP Private Use Area)
- `0xF0000..=0xFFFFD` (Plane 15 Supplementary Private Use Area-A)
- `0x100000..=0x10FFFD` (Plane 16 Supplementary Private Use Area-B)

Unicode blocks (fixed ranges):

- Arrows `0x2190..=0x21FF`
- Dingbats `0x2700..=0x27BF`
- Emoticons `0x1F600..=0x1F64F`
- Miscellaneous Symbols `0x2600..=0x26FF`
- Enclosed Alphanumerics `0x2460..=0x24FF`
- Enclosed Alphanumeric Supplement `0x1F100..=0x1F1FF`
- Miscellaneous Symbols and Pictographs `0x1F300..=0x1F5FF`
- Transport and Map Symbols `0x1F680..=0x1F6FF`

### Faithfulness and scope notes

- The supplementary PUA ranges stop at `..FFFD` / `..FFFD` because the last two
  code points of each plane are noncharacters (general category `Cn`, not `Co`),
  so the upstream `other_private_use` test excludes them.
- `is_symbol` is `pub(crate)` (upstream `isSymbol` is `pub fn`); a private
  `is_private_use` helper holds the three PUA ranges.
- `u32` codepoints, consistent with Experiment 227.
- Do **not** port `constraintWidth` (needs a `terminal::page` cell row and grid
  width) or the `Contents` builder; `constraintWidth` is the next slice.
- No C ABI, header, or ABI inventory changes; no new dependencies; no generated
  table.

## Changes

1. Extend `roastty/src/renderer/cell.rs`:
   - Add `pub(crate) fn is_symbol(cp: u32) -> bool` returning
     `is_private_use(cp) || <8-block match>`, with each block range commented by
     name.
   - Add a private `fn is_private_use(cp: u32) -> bool` matching the three `Co`
     ranges.

2. Tests in `renderer/cell.rs`:
   - `is_symbol_private_use`: `0xDFFF` false, `0xE000`/`0xF8FF` true, `0xF900`
     false; `0xEFFFF` false, `0xF0000`/`0xFFFFD` true, `0xFFFFE` false (the
     noncharacter); `0x100000`/`0x10FFFD` true, `0x10FFFE` false.
   - `is_symbol_blocks`: for each of the 8 blocks, the low and high edge true
     and a just-outside value false — choosing just-outside values that are not
     in an adjacent symbol block (e.g. Arrows `0x218F` false / `0x2200` false;
     Enclosed Alphanumerics `0x245F` false / `0x2500` false; Transport `0x1F67F`
     false / `0x1F700` false).
   - `is_symbol_excludes_general_symbols`: `'+'` (U+002B), `'$'` (U+0024), and
     `'a'` are **not** symbols, confirming the block-scoped definition rather
     than Unicode general symbol categories.

3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo test -p roastty renderer::cell
cargo test -p roastty renderer
cargo test -p roastty
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/renderer/cell.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `is_symbol` matches the `uucode_config.zig` definition exactly (PUA `Co` plus
  the 8 named blocks, with the noncharacter-excluding PUA upper bounds);
- the block-edge and PUA tests pass, including the general-symbol exclusion;
- `constraintWidth`/`Contents` are not pulled in;
- no C ABI, header, or ABI inventory changes;
- `cargo fmt` accepted and `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if a block boundary turns out to differ from the
assumed Unicode range and needs confirmation against the generator output.

The experiment **fails** if any range diverges from the `uucode_config.zig`
definition (wrong block bound, including the supplementary-PUA noncharacters, or
treating general-category symbols as symbols), or if any public API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no issues**.

Review artifacts:

- Prompt: `logs/codex-review/20260602-073745-189172-prompt.md`
- Result: `logs/codex-review/20260602-073745-189172-last-message.md`

Codex confirmed, reading `uucode_config.zig` and `renderer/cell.zig`, that
`is_symbol` is exactly `general_category == other_private_use` plus the eight
named blocks, that reconstructing it as fixed ranges is faithful (Unicode block
membership is range-based, including unassigned codepoints inside the block),
that all eleven ranges are correct, that stopping the supplementary PUA at
`..FFFD` correctly excludes the plane noncharacters (`Cn`, not `Co`), and that
the test plan (PUA boundaries, block edges, `+`/`$` general-symbol exclusion) is
sufficient. No changes were required.

## Result

**Result:** Pass

Extended `roastty/src/renderer/cell.rs` with `pub(crate) is_symbol` and a
private `is_private_use`. `is_symbol` is
`is_private_use(cp) || <8-block match>`, each block range commented by name;
`is_private_use` matches the three Private-Use (`Co`) ranges with the
supplementary planes stopping at `..FFFD` to exclude the plane noncharacters. No
generated table — the ranges reproduce the upstream `uucode`-generated
`is_symbol` exactly.

Tests added (3): `is_symbol_private_use` (BMP and both supplementary PUAs, with
the noncharacter boundaries), `is_symbol_blocks` (low/high edge and a
just-outside value for each of the eight blocks), and
`is_symbol_excludes_general_symbols` (`+`, `$`, `a` are not symbols, proving the
block-scoped definition).

### Verification

```bash
cargo fmt -p roastty
cargo test -p roastty renderer::cell
cargo test -p roastty renderer
cargo test -p roastty
```

Observed:

- `renderer::cell`: 11 passed (8 from Exp 227 + 3 new).
- Full `roastty`: 2247 unit tests passed (2244 prior + 3 new), plus the C ABI
  harness passed.
- `cargo fmt -p roastty -- --check`: clean.
- `cargo build -p roastty`: no warnings.
- No-`ghostty`-name gates passed for `roastty/src/renderer/cell.rs` and for
  `roastty/src/lib.rs`, `roastty/include/roastty.h`,
  `roastty/tests/abi_harness.c`.
- `git diff --check`: clean.

No C ABI, header, or ABI inventory changes; `constraintWidth`/`Contents` not
pulled in.

### Completion Review

Codex reviewed the completed implementation and found **no issues** ("nothing
should change before the result commit").

Review artifacts:

- Prompt: `logs/codex-review/20260602-073952-313247-prompt.md`
- Result: `logs/codex-review/20260602-073952-313247-last-message.md`

Codex confirmed the implementation matches `uucode_config.zig`'s
`computeIsSymbol` definition (PUA `Co` plus the eight named blocks, with the
noncharacter-excluding supplementary-PUA bounds), that the three tests cover the
right behavior, and that visibility (`is_symbol` `pub(crate)`, `is_private_use`
private) and the `matches!` patterns are clean.

## Conclusion

Experiment 228 succeeds. `renderer::cell` now has the faithful `is_symbol`
predicate, reconstructed as fixed ranges from the `uucode_config.zig` definition
rather than the generated lookup table — a much smaller, dependency-free port
that is byte-for-byte identical to upstream. Both Codex gates passed with zero
findings.

The next slice (Experiment 229) is `constraintWidth`, the last standalone
function in `cell.zig` before the `Contents` builder. It uses `is_symbol`,
`is_graphics_element`, and `is_space` (all now landed) plus a row of cells with
codepoint and grid-width access, so its main new dependency is a
`terminal::page` cell-row representation — which the design will need to map to
Roastty's render cell snapshot or page cell type.
