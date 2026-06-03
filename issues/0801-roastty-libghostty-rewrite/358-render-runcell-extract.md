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

# Experiment 358: decoding a terminal row into RunCells

## Description

The shaping subsystem is complete, but its input — `font::run::RunCell` — is
populated only in tests. This experiment builds the **renderer↔font bridge**: a
function that decodes a terminal page row into the `RunCell`s the `RunIterator`
consumes. Because roastty's terminal `Cell` accessors (`codepoint()`/
`content_tag()`/`wide()`/`style_id()`/`is_empty()`/`grapheme()`), the grapheme
lookup, and the style table are all `pub(super)` (terminal-internal), the
decoder lives in `terminal::page` — the only place with that access — and
produces `font::run::RunCell`s for the shaper.

## Upstream behavior

Upstream's shaper reads a row's cells via `shape.RunOptions.cells` — a
`MultiArrayList(terminal.RenderState.Cell).Slice` carrying each cell's content,
grapheme, and style. The `RenderState` is built from the terminal screen by the
renderer. roastty's terminal `Cell` is the packed page storage; the renderer
can't read it across the `pub(super)` boundary, so the terminal exposes a
decoder that produces the shaper's per-cell input directly (the `RunCell`s of
Experiment 355) — the same data, adapted to roastty's module boundary.

## Module coupling

`font::run` already imports `terminal::style` (Experiment 353), so `font` ↔
`terminal` are already coupled. This experiment adds the reverse reference
(`terminal::page` → `font::run::{RunCell, Wide}`) for the decoder. Rust permits
intra-crate mutual module references, and placing the decoder in
`terminal::page` is the only option that has `pub(super)` cell access without
widening a broad set of accessors to `pub(crate)`. The decoder is the **one**
terminal→font touch point; the alternative (widening ~10 `Cell`/`Page`
accessors + two enums to `pub(crate)` and building `RunCell`s in the renderer)
is broader and is noted as a possible later refactor.

## Rust mapping (`roastty/src/terminal/page.rs`)

```rust
use crate::font::run::{RunCell, Wide as RunWide};

impl Page {
    /// Decode row `y` into the [`RunCell`]s the font run iterator consumes: each
    /// cell's codepoint, grapheme codepoints, effective style and style id, wide
    /// kind, emptiness, and whether it is plain-codepoint content. The renderer
    /// wraps the result (adding the selection/cursor) into a
    /// `font::run::RunOptions`.
    pub(crate) fn shape_run_cells(&self, y: usize) -> Vec<RunCell> {
        let row = self.get_row(y);
        self.get_cells(row)
            .iter()
            .enumerate()
            .map(|(x, cell)| {
                let graphemes = if cell.has_grapheme() {
                    self.lookup_grapheme_at(x, y).unwrap_or_default()
                } else {
                    Vec::new()
                };
                let style_id = cell.style_id();
                // `RefCountedSet::get` asserts `id > 0`; the default id has no
                // stored entry, so it maps to the default style.
                let style = if style_id == style::DEFAULT_ID {
                    style::Style::default()
                } else {
                    self.get_style(style_id)
                };
                RunCell {
                    codepoint: cell.codepoint(),
                    graphemes,
                    style,
                    style_id,
                    wide: match cell.wide() {
                        Wide::Narrow => RunWide::Narrow,
                        Wide::Wide => RunWide::Wide,
                        Wide::SpacerTail => RunWide::SpacerTail,
                        Wide::SpacerHead => RunWide::SpacerHead,
                    },
                    is_empty: cell.is_empty(),
                    is_codepoint: matches!(
                        cell.content_tag(),
                        ContentTag::Codepoint | ContentTag::CodepointGrapheme
                    ),
                }
            })
            .collect()
    }
}
```

(`style_id` is `style::Id`; `RunCell.style_id` is the same width — converted if
the alias differs from `u16`.)

## Scope / faithfulness notes

- **Ported (bridged)**: the decode of a terminal page row into the shaper's
  per-cell input — codepoint, graphemes (via `lookup_grapheme_at`), the
  effective style (via `get_style(style_id)`) and `style_id`, the wide kind
  (mapped from `terminal::page::Wide` to `font::run::Wide`), emptiness, and the
  plain-codepoint flag (content tag `Codepoint`/`CodepointGrapheme`).
- **Faithful**: the per-cell data matches what upstream's run iterator reads off
  the `cells`/`graphemes`/`styles` slices; a cell with no grapheme bit carries
  an empty grapheme list; background-color cells (`BgColor*` tags) are not
  plain-codepoint content (`is_codepoint == false`).
- **Deferred**: assembling the `RunOptions` (adding the row's
  `selection`/`cursor_x` — a renderer/screen concern) and the draw-path wiring
  (feeding `TextRun`s + shaped glyphs into the renderer). (Consumed by tests
  now; the renderer caller is a later experiment.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/terminal/page.rs`: add `Page::shape_run_cells`; import
   `font::run::{RunCell, Wide}`.
2. Tests (in `page.rs`): build a small page, write cells (a plain `'A'`, an
   `'e'` with a combining grapheme via `set_graphemes_at`, and a styled cell),
   then assert `shape_run_cells(0)`:
   - the row length matches the column count;
   - `'A'` → `codepoint 'A'`, no grapheme, narrow, not empty, plain codepoint;
   - `'e' + U+0301` → `codepoint 'e'`, `graphemes == [0x0301]`,
     `has_grapheme()`;
   - the trailing unwritten cells are empty (`is_empty`, `codepoint 0`);
   - a styled cell's `style`/`style_id` round-trip via `get_style`.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty shape_run_cells
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Page::shape_run_cells` decodes a row into `RunCell`s carrying exactly the
  per-cell shaping data (codepoint, graphemes, style, style id, wide, empty,
  is-codepoint), faithful to what the run iterator reads;
- the decode tests pass, and the existing tests still pass;
- the `RunOptions` assembly and the draw-path wiring stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the decode omits or mis-maps a field the run
iterator needs (wrong wide mapping, missing graphemes, wrong is-codepoint), or
any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **two Required
findings**, both now fixed:

- **Required (fixed):** `self.get_style(style_id)` panics on unstyled cells —
  `style::DEFAULT_ID == 0` and `RefCountedSet::get` asserts `id > 0`. The decode
  now maps `style_id == DEFAULT_ID` to `style::Style::default()` and only calls
  `get_style` for a real id.
- **Required (fixed):** the per-cell grapheme gate used `cell.grapheme()`, but
  that is a `Row` flag — the `Cell` accessor is `cell.has_grapheme()`. Changed
  to `cell.has_grapheme()`.

Codex's rulings: the contained `terminal::page → font::run` dependency is the
right call here (`font::run` already depends on `terminal::style`; widening all
packed- cell internals to `pub(crate)` would be broader and leakier — keep the
narrow terminal-side bridge); the field set is correct for the `RunIterator`
(codepoint, graphemes, style, style_id, wide, is_empty, is_codepoint);
`is_codepoint = content_tag ∈ {Codepoint, CodepointGrapheme}` is right
(background- color cells → `false`); and `style::Id` is `StyleCountInt = u16`,
so the `style_id` is sound (a checked conversion is used defensively in case the
alias changes). One later-integration caveat (not for this experiment): `Page`
is `pub(super)`, so the draw-path wiring will need a terminal/`PageList`-facing
wrapper to call this from the renderer.

Review artifacts:

- Prompt: `logs/codex-review/20260603-165312-347589-prompt.md` (design)
- Result: `logs/codex-review/20260603-165312-347589-last-message.md` (design)
