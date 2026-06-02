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

# Experiment 229: Port Renderer `constraint_width`

## Description

Port `constraintWidth` from upstream `renderer/cell.zig` into the
`renderer::cell` module — the last standalone function in `cell.zig` before the
`Contents` builder. It decides the `constraint_width` used when rendering a
cell's glyph: symbol-like glyphs may extend to two cells when there is room and
the previous glyph was not also a (non-graphics) symbol.

Upstream signature and logic:

```
pub fn constraintWidth(raw_slice: []const terminal.page.Cell, x: usize, cols: usize) u2 {
    const cell = raw_slice[x];
    const cp = cell.codepoint();
    const grid_width = cell.gridWidth();
    if (grid_width > 1) return grid_width;          // wide cells are always 2
    if (!isSymbol(cp)) return grid_width;           // non-symbols use grid width
    if (x == cols - 1) return 1;                     // end of screen -> 1
    if (x > 0) {                                      // prev symbol (non-graphics) -> 1
        const prev_cp = raw_slice[x - 1].codepoint();
        if (isSymbol(prev_cp) and !isGraphicsElement(prev_cp)) return 1;
    }
    const next_cp = raw_slice[x + 1].codepoint();    // next blank/space -> 2
    if (next_cp == 0 or isSpace(next_cp)) return 2;
    return 1;
}
```

It reads the codepoint of cells `x-1`, `x`, `x+1` and the grid width of cell
`x`. `is_symbol`, `is_graphics_element`, and `is_space` all landed in
Experiments 227–228. The new dependency is **reading codepoint and grid width
from a cell**.

### Design decision: cell access

Roastty's packed cell is `terminal::page::Cell` (a `u64`); the unpacking lives
in its `pub(super)` `codepoint()` and `grid_width()` accessors, reachable only
within the `terminal` module. `renderer::cell` cannot read them today, and the
render snapshot's cell (`RenderStateCellSnapshot.raw`) is the same raw `u64`
with no renderer-visible unpacking. Two faithful options:

1. **Operate on page cells (recommended).** Widen the minimal surface —
   `terminal::page::Cell` and its `codepoint()` / `grid_width()` accessors — to
   `pub(crate)`, and make `constraint_width` take `&[Cell]` exactly like
   upstream's `[]const terminal.page.Cell`. This is faithful to upstream, keeps
   the cell bit-layout single-sourced in `terminal::page`, and exposes only the
   two read accessors the renderer needs (not the mutators). Blast radius: three
   visibility widenings in `terminal` (the `page` module path or a
   `pub(crate) use page::Cell` re-export, plus the two accessors).

2. **Renderer-local view.** Define a small
   `CellInfo { codepoint: u32, grid_width: u8 }` and have `constraint_width`
   take `&[CellInfo]`, with callers building the view. This avoids touching
   `terminal` visibility but diverges from the upstream signature and pushes an
   extra mapping onto every future caller; it also has no real consumer yet to
   define the mapping.

This experiment uses **Option 2** (revised after design review). Option 1 lets
the renderer _read_ cells, but the `renderer::cell` tests cannot _construct_
them: `Cell::init`, `set_wide`, and `Wide` are all `pub(super)` to the terminal
module, so Option 1 would force either a test-only constructor or moving the
behavior tests out of `renderer::cell` — i.e. widening more than the stated
minimal read surface. `constraint_width` is a **pure** function whose only
future caller (the `Contents` builder) does not exist yet, so binding it to
`terminal::page::Cell` now is premature coupling with no consumer to justify it.

Instead, `constraint_width` takes a slice of a renderer-local
`CellInfo { codepoint: u32, grid_width: u8 }` — exactly the per-cell data
upstream reads (`codepoint()` of `x-1`/`x`/`x+1`, `gridWidth()` of `x`). The
branch **logic is byte-for-byte faithful**; only the input adapter differs, an
idiomatic-Rust adaptation the issue permits. When the `Contents` builder is
ported it will map its real cell source (page cells or render snapshot) into
`CellInfo` at the call site — and that slice will decide, with a concrete
consumer in hand, whether to widen `terminal::page::Cell` visibility. This keeps
the present slice self-contained, trivially testable, and touches no other
module.

### Faithfulness and scope notes

