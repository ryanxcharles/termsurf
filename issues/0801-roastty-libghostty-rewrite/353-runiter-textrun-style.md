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

# Experiment 353: the run iterator's TextRun and style break

## Description

The `RunIterator` (upstream `shaper/run.zig`) emits a `TextRun` per run and
breaks a run when a cell's style changes (allowing background-color differences
via `comparableStyle`). This experiment ports the **`TextRun` value type** (the
run iterator's output) and the **`comparableStyle`** helper into `font/run.rs`,
completing the run iterator's scaffolding before the cell-walking `next()` loop.

## Upstream behavior (`shaper/run.zig`)

```zig
pub const TextRun = struct {
    hash: u64,                       // position-independent content hash (cache key)
    offset: u16,                     // the run's start column in the row
    cells: u16,                      // number of cells the run produced
    grid: *font.SharedGrid,          // the grid that built the run
    font_index: font.Collection.Index, // the font for the run's glyphs
};

/// A style that, when compared, must be identical for a run to continue.
fn comparableStyle(style: terminal.Style) terminal.Style {
    var s = style;
    // Background colors may differ — the cell background is painted regardless,
    // and the glyph lands on top in the glyph's color.
    s.bg_color = .none;
    return s;
}
```

`next()` compares `comparableStyle(prev) == comparableStyle(cur)`; a run breaks
when they differ (i.e. any style attribute other than `bg_color` changed).

## Rust mapping (`roastty/src/font/run.rs`)

```rust
use crate::font::collection::Index;
use crate::terminal::style::{Color, Style as TermStyle};

/// A single text run produced by the run iterator: one row's worth of cells that
/// share a font and comparable style. Faithful port of upstream `TextRun` — the
/// `grid` pointer is omitted (roastty resolves the face from `font_index` at the
/// call site rather than carrying the grid).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TextRun {
    /// A position-independent content hash, for caching shaping results.
    pub hash: u64,
    /// The run's start column in the row (added to each shaped cell's `x`).
    pub offset: u16,
    /// The number of cells the run produced.
    pub cells: u16,
    /// The font index for the run's glyphs.
    pub font_index: Index,
}

/// The style that must be identical for a run to continue: the cell's style with
/// the background color cleared. Background colors may differ within a run — the
/// cell background is painted regardless and the glyph lands on top in its own
/// color. Faithful port of upstream `comparableStyle`.
pub(crate) fn comparable_style(mut style: TermStyle) -> TermStyle {
    style.bg_color = Color::None;
    style
}
```

## Scope / faithfulness notes

- **Ported**: the `TextRun` value type and `comparableStyle` — the run
  iterator's output and its style-break comparison.
- **Faithful**: `comparable_style` clears only `bg_color` (leaving `fg_color`,
  `underline_color`, and `flags` intact), so two styles differing **only** in
  background compare equal — matching upstream; the `TextRun` fields (`hash`,
  `offset`, `cells`, `font_index`) match upstream.
- **Faithful simplification**: `TextRun` omits upstream's `grid: *SharedGrid`
  pointer — roastty has no `SharedGrid`; the caller (the eventual `next()` /
  shaper wiring) resolves the face from `font_index` via the
  `CodepointResolver`.
- **Deferred** (to `next()`): producing a `TextRun` (computing `hash`/`offset`/
  `cells`), the cell-walking loop that calls `comparable_style`, and the
  selection/cursor/spacer breaks. (Consumed by tests now; `#![allow(dead_code)]`
  covers the not-yet-wired path.)
- No C ABI/header/ABI-inventory change (internal Rust). This adds the first
  `font` → `terminal` dependency (`crate::terminal::style`), expected since the
  run iterator bridges the two (upstream `run.zig` imports `terminal`).

## Changes

1. `roastty/src/font/run.rs`: add the `TextRun` struct and `comparable_style`;
   import `crate::terminal::style::{Color, Style}` and
   `crate::font::collection::Index`.
2. Tests (in `run.rs`):
   - `comparable_style_clears_bg`: a style with a non-`None` `bg_color` has it
     cleared to `Color::None`, with `fg_color`/`underline_color`/`flags`
     unchanged.
   - `comparable_style_bg_only_equal`: two styles differing **only** in
     `bg_color` are equal after `comparable_style`; two differing in `fg_color`
     remain unequal.
   - `text_run_fields`: a `TextRun` constructs and round-trips its fields (and
     is `Copy`/`PartialEq`).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty comparable_style
cargo test -p roastty text_run
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `comparable_style` clears only `bg_color` (faithful to upstream's
  `comparableStyle`), and `TextRun` carries
  `hash`/`offset`/`cells`/`font_index`;
- the clears-bg, bg-only-equal, and text-run tests pass, and the existing tests
  still pass;
- the cell-walking `next()` and the selection/cursor/spacer breaks stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if `comparable_style` clears the wrong field(s), the
`TextRun` shape diverges, or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
findings**. It confirmed: the `TextRun` shape is a clean port —
`hash`/`offset`/`cells`/`font_index` preserve the run data, and omitting
upstream's `grid: *SharedGrid` is appropriate (roastty has no `SharedGrid` and
routes face resolution through `CodepointResolver`/`Index`); `comparable_style`
clearing only `bg_color` is faithful (it must leave
`fg_color`/`underline_color`/`flags` untouched); the `font` → `terminal::style`
dependency is acceptable (the run iterator is the terminal↔font bridge) and the
privacy works (`terminal::style`, `Style`, and `Color` are all `pub(crate)`);
and deferring `next()` production, the `hash`/`offset`/`cells` computation, and
the selection/cursor/spacer breaks is clean. It flagged one implementation
detail (already in the design): alias `terminal::Style` as `TermStyle` (via
`Style as TermStyle`) to avoid colliding with `crate::font::Style`, which
`font_style` already uses in `run.rs`.

