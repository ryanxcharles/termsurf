+++
[implementer]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 85: Port Plain Formatter Pin Map

## Description

Port upstream `terminal/formatter.zig::PageListFormatter.pin_map` behavior for
Roastty's private plain PageList formatter.

Experiment 84 added a private byte-indexed plain point map. Upstream uses that
lower-level point map as an intermediate: `PageFormatter` maps bytes to page
coordinates, and `PageListFormatter` converts those coordinates to `Pin`s. This
experiment ports that next layer for plain output only.

VT and HTML pin maps remain out of scope because their point maps have not been
ported yet. This experiment should not invent styled mapping semantics.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for:
     - `PinMap`;
     - `PageListFormatter.pin_map`;
     - the conversion from `point_map.items` to pins after each page chunk;
     - point-map and pin-map tests around plain formatting.
   - Use existing Roastty code:
     - `roastty/src/terminal/page_list.rs`;
     - `PageList::plain_string_with_point_map(...)` from Experiment 84;
     - `PageList::pin(point::Point::screen(...))`;
     - `Pin` validation/tracking helpers.
   - Do not modify `vendor/ghostty/`.

2. Add a private pin-map result type.
   - Add a private result shape in `page_list.rs`, for example:

     ```rust
     struct PageStringWithPinMap {
         text: String,
         pin_map: Vec<Pin>,
     }
     ```

   - Keep the type private.
   - Do not expose it through the C ABI, app, renderer, clipboard, public API,
     or other crates.

3. Add a private plain pin-map entry point.
   - Add a private PageList method used by tests and future formatter work, for
     example:

     ```rust
     fn plain_string_with_pin_map(
         &self,
         options: PlainStringWithMapOptions<'_>,
     ) -> PageStringWithPinMap
     ```

   - It should call the Experiment 84 point-map path, then convert each
     `point::Coordinate` in the resulting map to a `Pin` using the PageList's
     screen-domain point conversion.
   - Preserve one map entry per output byte. `text.len()` must equal
     `pin_map.len()`.
   - Existing `selection_string()`, `dump_string()`, `page_string()`, and
     `plain_string_with_point_map()` must keep their existing behavior.

4. Match upstream pin semantics.
   - Every emitted byte maps to the `Pin` for the source coordinate that
     Experiment 84 recorded.
   - Generated blank-cell spaces map to the same reverse-walk source cells as
     the point map, but as pins.
   - Row-ending and pending blank-row newlines map to the same coordinates as
     the point map, but as pins.
   - Multi-page output must preserve the correct source node per byte. This is
     the key behavior that point maps alone cannot prove.
   - Invalid or garbage selections return empty `text` and an empty `pin_map`.
   - If a point cannot convert to a valid pin, return empty output/map rather
     than silently dropping entries or producing a shorter map.

5. Add upstream-equivalent tests.
   - Add tests for:
     - single-line ASCII output;
     - Unicode output where one cell emits multiple bytes that all map to the
       same pin;
     - wide character output from a spacer-tail selection start;
     - generated blank-cell spaces using upstream reverse-walk order;
     - explicit source spaces preserving source order;
     - trimmed trailing spaces producing no pin-map entries;
     - multiline output and newline mapping;
     - leading blank rows;
     - prior trailing-state rows and cells across page chunks;
     - codepoint-map string replacement mapping all replacement bytes to the
       original source pin;
     - codepoint-map single-codepoint replacement mapping replacement bytes to
       the original source pin;
     - multi-page output proving pins point at the correct source node on both
       sides of a page boundary;
     - invalid and garbage selection endpoints returning empty text/map.
   - Keep no-map regression assertions for existing `selection_string`,
     `dump_string`, `page_string`, and `plain_string_with_point_map()`.

6. Keep scope narrow.
   - Do not implement VT or HTML point maps or pin maps.
   - Do not add `ScreenFormatter`, `TerminalFormatter`, `Screen`, `Terminal`,
     parser state, cursor state, terminal extras, hyperlinks, writer
     abstraction, public ABI, app, renderer, clipboard, PTY, or UI behavior.
   - Do not expose pin maps outside `page_list.rs`.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty pin_map
     cargo test -p roastty point_map
     cargo test -p roastty page_string
     cargo test -p roastty dump_string
     cargo test -p roastty selection_string
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real findings before proceeding.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - helper/type names and location;
     - whether the returned map is byte-indexed;
     - how point-to-pin conversion handles invalid points;
     - which upstream plain pin-map behaviors were ported;
     - which VT/HTML pin-map, `ScreenFormatter`, and `TerminalFormatter`
       behaviors remain deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has a private plain formatter pin-map path that returns one `Pin` per
  output byte;
- the pin map is derived from the Experiment 84 point-map behavior rather than a
  separate formatter traversal;
