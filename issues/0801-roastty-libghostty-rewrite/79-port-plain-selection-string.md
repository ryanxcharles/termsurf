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

# Experiment 79: Port Plain Selection String

## Description

Port the plain-text path behind upstream `Screen.selectionString()` into
Roastty's current PageList-centered terminal model.

Experiments 65-78 completed the selection geometry primitives: selection value
shape, containment, adjustment, drag selection, word selection, line selection,
select-all, select-output, and line iteration. Those helpers now need the next
upstream layer that turns a `Selection` into copied text.

Upstream routes `Screen.selectionString()` through `ScreenFormatter` and
`PageListFormatter` with:

- `emit = .plain`;
- `unwrap = true`;
- caller-controlled `trim`;
- `content = .{ .selection = opts.sel }`.

This experiment should port only that plain-text extraction behavior. It should
not port the full formatter stack yet: no VT output, no HTML output, no style
serialization, no hyperlink serialization, no cursor/extra terminal state, and
no string-to-pin map. Those remain future formatter slices.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/Screen.zig` for:
     - `selectionString`;
     - tests named `Screen: selectionString ...`;
     - line-iterator tests that use `selectionString`.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for the plain paths in:
     - `ScreenFormatter`;
     - `PageListFormatter`;
     - `PageFormatter::formatWithState`;
     - `PageFormatter::writeCell`;
     - `PageFormatter::writeCodepoint`.
   - Use existing Roastty code:
     - `roastty/src/terminal/page_list.rs`;
     - `roastty/src/terminal/page.rs`;
     - `roastty/src/terminal/selection.rs`.
   - Do not modify `vendor/ghostty/`.

2. Add a private plain selection-string option shape.
   - Preferred shape:

     ```rust
     #[derive(Debug, Clone, Copy, PartialEq, Eq)]
     struct SelectionStringOptions {
         selection: Option<selection::Selection>,
         trim: bool,
     }
     ```

   - `selection = Some(...)` is the direct upstream `Screen.selectionString()`
     equivalent, because upstream `Screen.SelectionString.sel` is required.
   - `selection = None` is a private PageListFormatter-compatible extension, not
     `Screen.selectionString()` behavior. It should format the whole
     screen-domain PageList content, matching lower-level upstream
     `ScreenFormatter.Content.selection: null` / `PageListFormatter` behavior.
   - Keep it private. Do not add public API or C ABI exposure.

3. Add private PageList plain selection-string extraction.
   - Preferred shape:

     ```rust
     fn selection_string(&self, opts: SelectionStringOptions) -> String
     ```

   - The helper should:
     - order selection endpoints using existing selection top-left /
       bottom-right logic;
     - support rectangular selections by applying the selected x-range to every
       row;
     - use the full screen domain when `selection` is `None`;
     - unwrap soft-wrapped rows, matching upstream `selectionString`;
     - use `trim` to drop trailing spaces and trailing blank cells before a row
       break;
     - preserve interior blank lines;
     - carry upstream-equivalent trailing blank row/cell state across page
       chunks, matching `PageFormatter.TrailingState`;
     - return an empty string when the selected range contains no text;
     - treat invalid or garbage selection endpoints as an empty string rather
       than panicking.
   - This may be implemented directly in `page_list.rs` as a first private plain
     formatter, or as a small private formatter module if that keeps the code
     clearer. Do not introduce a public `ScreenFormatter` facade in this
     experiment.

4. Match upstream cell text emission for the plain path.
   - Emit a cell's base codepoint as UTF-8.
   - If the cell has attached grapheme codepoints, append them in stored order.
   - Skip wide spacer cells (`SpacerHead` and `SpacerTail`) as upstream does.
   - If the selection starts on a wide-character tail, include the whole wide
     character by moving the row start to the prior cell.
   - If the selection starts on a wide spacer head, skip that physical row just
     as upstream `PageFormatter` does.
   - If the selection ends on a wide-character spacer head and the next row is
     available while unwrapping, include the continuation row in the same way
     upstream does.
   - Emit blank cells as spaces only when a later nonblank cell on the same
     logical output line requires those spaces to preserve interior columns.

5. Add upstream-equivalent tests.
   - Port these `Screen.selectionString` tests as direct string-output tests:
     - basic;
     - start outside of written area;
     - end outside of written area;
     - trim space;
     - trim empty line;
     - soft wrap;
     - wide char;
     - wide char with header;
     - empty with soft wrap;
     - zero width joiner;
     - rectangle basic;
     - rectangle with end-of-line clipping;
     - rectangle with blank-line breaks;
     - multi-page.
   - The multi-page tests must include at least one case where pending trailing
     blank rows or blank cells cross a page chunk boundary, so the
     implementation proves it carries upstream-equivalent `TrailingState` rather
     than treating each page independently.
   - Update the line-iterator tests from Experiment 78, or add companion tests,
     so at least the upstream-equivalent line-iterator cases verify copied text
     as well as selection bounds.
   - Add Roastty-specific guard tests:
     - `selection = None` formats the full screen-domain content as a private
       PageListFormatter-compatible extension;
     - invalid and garbage endpoints return an empty string;
     - tracked selections format from their current tracked pins;
     - selection extraction across scrollback uses screen-domain pins rather
       than active-viewport-only coordinates.
     - starting a selection on a `SpacerHead` skips that row instead of emitting
       a partial wide character.

6. Keep scope narrow.
   - Do not add VT or HTML formatter output.
   - Do not add style, hyperlink, palette, cursor, charset, keyboard protocol,
     protection, or extra terminal-state serialization.
   - Do not add pin-map / byte-map support.
   - Do not add `Screen`, `Terminal`, parser, renderer, app, platform input,
     clipboard, gesture state, public ABI, or UI wiring.
   - Do not expose this helper outside the terminal module yet.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty selection_string
     cargo test -p roastty line_iterator
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
     - helper names and location;
     - which upstream selection-string tests were ported;
     - which formatter features are intentionally deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has a private PageList plain selection-string helper equivalent to
  upstream `Screen.selectionString()` for `.plain` output;
- non-rectangular selections, rectangular selections, full-content formatting,
  soft-wrap unwrapping, trimming, blank-line preservation, wide characters,
  attached graphemes, and multi-page selections match the ported upstream tests;
- invalid or garbage endpoints return an empty string instead of panicking;
- line-iterator selections can be converted to the expected copied text;
- no VT/HTML formatter output, style/hyperlink serialization, extra terminal
  state, pin-map support, `Screen`, `Terminal`, parser, renderer, app, platform
  input, clipboard, gesture state, public ABI, or UI wiring is added;
- `cargo fmt`, targeted selection-string tests, targeted line-iterator tests,
  PageList tests, and full `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- ordinary single-page selection strings work, but a specific upstream behavior
  such as rectangle clipping, wide spacer boundaries, grapheme emission, or
  multi-page unwrapping exposes a missing lower-level primitive that should be
  split into the next experiment.

