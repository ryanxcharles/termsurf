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

# Experiment 351: the run iterator's grapheme font resolution

## Description

The `RunIterator` (upstream `shaper/run.zig`) walks a terminal row, groups cells
into runs by font index, and feeds each run to the shaper. Its per-cell font
resolution — `indexForCell` — finds the font index that can render a cell,
including a multi-codepoint **grapheme** (it must find one font that supports
_every_ codepoint of the grapheme). This experiment ports that resolution core,
built on roastty's existing `CodepointResolver` (`get_index`), plus a
`has_codepoint` wrapper (the `SharedGrid.hasCodepoint` analog). The
terminal-cell extraction (`cell.codepoint()`/`hasGrapheme()`), the kitty
placeholder, and the row cell-grouping stay deferred to the `RunIterator`
proper.

## Upstream behavior (`shaper/run.zig` `indexForCell`)

```zig
fn indexForCell(self, alloc, cell, graphemes, style, presentation) !?Index {
    if (cell.isEmpty() or cell.codepoint() == 0 or cell.codepoint() == …placeholder)
        return grid.getIndex(' ', style, presentation);     // empty → space

    const primary_cp = cell.codepoint();
    const primary = grid.getIndex(primary_cp, style, presentation) orelse return null;
    if (!cell.hasGrapheme()) return primary;                 // common: single cp

    // A grapheme: collect a font index per codepoint, then find one that covers
    // them all.
    var candidates = .{primary};
    for (graphemes) |cp| {
        if (cp == 0xFE0E or cp == 0xFE0F or cp == 0x200D) continue;  // skip ZWJ/VS
        candidates.append(grid.getIndex(cp, style, null) orelse return null);
    }
    for (candidates) |idx| {
        if (!grid.hasCodepoint(idx, primary_cp, presentation)) continue;
        for (graphemes) |cp| {
            if (cp == 0xFE0E or cp == 0xFE0F or cp == 0x200D) continue;
            if (!grid.hasCodepoint(idx, cp, null)) break;
        } else return idx;   // this candidate covers every codepoint
    }
    return null;
}
```

The primary codepoint uses the cell's `presentation`; grapheme components use
`null` (default presentation). Emoji ZWJ/variation selectors (`U+200D`,
`U+FE0E`, `U+FE0F`) are skipped — emoji fonts commonly lack them as standalone
glyphs. A grapheme renders only if **one** candidate font covers the primary and
all (non-skipped) components.

## Rust mapping (`roastty/src/font/codepoint_resolver.rs`)

- Extract the `Option<Presentation>` → `PresentationMode` mapping currently
  inline in `get_index` into a free `presentation_mode(cp, p)` (the `get_index`
  mapping: `None` → `Default(emoji/text)` from the UCD) and reuse it in
  `get_index` (no behavior change).
- Add the `SharedGrid.hasCodepoint` analog — which maps presentation
  **differently**: upstream's `hasCodepoint` treats `null` as `Any` (not the UCD
  default), so a grapheme component need only be _present_ in the face, not
  match its default presentation (emoji fonts commonly cover ZWJ-combined
  components without their standalone default presentation):

  ```rust
  /// `hasCodepoint`'s presentation mapping: an explicit presentation is required,
  /// but `None` accepts any presentation. (Differs from `presentation_mode`, which
  /// uses the UCD default for `None`.)
  fn has_codepoint_mode(p: Option<Presentation>) -> PresentationMode {
      match p {
          Some(v) => PresentationMode::Explicit(v),
          None => PresentationMode::Any,
      }
  }

  /// Whether the face at `idx` covers `cp` for the requested presentation
  /// (`None` ⇒ any). Faithful analog of upstream `SharedGrid.hasCodepoint`.
  pub(crate) fn has_codepoint(&self, idx: Index, cp: u32, p: Option<Presentation>) -> bool {
      self.collection.has_codepoint(idx, cp, has_codepoint_mode(p))
  }
  ```

