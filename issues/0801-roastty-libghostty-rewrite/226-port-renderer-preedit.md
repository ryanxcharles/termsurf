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

# Experiment 226: Port Renderer Preedit

## Description

Port the `Preedit` type from upstream `renderer/State.zig` into a new
`renderer::state` module. `Preedit` holds the IME dead-key / preedit text that
is rendered over the cursor, and its `range` method computes where that text is
placed (with right-edge shifting and codepoint truncation). Experiment 223's
cursor `style()` already consumes a `preedit` flag; this lands the actual
preedit data type and its placement logic.

`renderer/State.zig` also defines the live render `State` struct (a mutex, a
`*Terminal`, an optional `*Inspector`, and a `Mouse`) and a `Mouse` struct.
Those belong to the live-renderer threading model that Roastty does not have yet
(its render state is the snapshot-based `RenderStateScalar`), and `Mouse`
depends on `input.Mods`, which is not ported. Porting them now would be
premature stubs with no consumer, so this experiment ports **only `Preedit`**,
which is self-contained (it depends only on a cell-count integer) and has
focused upstream tests.

This fits the risk-based sizing rule: one coherent surface (a `Preedit` type),
predictable tests (two upstream `range` tests port directly), one mechanism (the
`range` placement computation), localized failure.

### Types and behavior to port

- `Preedit::Codepoint { codepoint: u32, wide: bool }` — `codepoint` mirrors
  upstream `u21` (Rust has no `u21`; `u32` stores any Unicode scalar and the
  value is not used by `range`/`width`). `wide` defaults to `false`.
- `Preedit { codepoints: Vec<Codepoint> }` — owns its codepoints. Upstream's
  manual `deinit`/`clone` over a borrowed slice become Rust ownership: `Vec`
  drops automatically and derives `Clone`, so no explicit `deinit`/`clone`
  methods are needed.
- `width(&self) -> usize`: `2` per wide codepoint, `1` otherwise.
- `range(&self, start: Unit, max: Unit) -> PreeditRange` where `Unit = u16`
  mirrors `terminal::size::CellCountInt`. Returns
  `PreeditRange { start: Unit, end: Unit, cp_offset: usize }`. The algorithm,
  ported exactly:
  - `max_width = max - start + 1` (`max` is inclusive);
  - accumulate width from the **end** of the codepoints; if the running width
    exceeds `max_width`, stop with that width and `cp_offset = reverse_i` (the
    index reached); if it never exceeds, the full width is used with
    `cp_offset = 0`;
  - `end = if w > 0 { start + (w - 1) } else { start }`;
  - `start_offset = if end > max { end - max } else { 0 }`;
  - return `start -| start_offset`, `end -| start_offset` (saturating), and
    `cp_offset`.

### Faithfulness notes

- `Unit = u16` mirrors `terminal::size::CellCountInt` (private to the terminal
  module), defined locally as in `renderer::size`.
- Width accumulation and `max_width` use `u16`, matching upstream `CellCountInt`
  arithmetic; preedit strings are short so this does not overflow in practice.
- Zig saturating `-|` on the final `start`/`end` maps to `u16::saturating_sub`.
- The loop replicates upstream's reverse iteration and early-exit precisely:
  breaking sets `cp_offset` to the index reached and keeps the accumulated `w`;
  completing leaves `cp_offset = 0` and `w` at the full width.

### Scope limits

- Do **not** port the live `State` struct (mutex / `*Terminal` / inspector) or
  `Mouse` — those wait for the renderer threading model and `input.Mods`.
- Only a new `renderer/state.rs` and its `mod` wiring; no C ABI, header, or ABI
  inventory changes; no new dependencies.

## Changes

1. Create `roastty/src/renderer/state.rs`:
   - Module-level `#![allow(dead_code)]` with a "consumed by later renderer
     slices" comment; "upstream `renderer/State.zig`" attribution (no literal
     `ghostty` token).
   - `pub(crate) type Unit = u16;` (comment: mirrors
     `terminal::size::CellCountInt`).
   - `pub(crate) struct Codepoint { pub codepoint: u32, pub wide: bool }`
     (`Debug, Clone, Copy, PartialEq, Eq`).
   - `pub(crate) struct Preedit { pub codepoints: Vec<Codepoint> }`
     (`Debug, Clone, Default, PartialEq, Eq`).
   - `pub(crate) struct PreeditRange { pub start: Unit, pub end: Unit, pub cp_offset: usize }`
     (`Debug, Clone, Copy, PartialEq, Eq`).
   - `impl Preedit { pub(crate) fn width(&self) -> usize; pub(crate) fn range(&self, start: Unit, max: Unit) -> PreeditRange }`
     with the exact upstream algorithm. Both are `pub(crate)` (upstream exposes
     them as `pub fn`) so sibling renderer modules can call them.

2. Wire the module from `roastty/src/renderer/mod.rs` with
   `pub(crate) mod state;` (kept internal; no public API or ABI).

