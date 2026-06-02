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

# Experiment 224: Port Renderer Sizing Value Types

## Description

Begin porting Ghostty's `renderer/size.zig` into a new `renderer::size` module.
This slice ports the **sizing value types and their arithmetic** — `CellSize`,
`ScreenSize`, `GridSize`, and `Padding` — which are the foundation every later
renderer geometry slice (cell quads, cursor placement, coordinate conversion)
builds on.

`renderer/size.zig` is 458 lines spanning several coupled types plus the `Size`
aggregate, the `Coordinate` union and its conversions, and the `PaddingBalance`
enum with `Size.balancePadding`. Per the issue's risk-based sizing rule, that is
more than one coherent slice, so it is split:

- **This experiment (224):** the leaf value types `CellSize`, `ScreenSize`,
  `GridSize`, `Padding` and their methods. These are pure integer/float
  arithmetic with predictable, directly portable tests.
- **A follow-up (225):** the `Size` aggregate, the `Coordinate` union with
  `convert`, the `PaddingBalance` enum, and `Size.balancePadding`, which build
  on these value types.

This is pure math with no framework dependency, so it sits comfortably within
the sizing rule: one coherent surface (a new `renderer/size.rs`), predictable
tests, one mechanism, localized failure.

### Types and behavior to port

- `CellSize { width: u32, height: u32 }` — a glyph cell's pixel size.
- `ScreenSize { width: u32, height: u32 }`:
  - `sub_padding(padding) -> ScreenSize`: subtract horizontal/vertical padding
    using **saturating** subtraction (Zig `-|`) so a 1x1 screen cannot
    underflow.
  - `blank_padding(padding, grid, cell) -> Padding`: compute the leftover blank
    space to the right and bottom when padding is unbalanced; `self` is the
    unpadded screen; uses saturating subtraction; returns
    `Padding { top: 0, bottom: leftover_height, right: leftover_width, left: 0 }`.
  - `equals(other) -> bool`.
