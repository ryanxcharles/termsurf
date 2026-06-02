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

# Experiment 125: Port CSI Scroll Up and Down

## Description

Continue the row/scroll mutation subsystem by porting Ghostty's `CSI S` and
`CSI T` commands together:

- `CSI S` / `CSI 1 S` scrolls the active scrolling region upward by one row;
- `CSI n S` scrolls upward by `n` rows;
- `CSI T` / `CSI 1 T` scrolls the active scrolling region downward by one row;
- `CSI n T` scrolls downward by `n` rows.

Upstream Ghostty parses these in `vendor/ghostty/src/terminal/stream.zig`:

- final `S` emits `.scroll_up`;
- final `T` emits `.scroll_down`;
- no params means count `1`;
- one param emits that exact count, including `0`;
- more than one param is invalid and dispatches no action;
- any private/intermediate form is invalid for these commands.

Upstream Ghostty executes these in
`vendor/ghostty/src/terminal/Terminal.zig::scrollUp` and
`Terminal.zig::scrollDown`:

- both commands preserve the absolute cursor position and pending-wrap state;
- `scrollDown(count)` moves temporarily to the top-left of the active scrolling
  region, calls `insertLines(count)`, then restores cursor and pending wrap;
- `scrollUp(count)` usually moves temporarily to the top-left of the active
  scrolling region, calls `deleteLines(count)`, then restores cursor and pending
  wrap;
- `scrollUp(count)` has a special scrollback-preserving path when
  `top_margin == 0`, `left_margin == 0`, and `right_margin == cols - 1`: rows
  scrolled off the top of the region are moved into scrollback like xterm,
  instead of being discarded by `deleteLines`;
- the scroll-up count is clamped to the active scroll-region height before
  scrollback rows are created;
- `max_scrollback = 0` still scrolls visually but does not retain history;
- if the top margin is non-zero or left/right margins are active, `scrollUp`
  uses the delete-lines path and does not create scrollback.

This experiment ports both stream actions and terminal execution. It may reuse
the `insert_lines_basic()` and `delete_lines_basic()` primitives from
Experiments 123 and 124 for all non-scrollback paths. The load-bearing new piece
is the `scrollUp` scrollback path: Roastty must add either a
`cursor_scroll_above_basic()` helper or an equivalent PageList/Screen primitive
that can scroll rows at and above an arbitrary bottom margin into scrollback
while preserving rows below that margin.

