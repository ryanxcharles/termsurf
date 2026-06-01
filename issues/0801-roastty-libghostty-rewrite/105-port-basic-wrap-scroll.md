# Experiment 105: Port Basic Wrap Scroll

## Description

Continue the narrow `Terminal.print()` port by replacing Experiment 104's
bottom-row `ScrollUnsupported` stop with the first useful scrollback behavior:
when a pending wrap is consumed on the bottom row of the full active screen,
Roastty should scroll/grow the active `PageList`, keep the cursor on the active
bottom row, and print the next one-cell character there.

The important coordinate correction is part of this experiment. Experiment 104's
basic print helpers use `Point::screen`-style addressing because there was no
scrollback yet, so active coordinates and screen coordinates were identical.
After scrollback exists, Ghostty's cursor `x`/`y` are active-area coordinates,
not absolute history coordinates. This experiment must convert the basic print,
cell preflight, row-wrap, row-continuation, and dirty-test helper paths to the
active coordinate domain before adding bottom-row growth. Otherwise the first
scroll would make future printable characters land in historical rows instead of
the visible active bottom row.

This experiment ports only the full-width, primary active-screen path needed by
basic printable wrap. It does not implement scroll regions, no-scrollback row
rotation, index control dispatch, margins, SGR background preservation, wide
characters, Unicode width, styles, hyperlinks, semantic prompt state, alternate
screen behavior, public API, or public ABI.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/Terminal.zig` for:
     - `print()`;
     - `printWrap()`;
     - `index()`;
     - `Terminal: input with basic wraparound`;
     - `Terminal: index bottom of primary screen`;
     - `Terminal: scrollUp creates scrollback in primary screen`.
   - Use `vendor/ghostty/src/terminal/Screen.zig` for:
     - `cursorDownScroll()`;
     - `Screen: scrolling`;
     - `Screen: scrolling with a single-row screen with scrollback`.
   - Do not modify `vendor/ghostty/`.

2. Convert basic print addressing from screen coordinates to active coordinates.
   - Replace the Experiment 104 helper names/implementations that say `screen`
     but operate on cursor `y`:
     - `write_basic_screen_cell`;
     - `check_basic_screen_cell`;
     - `set_screen_row_wrap`;
     - `set_screen_row_wrap_continuation`;
     - related test-only row wrap helpers.
   - The replacement helpers should use `point::Point::active(...)`, because the
     cursor's `x`/`y` are active-area coordinates.
   - Keep existing Experiment 104 tests passing before adding the scroll case;
     with no scrollback, active and screen coordinates are still equivalent.

3. Add the smallest active full-width scroll helper.
   - Add a private helper on `PageList` or `Screen` that grows the active
     `PageList` by one row for the full-width bottom-row case.
   - The helper should use existing `PageList::grow()` / `grow_rows(1)` behavior
     so scrollback bookkeeping, viewport movement, pruning, and integrity checks
     stay in one place.
   - Do not implement no-scrollback row rotation. Current
     `PageList::init(..., max_size)` uses growable scrollback semantics by
     default; no-scrollback compatibility can be a later experiment.
   - Do not implement partial scroll regions or left/right margins.

4. Replace bottom-row pending-wrap failure with scrollback growth.
   - In `Screen::print_basic_cell`, when `pending_wrap` is true and
     `cursor.y == rows - 1`:
     - grow/scroll the active `PageList` by one row;
     - after successful growth, mark the old active bottom row wrapped;
     - keep the cursor at active `x = 0`, `y = rows - 1`;
     - clear `pending_wrap`;
     - mark the active destination row as a wrap continuation;
     - print the incoming character.
   - When `pending_wrap` is true and `cursor.y < rows - 1`, keep the Experiment
     104 no-scroll behavior.
   - Preserve the rule that a right-edge write sets `pending_wrap = true` and
     does not mark the row wrapped until the next printable character consumes
     the pending wrap.

5. Preserve safety boundaries.
   - Keep the managed-cell preflight from Experiment 104.
   - For non-bottom pending wrap, preflight the active destination row before
     mutating wrap/cursor state.
   - For bottom-row scroll, perform the grow before moving the cursor or writing
     the new character. If growth fails, return a private allocation error
     without writing the incoming character.
   - Keep growth failure transactional: do not clear `pending_wrap`, move the
     cursor, write the incoming character, mark the destination row as a wrap
     continuation, or mark the old row wrapped until the grow has succeeded.
     This is slightly stricter than upstream's `printWrap()` ordering, but it
     matches Roastty's current fallible-helper style and avoids partial state
     mutation in this narrow slice.
   - Keep explicit private errors for unsupported non-ASCII codepoints other
     than `U+FFFD`.

6. Add or update tests.
   - Keep all Experiment 104 tests.
   - Update the bottom-row test so it proves bottom-row pending wrap now scrolls
     and writes, instead of returning `ScrollUnsupported`.
   - Add a 5-column, 2-row test that feeds `helloworldabc12` and verifies:
     - visible plain text is `world\nabc12`;
     - unwrap output for the visible active area is `worldabc12`;
     - the top scrolled row `hello` remains in history/scrollback and is
       observed by a mandatory test path, using existing PageList/history
       formatter helpers if available or a narrow `#[cfg(test)]` observer if
       not;
     - cursor ends at active row 1, column 4, with `pending_wrap = true`;
     - the row containing `world` is wrapped;
     - the row containing `abc12` is a wrap continuation.
   - Add a split-feed bottom-row test: feed `helloworld`, verify the cursor is
     on the active bottom row with `pending_wrap = true`, then feed `a` in a
     separate `next_slice` call and verify the scroll/write behavior. This
     proves bottom-row pending wrap survives terminal feed boundaries.
   - Add a test that writes after scrolling and proves the new character lands
     in the active bottom row, not in the historical row that used to have the
     same absolute screen coordinate. This is the regression test for the active
     vs. screen coordinate fix.
   - Add a dirty-state test that clears dirt immediately before consuming the
     bottom-row pending wrap and verifies the visible rows affected by the
     scroll/write are dirty.
   - Keep managed-cell failure tests for non-bottom pending wrap.
   - Do not add allocation-failure injection unless the existing test
     infrastructure already supports it.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty stream
     cargo test -p roastty terminal_formatter
     cargo test -p roastty terminal::terminal
     cargo test -p roastty screen_formatter
     cargo test -p roastty page_string
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix all real design findings before implementation.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real result findings before proceeding.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - active-coordinate conversion;
     - bottom-row scroll/grow behavior;
     - cursor and pending-wrap behavior after scroll;
     - row wrap and wrap-continuation behavior after scroll;
     - dirty-state behavior;
     - what remains deferred from upstream `printWrap()` / `index()` /
       `cursorDownScroll()`;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- basic printable cursor `x`/`y` operations use active-area coordinates, not
  absolute screen/history coordinates;
