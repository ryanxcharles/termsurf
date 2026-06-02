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

# Experiment 87: Port HTML Formatter Point Map

## Description

Port upstream `terminal/formatter.zig::PageFormatter.point_map` behavior for
Roastty's private HTML PageList formatter path.

Experiment 86 completed VT point maps and deliberately left HTML map behavior
deferred. HTML is more complex than VT because the formatter emits bytes that do
not correspond directly to terminal text: wrapper `<div>` tags, style wrapper
tags, escaped ASCII entities, non-ASCII numeric entities, and closing tags.
Upstream still provides one point-map entry per output byte. This experiment
ports that behavior for HTML output only.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for:
     - `PageFormatter.point_map`;
     - HTML branches in `formatWithState`;
     - the opening monospace wrapper mapping;
     - `formatStyleOpen`;
     - `formatStyleClose`;
     - `writeCodepoint` HTML escaping and numeric entities;
     - HTML point-map tests.
   - Use existing Roastty code:
     - `roastty/src/terminal/page_list.rs`;
     - `StyledPageFormat`;
     - `PageList::page_string_with_point_map(...)`;
     - Experiment 86's VT point-map plumbing;
     - existing HTML output tests.
   - Do not modify `vendor/ghostty/`.

2. Thread the point-map sink through HTML styled formatting.
   - In `PageList::page_string_between(...)`, when `PageOutputFormat::Html` is
     requested and a point map is present, pass the chunk's screen-domain row
     base and point-map sink into `StyledPageFormat`.
   - Remove the Experiment 86 deferred-HTML behavior where HTML output leaves
     the point map empty.
   - Keep existing HTML output text exactly unchanged.

3. Map every emitted HTML byte to a source coordinate.
   - The opening wrapper
     `<div style="font-family: monospace; white-space: pre;">` maps every byte
     to `(0, page_screen_y_base)` for the current page chunk, matching
     upstream's `(0, 0)` page-local behavior translated into PageList
     screen-domain coordinates.
   - Normal ASCII text bytes map to the source cell coordinate.
   - Escaped ASCII entity bytes map to the original source cell:
     - `<` as `&lt;`;
     - `>` as `&gt;`;
     - `&` as `&amp;`;
     - `"` as `&quot;`;
     - `'` as `&#39;`.
   - Non-ASCII numeric entity bytes map to the original source cell, for example
     `é` as `&#233;` and combining marks as numeric entities.
   - Grapheme entity bytes map to the base cell coordinate.
   - Codepoint-map replacement bytes map to the original source cell after HTML
     escaping or numeric-entity expansion.
   - Generated blank-cell spaces map to the same reverse-walk source cells as
     plain and VT point maps.
   - Background-only styled spaces map to the background cell.
   - HTML style-open bytes map to the cell that caused the style transition.
   - HTML style-close bytes map to the previous emitted coordinate.
   - The final closing wrapper `</div>` maps every byte to the previous emitted
     coordinate, matching upstream's closing-wrapper behavior.
   - Multi-page chunks must use PageList screen-domain coordinates, not
     page-local rows.

4. Preserve existing behavior and scope.
   - Existing `page_string(...)` HTML output must not change.
   - Existing plain and VT point maps must not change.
   - Existing plain pin maps must not change.
   - Do not add HTML hyperlink tag emission in this experiment. Experiment 82
     deliberately left current HTML output without `<a>` wrappers; preserve that
     behavior and map visible text bytes from hyperlinked cells normally.
   - Do not add HTML pin maps yet. Pin maps depend on this point map and should
     be a later experiment.
   - Do not add VT pin maps in this experiment.
   - Do not add `ScreenFormatter`, `TerminalFormatter`, `Screen`, `Terminal`,
     parser state, cursor state, terminal extras, writer abstraction, public
     ABI, app, renderer, clipboard, PTY, or UI behavior.

5. Add upstream-equivalent tests.
   - Add HTML point-map tests for:
     - opening and closing wrapper bytes;
     - unstyled single-line output;
     - ASCII escaping for `<`, `>`, `&`, `"`, and `'`;
     - non-ASCII numeric entities;
     - grapheme numeric entities mapping to the base cell;
     - style-open and style-close bytes;
     - final wrapper close bytes mapping to the previous emitted coordinate;
     - hyperlinked cells still emitting and mapping visible text bytes without
       adding `<a>` tags;
     - generated blank-cell spaces using reverse-walk order;
     - background-only cells that emit a styled space;
     - codepoint-map single-codepoint and string replacements, including
       replacement values that require HTML escaping;
     - multi-page output proving screen-domain coordinates cross PageList page
       chunks correctly;
     - invalid and garbage selection endpoints returning empty text/map.
   - Update the Experiment 86 deferred-HTML guard so it now expects mapped HTML
     output instead of an empty map, or replace it with a more precise HTML
     point-map test.
   - Keep no-map regression assertions for existing HTML `page_string`, plain
     point-map, VT point-map, and plain pin-map behavior.

6. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty html_point_map
     cargo test -p roastty vt_point_map
     cargo test -p roastty point_map
     cargo test -p roastty pin_map
     cargo test -p roastty page_string
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

7. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real findings before proceeding.

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - helper/type names and location;
     - whether HTML point maps are byte-indexed;
     - how wrapper, style, escape, and numeric-entity bytes are mapped;
     - how multi-page screen-domain coordinates are preserved;
     - which HTML pin-map, VT pin-map, `ScreenFormatter`, and
       `TerminalFormatter` behaviors remain deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has a private HTML point-map path that returns one screen-domain
  coordinate per output byte;
