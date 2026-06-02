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

# Experiment 88: Port Styled Formatter Pin Maps

## Description

Port upstream `terminal/formatter.zig::PageListFormatter.pin_map` behavior for
Roastty's private PageList formatter path across all currently supported output
formats: plain, VT, and HTML.

Experiment 85 added a private plain pin-map helper by converting the plain point
map to `Pin`s. Experiments 86 and 87 completed VT and HTML point maps. Upstream
does not have separate pin-map traversal logic for styled output; it asks the
page formatter for a point map, then converts every point-map coordinate to a
`Pin` in PageList context. Roastty can now do the same for all three output
formats.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for:
     - `PageListFormatter.pin_map`;
     - point-map-to-pin conversion after each page chunk;
     - pin-map tests for plain, VT, and higher-level formatter output.
   - Use existing Roastty code:
     - `roastty/src/terminal/page_list.rs`;
     - `PageList::page_string_with_point_map(...)`;
     - `PageList::plain_string_with_pin_map(...)`;
     - `PageList::pin(point::Point::screen(...))`;
     - Experiment 85 plain pin-map tests;
     - Experiment 86 VT point-map tests;
     - Experiment 87 HTML point-map tests.
   - Do not modify `vendor/ghostty/`.

2. Add a format-general private pin-map entry point.
   - Keep `PageStringWithPinMap` private.
   - Add or refactor to a private helper such as:

     ```rust
     fn page_string_with_pin_map(
         &self,
         options: PageStringOptions<'_>,
     ) -> PageStringWithPinMap
     ```

   - It should call `PageList::page_string_with_point_map(...)`, then convert
     every screen-domain coordinate in the resulting point map to a `Pin`.
   - Preserve one map entry per output byte. `text.len()` must equal
     `pin_map.len()`.
   - If any point cannot convert to a valid pin, return empty text and an empty
     pin map rather than silently dropping entries or producing a shorter map.
   - Refactor `plain_string_with_pin_map(...)` to delegate to this general
     helper with `PageOutputFormat::Plain`, preserving existing plain behavior.

3. Match upstream pin semantics for all currently mapped formats.
   - Plain pin maps keep Experiment 85 behavior.
   - VT pin maps derive directly from Experiment 86 VT point maps:
     - style-open bytes map to the pin for the style-transition cell;
     - style-close bytes map to the previous emitted pin;
     - `\r\n` bytes map byte-for-byte to the point-map coordinates;
     - generated blanks, graphemes, background-only cells, styled empty cells,
       codepoint replacements, and multi-page chunks map to the corresponding
       source pins.
   - HTML pin maps derive directly from Experiment 87 HTML point maps:
     - wrapper bytes map to the chunk row-base pin;
     - escaped entities and numeric entities map to the original source pin;
     - style wrapper bytes and final close bytes map as their point-map
       coordinates specify;
     - hyperlinked-cell text remains mapped without adding `<a>` tags;
     - generated blanks, graphemes, background-only cells, styled empty cells,
       codepoint replacements, and multi-page chunks map to the corresponding
       source pins.
   - Multi-page output must preserve source-node identity for bytes on both
     sides of a page boundary.

4. Preserve existing behavior and scope.
   - Existing `selection_string()`, `dump_string()`, `page_string()`,
     `plain_string_with_point_map()`, `page_string_with_point_map()`, and
     `plain_string_with_pin_map()` behavior must not regress.
   - Do not add public ABI, app, renderer, clipboard, PTY, UI behavior,
     `ScreenFormatter`, `TerminalFormatter`, `Screen`, `Terminal`, parser state,
     cursor state, terminal extras, writer abstraction, or HTML hyperlink tag
     emission.
   - Do not expose pin maps outside `page_list.rs`.

5. Add upstream-equivalent tests.
   - Add tests for:
     - the new general helper returning the same plain pin map as the existing
       plain helper;
     - VT unstyled output;
     - VT style-open and final style-close bytes;
     - VT `\r\n` newline bytes;
     - VT generated blanks and codepoint replacements;
     - HTML wrapper bytes;
     - HTML escaped entities and numeric entities;
     - HTML style wrapper bytes and styled empty cells;
     - HTML hyperlinked-cell text without `<a>` emission;
     - HTML final wrapper close bytes;
     - multi-page VT and HTML output proving source-node identity is preserved;
     - invalid and garbage selection endpoints returning empty text/map.
   - Keep no-map regression assertions for existing `selection_string`,
     `dump_string`, `page_string`, all point-map helpers, and the plain pin-map
     helper.

6. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty styled_pin_map
     cargo test -p roastty pin_map
     cargo test -p roastty html_point_map
     cargo test -p roastty vt_point_map
     cargo test -p roastty point_map
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
     - whether the returned maps are byte-indexed;
     - how invalid point-to-pin conversion is handled;
     - how VT style/newline bytes and HTML wrapper/entity/style bytes are mapped
       to pins;
     - how multi-page source-node identity is preserved;
     - which `ScreenFormatter` and `TerminalFormatter` behaviors remain
       deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has a private format-general PageList formatter pin-map path that
  returns one `Pin` per output byte for plain, VT, and HTML output;
