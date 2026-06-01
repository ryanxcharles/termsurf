# Experiment 122: Port CSI Delete Character

## Description

Continue the stream/action port by adding Ghostty's delete-character form:

- `CSI P` / `CSI 1 P` -> delete one character at the cursor;
- `CSI n P` -> delete `n` characters at the cursor;
- remaining cells to the right shift left;
- blank cells are inserted at the right edge of the active horizontal margin;
- the cursor position is preserved.

Upstream Ghostty parses this in `vendor/ghostty/src/terminal/stream.zig`:

- final `P` emits `.delete_chars`;
- no params means count `1`;
- one param emits that exact count, including `0`;
- more than one param is invalid and dispatches no action;
- any private/intermediate form is invalid for this command.

Upstream Ghostty executes this in
`vendor/ghostty/src/terminal/Terminal.zig::deleteChars`:

- count `0` is a no-op;
- if the cursor is outside the active left/right scrolling margins, the command
  is a no-op;
- the effective count is clamped to the remaining cells from the cursor through
  the right margin;
- cells after the deleted range shift left within the horizontal margin;
- vacated cells at the right margin are cleared;
- the cursor row's soft-wrap state is reset;
- pending wrap is cleared when a delete actually happens;
- the cursor row is dirtied;
- the cursor position is preserved.

This experiment ports the current basic DCH behavior into Roastty. It should
reuse Page/PageList cell move and clear primitives where possible, but it must
not silently use an incompatible primitive: Ghostty allows same-row overlapping
left shifts for DCH, while Roastty's existing `Page::move_cells()` explicitly
rejects same-row overlapping ranges. If no safe same-row shift helper exists,
add a narrow helper for DCH rather than weakening `move_cells()` globally.