- supported one-cell printable characters at the right edge still write the cell
  and set pending wrap;
- consuming pending wrap on a non-bottom row still wraps before writing, with
  the same row metadata and dirty behavior as Experiment 104;
- consuming pending wrap on the bottom row grows/scrolls the active PageList,
  keeps the cursor on the active bottom row, writes the next character there,
  and preserves the scrolled row in scrollback;
- the scrolled history row is mechanically tested, not inferred from visible
  output alone;
- bottom-row pending wrap survives across separate `next_slice` feed calls
  before scroll/write;
- after a bottom-row scroll, additional printable characters write to the active
  bottom row rather than a historical row;
- soft-wrap formatter behavior remains correct for the visible active area;
- managed-cell, unsupported non-ASCII, and invalid-point protections remain;
- no scroll regions, no-scrollback rotation, margins, insert mode, Unicode
  width, wide characters, zero-width characters, grapheme clustering, charsets,
  styles, hyperlinks, semantic prompt state, CSI, OSC, DCS, APC, PTY IO, public
  API, or public ABI are added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- `PageList` lacks a safe active-coordinate growth/write path and a prerequisite
  PageList experiment is needed first; or
- the visible active area can scroll correctly, but existing formatter helpers
  cannot yet expose the scrolled `hello` history row without a broader
  scrollback formatter experiment.

The experiment fails if:

- bottom-row pending wrap still returns `ScrollUnsupported`;
- printable data after scroll lands in historical rows because the cursor path
  still uses `Point::screen`;
- scrollback growth loses or corrupts the scrolled row;
- row wrap or wrap-continuation metadata is missing after a successful
  bottom-row pending wrap;
- dirty-state tests pass only because rows were already dirty before the
  bottom-row wrap operation;
- unsupported non-ASCII or managed-cell paths regress to silent writes or silent
  drops;
- public API or ABI changes are added.

## Design Review

Codex reviewed this design before implementation.

Initial review artifacts:

- Prompt: `logs/codex-review/20260601-012217-049602-prompt.md`
- Result: `logs/codex-review/20260601-012217-049602-last-message.md`

Codex found three real design issues:

- scrollback preservation was optional, but this experiment's core promise
  requires mechanically proving the scrolled row remains in history;
- grow-failure transactionality was ambiguous;
- the design did not require a split-feed bottom-row pending-wrap test.

All three findings were applied. The design now requires a mandatory history
observer/test path, transactional grow failure semantics, and a split-feed
bottom-row test.

Second review artifacts:

- Prompt: `logs/codex-review/20260601-012357-881161-prompt.md`
- Result: `logs/codex-review/20260601-012357-881161-last-message.md`

