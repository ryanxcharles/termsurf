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

# Experiment 401: the preedit placement loop

## Description

`add_preedit_cell` (Experiment 400) renders one preedit codepoint; this
experiment ports the **placement loop** that drives it — iterating the preedit
codepoints over the `PreeditRange`, computing each cell coordinate, and
advancing the column by the codepoint's width. Upstream walks
`preedit.codepoints[range.cp_offset..]` from the range's start column, calling
`addPreeditCell` at each `(x, y)` and advancing `x` by 2 for a wide codepoint,
else 1. This experiment adds `add_preedit`, reusing `add_preedit_cell` and the
already-ported `Preedit`/`PreeditRange` state (`renderer/state.rs`). The
integration into `rebuild_viewport` (computing the range/row from the cursor and
skipping the under-preedit cells) is deferred.

## Upstream behavior

In `rebuildCells` (`renderer/generic.zig`), after the cursor block:

```zig
// Setup our preedit text.
if (preedit) |preedit_v| {
    const range = preedit_range orelse break :preedit;
    var x = range.x[0];
    for (preedit_v.codepoints[range.cp_offset..]) |cp| {
        self.addPreeditCell(cp, .{ .x = x, .y = range.y }, state.colors.foreground)
            catch { … };
        x += if (cp.wide) 2 else 1;
    }
}
```

So: from the range's start column `range.x[0]`, iterate the codepoints starting
at `range.cp_offset` (the leading offset that fit the preedit into the available
cells); render each at column `x` on the range's row `range.y` with the default
foreground; advance `x` by the codepoint's cell width (2 wide / 1 narrow).

## Rust mapping (`roastty/src/renderer/cell.rs`)

```rust
/// Place a `preedit`'s codepoints over the cursor: from `range.start`, render each
/// codepoint (from `range.cp_offset` onward) via [`add_preedit_cell`] at `(x, y)`
/// with `screen_fg`, advancing `x` by the codepoint's cell width (2 wide / 1
/// narrow). `y`/`cols` are the cursor row and the row's column count. Faithful port
/// of upstream's preedit placement loop in `rebuildCells`.
pub(crate) fn add_preedit(
    contents: &mut Contents,
    grid: &mut SharedGrid,
    preedit: &Preedit,
    range: PreeditRange,
    y: u16,
    cols: u16,
    screen_fg: [u8; 3],
) -> Result<(), ResolverRenderError> {
    let mut x = range.start;
    for cp in &preedit.codepoints[range.cp_offset..] {
        add_preedit_cell(contents, grid, cp.codepoint, cp.wide, [x, y], cols, screen_fg)?;
        x += if cp.wide { 2 } else { 1 };
    }
    Ok(())
}
```

`Preedit`/`PreeditRange`/`Codepoint` (`renderer/state.rs`, already ported) are
imported. The `cp_offset` slices the codepoints (upstream
`codepoints[cp_offset..]`); `x` starts at `range.start` (upstream `range.x[0]`)
and advances by the width. `range.y` is the separate `y` parameter (roastty's
`PreeditRange` carries the column `start`/`end`, not the row).

## Scope / faithfulness notes

- **Ported (bridged)**: `add_preedit` — the preedit placement loop driving
  `add_preedit_cell` over the `PreeditRange`.
- **Faithful**: the loop starts `x` at `range.start`, iterates the codepoints
  from `range.cp_offset`, renders each at `[x, y]` via `add_preedit_cell`
  (Experiment 400), and advances `x` by `if cp.wide { 2 } else { 1 }` —
  upstream's exact loop. The row `y` and the foreground `screen_fg` (upstream's
  `range.y` and `state.colors.foreground`) are parameters.
- **Faithful adaptation**: roastty's `PreeditRange` carries the column
  `start`/`end` (upstream `range.x[0..1]`) but not the row, so the cursor row
  `y` is a parameter; `cols` is passed through to `add_preedit_cell`. The loop
  uses `?` (propagating a render error), where upstream catches and logs per
  cell and continues — the same established renderer-helper adaptation; only the
  final assembly (where a per-cell failure should not abort the row) would
  revisit this, which is deferred with the integration.