This experiment intentionally does not implement `CSI L`/`CSI M` insert/delete
line first. Those commands are the immediately preceding upstream stream
actions, but they require a larger row-shifting design across vertical scrolling
regions. `CSI P` is the next narrower adjacent CSI mutation and gives Roastty
the row-local shift primitive needed by later editing commands.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/stream.zig` for `CSI P` parsing:
     - no params -> count `1`;
     - one param -> exact count;
     - multiple params -> invalid;
     - private/intermediate forms -> invalid.
   - Use `vendor/ghostty/src/terminal/Terminal.zig::deleteChars` for terminal
     semantics.
   - Use upstream tests around `Terminal: deleteChars` as the behavior
     checklist.
   - Do not modify `vendor/ghostty/`.

2. Extend the private stream action.
   - Add `Action::DeleteChars { count }` in `roastty/src/terminal/stream.rs`.
   - Keep the action internal to the terminal module.
   - Do not add public API or ABI surface.

3. Extend CSI dispatch for final `P`.
   - Add a `delete_chars_action()` helper.
   - Accept:
     - `CSI P`;
     - `CSI 0 P`;
     - `CSI ; P`;
     - `CSI 1 P`;
     - `CSI 1 ; P`;
     - larger single numeric params, clamped to `u16::MAX` by the current
       parser's numeric accumulator behavior.
   - Reject and dispatch no action for:
     - private forms such as `CSI ? P` and `CSI > P`;
     - real multi-param forms such as `CSI 1 ; 2 P` and `CSI ;; P`;
     - colon/mixed separators;
     - any byte that the current parser treats as invalid;
     - direct C1 CSI byte `0x9b`, which remains out of scope and follows the
       current UTF-8 replacement behavior.
   - Preserve parser ground-state behavior on handler errors.
   - Preserve pending invalid UTF-8 behavior: if an incomplete invalid UTF-8
     sequence is interrupted by `CSI P`, dispatch `U+FFFD` before the
     delete-character action.

4. Add a safe row-local delete primitive.
   - Add the narrow Page/PageList helper needed to shift cells left within one
     active row.
   - The helper should:
     - operate on one active row only;
     - shift the source range `[cursor + count, right_margin + 1)` left to
       `[cursor, right_margin + 1 - count)`;
     - clear the vacated range at the right side of the horizontal margin;
     - release managed memory for overwritten and cleared cells;
     - preserve moved grapheme/style/hyperlink ownership correctly;
     - mark the cursor row dirty;
     - preserve row metadata until the Screen/Terminal layer resets wrap state.
   - Do not weaken `Page::move_cells()`'s same-row overlap rejection unless
     Codex review and implementation evidence show that a general overlap-safe
     move is the correct primitive. Prefer a DCH-specific helper if the scope is
     uncertain.
   - Stop and record Partial if managed-memory movement cannot be made
     integrity-safe in this experiment.

5. Add screen/terminal delete-character behavior.
   - Add
     `Screen::delete_chars_basic(count, rows, cols, left_margin, right_margin)`.
   - Pass only the existing `Terminal::scrolling_region` horizontal bounds into
     the helper:
     - `left_margin = scrolling_region.left`;
     - `right_margin = scrolling_region.right`.
   - Do not pass the whole `ScrollingRegion` into `Screen`, and do not gate DCH
     on the vertical top/bottom scrolling region. Upstream `deleteChars` checks
     only horizontal margins.
   - If `count == 0`, do nothing:
     - no dirty rows;
     - no pending-wrap change;
     - no cursor movement.
   - If the cursor is outside the horizontal scrolling margins, do nothing and
     preserve pending wrap, matching upstream's existing behavior.
   - Clamp count to the remaining columns from cursor through right margin.
   - Shift cells left and blank the vacated right-side cells.
   - Reset cursor-row soft-wrap metadata after an actual delete:
     - clear pending wrap;
     - clear cursor row `wrap` if set;
     - clear the next active row's `wrap_continuation` if the cursor row was
       wrapped and a next row exists.
   - Preserve cursor position.
   - Do not mutate rows above or below the cursor, except for the next row's
     wrap-continuation metadata when resetting a wrapped cursor row.
   - Do not mutate scrollback.
   - Current-SGR blank-cell coloring remains deferred because Roastty's current
     basic print path does not yet write cells with cursor style. Add tests
     proving default blank behavior now, and document SGR-preserving blanks as
     deferred until the SGR mutation path exists.

6. Route terminal actions.
   - In `TerminalStreamHandler`, route `Action::DeleteChars` to the new helper.
   - Reuse the existing error conversion style.
   - Existing print, linefeed, cursor, positioning, tab, erase-display,
     erase-line, formatter, PageList, and ABI behavior must keep passing
     unchanged.

7. Add tests.
   - Stream parser tests:
     - `A\x1b[PB` dispatches print `A`, delete count `1`, print `B`;
     - `CSI P` dispatches count `1`;
     - `CSI 0 P` and `CSI ; P` dispatch count `0`;
     - `CSI 1 P` and `CSI 1 ; P` dispatch count `1`;
     - larger single params dispatch their parsed/clamped value;
     - real multi-param, colon-param, mixed-separator, and invalid-private forms
       dispatch no action and do not leak printable final bytes;
     - split-feed `CSI P` and `CSI 3 P` dispatch correctly;
     - pending invalid UTF-8 emits `U+FFFD` before same-slice and split-feed
       `CSI P`;
     - direct C1 CSI byte `0x9b` followed by `P` remains out of scope and
       dispatches `U+FFFD` plus printable `P`;
     - handler errors from delete-character leave the parser in ground state;
     - existing cursor, positioning, line, tab, erase-display, erase-line, and
       `CSI I` behavior remains unchanged.
   - Terminal tests:
     - count `1` deletes the cursor cell and shifts the row suffix left;
     - count `2` deletes two cells;
     - count larger than the remaining margin clamps to the remaining margin;
     - count `0` is a no-op and preserves pending wrap;
     - cursor position is preserved;
     - pending wrap is cleared after an actual delete;
     - cursor-row wrap metadata is reset after an actual delete;
     - left/right horizontal margins constrain the shift and blank range;
     - DCH still works when the cursor row is outside the vertical
       `scrolling_region.top..bottom` bounds as long as the cursor column is
       inside the horizontal margin;
     - cursor outside horizontal margins is a no-op and preserves pending wrap;
     - rows above, rows below, and scrollback are not mutated;
     - affected cursor row becomes dirty;
     - unaffected rows do not become dirty, except next-row continuation dirty
       state if wrap-continuation metadata is actually cleared;
     - managed grapheme/style/hyperlink movement and cleanup stays
       integrity-safe, either through targeted Page/PageList tests or terminal
       tests using existing test helpers;
     - non-managed cell metadata moves with shifted cells, including a test that
       seeds a protected cell in the shifted source range and verifies the
       protection bit lands on the shifted destination cell;
     - unsupported/invalid `CSI P` forms do not mutate terminal state;
     - split-feed `CSI P` mutates terminal state correctly.
   - Existing stream, cursor movement, positioning, tabstop, erase-display,
     erase-line, formatter, PageList, and ABI tests must keep passing.

8. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty stream
     cargo test -p roastty terminal::terminal
     cargo test -p roastty terminal::page_list
     cargo test -p roastty terminal_formatter
     cargo test -p roastty screen_formatter
     cargo test -p roastty page_string
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

9. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix all real design findings before implementation.
   - Record the design-review outcome in this experiment file before
     implementation.
   - Commit the approved design before implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real result findings before proceeding.
   - Commit the approved result separately from the design commit.

10. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - accepted `CSI P` forms;
      - rejected `CSI P` forms;
      - terminal behavior for count `0`, count `1`, larger counts, clamping,
        margins, pending wrap, cursor preservation, and row metadata;
      - managed-memory safety evidence;
      - dirty-row behavior;
      - deferred SGR blank-cell coloring and wide-boundary behavior, if still
        deferred;
      - confirmation that existing raw print, cursor, positioning, line, tab,
        erase-display, erase-line, formatter, PageList, and ABI behavior did not
        regress;
      - verification command output summary;
      - Codex design-review outcome;
      - Codex result-review outcome.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Design Review

Codex reviewed the initial design and found two real issues:
`logs/codex-review/20260601-043724-379119-last-message.md`.

- The first design passed the whole `ScrollingRegion` into
  `Screen::delete_chars_basic`, which risked accidentally gating DCH on vertical
  top/bottom margins. The design now passes only horizontal `left_margin` /
  `right_margin` and explicitly tests that vertical bounds do not suppress DCH.
- The first design covered managed memory movement but did not explicitly test
  non-managed cell metadata. The design now requires a protected-bit movement
  test for shifted cells.

Codex re-reviewed the updated design and reported no findings:
`logs/codex-review/20260601-044034-466353-last-message.md`.

The second review confirmed the prior findings were fixed and the design is
ready to commit before implementation.

## Verification

The experiment passes if:

- `CSI P` dispatches and performs delete-character count `1`;
- `CSI 0 P` and `CSI ; P` dispatch count `0` and no-op at terminal level;
- single-count `CSI n P` forms dispatch and perform the corresponding clamped
  row-local delete;
- invalid/private/colon/mixed/multi-param forms dispatch no delete-character
  action and do not leak printable bytes;
- direct C1 CSI byte `0x9b` remains out of scope and follows current raw-C1
  UTF-8 replacement behavior;
- pending invalid UTF-8 emits `U+FFFD` before the delete-character action;
- handler errors leave the parser in ground state;
- delete-character shifts only the intended cells inside the active horizontal
  margin;
- vertical scrolling-region bounds do not suppress DCH when the cursor column is
  inside the horizontal margin;
- cursor position is preserved;
- actual deletes clear pending wrap and reset cursor-row wrap metadata;
- count `0` and outside-margin no-ops preserve pending wrap;
- rows above, rows below, and scrollback are not mutated;
- affected row becomes dirty and unaffected rows do not, except for deliberate
  next-row wrap-continuation metadata changes;
- managed grapheme/style/hyperlink movement and cleanup remains integrity-safe;
- non-managed cell metadata such as the protected bit moves with shifted cells;
- existing raw print, linefeed, cursor, positioning, tabstop, erase-display,
  erase-line, PageList, formatter, and ABI behavior remains unchanged;
- no insert/delete line, scroll up/down, SGR, Unicode-width, wide-character
  rendering, public API, ABI, or non-macOS behavior is added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- stream dispatch is correct but terminal mutation exposes a missing Page or
  PageList primitive that should be designed separately;
- plain-cell delete works but managed grapheme/style/hyperlink movement cannot
  be made integrity-safe in this slice;
- full-width delete works but horizontal-margin support needs a separate helper;
- behavior is correct for cells but dirty tracking or row metadata requires an
  additional narrow primitive before the result can be called complete.

The experiment fails if:

- it changes unrelated cursor, tab, print, erase-display, erase-line, formatter,
  or ABI behavior;
- it treats `CSI 0 P` as count `1`;
- it mutates cells outside the active horizontal margin;
- it corrupts PageList integrity or leaks managed memory;
- it silently implements incompatible placeholder delete semantics;
- it adds unrelated insert/delete line, scroll up/down, public API, ABI, or
  non-macOS behavior.