The experiment fails if:

- selection strings cannot be implemented without adding the full
  `ScreenFormatter`, public ABI, parser, renderer, app, platform input, or
  clipboard behavior;
- soft-wrapped rows insert newlines where upstream unwraps them;
- hard-wrapped rows lose required newlines;
- trimming removes interior spaces or blank lines;
- invalid pins panic;
- tests or formatting fail.

## Design Review

Codex reviewed the initial design and found three real blockers:

- `selection = None` was incorrectly described as upstream
  `Screen.selectionString()` behavior, even though upstream requires a concrete
  selection there and only lower-level formatter content supports null
  selection;
- multi-page formatting did not explicitly require carrying
  `PageFormatter.TrailingState`-equivalent blank row/cell state across page
  chunks;
- start-on-`SpacerHead` behavior was missing from the wide-character
  requirements.

The design now distinguishes direct `Screen.selectionString()` equivalence from
the private full-content formatter extension, requires cross-page trailing state
coverage, and includes a focused `SpacerHead` start requirement and test.

Follow-up Codex review approved the updated design with no remaining blockers.

## Result

**Result:** Pass

Implemented private plain selection-string extraction in
`roastty/src/terminal/page_list.rs`:

- `SelectionStringOptions` carries the optional PageList-level selection and
  trim flag.
- `PlainTrailingState` carries pending blank row/cell state across page chunks,
  matching the plain `PageFormatter.TrailingState` behavior this experiment
  needed.
- `PlainPageFormat` formats one page chunk of plain text, with soft-wrap
  unwrapping, trimming, rectangular selection bounds, wide-cell spacer handling,
  and grapheme emission.
- `PageList::selection_string()` orders selection bounds, supports rectangular
  selections, handles the private `selection = None` full-content formatter
  extension, validates endpoints, and returns an empty string for invalid or
  garbage endpoints.

The implementation remains private to the terminal module. It does not add VT or
HTML output, style or hyperlink serialization, extra terminal state,
pin-map/byte-map support, `Screen`, `Terminal`, parser, renderer, app, platform
input, clipboard, gesture state, public ABI, or UI wiring.

Ported or covered the upstream-equivalent plain selection-string behavior for:

- basic selection strings;
- selections starting outside written content;
- selections ending outside written content;
- trim and no-trim trailing spaces;
- trim and no-trim empty-line preservation;
- soft-wrap unwrapping;
- wide characters, including start-on-tail and end-on-spacer-head cases;
- zero-width-joiner grapheme emission;
- rectangular selections, including end-of-line clipping and blank-line breaks;
- multi-page selections;
- line-iterator selections copied as text.

Added Roastty-specific guard coverage for:

- the private `selection = None` full screen-domain formatter extension;
- invalid and garbage endpoints returning an empty string;
- tracked selections reading current tracked pin values;
- screen-domain selection across scrollback;
- cross-page trailing blank-cell state.

Verification passed:

```bash
cargo fmt
cargo test -p roastty selection_string
cargo test -p roastty line_iterator
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

Observed results:

- `cargo test -p roastty selection_string`: 22 passed.
- `cargo test -p roastty line_iterator`: 8 passed.
- `cargo test -p roastty terminal::page_list`: 405 passed.
- `cargo test -p roastty`: 698 unit tests passed, ABI harness passed, and
  doctests passed.

Codex reviewed the completed implementation and found one real blocker: the
wide-character-with-header test did not actually exercise the upstream
end-on-`SpacerHead` path. The fixture now places `SpacerHead` at the selected
end column and the wide cell on the wrapped continuation row. Verification
passed again after the fix.

Follow-up Codex review found no remaining blockers and approved recording the
experiment result.

## Conclusion

Experiment 79 successfully ports the plain-text selection extraction path needed
above Roastty's PageList selection primitives. Roastty can now convert ordinary,
rectangular, wrapped, wide-character, grapheme, tracked, and multi-page
selections into copied plain text with upstream-style trimming and unwrapping.

The next experiment can continue upward from copied plain selections into the
next small upstream selection/prompt helper or begin decomposing the remaining
formatter outputs into separate VT/HTML/pin-map slices.
