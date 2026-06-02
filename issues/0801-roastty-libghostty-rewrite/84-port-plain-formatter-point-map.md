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

# Experiment 84: Port Plain Formatter Point Map

## Description

Port upstream `terminal/formatter.zig::PageFormatter.point_map` behavior for
Roastty's private plain PageList formatter.

Experiments 81-83 built the private PageList formatter path for plain, VT, HTML,
and codepoint replacement. Upstream's next formatter primitive is the point map:
for every byte written to formatted output, record the source cell coordinate
that produced that byte. This is needed before higher-level pin maps can be
faithfully implemented, because upstream `PageListFormatter` builds a per-page
point map first and then converts those points to pins.

This experiment intentionally ports only the plain-output point-map slice. VT
and HTML point maps require mapping generated style wrappers, resets, and HTML
container bytes; those are real but should be designed separately after the
plain mapping semantics are correct.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for:
     - `PageFormatter.point_map`;
     - `PageFormatter.formatWithState`;
     - point-map writes around blank cells, newlines, wide characters, grapheme
       bytes, prior trailing state, trimming, and codepoint-map replacements;
     - tests beginning near `test "Page plain ..."` that assert `point_map`.
   - Use existing Roastty code:
     - `roastty/src/terminal/page_list.rs`;
     - `point::Coordinate`;
     - Experiment 81 plain formatter helpers;
     - Experiment 83 `codepoint_map` helpers.
   - Do not modify `vendor/ghostty/`.

2. Add private formatter option/result types.
   - Add a private result shape in `page_list.rs`, for example:

     ```rust
     struct PageStringWithMap {
         text: String,
         point_map: Vec<point::Coordinate>,
     }
     ```

   - Add a private option shape that carries the plain formatter options plus an
     optional codepoint map, for example:

     ```rust
     struct PlainStringWithMapOptions<'a> {
         selection: Option<selection::Selection>,
         trim: bool,
         unwrap: bool,
         codepoint_map: Option<&'a [CodepointMapEntry]>,
     }
     ```

   - Keep the type private.
   - Do not expose it through the C ABI, app, renderer, clipboard, public API,
     or other crates.

3. Add a private plain mapping entry point.
   - Add a private PageList method used by tests and future formatter work, for
     example:

     ```rust
     fn plain_string_with_point_map(
         &self,
         options: PlainStringWithMapOptions<'_>,
     ) -> PageStringWithMap
     ```

     or a private `page_string_with_point_map(...)` equivalent if that fits the
     existing helper stack better.

   - Existing `selection_string()`, `dump_string()`, and `page_string()` must
     keep returning exactly the same `String` values and must not allocate a map
     unless the new private point-map entry point is used.
   - Preserve that no-allocation property structurally: use `None` for the
     normal formatter's internal map sink and allocate the `Vec<Coordinate>`
     only in the point-map entry point. Do not add an always-present map field
     to the existing string-only result path.
   - The new point-map entry point should reuse the existing plain formatter
     code path, not duplicate selection/chunk traversal logic.

4. Track one coordinate per output byte.
   - Add helper functions that write to the output string and append matching
     map coordinates:
     - pushing an ASCII byte appends one coordinate;
     - pushing a UTF-8 `char` appends the source coordinate once per UTF-8 byte;
     - pushing a replacement `String` appends the original source coordinate
       once per UTF-8 byte in the replacement string;
     - pushing generated spaces from accumulated blank cells appends the
       coordinate of each blank cell using upstream's reverse-walk order from
       the later source cell;
     - pushing newlines appends the coordinate upstream uses for that newline.
   - Keep codepoint-map replacement one-shot: replacement bytes map to the
     original source coordinate and are not recursively remapped.