Review artifacts:

- Prompt: `logs/codex-review/20260603-150153-572145-prompt.md` (design)
- Result: `logs/codex-review/20260603-150153-572145-last-message.md` (design)

## Result

**Result:** Pass

The run iterator's output type and style-break helper are ported.

- `roastty/src/font/run.rs`: added `TextRun { hash, offset, cells, font_index }`
  (the run iterator's output; the upstream `grid` pointer is omitted — roastty
  resolves the face from `font_index` via the `CodepointResolver`) and
  `comparable_style(style)` (clears `bg_color` so only background differences
  are ignored for run continuation). Imported `crate::font::collection::Index`
  and `crate::terminal::style::{Color, Style as TermStyle}` — the first `font` →
  `terminal` dependency, aliased to avoid colliding with the `font::Style`
  already used by `font_style`.

Tests: `comparable_style_clears_bg` (`bg_color` → `None`, fg/underline/flags
unchanged), `comparable_style_bg_only_equal` (background-only difference
compares equal; a foreground difference does not), `text_run_fields`
(construction, `Copy`, `PartialEq`). All pass.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2786 passed, 0 failed (+3, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The run iterator's scaffolding is complete: its output (`TextRun`), the
font-resolution core (`index_for_grapheme`, Exp 351), and all the per-cell
decision helpers
(`font_style`/`is_bad_ligature_break`/`presentation_for_grapheme`, Exp 352;
`comparable_style`, this experiment) are in place in `font/run.rs`.

The one remaining piece is the cell-walking `RunIterator.next()` loop itself: it
walks a terminal row's cells (a `RunOptions`/cells input over `terminal/page.rs`
`Cell`s), extracts each cell's codepoint/graphemes/style/wide-kind, applies
these now-ported helpers and breaks, accumulates the `(codepoint, cluster)`
stream and the position-independent hash, and emits a `TextRun`. Every decision
it makes is now ported and gated; what's left is the iteration that reads the
terminal grid.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no Required findings**. It confirmed `comparable_style` clears only `bg_color`
(preserving foreground, underline, and flags), matching upstream; `TextRun`
carries `hash`/`offset`/`cells`/`font_index` with the `grid` omission being a
sound roastty simplification (no `SharedGrid`; resolution goes through
`font_index` + the resolver); the `TermStyle` alias avoids the `font::Style`
collision and the `font` → `terminal::style` dependency is appropriate for this
run-iterator bridge; and the deferred `next()` loop, hash production, and
selection/cursor/spacer breaks are intact (no accidental wiring). It ran the
targeted tests — both filters passed.

Review artifacts:

- Result review: `logs/codex-review/20260603-150413-288896-last-message.md`
