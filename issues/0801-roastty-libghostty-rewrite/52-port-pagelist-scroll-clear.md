# Experiment 52: Port PageList Scroll Clear

## Description

Port upstream PageList `scrollClear`.

`scrollClear` clears the visible active screen by scrolling written active
content into scrollback. It does not directly blank rows and it does not update
the viewport. Instead, it scans the active area from the bottom upward, finds
the first non-empty row, computes how many active rows need to scroll, and calls
`grow()` that many times.

This experiment should add the PageList helper and tests only. It must not wire
scroll-clear escape sequences, parser behavior, renderer delivery, terminal
screen APIs, app behavior, or public ABI.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/PageList.zig` as the source of truth for:
     - `PageList.scrollClear`;
     - the `Cell.isEmpty` behavior it relies on;
     - related scroll/viewport tests around the upstream `scrollClear` cases.
   - Do not modify `vendor/ghostty/`.

2. Add `PageList::scroll_clear`.
   - Scan only the active area, not the whole page list.
   - Scan from bottom to top, matching upstream.
   - Treat a row as non-empty if any visible cell in `0..self.cols` is not
     `Cell::is_empty()`.
   - Compute the same `non_empty` count as upstream:
     - all-empty active area scrolls `0` rows;
     - content on the bottom active row scrolls `self.rows` rows;
     - content on active row `y` scrolls `y + 1` rows.
   - Call `grow()` once per row to scroll content into history.
   - Return a narrow internal error if `grow()` fails, following the existing
     Rust helper style.
   - End successful execution with full `verify_integrity`.

3. Preserve viewport behavior.
   - Do not call `fixup_viewport`.
   - Do not force `viewport` to `Active`.
   - Preserve upstream behavior that `scrollClear` does not update the viewport.
   - Add tests proving active, top, and pinned viewport states are preserved.

4. Preserve scope.
   - Do not implement:
     - parser CSI integration;
     - terminal screen clear APIs;
     - renderer/app notifications;
     - public C ABI additions;
     - resize/reflow;
     - row/cell/prompt iterators;
     - selection/search behavior.
   - Do not add `ghostty` names except when citing upstream paths or test
     provenance.

5. Add tests.
   - Empty active area:
     - initialize a blank list;
     - call `scroll_clear`;
     - verify `total_rows`, page count, row contents, and viewport are
       unchanged.
   - Empty active area with non-empty history:
     - create history above the active area;
     - place content only in the history row immediately above active;
     - leave active rows empty;
     - call `scroll_clear`;
     - verify zero rows scroll, proving history rows are not counted as active
       content.
   - Top active row has content:
     - place content only in active row `0`;
     - call `scroll_clear`;
     - verify exactly one row is added to history and active rows remain blank.
   - Middle active row has content:
     - place content only in an active middle row;
     - call `scroll_clear`;
     - verify `y + 1` rows are added to history.
   - Bottom active row has content:
     - place content only in the bottom active row;
     - call `scroll_clear`;
     - verify `self.rows` rows are added to history.
   - Active spans partial pages:
     - create a list where history and active rows share the first active page;
     - place content in an active row across that page boundary;
     - call `scroll_clear`;
     - verify the scroll count is based on active coordinates (`y + 1`), not
       page-local row coordinates.
   - Styled/non-text content:
     - verify styled text, grapheme text, hyperlink text, spacer cells, and
       background-color-only cells count as non-empty according to
       `Cell::is_empty`.
   - Viewport preservation:
     - with viewport `Active`, call `scroll_clear` and verify it stays active;
     - with viewport `Top`, call `scroll_clear` and verify it stays top;
     - with viewport `Pin` and a cached offset, call `scroll_clear` and verify
       the viewport remains pinned and cache behavior is not forcibly fixed up
       by `scroll_clear`.

6. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

7. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - active-area scan behavior;
     - non-empty row count behavior;
     - grow/error behavior;
     - viewport preservation behavior;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- empty active content scrolls zero rows;
- content on active row `y` scrolls exactly `y + 1` rows;
- content on the bottom active row scrolls exactly `self.rows` rows;
- the scan is limited to active rows and does not consider older history rows;
- non-empty detection matches `Cell::is_empty`;
- scroll clear uses `grow()` rather than directly blanking rows;
- viewport state is preserved and `fixup_viewport` is not called;
- full `verify_integrity` passes after successful scroll clear;
- no parser, renderer, app, terminal API, public ABI, resize/reflow, iterator,
  selection, or search work is introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- an independent agent reviews the experiment design and completed result and
  approves them, or all real findings are fixed.

The experiment is partial if:

- active scanning works, but preserving pinned viewport cache reveals an
  existing viewport invariant problem that must be designed separately.

The experiment fails if:

- it clears rows directly instead of scrolling via `grow()`;
- it scans history rows as active content;
- it scrolls the wrong number of rows for top, middle, or bottom active content;
- it changes viewport state contrary to upstream behavior;
- it expands into parser, renderer, app, ABI, resize/reflow, iterator,
  selection, or search work;
- tests or formatting fail.