5. Match scoped upstream mapping semantics.
   - Plain output bytes map as follows:
     - normal single-byte text maps to its cell coordinate;
     - multi-byte Unicode text maps every UTF-8 byte to the source cell
       coordinate;
     - attached grapheme codepoint bytes map to the same source cell coordinate
       as the base cell;
     - wide-character bytes map to the visible wide cell coordinate, including
       when formatting starts from a spacer tail and upstream rewinds to the
       wide cell;
     - generated alignment spaces from skipped blank cells map to the blank
       cells they represent;
     - when multiple generated alignment spaces are flushed before a later
       source cell, map order must match upstream's reverse walk: for blanks at
       `x = 1` and `x = 2` before a later source cell at `x = 3`, the first
       emitted space maps to `x = 2`, the second maps to `x = 1`, and then the
       later source cell's bytes map to `x = 3`;
     - explicit source spaces that are emitted because `trim == false` map to
       their source cells;
     - trimmed spaces do not emit bytes and therefore do not add map entries;
     - inserted row-ending newlines map to the last emitted source coordinate
       for that row;
     - when pending blank rows are emitted and a prior map entry exists, the
       first pending newline reuses that prior coordinate, matching upstream's
       use of the previous map entry;
     - when pending blank rows are emitted with no prior map entry, the first
       pending newline maps to `(0, 0)`;
     - subsequent pending blank-row newlines map to `x = 0` and incrementing row
       coordinates, converted to PageList screen-domain coordinates.
   - `PageList` multi-page formatting should return coordinates in the
     screen-domain coordinate space already used by `point::Coordinate`, not
     per-page local coordinates that lose the scrollback offset.

6. Add upstream-equivalent tests.
   - Add tests for:
     - plain single-line ASCII output;
     - soft-wrapped unwrapped output;
     - wide character from the visible wide cell;
     - wide character when selection starts from the spacer tail;
     - multiline output and newline mapping;
     - rectangle output and newline mapping;
     - blank lines and leading blank rows;
     - multiple generated blanks before a later source cell, asserting exact
       reverse-walk coordinate order;
     - trailing whitespace with `trim == true`;
     - trailing whitespace with `trim == false`;
     - prior trailing-state rows across page chunks;
     - prior trailing-state cells on wrap continuations;
     - attached grapheme bytes mapping to the base cell;
     - codepoint-map single-codepoint and string replacements mapping to the
       original source cell;
     - generated alignment spaces mapping to blank-cell coordinates while
       remaining unaffected by codepoint replacement.
   - Add a multi-page PageList test so the map proves screen-domain coordinates
     remain correct across page chunks.
   - Add a multi-page pending-blank-row test with concrete expected coordinates
     around a page boundary.
   - Add invalid and garbage selection endpoint coverage: the point-map entry
     point should return empty `text` and an empty `point_map`.
   - Add no-map regression assertions for existing `selection_string`,
     `dump_string`, and `page_string` behavior.

7. Keep scope narrow.
   - Do not implement `PinMap` in this experiment.
   - Do not implement VT or HTML point maps in this experiment.
   - Do not add `ScreenFormatter`, `TerminalFormatter`, `Screen`, `Terminal`,
     parser state, cursor state, terminal extras, hyperlinks, writer
     abstraction, public ABI, app, renderer, clipboard, PTY, or UI behavior.

8. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty point_map
     cargo test -p roastty page_string
     cargo test -p roastty dump_string
     cargo test -p roastty selection_string
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

9. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real findings before proceeding.

10. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - helper/type names and location;
      - whether the returned map is byte-indexed and in screen coordinates;
      - which upstream point-map behaviors were ported;
      - which upstream pin-map, VT-map, and HTML-map behaviors remain deferred;
      - verification command output summary;
      - Codex design-review outcome;
      - Codex result-review outcome.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has a private plain formatter point-map path that returns one
  `point::Coordinate` per output byte;
- normal text, Unicode text, graphemes, wide cells, spacer-tail starts, emitted
  blank-cell spaces, explicit spaces, trimmed spaces, newlines, prior trailing
  state, codepoint replacements, and multi-page PageList chunks match the scoped
  upstream point-map behavior;
- the returned coordinates are screen-domain coordinates for PageList output;
- existing no-map `selection_string`, `dump_string`, and `page_string` behavior
  remains unchanged;
- the implementation does not allocate a point map for existing string-only
  formatter calls;
- no `PinMap`, VT point maps, HTML point maps, `ScreenFormatter`,
  `TerminalFormatter`, `Screen`, `Terminal`, parser state, cursor state,
  terminal extras, hyperlinks, writer abstraction, public ABI, app, renderer,
  clipboard, PTY, or UI behavior is added;