Do not implement `CSI @`, `CSI X`, `CSI Z`, SGR, OSC, DCS, alternate-screen
semantics, Kitty graphics, or public ABI in this experiment. Wide-character
boundary repair and current-SGR blank-cell coloring remain deferred under the
same constraints recorded in Experiments 123 and 124.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/stream.zig` for `CSI S`/`CSI T` parsing.
   - Use `vendor/ghostty/src/terminal/Terminal.zig::scrollUp` and
     `Terminal.zig::scrollDown` for execution semantics.
   - Use upstream tests around `Terminal: scrollUp` and `Terminal: scrollDown`
     as the behavior checklist, especially cursor/pending-wrap preservation,
     scrollback creation, `max_scrollback = 0`, top/bottom margins, left/right
     margins, hyperlink/managed metadata movement, and count clamping.
   - Do not modify `vendor/ghostty/`.

2. Extend the private stream action.
   - Add `Action::ScrollUp { count }` and `Action::ScrollDown { count }` in
     `roastty/src/terminal/stream.rs`.
   - Keep both actions internal to the terminal module.
   - Do not add public API or ABI surface.

3. Extend CSI dispatch for finals `S` and `T`.
   - Add `scroll_up_action()` and `scroll_down_action()` helpers.
   - Accept:
     - `CSI S` and `CSI T`;
     - `CSI 0 S` and `CSI 0 T`;
     - `CSI ; S` and `CSI ; T`;
     - `CSI 1 S` / `CSI 1 ; S`;
     - `CSI 1 T` / `CSI 1 ; T`;
     - larger single numeric params, clamped to `u16::MAX` by the current parser
       accumulator behavior.
   - Reject and dispatch no action for:
     - private forms such as `CSI ? S`, `CSI > S`, `CSI ? T`, `CSI > T`;
     - real multi-param forms such as `CSI 1 ; 2 S` and `CSI 1 ; 2 T`;
     - colon/mixed separators;
     - any byte that the current parser treats as invalid;
     - direct C1 CSI byte `0x9b`, which remains out of scope and follows the
       current UTF-8 replacement behavior.
   - Preserve parser ground-state behavior on handler errors.
   - Preserve pending invalid UTF-8 behavior: if an incomplete invalid UTF-8
     sequence is interrupted by `CSI S` or `CSI T`, dispatch `U+FFFD` before the
     scroll action.

4. Add `Screen::scroll_down_basic()`.
   - Preserve `cursor.x`, `cursor.y`, and `cursor.pending_wrap` across the
     command.
   - Temporarily move the cursor to `(left_margin, top_margin)`.
   - Reuse
     `insert_lines_basic(count, top_margin, bottom_margin, left_margin, right_margin, full_width)`.
   - Restore the original cursor and pending-wrap state after the helper
     returns.
   - Count `0` is a no-op except for the temporary move/restore being
     unobservable. It must not dirty rows, mutate cells, or change pending wrap.
   - Since upstream calls `insertLines` from the top of the scrolling region,
     scroll down is not gated by the user's original cursor being inside the
     scrolling region. Preserve that behavior.

5. Add `Screen::scroll_up_basic()`.
   - Preserve `cursor.x`, `cursor.y`, and `cursor.pending_wrap` across the
     command.
   - If `count == 0`, do nothing and preserve pending wrap.
   - If `top_margin == 0`, `left_margin == 0`, and `right_margin == cols - 1`,
     use the scrollback path:
     - clamp count to `bottom_margin + 1`;
     - scroll rows at and above `bottom_margin` upward into scrollback;
     - blank the vacated rows at the bottom of that region;
     - preserve rows below `bottom_margin`;
     - preserve rows outside the full-width region by construction;
     - create scrollback rows when `max_scrollback` allows it;
     - visually scroll even when `max_scrollback = 0`, without retaining
       history;
     - dirty the affected active rows consistently with existing scroll/grow
       behavior;
     - preserve managed grapheme/style/hyperlink ownership and PageList
       integrity.
   - If the scrollback path cannot be implemented safely with current PageList
     primitives, stop and record Partial. Do not silently replace it with the
     delete-lines path, because that would pass some visual tests while losing
     xterm-compatible scrollback behavior.
   - For all other margin configurations, temporarily move the cursor to
     `(left_margin, top_margin)`, call
     `delete_lines_basic(count, top_margin, bottom_margin, left_margin, right_margin, full_width)`,
     and restore the original cursor and pending-wrap state.
   - Current-SGR blank-cell coloring remains deferred because Roastty's current
     basic print path does not yet write cells with cursor style.

6. Route terminal actions.
   - In `TerminalStreamHandler`, route `Action::ScrollUp` and
     `Action::ScrollDown` to the new `Screen` helpers.
   - Pass the current terminal size and scrolling-region bounds.
   - Reuse the existing error conversion style.
   - Existing print, linefeed, cursor, positioning, tab, erase-display,
     erase-line, delete-character, insert-line, delete-line, formatter,
     PageList, and ABI behavior must keep passing unchanged.

7. Add tests.
   - Stream parser tests:
     - `A\x1b[SB` dispatches print `A`, scroll-up count `1`, print `B`;
     - `A\x1b[TB` dispatches print `A`, scroll-down count `1`, print `B`;
     - `CSI S` / `CSI T` dispatch count `1`;
     - `CSI 0 S`, `CSI ; S`, `CSI 0 T`, and `CSI ; T` dispatch count `0`;
     - `CSI 1 S`, `CSI 1 ; S`, `CSI 1 T`, and `CSI 1 ; T` dispatch count `1`;
     - larger single params dispatch their parsed/clamped value;
     - real multi-param, colon-param, mixed-separator, and invalid/private forms
       dispatch no action and do not leak printable final bytes;
     - split-feed `CSI S`, `CSI 3 S`, `CSI T`, and `CSI 3 T` dispatch correctly;
     - pending invalid UTF-8 emits `U+FFFD` before same-slice and split-feed
       `CSI S` / `CSI T`;
     - direct C1 CSI byte `0x9b` followed by `S` or `T` remains out of scope and
       dispatches `U+FFFD` plus printable final byte;
     - handler errors from scroll-up/down leave the parser in ground state;
     - existing parser behavior for cursor, positioning, line, tab,
       erase-display, erase-line, delete-character, insert-line, delete-line,
       and `CSI I` remains unchanged.
   - PageList/Screen tests for the scrollback path:
     - full-width top scroll-up creates scrollback when `max_scrollback > 0`;
     - scrolling up by more than one row creates the expected number of
       scrollback rows and clamps to region height;
     - `max_scrollback = 0` scrolls visually but leaves no retained scrollback;
     - `top_margin == 0` with `bottom_margin < rows - 1` scrolls only rows at
       and above the bottom margin and preserves rows below it;
     - managed grapheme/style/hyperlink metadata moved into scrollback remains
       owned exactly once;
     - vacated active rows are default blanks under the current basic model;
     - PageList integrity holds across page boundaries.
   - Terminal tests:
     - `CSI S` simple full-width top-region scroll produces the upstream
       `DEF\nGHI` shape and creates scrollback when enabled;
     - `CSI S` with `max_scrollback = 0` scrolls visually without retained
       history;
     - `CSI S` with a non-zero top margin uses the delete-lines path and does
       not create scrollback;
     - `CSI S` with left/right margins shifts only bounded columns and does not
       create scrollback;
     - `CSI S` preserves cursor position and pending wrap;
     - `CSI S` count `0` is a no-op;
     - `CSI T` simple scroll-down produces the upstream `\nABC\nDEF\nGHI` shape;
     - `CSI T` with top/bottom margins constrains the shifted/cleared rows;
     - `CSI T` with left/right margins shifts only bounded columns;
     - `CSI T` with the original cursor outside the top/bottom margins still
       mutates the configured scroll region and preserves cursor/pending wrap;
     - `CSI T` with the original cursor outside the left/right margins still
       mutates the configured scroll region and preserves cursor/pending wrap;
     - `CSI T` preserves cursor position and pending wrap;
     - `CSI T` count `0` is a no-op;
     - oversized counts for both commands clamp to the active region height;
     - split-feed `CSI S` / `CSI T` mutates terminal state correctly;
     - unsupported/invalid `CSI S` / `CSI T` forms do not mutate terminal state.
   - Existing stream, cursor movement, positioning, tabstop, erase-display,
     erase-line, delete-character, insert-line, delete-line, formatter,
     PageList, and ABI tests must keep passing.

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
      - accepted and rejected `CSI S` / `CSI T` forms;
      - terminal behavior for count `0`, count `1`, larger counts, clamping,
        margins, pending-wrap preservation, and cursor restoration;
      - scroll-up scrollback behavior for full-width top regions;
      - `max_scrollback = 0` behavior;
      - proof that non-scrollback margin paths reuse insert/delete-line
        semantics correctly;
      - PageList integrity and managed-memory safety evidence;
      - dirty-row behavior;
      - deferred SGR blank-cell coloring and wide-boundary behavior, if still
        deferred;
      - confirmation that existing raw print, cursor, positioning, line, tab,
        erase-display, erase-line, delete-character, insert-line, delete-line,
        formatter, PageList, and ABI behavior did not regress;
      - verification command output summary;
      - Codex design-review outcome;
      - Codex result-review outcome.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `CSI S` and `CSI T` dispatch and execute counts exactly as upstream's
  one-param/default-count parser shape requires;
- invalid/private/colon/mixed/multi-param forms dispatch no scroll action and do
  not leak printable bytes;
- direct C1 CSI byte `0x9b` remains out of scope and follows current raw-C1
  UTF-8 replacement behavior;
- pending invalid UTF-8 emits `U+FFFD` before the scroll action;
- handler errors leave the parser in ground state;
- `CSI T` behaves like moving to the top of the scrolling region, inserting
  lines, and restoring cursor/pending wrap;
- `CSI T` is not gated by the original cursor location; if the original cursor
  is outside the vertical or horizontal margins, the configured scroll region
  still scrolls and the original cursor/pending-wrap state is restored;
- `CSI S` uses delete-lines semantics for non-zero top margins and left/right
  margins;
- `CSI S` uses the scrollback-preserving path for top-origin full-width regions;
- `CSI S` with `max_scrollback = 0` scrolls visually without retaining history;
- both commands preserve absolute cursor position and pending-wrap state;
- count `0` is a no-op for both commands;
- oversized counts clamp to the active region height;
- margin-constrained scrolls mutate only the intended rows/columns;
- rows outside the active scroll region are preserved;
- managed grapheme/style/hyperlink movement and cleanup remains integrity-safe;
- vacated blank cells are default cells with default style under the current
  basic model;
- existing raw print, linefeed, cursor, positioning, tabstop, erase-display,
  erase-line, delete-character, insert-line, delete-line, PageList, formatter,
  and ABI behavior remains unchanged;
- no unrelated CSI, SGR, OSC, DCS, public API, ABI, or non-macOS behavior is
  added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- stream dispatch is correct but the scrollback-preserving `CSI S` path needs a
  dedicated PageList primitive before it can be integrity-safe;
- `CSI T` and non-scrollback `CSI S` paths work, but full-width top-region
  scrollback behavior cannot be implemented safely in this slice;
- visual scroll behavior is correct but managed grapheme/style/hyperlink
  movement into scrollback needs a separate primitive;
- behavior is correct for cells but dirty tracking or row metadata requires an
  additional narrow primitive before the result can be called complete.

The experiment fails if:

- it implements `CSI S` by always calling delete-lines and silently loses
  required scrollback behavior;
- it changes unrelated cursor, tab, print, erase-display, erase-line,
  delete-character, insert-line, delete-line, formatter, or ABI behavior;
- it treats `CSI 0 S` or `CSI 0 T` as count `1`;
- it mutates cells outside the active scrolling margins;
- it corrupts PageList integrity or leaks managed memory;
- it changes cursor position or pending-wrap state after a scroll command;
- it adds unrelated CSI, SGR, OSC, DCS, public API, ABI, or non-macOS behavior.

## Design Review

Codex reviewed the initial design and found one real issue:
`logs/codex-review/20260601-053053-626817-last-message.md`.

- The design correctly stated that `CSI T` scroll-down must not be gated by the
  user's original cursor position, but the test checklist did not explicitly
  require Ghostty's upstream cases where the original cursor is outside the
  top/bottom scrolling region or outside the left/right margins. The terminal
  test checklist and pass criteria now require both cases.

Codex re-reviewed the updated design and reported no remaining blocking
findings: `logs/codex-review/20260601-053457-547975-last-message.md`.

The design is ready to commit before implementation.

## Result

**Result:** Pass

Experiment 125 ports Ghostty's `CSI S` / `CSI T` scroll-up and scroll-down
commands across the current private Roastty terminal stack.

The stream parser now dispatches:

- `Action::ScrollUp { count }` for final `S`;
- `Action::ScrollDown { count }` for final `T`;
- default count `1` for `CSI S` and `CSI T`;
- exact count `0` for `CSI 0 S`, `CSI ; S`, `CSI 0 T`, and `CSI ; T`;
- exact count `1` for `CSI 1 S`, `CSI 1 ; S`, `CSI 1 T`, and `CSI 1 ; T`;
- larger single numeric params using the existing `u16::MAX` parser clamp.

The parser rejects the invalid forms required by the design:

- private forms such as `CSI ? S`, `CSI > S`, `CSI ? T`, and `CSI > T`;
- real multi-param forms such as `CSI 1 ; 2 S` and `CSI 1 ; 2 T`;
- colon and mixed-separator forms;
- direct raw C1 CSI byte `0x9b`, which remains out of scope and follows the
  current UTF-8 replacement behavior;
- handler errors leave the parser in ground state before returning the error;
- pending invalid UTF-8 dispatches `U+FFFD` before same-slice and split-feed
  scroll actions.

The terminal execution path now implements both commands:

- `CSI T` preserves the original cursor and pending-wrap state, temporarily
  moves to the top-left of the configured scrolling region, reuses
  `insert_lines_basic()`, and restores the original cursor state;
- `CSI T` is not gated by the original cursor location; tests cover the original
  cursor outside the vertical region and outside the horizontal margins;
- `CSI S` preserves the original cursor and pending-wrap state;
- `CSI S` uses the scrollback-preserving path when the region starts at the top
  and is full-width;
- `CSI S` creates scrollback when `max_scrollback > 0`;
- `CSI S` visually scrolls but retains no history when `max_scrollback = 0`;
- `CSI S` preserves rows below a partial bottom margin by growing scrollback and
  then inserting blanks at the bottom-margin boundary;
- `CSI S` falls back to the delete-lines path for non-zero top margins and for
  left/right margins, so bounded margin behavior stays aligned with Experiment
  124;
- both commands preserve pending wrap, preserve absolute cursor position, treat
  count `0` as a no-op, and clamp oversized counts to the active region height.

The implementation adds one narrow PageList query:
`PageList::scrollback_disabled()`. It is used only so `CSI S` can implement
Ghostty's `max_scrollback = 0` behavior: visual scroll without retained history.
No public API or ABI surface was added.

The scrollback path is covered by Screen and PageList tests for:

- full-width top-region scroll-up creating scrollback;
- partial bottom-margin scroll-up preserving rows below the margin;
- `max_scrollback = 0` discarding history while still scrolling visually;
- styled cells moving into scrollback without breaking the current managed-cell
  model.
- the exact PageList composition used by the scrollback path preserving style,
  grapheme, and hyperlink metadata with one live reference each;
- the same grow-plus-insert composition preserving integrity across a PageList
  page boundary.

Terminal tests cover simple `CSI S`, simple `CSI T`, margin-constrained paths,
oversized counts, count `0`, split-feed execution, original cursor restoration,
pending-wrap preservation, scrollback creation, and invalid forms not mutating
state. The non-scrollback margin paths reuse the existing insert/delete-line
semantics from Experiments 123 and 124.

Current-SGR blank-cell coloring, Unicode-width boundary repair, wide-character
rendering, alternate-screen semantics, Kitty graphics, and broader public ABI
work remain deferred. None of those were added or required for this slice.

Verification commands:

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

All commands passed. The full `cargo test -p roastty` run reported `1311`
library tests passing, the ABI harness test passing, and doc-tests passing.

Codex design review found one real issue in the initial design: explicit `CSI T`
coverage was needed for original-cursor-outside-margin cases. The design was
updated with those requirements and re-reviewed successfully before
implementation:

- initial design review:
  `logs/codex-review/20260601-053053-626817-last-message.md`;
- approved design re-review:
  `logs/codex-review/20260601-053457-547975-last-message.md`.

Codex result review found two real coverage gaps in the first completed result:
`logs/codex-review/20260601-054400-717810-last-message.md`.

- The managed metadata proof was too indirect: the Screen styled-cell test
  checked visible text and scrollback count, but not style/grapheme/hyperlink
  ownership.
- The scrollback path lacked a targeted page-boundary test for the PageList
  grow-plus-insert composition.

Both findings were fixed by adding PageList tests for managed metadata ownership
and page-boundary integrity, then rerunning the full verification chain above.
Codex re-reviewed the updated result and found no remaining blocking issues:
`logs/codex-review/20260601-054920-781576-last-message.md`. The re-review
explicitly confirmed that both prior blockers were resolved and that this
experiment is good enough to commit as `Pass`.

## Conclusion

`CSI S` and `CSI T` are now implemented for the current Roastty terminal model.
The row/scroll mutation surface now includes insert lines, delete lines, scroll
up, and scroll down, with scroll-up preserving xterm-style scrollback for
top-origin full-width regions.

The next experiment can continue to a neighboring CSI subsystem rather than
another line-mutation primitive. The likely candidates are the remaining basic
CSI mutation commands such as insert characters / erase characters, or a grouped
slice of small CSI reports/settings if those share one implementation surface.