3. Port the upstream tests into `renderer/state.rs`:
   - `preedit_range_covers_exact_cell_width` (upstream "preedit range covers
     exact cell width"): a single `'a'` at `start=2, max=9` →
     `start=2, end=2, cp_offset=0`; a single wide Hangul GA at `start=2, max=9`
     → `start=2, end=3, cp_offset=0`.
   - `preedit_range_shifts_left_at_right_edge` (upstream "preedit range shifts
     left at right edge"): a single wide Hangul GA at `start=9, max=9` →
     `start=8, end=9, cp_offset=0`.
   - `preedit_range_truncates_at_nonzero_offset`: four narrow codepoints at
     `start=8, max=9` → `max_width=2`, reverse widths `1, 2, 3` break at
     `reverse_i=1`, yielding `start=7, end=9, cp_offset=1`. This is the
     nonzero-`cp_offset` truncation case the two upstream tests do not cover.
   - Add a `preedit_width` check (mixed wide/narrow codepoints sum correctly).

4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo test -p roastty renderer::state
cargo test -p roastty renderer
cargo test -p roastty
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/renderer/state.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Preedit`, `Codepoint`, `PreeditRange`, `width`, and `range` are implemented
  with the exact upstream algorithm (reverse-accumulate, right-edge shift,
  saturating final subtraction);
- both upstream `range` tests pass (including the right-edge shift to
  `start=8, end=9`), plus the width check;
- the live `State`/`Mouse` are not pulled in;
- no C ABI, header, or ABI inventory changes;
- `cargo fmt` accepted and `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if `range` needs a `Size`/grid input not available
without a prerequisite slice.

The experiment **fails** if the `range` algorithm diverges from upstream (wrong
truncation index, wrong right-edge shift, or non-saturating final subtraction),
if `State`/`Mouse` scope leaks in, or if any public API/ABI changes.

## Design Review

Codex reviewed this design before implementation.

Review artifacts:

- Prompt: `logs/codex-review/20260602-072247-852393-prompt.md`
- Result: `logs/codex-review/20260602-072247-852393-last-message.md`

Codex confirmed the `range` algorithm matches upstream and traced all three
expected results (`a@2..9 → 2,2,0`; `wideGA@2..9 → 2,3,0`;
`wideGA@9..9 → 8,9,0`), agreed that deferring the live `State`/`Mouse` is the
right scope, and that `Vec` ownership replacing `deinit`/`clone` and `u32` for
the `u21` codepoint are acceptable.

Two real findings, fixed in the design above before this commit:

1. **(Medium)** `width`/`range` were specified as private; upstream exposes them
   as `pub fn`, so they are now `pub(crate) fn` for sibling renderer modules.
2. **(Medium)** the tests did not prove the nonzero-`cp_offset` truncation path
   — added `preedit_range_truncates_at_nonzero_offset` (four narrow codepoints,
   `start=8, max=9` → `start=7, end=9, cp_offset=1`).

## Result

**Result:** Pass

Added `roastty/src/renderer/state.rs` (module-level `#![allow(dead_code)]`,
"upstream `renderer/State.zig`" attribution) and wired `pub(crate) mod state;`
into `roastty/src/renderer/mod.rs`.

Implemented `Codepoint { codepoint: u32, wide: bool }`,
`Preedit { codepoints: Vec<Codepoint> }`,
`PreeditRange { start: Unit, end: Unit, cp_offset: usize }` (`Unit = u16`), and
`pub(crate)` `Preedit::width` and `Preedit::range`. `range` reproduces upstream
exactly: `max_width = max - start + 1`, reverse width accumulation with early
exit setting `cp_offset` to the reached index (full width / `cp_offset = 0`
otherwise), `end = if w > 0 { start + (w - 1) } else { start }`, and the
saturating right-edge shift. `Vec` ownership replaces upstream `deinit`/`clone`.
The live `State`/`Mouse` are deferred.

Tests added (4): `preedit_range_covers_exact_cell_width`,
`preedit_range_shifts_left_at_right_edge`,
`preedit_range_truncates_at_nonzero_offset` (the nonzero-`cp_offset` case), and
`preedit_width`.

### Verification

```bash
cargo fmt -p roastty
cargo test -p roastty renderer::state
cargo test -p roastty renderer
cargo test -p roastty
```

Observed:

- `renderer::state`: 4 passed.
- Full `roastty`: 2236 unit tests passed (2232 prior + 4 new), plus the C ABI
  harness passed.
- `cargo fmt -p roastty -- --check`: clean.
- `cargo build -p roastty`: no warnings.
- No-`ghostty`-name gates passed for `roastty/src/renderer/state.rs` and for
  `roastty/src/lib.rs`, `roastty/include/roastty.h`,
  `roastty/tests/abi_harness.c`.
- `git diff --check`: clean.

No C ABI, header, or ABI inventory changes; live `State`/`Mouse` not pulled in.

### Completion Review

Codex reviewed the completed implementation and found **no issues** ("nothing
should change before the result commit").

Review artifacts:

- Prompt: `logs/codex-review/20260602-072623-278764-prompt.md`
- Result: `logs/codex-review/20260602-072623-278764-last-message.md`

Codex confirmed `width`/`range` match upstream, that the Rust loop preserves the
Zig labeled-block semantics exactly (`w` persists after break, `cp_offset` set
only on early exit, no-break leaves `cp_offset = 0` with full width), that the
four tests pass for the right reasons (including the `7,9,1` truncation trace),
that there is no `u16` over/underflow in the tested domain, and that the
`pub(crate)` visibility, derives, `Vec` ownership, and module wiring are clean.

## Conclusion

Experiment 226 succeeds. Roastty's `renderer::state` module now holds the
`Preedit` type and its cell-placement `range` logic, which the IME/preedit
rendering path and Experiment 223's preedit-aware cursor `style()` will consume.
Both Codex gates passed (two design findings fixed; zero result findings).

The live render `State` struct (mutex, terminal pointer, inspector) and `Mouse`
remain deferred until the renderer threading model and `input` (`Mods`) are
available. The next renderer slice is likely `renderer/cell.zig` (per-cell
render data: backgrounds, glyph/text quads, and the cursor cell), which is large
and will need splitting, and which consumes the `Size`/`Coordinate` model (Exp
224–225), the cursor `Style` (Exp 223), and this `Preedit`.