- `GridSize { columns: Unit, rows: Unit }` where `Unit` is `u16`, mirroring
  upstream `GridSize.Unit = terminal_size.CellCountInt` (Roastty's
  `terminal::size::CellCountInt` is `u16` but `pub(super)` to the terminal
  module, so the renderer defines its own `u16` `Unit` with a comment, rather
  than widening the terminal type's visibility):
  - default `columns = 0, rows = 0`;
  - `init(screen, cell) -> GridSize` (delegates to `update`);
  - `update(&mut self, screen, cell)`: float-divide screen by cell, truncate
    toward zero, and clamp each axis to a minimum of `1` (matching upstream
    `@max(1, ...)`);
  - `equals(other) -> bool`.
- `Padding { top, bottom, right, left: u32 }` (all default `0`):
  - `balanced(screen, grid, cell) -> Padding`: split the leftover whitespace
    equally on each axis using float `floor` and a `max(0, ...)` clamp so a
    zero-sized screen yields zero padding;
  - `add(other) -> Padding`: component-wise addition;
  - `eql(other) -> bool`.

### Faithfulness notes

- Zig saturating subtraction `-|` maps to Rust `u32::saturating_sub`. Plain `+`
  and `*` map to plain Rust operators (matching upstream's non-saturating
  semantics); pixel-scale values do not overflow `u32` in practice, and Rust
  debug overflow checks mirror Zig safe-mode behavior.
- `grid.columns * cell.width` mixes `u16` and `u32` upstream via Zig coercion;
  in Rust the `u16` columns/rows are cast to `u32` for pixel math.
- `update` and `balanced` use `f32` exactly as upstream (`@floatFromInt` →
  arithmetic → `@intFromFloat`/`@floor`), so rounding matches. Cell dimensions
  are assumed non-zero, as upstream assumes.
- `GridSize::update` casts the `f32` quotient to `u16` with `as`. For realistic
  terminal grids the quotient is well below `u16::MAX`. Rust `as` **saturates**
  out-of-range floats (rather than Zig safe-mode's checked conversion), so on an
  impossible oversized grid Rust yields `u16::MAX` instead of trapping. This
  divergence is accepted (it only occurs on inputs that cannot arise from a real
  terminal); the `max(1, ...)` clamp still applies.

### Scope limits

- Do **not** port `Size`, `Coordinate`, `Coordinate.convert`, `PaddingBalance`,
  or `Size.balancePadding` — those are Experiment 225 and depend on these types.
- No C ABI, header, or ABI inventory changes.
- No new dependencies.

## Changes

1. Create `roastty/src/renderer/size.rs`.
   - Module-level `#![allow(dead_code)]` with a "consumed by later renderer
     slices" comment, matching the renderer-subtree convention.
   - Attribution comment referencing "upstream `renderer/size.zig`" (no literal
     `ghostty` token, per the renderer convention).
   - Define `pub(crate) struct CellSize`, `ScreenSize`, `GridSize`, `Padding`
     with the fields above; derive `Debug, Clone, Copy, PartialEq, Eq` and
     `Default` where upstream has defaults (`GridSize`, `Padding`).
   - Define `pub(crate) type Unit = u16;` (or inline `u16`) for grid units, with
     a comment that it mirrors `terminal::size::CellCountInt`.
   - Implement the methods listed above with exact upstream semantics
     (saturating where upstream saturates; float math where upstream uses
     floats). Prefer deriving `PartialEq` over hand-written `equals`/`eql`, but
     also provide the named `equals`/`eql`/`add`/`sub_padding`/`blank_padding`/
     `balanced`/`init`/`update` methods so later slices and tests call the same
     surface upstream exposes.

2. Wire the module from `roastty/src/renderer/mod.rs` with
   `pub(crate) mod size;` (kept internal; no public API or ABI).

3. Port the directly-applicable upstream tests into `renderer/size.rs`:
   - `padding_balanced_on_zero` (upstream "Padding balanced on zero"): a
     zero-sized screen yields `Padding::default()`.
   - `grid_size_update_exact` (upstream "GridSize update exact"): screen 100x40,
     cell 5x10 → 20 columns, 4 rows.
   - `grid_size_update_rounding` (upstream "GridSize update rounding"): screen
     20x40, cell 6x15 → 3 columns, 2 rows.
   - Add coverage for the methods not isolated upstream in this slice:
     - `padding_balanced_nonzero`: a screen with even leftover space splits
       left==right and top==bottom, both > 0;
     - `padding_balanced_floor_odd_leftover`: an **odd** leftover (e.g. grid
       leaves 5px horizontally) yields 2px per side, proving `floor` rather than
       `round`/`ceil`;
     - `screen_sub_padding_saturates`: subtracting padding larger than the
       screen saturates to 0 instead of underflowing;
     - `screen_blank_padding`: an unpadded screen larger than the grid reports
       the leftover on `right`/`bottom` only;
     - `screen_blank_padding_saturates`: when the padded grid is **larger** than
       the screen, `right`/`bottom` saturate to 0 (not underflow);
     - `padding_add` and `padding_eql`: component-wise add and equality;
     - `grid_size_update_min_one`: a screen smaller than one cell clamps to 1
       column / 1 row.

4. Format and test.
   - Run `cargo fmt` after Rust edits and accept its output.

## Verification

```bash
cargo fmt
cargo test -p roastty renderer::size
cargo test -p roastty renderer
cargo test -p roastty
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/renderer/size.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `renderer::size` implements `CellSize`, `ScreenSize`, `GridSize`, `Padding`
  with the exact upstream arithmetic (saturating subtraction, float-based grid
  sizing with min-1 clamp, balanced padding with floor and max-0 clamp);
- the three directly-ported upstream tests pass, plus the additional method
  coverage tests;
- no `Size`/`Coordinate`/`PaddingBalance` surface is pulled in;
- no C ABI, header, or ABI inventory changes are made;
- `cargo fmt` is accepted and `cargo test -p roastty` passes with no
  regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if a value type turns out to need a `Size`/
`Coordinate` dependency that should be reordered before this slice.

The experiment **fails** if the arithmetic diverges from upstream (e.g.
non-saturating subtraction, wrong rounding, or missing the min-1 grid clamp), if
`Size`/`Coordinate` scope leaks in, or if any public API/ABI changes.

## Design Review

Codex reviewed this design before implementation.

Review artifacts:

- Prompt: `logs/codex-review/20260602-070932-071174-prompt.md`
- Result: `logs/codex-review/20260602-070932-071174-last-message.md`

Codex confirmed the arithmetic plan is faithful (`u32::saturating_sub` matches
Zig `-|`; `GridSize::update` uses `f32`/truncation/`max(1)`; `Padding::balanced`
uses `f32`/`.floor()`/`max(0.0)`), the local renderer `Unit = u16` is the right
call (no need to widen `terminal::size::CellCountInt`), and the split is clean
(none of the four value types depend on `Size`/`Coordinate`/`PaddingBalance`).

It raised three real findings, all fixed in the design above before this commit:

1. **(Medium)** the balanced-padding tests did not prove `floor` for odd
   leftover space — added `padding_balanced_floor_odd_leftover` (5px leftover →
   2px per side, which `round`/`ceil` could not pass).
2. **(Medium)** the blank-padding tests did not prove saturating subtraction —
   added `screen_blank_padding_saturates` (padded grid larger than screen →
   `right`/`bottom` saturate to 0).
3. **(Low)** the `f32`→`u16` conversion in `update` differs from Zig safe-mode
   on out-of-range floats (Rust saturates) — documented this as an accepted
   divergence in the faithfulness notes.

## Result

**Result:** Pass

Added `roastty/src/renderer/size.rs` (module-level `#![allow(dead_code)]`,
"upstream `renderer/size.zig`" attribution) and wired `pub(crate) mod size;`
into `roastty/src/renderer/mod.rs`.

Implemented value types and methods with exact upstream arithmetic:

- `CellSize { width, height }`;
- `ScreenSize { width, height }` with `sub_padding` (saturating),
  `blank_padding` (saturating leftover), and `equals`;
- `GridSize { columns, rows }` over `Unit = u16` (mirroring
  `terminal::size::CellCountInt`), with `init`, `update` (`f32` divide →
  truncate → `max(1)`), and `equals`;
- `Padding { top, bottom, right, left }` with `balanced` (`f32` → `.floor()` →
  `max(0.0)`), `add`, and `eql`.

`PartialEq`/`Eq` are derived; the named `equals`/`eql` methods delegate to `==`
to preserve the upstream call surface for later slices.

Tests added (12): `padding_balanced_on_zero`, `padding_balanced_nonzero`,
`padding_balanced_floor_odd_leftover`, `grid_size_update_exact`,
`grid_size_update_rounding`, `grid_size_update_min_one`,
`screen_sub_padding_saturates`, `screen_blank_padding`,
`screen_blank_padding_saturates`, `padding_add`, `padding_eql`,
`size_equals_helpers`. This includes the three directly-ported upstream tests
plus the two extra cases Codex requested at design time (odd-leftover floor
proof and saturating blank padding).

### Verification

```bash
cargo fmt -p roastty
cargo test -p roastty renderer::size
cargo test -p roastty renderer
cargo test -p roastty
```

Observed:

- `renderer::size`: 12 passed.
- Full `roastty`: 2227 unit tests passed (2215 prior + 12 new), plus the C ABI
  harness passed.
- `cargo fmt -p roastty -- --check`: clean.
- `cargo build -p roastty`: no warnings.
- No-`ghostty`-name gates passed for `roastty/src/renderer/size.rs` and for
  `roastty/src/lib.rs`, `roastty/include/roastty.h`,
  `roastty/tests/abi_harness.c`.
- `git diff --check`: clean.

No `Size`/`Coordinate`/`PaddingBalance` surface, and no C ABI, header, or ABI
inventory changes.

### Completion Review

Codex reviewed the completed implementation and found **no issues** ("nothing
should change before the result commit").

Review artifacts:

- Prompt: `logs/codex-review/20260602-071307-390291-prompt.md`
- Result: `logs/codex-review/20260602-071307-390291-last-message.md`

Codex confirmed, against upstream `renderer/size.zig`, that every method's
arithmetic is faithful (saturating subtraction in `sub_padding`/`blank_padding`,
`f32`/truncate/`max(1)` in `update`, `f32`/`floor`/`max(0)` in `balanced`,
`blank_padding` leftover on `right`/`bottom` only), that the `u16`→`u32` and
`f32` casts are correct in the realistic renderer domain, that the 12 tests are
solid (including the floor-proof and saturation-proof cases), and that the
`#![allow(dead_code)]` plus derived `PartialEq` with named `equals`/`eql`
wrappers is a clean port of the upstream surface.

## Conclusion

Experiment 224 succeeds. Roastty's `renderer::size` module now has the core
sizing value types and arithmetic — `CellSize`, `ScreenSize`, `GridSize`,
`Padding` — that every later renderer geometry slice depends on, with faithful
saturating/float semantics and 12 passing tests. Both Codex gates passed (three
design findings fixed before implementation, zero result findings).

The next slice (Experiment 225) is the rest of `renderer/size.zig`: the `Size`
aggregate (`grid()`, `terminal()`), the `PaddingBalance` enum with
`Size.balancePadding`, and the `Coordinate` union with `convert` (surface ↔
terminal ↔ grid coordinate conversion). Those build directly on the value types
landed here, and the upstream tests deferred from this experiment
(`Size.balancePadding` ×2 and `coordinate conversion`) port there.