- **Deferred**: the integration into `rebuild_viewport` — computing the
  `PreeditRange` (via `Preedit::range`) and the row from the cursor viewport,
  the default foreground, and skipping the cells **under** the preedit; the
  Metal upload. (Consumed by tests now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`: add the `add_preedit` function; import
   `Preedit`/`PreeditRange` from `renderer/state.rs`.
2. Tests (in `cell.rs`):
   - a two-narrow-codepoint preedit (`'A'`, `'B'`) with `range.start = 1`,
     `cp_offset = 0`, `y = 0`, `cols = 8` → glyphs at columns 1 and 2 (each with
     its underline), i.e. four foreground cells with glyph columns `[1, 2]`;
   - `cp_offset = 1` skips the leading codepoint → only the second codepoint
     renders, at `range.start`;
   - a **wide**-then-narrow preedit (`['A' wide, 'B' narrow]`) at
     `range.start = 0` → `'A'` at column 0 (with a second underline at column
     1), then `'B'` at column **2** (x advanced by 2 for the wide codepoint).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty add_preedit
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `add_preedit` places the preedit codepoints from
  `range.start`/`range.cp_offset`, rendering each via `add_preedit_cell` at
  `[x, y]` and advancing `x` by the codepoint width — faithful to upstream's
  placement loop;
- the tests pass (the two-codepoint columns; the `cp_offset` skip; the wide
  advance), and the existing tests still pass;
- the `rebuild_viewport` integration and the Metal upload stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the placement is wrong (the wrong start column, the
`cp_offset` slice off, the width advance wrong), or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the loop is faithful — `x` starts at `range.start`,
iteration begins at `preedit.codepoints[range.cp_offset..]`, each codepoint
renders at `[x, y]`, and `x` advances by 2 for wide and 1 otherwise — and that
passing `y` and `screen_fg` as parameters is the right adaptation (roastty's
`PreeditRange` carries only the column range and `cp_offset`, while upstream's
row and foreground come from the surrounding render state). It confirmed the
`cp_offset` slice is sound when `range` comes from `Preedit::range` (that
construction keeps it within the codepoint vector bounds), and that the `?`
propagation is an acceptable bounded-slice adaptation consistent with the
renderer-helper pattern documented for Experiment 400 — with the caveat that the
final rebuild integration can revisit the swallow-and-continue behavior if
needed. It judged the tests sufficient (start placement, `cp_offset`, wide
advancement).

Review artifacts:

- Prompt: `logs/codex-review/20260604-061958-124017-prompt.md` (design)
- Result: `logs/codex-review/20260604-061958-124017-last-message.md` (design)

## Result

**Result:** Pass

The preedit placement loop is now ported.

- `roastty/src/renderer/cell.rs`:
  `add_preedit(contents, grid, preedit, range, y, cols, screen_fg)` —
  `let mut x = range.start; for cp in &preedit.codepoints[range.cp_offset..] { add_preedit_cell(.., cp.codepoint, cp.wide, [x, y], cols, screen_fg)?; x += if cp.wide { 2 } else { 1 }; }`.
  Imports `Preedit`/`PreeditRange` from `renderer/state.rs` (now live — removing
  some dead-code). `pub(crate)` and not yet called in production (the
  `rebuild_viewport` integration is deferred), reachable in the library crate,
  so no dead-code warning.

Test (in `cell.rs`): `add_preedit_places_codepoints_with_widths` — two narrow
codepoints from start 1 → glyph-pos columns `[1, 1, 2, 2]` (glyph + underline
per cell); `cp_offset = 1` skips the leading codepoint → only the second renders
at column 1 (`[1, 1]`); a wide-then-narrow preedit from start 0 →
`[0, 0, 1, 2, 2]` (`'A'` glyph@0, underline@0, second underline@1, then
`x += 2`, `'B'` glyph@2, underline@2).

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2860 passed, 0 failed (+1, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer) clean; `git diff --check` clean.

## Conclusion

The preedit placement loop is now ported faithfully as `add_preedit` — from the
range's start column, it renders each codepoint (from `cp_offset`) over the
cursor via `add_preedit_cell`, advancing the column by the codepoint's width.
With the preedit cell (Experiment 400), this loop, and the
`Preedit`/`PreeditRange` state, the preedit rendering is complete; the only
remaining preedit work is the **integration** into `rebuild_viewport` (computing
the range/row from the cursor viewport, the default foreground, and skipping the
under-preedit cells), which belongs with the broader rebuild assembly.

The remaining renderer-bridge work: the `rebuild_viewport` integration of the
cursor + preedit (and the under-preedit cell skipping); and the **Metal upload**
of `Contents` (the GPU buffer/uniform upload — the renderer boundary, for which
roastty already has the buffer/uniform types).

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed the implementation matches the approved design:
`x` starts at `range.start`, iteration begins at
`preedit.codepoints[range.cp_offset..]`, each codepoint renders through
`add_preedit_cell` at `[x, y]`, and `x` advances by 2 for wide and 1 for narrow
— with `y`/`cols`/`screen_fg` passed separately as the right adaptation around
roastty's column-only `PreeditRange`. It confirmed the tests cover the key
placement cases (consecutive narrow cells, `cp_offset` skipping, and wide- cell
advancement with the second underline occupying the next column), with the
change internal Rust only and no public C ABI/header impact. Nothing needed to
change before the result commit.

Review artifacts:

- Result review: `logs/codex-review/20260604-062225-311539-last-message.md`