- Upstream returns `u2`; Rust has no `u2`, so the port returns `u8` (values 1 or
  2, or the cell's grid width). `grid_width()` already returns `u8` (1 or 2).
- `next_cp == 0` (blank cell) and `is_space(next_cp)` both allow width 2.
- The function assumes `x < cols` and `raw_slice.len() >= cols`, and only reads
  `x + 1` when `x != cols - 1` (so it never indexes out of bounds), matching
  upstream. The port preserves those access bounds exactly.
- Do **not** port the `Contents` builder, `Key`, or `CellType` (shader/font
  dependencies) — those are later slices.
- No C ABI, header, or ABI inventory changes; no new dependencies.

## Changes

1. `roastty/src/renderer/cell.rs`:
   - Add `pub(crate) struct CellInfo { pub codepoint: u32, pub grid_width: u8 }`
     (`Debug, Clone, Copy, PartialEq, Eq`) — the per-cell view
     `constraint_width` reads.
   - Add
     `pub(crate) fn constraint_width(raw_slice: &[CellInfo], x: usize, cols: usize) -> u8`
     reproducing the upstream branch order exactly (wide early-return →
     non-symbol returns grid width → last column → previous non-graphics symbol
     → next blank/space → `1`), using the already-ported `is_symbol`,
     `is_graphics_element`, and `is_space`. Reads `raw_slice[x]`,
     `raw_slice[x-1]` (only when `x > 0`), and `raw_slice[x+1]` (only when
     `x != cols - 1`), matching upstream's bounds.
   - No `terminal` module changes.

2. Tests in `renderer/cell.rs`, building `CellInfo` slices directly (no terminal
   path needed). Port the spirit of upstream "Cell constraint widths" plus
   targeted cases:
   - a wide cell (`grid_width = 2`) returns 2 regardless of neighbors;
   - a non-symbol narrow cell returns its grid width (1);
   - a symbol at the last column returns 1;
   - a symbol preceded by a non-graphics symbol returns 1;
   - a symbol preceded by a graphics-element symbol is **not** constrained by
     the previous-symbol rule (proceeds to the next-cell check);
   - a symbol followed by a blank (`codepoint == 0`) returns 2;
   - a symbol followed by a space (`U+0020`) returns 2;
   - a symbol followed by a non-blank, non-space cell returns 1;
   - a symbol followed by a no-break space (`U+00A0`) returns 1 — the upstream
     NBSP guard proving `is_space` is the narrow predicate, not general Unicode
     whitespace.

3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo test -p roastty renderer::cell
cargo test -p roastty
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/renderer/cell.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `constraint_width` reproduces the upstream branch order and return values
  exactly, over the `CellInfo` view (Option 2), with no `terminal` module
  change;
- the ported "Cell constraint widths" behavior and the targeted cases pass,
  including the NBSP guard;
- no `Contents`/`Key`/`CellType` scope leaks in;
- no C ABI, header, or ABI inventory changes;
- `cargo fmt` accepted and `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if `constraint_width` turns out to need cell data
beyond `codepoint`/`grid_width` that `CellInfo` does not carry.

The experiment **fails** if the branch order or return values diverge from
upstream, if it prematurely couples to `terminal::page::Cell`, or if any public
C API/ABI changes.

## Design Review

Codex reviewed this design before implementation.

Review artifacts:

- Prompt: `logs/codex-review/20260602-074412-751097-prompt.md`
- Result: `logs/codex-review/20260602-074412-751097-last-message.md`

Codex confirmed the branch order and return logic are correctly captured (wide
early-return, non-symbol, last-column, previous non-graphics symbol, next
blank/space expansion, final `1`), that the `x + 1` access is safe because the
last-column check precedes it, that returning `u8` for `u2` is fine, and that
`next_cp == 0` correctly models a blank cell.

Two real findings, fixed in the design above before this commit:

1. **(Medium)** Option 1's minimal read-only widening was insufficient for the
   `renderer::cell` tests to _construct_ cells (`Cell::init`/`set_wide`/`Wide`
   are `pub(super)`), so it would have pressured broader terminal widening. The
   design switched to **Option 2** (a renderer-local `CellInfo` view), which is
   faithful in logic, needs no terminal change, and is trivially testable. The
   page-cell binding is deferred to the future `Contents` builder slice, which
   will have a concrete consumer to justify any visibility decision.
2. **(Low)** added the upstream NBSP case (a symbol followed by `U+00A0` returns
   1), which guards that `is_space` stays the narrow predicate rather than
   general Unicode whitespace.