Codex found one remaining text contradiction: the behavior section still said to
mark the old row wrapped before growth, while the safety section required
transactional growth failure. The behavior section was fixed to grow first and
mark the old active bottom row wrapped only after successful growth.

Clean design re-review artifacts:

- Prompt: `logs/codex-review/20260601-012437-119610-prompt.md`
- Result: `logs/codex-review/20260601-012437-119610-last-message.md`

Codex found no remaining design blockers and approved implementation.

## Result

**Result:** Pass.

Implemented the first bottom-row full-width wrap-scroll slice for the active
screen.

Active-coordinate conversion:

- The basic print helpers now operate in the active coordinate domain instead of
  the absolute screen/history coordinate domain.
- The cursor's `x`/`y` remain active-area coordinates after scrollback exists.
- Default screen formatting now formats the active visible area. A narrow
  test-only full-screen dump remains available to prove history rows are still
  present.

Bottom-row scroll/grow behavior:

- Consuming pending wrap on a non-bottom row keeps Experiment 104 behavior:
  preflight the active destination cell, mark the old row wrapped, move to the
  next active row, mark it as a wrap continuation, and write.
- Consuming pending wrap on the bottom row now grows the `PageList` by one row
  instead of returning `ScrollUnsupported`.
- Growth happens before cursor movement, pending-wrap clearing, destination
  wrap-continuation metadata, or the incoming character write. This preserves
  transactional failure semantics for the fallible grow path.
- After successful growth, the old active bottom row is marked wrapped, the
  cursor stays at active bottom row column 0, the new active bottom row is
  marked as a wrap continuation, and the incoming one-cell character is written.

Cursor and metadata behavior:

- `helloworldabc12` on a 5-column, 2-row terminal visibly formats as
  `world\nabc12`; the full screen/history dump is `hello\nworld\nabc12`.
- The same visible active area unwraps as `worldabc12`.
- The cursor ends at active row 1, column 4, with `pending_wrap = true`.
- The active row containing `world` is marked wrapped.
- The active row containing `abc12` is marked as a wrap continuation.
- A split-feed test proves bottom-row pending wrap survives across separate
  `next_slice` calls before scrolling and writing.
- A post-scroll write-placement test proves printable data lands in the active
  bottom row, not in the historical row that previously shared the same absolute
  screen coordinate.
- Dirty-state testing clears prior dirt before consuming bottom-row pending wrap
  and verifies both visible active rows are dirtied by the scroll/write.

This experiment did not implement scroll regions, no-scrollback row rotation,
index control dispatch, margins, SGR background preservation, insert mode,
Unicode width, wide characters, zero-width characters, grapheme clustering,
charsets, styles, hyperlinks, semantic prompt state, alternate screen behavior,
CSI, OSC, DCS, APC, PTY IO, public API, or public ABI.

Verification run:

```text
cargo fmt
cargo test -p roastty stream
cargo test -p roastty terminal_formatter
cargo test -p roastty terminal::terminal
cargo test -p roastty screen_formatter
cargo test -p roastty page_string
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

Results:

- `cargo fmt` passed.
- `cargo test -p roastty stream` passed 90 tests.
- `cargo test -p roastty terminal_formatter` passed 67 tests.
- `cargo test -p roastty terminal::terminal` passed 85 tests.
- `cargo test -p roastty screen_formatter` passed 55 tests.
- `cargo test -p roastty page_string` passed 12 tests.
- `cargo test -p roastty terminal::page_list` passed 524 tests.
- Full `cargo test -p roastty` passed 991 unit tests, the ABI harness, and
  doc-tests.

Codex design review passed after all real design findings were fixed.

Result-review artifacts:

- Prompt: `logs/codex-review/20260601-012955-206976-prompt.md`
- Result: `logs/codex-review/20260601-012955-206976-last-message.md`

Codex found no blocking correctness, upstream-fidelity, coordinate-domain,
formatter, scrollback-preservation, transactionality, dirty/wrap metadata,
missing-test, or scope findings. Codex approved the result for commit.

## Conclusion

Roastty's narrow basic print path now has Ghostty-style pending wrap through the
bottom of the visible active area, including scrollback growth and active-domain
cursor semantics. The implementation can fill the bottom row, carry pending wrap
across feed calls, scroll the active area on the next printable character,
preserve the scrolled row in history, and continue writing into the visible
bottom row afterward.

The next print experiment should continue along upstream `Terminal.print()` /
`printWrap()` by choosing the next smallest missing behavior: either explicit
index control dispatch on the active full-width path, no-scrollback row
rotation, or wraparound mode plumbing. Margins, styles, hyperlinks, wide
characters, graphemes, and full scroll-region behavior remain deferred.