- the HTML output string is unchanged from existing `page_string(...)` behavior;
- wrapper tags, normal text, escaped ASCII entities, non-ASCII numeric entities,
  graphemes, style opens/closes, final wrapper close, generated blanks,
  background-only cells, codepoint replacements, hyperlinked-cell text bytes
  without `<a>` emission, and multi-page chunks map to the correct source
  coordinates;
- existing plain point maps, VT point maps, and plain pin maps remain unchanged;
- invalid or unconvertible selections return empty output/map rather than a
  short or partially invalid map;
- no HTML pin maps, VT pin maps, `ScreenFormatter`, `TerminalFormatter`,
  `Screen`, `Terminal`, parser state, cursor state, terminal extras, writer
  abstraction, public ABI, app, renderer, clipboard, PTY, or UI behavior is
  added;
- `cargo fmt`, targeted HTML point-map tests, VT point-map tests, point-map and
  pin-map regression tests, page-string tests, PageList tests, and full
  `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- basic HTML maps work, but cross-page wrapper or closing-tag mapping exposes a
  missing helper that should be split into a follow-up.

The experiment fails if:

- HTML point maps cannot be implemented without adding pin maps, higher-level
  formatter wrappers, public API, app, renderer, PTY, clipboard, or UI behavior;
- HTML output changes while adding point maps;
- the map records one entry per character instead of one entry per output byte;
- wrapper, style, escape, numeric-entity, or closing bytes map to the wrong
  source coordinates;
- multi-page output loses screen-domain row identity;
- existing plain or VT formatter maps regress;
- tests or formatting fail.

## Design Review

Codex reviewed the initial design and agreed that HTML point maps are the right
next formatter slice after VT point maps. It found one blocker: the first draft
included HTML hyperlink-open and hyperlink-close tag mapping even though current
Roastty HTML output deliberately does not emit `<a>` wrappers from hyperlinked
cells. That contradicted the requirement to keep existing HTML output unchanged.

The design now keeps hyperlink tag emission out of scope and instead requires a
guard that hyperlinked cells continue to emit and map visible text bytes without
adding `<a>` tags. Follow-up Codex review found no blockers and approved the
experiment for implementation.

## Result

**Result:** Pass

Implemented private HTML point-map support in
`roastty/src/terminal/page_list.rs`.

`StyledPageFormat` now uses the existing point-map sink for both VT and HTML
output. HTML point maps are byte-indexed: the returned map has one screen-domain
`point::Coordinate` per emitted output byte. Existing HTML output strings remain
unchanged.

The implementation maps:

- the opening wrapper `<div style="font-family: monospace; white-space: pre;">`
  to the current page chunk's screen row base;
- normal text bytes to the source cell;
- escaped ASCII entities (`&lt;`, `&gt;`, `&amp;`, `&quot;`, `&#39;`) to the
  original source cell;
- non-ASCII numeric entities such as `&#233;` and grapheme numeric entities to
  the original/base source cell;
- style-open bytes to the cell that caused the style transition;
- style-close bytes to the previous emitted coordinate;
- generated blank-cell spaces with the same reverse-walk mapping used by plain
  and VT point maps;
- background-only and styled-empty emitted spaces to their styled source cell;
- codepoint-map replacement bytes to the original source cell after HTML
  escaping or numeric-entity expansion;
- the final closing wrapper `</div>` to the previous emitted coordinate;
- multi-page chunks with PageList screen-domain coordinates.

Hyperlink `<a>` emission remains deferred. This experiment preserves the current
behavior where hyperlinked cells emit visible text without an anchor wrapper and
maps those visible text bytes normally.

No HTML pin maps, VT pin maps, `ScreenFormatter`, `TerminalFormatter`, `Screen`,
`Terminal`, parser state, cursor state, terminal extras, writer abstraction,
public ABI, app, renderer, clipboard, PTY, or UI behavior was added.

Verification passed:

```text
cargo fmt: passed
cargo test -p roastty html_point_map: passed, 10 tests
cargo test -p roastty vt_point_map: passed, 11 tests
cargo test -p roastty point_map: passed, 53 tests
cargo test -p roastty pin_map: passed, 12 tests
cargo test -p roastty page_string: passed, 12 tests
cargo test -p roastty terminal::page_list: passed, 515 tests
cargo test -p roastty: passed, 808 unit tests, ABI harness, and doctests
```

Codex design review found one blocker in the initial plan: hyperlink tag mapping
contradicted the requirement to preserve current no-`<a>` HTML output. The
design was updated to keep hyperlink tag emission out of scope, and follow-up
review approved it.

Codex result review found one blocker in the first implementation: styled empty
cells with a later visible cell emitted a mapped HTML space without a
corresponding point-map entry. The implementation now maps that emitted blank
before returning, and `html_point_map_styled_empty_cell_maps_emitted_space`
covers the case. Follow-up Codex review found no remaining blockers.

## Conclusion

Experiment 87 completes the HTML point-map layer for PageList formatter output.
Roastty now has byte-indexed point maps for plain, VT, and HTML output, plus
plain pin maps. The next formatter map slice can derive styled pin maps from the
completed styled point maps, while higher-level `ScreenFormatter` and
`TerminalFormatter` wrappers remain deferred.
