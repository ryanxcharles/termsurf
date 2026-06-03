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

# Experiment 355: the run iterator's input view

## Description

`RunIterator.next()` reads a terminal row's cells via three parallel slices
(`cells`/`graphemes`/`styles`) plus the run options (selection, cursor). In
roastty the terminal `Cell` accessors (`codepoint()`/`content_tag()`/`wide()`)
and the `Wide` enum are `pub(super)` — private to the `terminal` module — so the
`font` run iterator cannot read terminal cells directly. This experiment models
the **input the run iterator reads**: a `RunCell` (the decoded per-cell data)
and `RunOptions` (the row's cells plus the selection/cursor) in `font/run.rs`.
The renderer (which has terminal-internal access) populates these; the
cell-walking `next()` loop (deferred) iterates them.

## Upstream behavior (`shaper/run.zig`, `shape.zig`)

```zig
// shape.RunOptions: the input to a run iterator.
pub const RunOptions = struct {
    grid: *SharedGrid,
    cells: MultiArrayList(terminal.RenderState.Cell).Slice,  // cells/graphemes/styles
    selection: ?[2]u16 = null,
    cursor_x: ?usize = null,
};

// next() reads, per cell `j`: cells[j].isEmpty(), cells[j].hasStyling(),
// styles[j], cells[j].wide, cells[j].content_tag == .codepoint,
// cells[j].codepoint(), cells[j].hasGrapheme(), graphemes[j], cells[j].style_id.
```

The run iterator reads exactly: each cell's **codepoint**, its **grapheme**
codepoints (empty if none), its effective **style** and **style id**, its **wide
kind** (to skip spacers), whether it is **empty**, and whether its content is a
plain **codepoint** (for the bad-ligature guard). Plus the **selection** bounds
and **cursor** column from the options.

## Rust mapping (`roastty/src/font/run.rs`)

```rust
/// The wide kind of a cell: a normal narrow/wide cell, or a spacer that pads a
/// wide cell or wraps a line. Mirrors `terminal::page::Wide` (which is
/// `pub(super)`); the renderer maps the terminal value to this. Faithful port of
/// the `Wide` cases the run iterator distinguishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Wide {
    #[default]
    Narrow,
    Wide,
    SpacerTail,
    SpacerHead,
}

/// The decoded per-cell data the run iterator reads from a terminal row — what
/// upstream reads off the `cells`/`graphemes`/`styles` slices. The renderer
/// extracts this from a terminal `Cell` (whose accessors are `pub(super)`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RunCell {
    /// The cell's primary Unicode scalar (`0` for an empty cell).
    pub codepoint: u32,
    /// The grapheme's additional codepoints, in order (empty if not a grapheme).
    pub graphemes: Vec<u32>,
    /// The cell's effective style (the default style if the cell is unstyled).
    pub style: TermStyle,
    /// The cell's style id (the fast-path equality `next()` uses before the
    /// `comparable_style` comparison).
    pub style_id: u16,
    /// The cell's wide kind (spacers are skipped by the run iterator).
    pub wide: Wide,
    /// Whether the cell is empty (rendered as a space).
    pub is_empty: bool,
    /// Whether the cell's content is a plain codepoint (vs a background-color
    /// cell) — the guard for the bad-ligature break.
    pub is_codepoint: bool,
}

impl RunCell {
    /// Whether the cell carries a grapheme (additional combined codepoints).
    pub(crate) fn has_grapheme(&self) -> bool {
        !self.graphemes.is_empty()
    }
}

/// The input to a run iterator: a terminal row's decoded cells plus the run
/// breaks. Faithful port of `shape.RunOptions` — `grid` is omitted (roastty
/// passes the `CodepointResolver` separately) and the cells are pre-decoded
/// `RunCell`s rather than a terminal `MultiArrayList` slice.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RunOptions {
    /// The row's cells, **left to right, including empty cells** (the run
    /// iterator's trimming, offset, and cluster math depend on positional
    /// indexes).
    pub cells: Vec<RunCell>,
    /// The `[start, end]` selection column bounds in this row, if any (a run
    /// breaks at a selection boundary).
    pub selection: Option<[u16; 2]>,
    /// The cursor's column in this row, if any (a run breaks around the cursor).
    pub cursor_x: Option<u16>,
}
```

## Scope / faithfulness notes

- **Ported**: the run iterator's input — `RunCell` (the decoded per-cell data
  `next()` reads) and `RunOptions` (the row cells + selection + cursor), plus
  the `Wide` cases the iterator distinguishes.
- **Faithful**: `RunCell` carries exactly the fields `next()` reads off a
  terminal cell (codepoint, graphemes, style, style id, wide, empty,
  is-codepoint); `RunOptions` mirrors `shape.RunOptions`'s
  `cells`/`selection`/`cursor_x`.
- **Faithful adaptation**: roastty pre-decodes cells into `RunCell`s (the
  renderer does this, since the terminal `Cell` accessors are `pub(super)`)
  rather than iterating a terminal `MultiArrayList` slice; and `RunOptions`
  omits the `grid: *SharedGrid` pointer (roastty passes the `CodepointResolver`
  to `next()` separately). `cursor_x` is a `u16` column (a terminal column fits
  `u16`).
- **Deferred** (to `next()`): the cell-walking loop that consumes a `RunOptions`
  and emits `TextRun`s; the renderer code that builds `RunCell`s from terminal
  cells. (Consumed by tests now; `#![allow(dead_code)]` covers the not-yet-wired
  path.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/font/run.rs`: add the `Wide` enum, the `RunCell` struct (with
   `has_grapheme`), and the `RunOptions` struct.
2. Tests (in `run.rs`):
   - `run_cell_has_grapheme`: a `RunCell` with empty `graphemes` reports
     `has_grapheme() == false`; one with codepoints reports `true`.
   - `run_options_default`: `RunOptions::default()` has no cells, no selection,
     and no cursor.
   - `run_cell_construction`: a `RunCell` constructs and round-trips its fields
     (and `RunOptions` holds a vector of them).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty run_cell
cargo test -p roastty run_options
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `RunCell`/`RunOptions`/`Wide` model exactly the input the run iterator reads
  (the enumerated fields), faithful to `shape.RunOptions` and the terminal cell;
- the has-grapheme, default, and construction tests pass, and the existing tests
  still pass;
- the cell-walking `next()` and the renderer's `RunCell` extraction stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the input view omits a field `next()` needs or
carries a wrong shape, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
findings**. It confirmed `RunCell` carries the data `next()` reads (codepoint,
graphemes, effective style, `style_id`, `wide`, emptiness, the plain-codepoint
guard); the `style_id` `u16` and the four `Wide` variants line up with terminal;
`has_styling` is derivable from `style_id != 0` and the invisible handling from
`style.flags.invisible` (as long as the renderer supplies the default style for
unstyled cells); omitting `grid` from `RunOptions` is sound (future `next()`
takes the `CodepointResolver` separately); pre-decoding into `RunCell` is the
right adaptation across the `pub(super)` boundary; the kitty placeholder needs
no separate field (the `codepoint` preserves it, and a placeholder constant can
be compared later); and `cursor_x: Option<u16>` / `selection: Option<[u16; 2]>`
are the right column scale.

The one caveat it raised — that future producers must pass row cells
**left-to-right including empty cells** (trimming, offset, and cluster math
depend on positional indexes) — is now documented on the `RunOptions.cells`
field.

Review artifacts:

- Prompt: `logs/codex-review/20260603-151526-810944-prompt.md` (design)
- Result: `logs/codex-review/20260603-151526-810944-last-message.md` (design)