- the pin map is derived from `page_string_with_point_map(...)` rather than a
  separate formatter traversal;
- plain pin maps remain identical to Experiment 85;
- VT pin maps correctly cover text, styles, final style close, `\r\n`, generated
  blanks, replacements, and multi-page chunks;
- HTML pin maps correctly cover wrappers, escaped/numeric entities, styles,
  styled empty cells, generated blanks, replacements, hyperlinked-cell text
  without `<a>` emission, final wrapper close, and multi-page chunks;
- multi-page output proves bytes map to pins in the correct source node;
- invalid or unconvertible coordinates return empty output/map rather than a
  short or partially invalid map;
- existing no-map string behavior and point-map behavior remain unchanged;
- no public ABI, app, renderer, clipboard, PTY, UI behavior, `ScreenFormatter`,
  `TerminalFormatter`, `Screen`, `Terminal`, parser state, cursor state,
  terminal extras, writer abstraction, or HTML hyperlink tag emission is added;
- `cargo fmt`, targeted styled-pin-map tests, existing pin-map tests, point-map
  tests, page-string tests, PageList tests, and full `cargo test -p roastty`
  pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- VT or HTML pin maps expose a small point-to-pin conversion gap across
  multi-page chunks that needs a helper split before all formats can pass.

The experiment fails if:

- styled pin maps require a separate traversal instead of deriving from point
  maps;
- the map records one entry per character instead of one entry per output byte;
- generated bytes map to the wrong source pins;
- multi-page output loses source-node identity;
- existing formatter output or point maps regress;
- tests or formatting fail.

## Design Review

Codex reviewed the design and found no blockers. It agreed that deriving styled
pin maps from the completed plain, VT, and HTML point-map path is the right next
formatter slice, and that the experiment correctly avoids adding a second
formatter traversal.

Codex specifically approved the byte-indexed map requirement, the empty
output/map behavior on failed point-to-pin conversion, the multi-page
source-node checks, and the scope boundary excluding `ScreenFormatter`,
`TerminalFormatter`, public API, app behavior, and HTML hyperlink tag emission.

## Result

**Result:** Pass

Implemented the private format-general PageList pin-map path in
`roastty/src/terminal/page_list.rs`:

- `PageStringWithPinMap` remains private.
- `PageList::page_string_with_pin_map(PageStringOptions<'_>)` now calls
  `PageList::page_string_with_point_map(...)` and converts each screen-domain
  coordinate with `PageList::pin(point::Point::screen(...))`.
- `PageList::plain_string_with_pin_map(...)` delegates to the general helper
  with `PageOutputFormat::Plain`, preserving the Experiment 85 plain behavior.

The returned maps are byte-indexed: every output byte has exactly one `Pin`, and
tests assert `text.len() == pin_map.len()`. If any point-to-pin conversion
fails, the helper returns empty text and an empty pin map rather than producing
a partial map.

VT pin maps now come from the VT point-map path. Style-open bytes map to the
style-transition cell, final style-close bytes map to the previous emitted cell,
`\r\n` bytes map byte-for-byte through the point map, and generated blanks plus
codepoint replacements map to their source cells.

HTML pin maps now come from the HTML point-map path. Wrapper bytes map to the
row-base source pin, escaped and numeric entities map to the original source
cell, style wrapper bytes and final close bytes map through their point-map
coordinates, styled empty cells map their emitted space, generated blanks and
codepoint replacements map to source cells, and hyperlinked-cell text remains
mapped without emitting `<a>` tags.

Multi-page VT and HTML tests prove bytes on each side of a page boundary keep
the correct source-node identity. `ScreenFormatter` and `TerminalFormatter`
pin-map behavior remains deferred, as does public API exposure, app behavior,
writer abstraction work, and HTML hyperlink tag emission.

Verification passed:

```bash
cargo fmt
cargo test -p roastty styled_pin_map      # 9 unit tests passed
cargo test -p roastty pin_map             # 21 unit tests passed
cargo test -p roastty html_point_map      # 10 unit tests passed
cargo test -p roastty vt_point_map        # 11 unit tests passed
cargo test -p roastty point_map           # 53 unit tests passed
cargo test -p roastty page_string         # 12 unit tests passed
cargo test -p roastty terminal::page_list # 524 unit tests passed
cargo test -p roastty                     # 817 unit tests passed; ABI harness passed
```

Codex design review found no blockers and approved the experiment design. Codex
result review first found no blockers but suggested direct HTML
codepoint-replacement coverage as a useful addition. I added direct styled
pin-map coverage for VT unstyled output and HTML generated blanks plus codepoint
replacements, reran the full verification matrix, and reran Codex result review.
The final review found no remaining blockers.

## Conclusion

Experiment 88 completed upstream-style PageList pin maps for all currently
supported private formatter output formats: plain, VT, and HTML. The
implementation deliberately reuses point maps as the single traversal source,
which matches upstream's architecture and avoids a second formatter walk.

The next formatter work can move beyond PageList-private string, point-map, and
pin-map support into the next upstream formatter layer, while keeping
`ScreenFormatter` and `TerminalFormatter` as explicit future scope.