- Port `indexForCell`'s core (taking the already-extracted `primary_cp` +
  `graphemes`, not a terminal `Cell`):
  ```rust
  /// The font index that renders `primary_cp` and, if `graphemes` is non-empty,
  /// every codepoint of the grapheme — a font covering them all. `primary_cp == 0`
  /// resolves the space cell. Faithful port of upstream `indexForCell` (the
  /// terminal-`Cell` extraction and kitty placeholder are the `RunIterator`'s).
  pub(crate) fn index_for_grapheme(
      &mut self,
      primary_cp: u32,
      graphemes: &[u32],
      style: Style,
      presentation: Option<Presentation>,
  ) -> Option<Index> {
      const ZWJ_VS: [u32; 3] = [0x200D, 0xFE0E, 0xFE0F];
      if primary_cp == 0 {
          return self.get_index(' ' as u32, style, presentation);
      }
      let primary = self.get_index(primary_cp, style, presentation)?;
      if graphemes.is_empty() {
          return Some(primary);
      }
      let mut candidates = vec![primary];
      for &cp in graphemes {
          if ZWJ_VS.contains(&cp) {
              continue;
          }
          candidates.push(self.get_index(cp, style, None)?);
      }
      for &idx in &candidates {
          if !self.has_codepoint(idx, primary_cp, presentation) {
              continue;
          }
          if graphemes
              .iter()
              .filter(|cp| !ZWJ_VS.contains(cp))
              .all(|&cp| self.has_codepoint(idx, cp, None))
          {
              return Some(idx);
          }
      }
      None
  }
  ```

## Scope / faithfulness notes

- **Ported**: `indexForCell`'s font-resolution core — the empty-cell → space
  fallback, the single-codepoint fast path, the grapheme candidate collection
  (skipping `U+200D`/`U+FE0E`/`U+FE0F`), and the common-font search — plus the
  `has_codepoint(idx, cp, presentation)` grid wrapper.
- **Faithful**: the primary uses the cell `presentation`, grapheme components
  use `null`; the ZWJ/VS skip set and the "one font must cover all" rule match
  upstream; the `presentation_mode` extraction preserves `get_index`'s exact
  mapping; and crucially `has_codepoint`'s `None` → `Any` (not the UCD default)
  matches upstream's `SharedGrid.hasCodepoint`, so grapheme components are
  satisfied by mere presence.
- **Deferred** (to the `RunIterator`): extracting `primary_cp`/`graphemes`/
  `hasGrapheme` from a terminal `Cell`, the kitty unicode placeholder check, the
  row cell-grouping into runs, and the run hash/`TextRun`. (Consumed by tests
  now; the font module's `#![allow(dead_code)]` covers the not-yet-wired path.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/font/codepoint_resolver.rs`: extract `presentation_mode`; add
   `has_codepoint`; add `index_for_grapheme`.
2. Tests (in `codepoint_resolver.rs`, using the existing `menlo_resolver()`):
   - `index_for_grapheme_simple`: `index_for_grapheme('A', &[], Regular, None)`
     equals `get_index('A', Regular, None)` (the single-codepoint path).
   - `index_for_grapheme_empty_is_space`:
     `index_for_grapheme(0, &[], Regular, None)` equals
     `get_index(' ', Regular, None)` (the empty-cell fallback).
   - `index_for_grapheme_multi`: a **synthetic** grapheme `('A', &['B'])` (both
     in Menlo) returns the regular index — exercising the candidate collection
     and the common-font search deterministically (the regular face covers
     both).
   - `index_for_grapheme_skips_zwj`: `('A', &[0x200D])` returns the regular
     index — the ZWJ is skipped in both the candidate collection and the
     coverage check (without the skip, `get_index(0x200D)` would fail the
     grapheme).
   - `has_codepoint_basic`:
     `has_codepoint(get_index('A', Regular, None).unwrap(), 'A', None)` is
     `true`; a control character no face renders is `false`.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty index_for_grapheme
cargo test -p roastty has_codepoint
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `index_for_grapheme` reproduces `indexForCell`'s resolution (empty→space,
  single-cp fast path, grapheme candidate collection with the ZWJ/VS skip, and
  the common-font search), and `has_codepoint` wraps the collection with the
  faithful presentation mapping;
- the simple/empty/multi/zwj and has_codepoint tests pass, and the existing
  `get_index` tests still pass (the `presentation_mode` extraction is
  behavior-preserving);
- the terminal-`Cell` extraction, the kitty placeholder, the cell-grouping, and
  the `TextRun` stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the resolution diverges from upstream (wrong
presentation per codepoint, missing ZWJ/VS skip, wrong coverage rule), the
`presentation_mode` extraction changes `get_index`, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and found **one Required
finding**, now fixed:

- **Required (fixed):** the draft's `has_codepoint` wrapper used
  `presentation_mode(cp, p)` — which maps `None` → `Default(emoji/text)` from
  the UCD. But upstream's `SharedGrid.hasCodepoint` maps `null` → **`Any`**
  (`if (p) |v| .{ .explicit = v } else .{ .any = {} }`). Using the UCD default
  would be stricter and diverge from `indexForCell`, which calls
  `hasCodepoint(idx, cp, null)` for grapheme components precisely so an emoji
  font can satisfy components like the male/female signs without their
  standalone default presentation. The design now splits the mapping:
  `presentation_mode` (`None` → UCD `Default`) for `get_index`, and a new
  `has_codepoint_mode` (`None` → `Any`) for the `has_codepoint` wrapper.

Codex confirmed the rest is faithful: `primary_cp == 0 → space` is the right
extracted-cell analog; the primary/components presentation split is correct once
`has_codepoint(None) == Any`; the ZWJ/VS skip set matches upstream; the Rust
`.all(...)` reproduces the Zig `for … else` semantics including the vacuous-true
case (all components ZWJ → the candidate is returned); and the `&mut self`
`get_index` calls followed by `&self` `has_codepoint` checks sequence cleanly.

Review artifacts:

- Prompt: `logs/codex-review/20260603-144737-938874-prompt.md` (design)
- Result: `logs/codex-review/20260603-144737-938874-last-message.md` (design)

## Result

**Result:** Pass

The run iterator's per-cell font resolution is ported.

- `roastty/src/font/codepoint_resolver.rs`: extracted `presentation_mode(cp, p)`
  (`None` → UCD `Default`) from `get_index` (behavior-preserving); added
  `has_codepoint_mode(p)` (`None` → `Any`) and the `has_codepoint(idx, cp, p)`
  grid wrapper; and
  `index_for_grapheme(primary_cp, graphemes, style, presentation)` — the
  empty-cell → space fallback, the single-codepoint fast path, the grapheme
  candidate collection (skipping `U+200D`/`U+FE0E`/`U+FE0F`), and the
  common-font search (primary uses the cell presentation, components use
  `None`).

Tests (using `menlo_resolver()`): `index_for_grapheme_simple` (`'A'`, `[]` ==
`get_index('A')`), `index_for_grapheme_empty_is_space` (`0` ==
`get_index(' ')`), `index_for_grapheme_multi` (synthetic `('A', ['B'])` → the
regular index, exercising the candidate + common-font search),
`index_for_grapheme_skips_zwj` (`('A', [0x200D])` → the regular index),
`has_codepoint_basic` (covers `'A'`, not a `BEL`). All pass.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2780 passed, 0 failed (+5, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates clean; `git diff --check` clean.

## Conclusion

The `RunIterator`'s font-resolution core (`indexForCell`) is ported, built on
the existing `CodepointResolver` and the new `has_codepoint` grid wrapper —
including the multi-codepoint grapheme "one font must cover all" search with the
ZWJ/VS skip. This is the first piece of the `RunIterator` orchestration.

The remaining `RunIterator` work: extracting
`primary_cp`/`graphemes`/`hasGrapheme` from a terminal `Cell` (roastty's
`terminal/page.rs` cell), the kitty unicode placeholder check, the row
cell-grouping into runs (the `next()` loop that groups consecutive same-font
cells, handling spacers/selection/cursor breaks), and the `TextRun` value type
with its position-independent run hash. These thread the `terminal/` grid and
`renderer/` state types.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no Required findings**. It confirmed the split is correct — `get_index` uses
`presentation_mode` (`None` → UCD `Default`) while `has_codepoint` uses
`has_codepoint_mode` (`None` → `Any`), matching `SharedGrid.hasCodepoint` and
resolving the design-gate finding — and that `index_for_grapheme` faithfully
ports `indexForCell` (empty → space, the single-codepoint fast path, the
candidate collection with the ZWJ/VS skip, the primary using the cell
presentation and components using `None`, and the `.filter(...).all(...)`
reproducing Zig's `for … else` including the vacuous-true case). It verified the
deferred scope is intact (no terminal-cell extraction, kitty placeholder,
grouping, or `TextRun` introduced) and ran the targeted tests
(`index_for_grapheme`: 4 passed; `has_codepoint`: 5 passed).

Review artifacts:

- Result review: `logs/codex-review/20260603-145146-831869-last-message.md`