- `cargo fmt`, targeted point-map tests, formatter regression tests, PageList
  tests, and full `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- plain point maps work for single-page output but a cross-page trailing-state
  coordinate edge reveals that PageList needs a small coordinate-conversion
  helper before the result can be considered complete.

The experiment fails if:

- byte-to-coordinate mapping cannot be implemented without adding pin maps,
  styled point maps, `Screen`, `Terminal`, public API, app, renderer, PTY,
  clipboard, or UI behavior;
- the map records one entry per character instead of one entry per output byte;
- replacement bytes map to replacement coordinates instead of the original
  source cell;
- generated blank-cell spaces are unmapped or mapped to the wrong source cell;
- no-map formatter behavior regresses;
- tests or formatting fail.

## Design Review

Codex reviewed the initial design and agreed that plain point maps are the right
next formatter slice after Experiment 83, with VT/HTML point maps and pin maps
deferred.

Codex found three real design gaps:

- the point-map entry point needed to accept `codepoint_map`, because
  replacement bytes must map to the original source cell;
- generated blank-cell coordinate order needed to match upstream's reverse walk
  from the later source cell, not natural left-to-right order;
- pending blank-row newline coordinates across PageList chunks needed concrete
  expectations.

The design now adds `PlainStringWithMapOptions`, explicitly preserves no-map
string-only allocation behavior, specifies upstream reverse-walk blank mapping,
pins down pending newline coordinates, and requires invalid/garbage endpoint
coverage. Final Codex re-review found no blockers and approved the plan for
implementation.

## Result

**Result:** Pass

Implemented private plain point-map formatting in
`roastty/src/terminal/page_list.rs`.

The new private result and option types are:

- `PlainStringWithMapOptions<'a>`;
- `PageStringWithMap`.

The new private entry point is `PageList::plain_string_with_point_map(...)`,
which returns both formatted text and a byte-indexed `Vec<point::Coordinate>`.
The coordinates are PageList screen-domain coordinates, not per-page local
coordinates. Existing string-only calls still pass no point-map sink and do not
allocate a point map.

The implementation threads an optional point-map sink through the existing plain
formatter path. It records one coordinate per emitted UTF-8 byte for:

- ASCII and Unicode codepoints;
- attached grapheme codepoints;
- wide characters, including selections that start on a spacer tail;
- explicit source spaces;
- generated blank-cell spaces;
- row-ending and pending blank-row newlines;
- one-shot codepoint-map replacements.

Generated blank-cell spaces match upstream's reverse-walk mapping order. For
example, if blank cells at `x = 1` and `x = 2` are flushed before a source cell
at `x = 3`, the emitted spaces map to `x = 2` and then `x = 1`. Codepoint-map
replacement bytes map to the original source cell, including multi-byte
replacement strings and replacement codepoints.

This experiment did not add `PinMap`, VT point maps, HTML point maps,
`ScreenFormatter`, `TerminalFormatter`, `Screen`, `Terminal`, parser state,
cursor state, terminal extras, hyperlinks, writer abstraction, public ABI, app,
renderer, clipboard, PTY, or UI behavior.

Verification passed:

```text
cargo fmt
cargo test -p roastty point_map           # 31 passed
cargo test -p roastty page_string         # 12 passed
cargo test -p roastty dump_string         # 13 passed
cargo test -p roastty selection_string    # 22 passed
cargo test -p roastty terminal::page_list # 482 passed
cargo test -p roastty                     # 775 unit tests + ABI harness + doctests passed
```

Codex design review required three fixes before implementation:

- make the point-map entry point accept `codepoint_map`;
- specify upstream's reverse-walk generated blank-cell mapping order;
- pin down pending blank-row newline coordinates across PageList chunks.

Those changes were made and Codex approved the final design.

Codex result review first found three missing tests from the design:

- leading blank rows;
- single-codepoint replacement mapping;
- trailing spaces with `trim == false`.

Those tests were added, verification was rerun, and the second Codex result
review found no blockers.

## Conclusion

Experiment 84 completes the private plain-output point-map slice needed before
future pin-map work. The remaining formatter mapping work is explicit and still
deferred: higher-level `PinMap` conversion plus VT/HTML point maps for generated
style and wrapper bytes.