- normal text, Unicode text, wide cells, spacer-tail starts, generated
  blank-cell spaces, explicit spaces, trimmed spaces, newlines, prior trailing
  state, codepoint replacements, and multi-page PageList chunks map to the
  correct source pins;
- multi-page output proves bytes map to pins in the correct source node;
- invalid or unconvertible coordinates return empty output/map rather than a
  short or partially invalid map;
- existing no-map string behavior and point-map behavior remain unchanged;
- no VT/HTML point maps, VT/HTML pin maps, `ScreenFormatter`,
  `TerminalFormatter`, `Screen`, `Terminal`, parser state, cursor state,
  terminal extras, hyperlinks, writer abstraction, public ABI, app, renderer,
  clipboard, PTY, or UI behavior is added;
- `cargo fmt`, targeted pin-map tests, point-map regression tests, formatter
  regression tests, PageList tests, and full `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- single-page pin maps work but multi-page conversion reveals that the current
  PageList screen-coordinate-to-pin helper needs a small private wrapper before
  pin maps can be completed.

The experiment fails if:

- pin maps cannot be implemented without adding styled maps, `Screen`,
  `Terminal`, public API, app, renderer, PTY, clipboard, or UI behavior;
- the map records one entry per character instead of one entry per output byte;
- generated bytes map to the wrong source pins;
- multi-page output loses source-node identity;
- no-map formatter behavior regresses;
- tests or formatting fail.

## Design Review

Codex reviewed the initial design and found no blockers. It agreed that private
plain pin maps are the right next formatter slice after Experiment 84 because
upstream builds `PinMap` by converting the lower-level point map after each
PageList chunk.

Codex requested one clarification before implementation: the required test list
needed to explicitly include explicit source spaces, trimmed trailing spaces,
and prior trailing-state rows/cells, rather than relying only on the point-map
regression suite plus derivation from point maps.

The design now requires those pin-map tests directly. Follow-up Codex review
found no blockers and approved the plan for implementation.

## Result

**Result:** Pass

Implemented the private plain formatter pin-map layer in
`roastty/src/terminal/page_list.rs`.

The new private `PageStringWithPinMap` result type carries `text` plus a
byte-indexed `pin_map`, and the new private
`PageList::plain_string_with_pin_map(...)` helper derives its map from
`PageList::plain_string_with_point_map(...)`. Each point-map coordinate is
converted with `PageList::pin(point::Point::screen(...))`; if any coordinate
cannot become a valid pin, the helper returns empty text and an empty map rather
than producing a partial result.

The implementation ports the plain upstream pin-map behavior for:

- single-line ASCII;
- Unicode and grapheme bytes sharing the source pin;
- wide cells and spacer-tail selection starts;
- generated blank-cell spaces using the same reverse-walk source cells as the
  point map;
- explicit spaces and trimmed trailing spaces;
- multiline output, leading blank rows, and newline mapping;
- prior trailing state across wrapped rows and page chunks;
- string and single-codepoint codepoint-map replacements;
- multi-page source-node preservation;
- invalid and garbage selection endpoints.

While adding the cross-page pin-map test, the implementation found and fixed a
supporting point-map bug from Experiment 84: generated blank cells carried into
the first row of the next PageList page were reverse-walking in page-local
coordinates. `push_blank_cells_plain(...)` now walks backward in screen-domain
coordinates before creating point-map entries, so carried blank cells can cross
page boundaries correctly.

No VT/HTML point maps, VT/HTML pin maps, `ScreenFormatter`, `TerminalFormatter`,
`Screen`, `Terminal`, parser state, cursor state, terminal extras, hyperlinks,
writer abstraction, public ABI, app, renderer, clipboard, PTY, or UI behavior
was added.

Verification passed:

```text
cargo fmt: passed
cargo test -p roastty pin_map: passed, 12 tests
cargo test -p roastty point_map: passed, 32 tests
cargo test -p roastty page_string: passed, 12 tests
cargo test -p roastty dump_string: passed, 13 tests
cargo test -p roastty selection_string: passed, 22 tests
cargo test -p roastty terminal::page_list: passed, 494 tests
cargo test -p roastty: passed, 787 unit tests, ABI harness, and doctests
```

Codex design review approved the narrowed private plain pin-map plan after the
test list was tightened to require explicit source spaces, trimmed trailing
spaces, and prior trailing-state rows/cells.

Codex result review found no blockers. It specifically noted that the cross-page
wrap-continuation test covers the earlier missing case and that the
implementation stays derived from the point-map path without scope creep.

## Conclusion

Experiment 85 completes the private plain PageList pin-map layer for Roastty.
The next formatter work can build on a byte-indexed plain point map and pin map,
while styled VT/HTML maps and the higher-level upstream formatter wrappers
remain deferred to later experiments.
