# Experiment 108: Port Basic Horizontal Tab

## Description

Continue the C0 execute-action port by adding basic horizontal tab (`HT`,
`0x09`).

Experiments 102-107 made printable text, pending wrap, LF, CR, and BS work in
the narrow active full-width stream path. Roastty already has the `Tabstops`
state ported and formatter-covered from Experiment 100, but incoming `HT` bytes
are still ignored. In Ghostty, stream `horizontal_tab` calls
`Terminal.horizontalTab()`, which moves the cursor right until it reaches the
next tabstop or the right edge of the current scrolling region.

This experiment ports only the default active full-width behavior:

- dispatch `0x09` as a private `HorizontalTab` action;
- move to the next tabstop strictly after the current cursor column;
- clamp at the active right edge (`cols - 1`) when no later tabstop exists;
- preserve pending wrap, matching Ghostty's lower-level `Screen.cursorRight()`
  path;
- do not modify cells;
- do not dirty rows just because the cursor moved.

Margins, origin mode, horizontal-tab-back (`CBT`), tab-clear/set CSI actions,
wide characters, and public API/ABI remain separate experiments.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/termio/stream_handler.zig` for `.horizontal_tab`.
   - Use `vendor/ghostty/src/terminal/Terminal.zig` for:
     - `horizontalTab()`;
     - `Terminal: horizontal tabs`;
     - `Terminal: horizontal tabs starting on tabstop`.
   - Do not modify `vendor/ghostty/`.

2. Extend the stream action surface privately.
   - Add private `Action::HorizontalTab`.
   - In ground state, dispatch `0x09` as `HorizontalTab`.
   - Keep other C0 controls outside `BS`, `HT`, `LF`, and `CR` ignored.
   - Preserve Experiment 102/106/107 behavior: if a pending invalid UTF-8
     sequence is interrupted by `HT`, dispatch `U+FFFD` before dispatching
     `HorizontalTab`.

3. Wire terminal stream handling to tabstop state.
   - Extend `Terminal::next_slice()` so `TerminalStreamHandler` can read the
     immutable `tabstops::Tabstops` while mutating the active `Screen`.
   - Do not clone tabstop state.
   - Do not move tabstop ownership out of `Terminal`.
   - Do not add public API or ABI.

4. Add active full-width horizontal-tab behavior.
   - Add a private `Screen` helper that receives the active column count and the
     current `Tabstops`.
   - The helper:
     - searches columns `(cursor.x + 1)..cols` for the first tabstop;
     - sets `cursor.x` to that tabstop if one exists;
     - otherwise sets `cursor.x` to `cols - 1`;
     - leaves `cursor.y` unchanged;
     - preserves `pending_wrap`;
     - does not dirty rows;
     - does not modify cells.
   - This deliberately mirrors Ghostty's "move first, then test tabstop"
     behavior: starting on a tabstop moves to the next tabstop, not to the same
     column.

5. Add tests.
   - Stream parser tests:
     - `A\tB` dispatches print, horizontal-tab, print in order;
     - other C0 controls besides `BS`, `HT`, `LF`, and `CR` remain ignored;
     - pending invalid UTF-8 dispatches `U+FFFD` before `HT`.
   - Terminal tests:
     - `1\tA` on a 20-column terminal writes `A` at column 8, leaving columns
       1-7 blank;
     - after clearing default tabstops and setting only column 3, `1\tA` writes
       `A` at column 3, proving `HT` uses `Terminal.tabstops` state rather than
       hard-coded 8-column arithmetic;
     - after clearing all tabstops, `HT` clamps to `cols - 1`, proving the
       no-tabstop fallback also reads current tabstop state;
     - repeated `HT` moves the cursor to columns 8, 16, then 19 on a 20-column
       terminal;
     - starting from column 8 moves to the next tabstop at column 16;
     - `HT` at the right edge stays at the right edge;
     - `HT` at the right edge preserves pending wrap, so `ABCDE\tX` on a
       5-column terminal wraps before writing `X`;
     - `HT` does not dirty rows by itself, verified by clearing dirty state
       before issuing `HT`;
     - split-feed `HT` works when printable bytes and `HT` arrive in separate
       `next_slice` calls.
   - Existing printable, pending-wrap, wrap-scroll, LF/CR, backspace, formatter,
     PageList, and stream tests must keep passing.

6. Verify.
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

7. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix all real design findings before implementation.
   - Record the design-review outcome in this experiment file before
     implementation.
   - Commit the approved design before implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real result findings before proceeding.
   - Commit the approved result separately from the design commit.

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - stream action changes;
     - terminal/tabstop borrow wiring;
     - active `HT` cursor behavior;
     - pending-wrap behavior;
     - dirty-state behavior;
     - what remains deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- the stream parser dispatches `HT` in order with printable actions;
- other C0 controls outside `BS`, `HT`, `LF`, and `CR` remain ignored;
- pending invalid UTF-8 emits `U+FFFD` before an interrupting `HT`;
- `1\tA` writes `A` at the next default tabstop, column 8;
- custom-tabstop tests prove `HT` reads `Terminal.tabstops` instead of
  hard-coding default 8-column stops;
- repeated `HT` advances to the next tabstop and clamps at the right edge;
- starting on a tabstop moves to the following tabstop;
- `HT` preserves pending wrap at the right edge;
- `HT` does not dirty rows or modify cells by itself;
- split-feed `HT` behaves the same as same-slice `HT`;
- no margins, origin mode, `CBT`, tab-clear/set CSI behavior, NEL, RI, C1
  controls, linefeed-mode changes, scroll regions, no-scrollback rotation,
  styles, hyperlinks, wide/Unicode handling, public API, or public ABI are
  added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- the default `HT` behavior works in `Screen`, but the stream handler needs a
  larger ownership refactor before it can read `Tabstops` safely.

The experiment fails if:

- `HT` remains silently ignored;
- `HT` lands on the current tabstop instead of the next tabstop;
- `HT` fails to clamp at the right edge;
- `HT` clears pending wrap at the right edge instead of preserving it;
- `HT` dirties rows or modifies cells without a following printable write;
- margin/origin/CSI tab behavior is added without a separate reviewed
  experiment;
- public API or ABI changes are added.

## Design Review

Codex reviewed this design before implementation.

Initial review artifacts:

- Prompt: `logs/codex-review/20260601-014832-213648-prompt.md`
- Result: `logs/codex-review/20260601-014832-213648-last-message.md`

Codex found one real design issue: the default-tabstop tests would allow an
implementation that hard-coded 8-column stops instead of using
`Terminal.tabstops`. The design was corrected to require custom-tabstop
coverage: clear defaults, set only column 3, and verify `HT` moves there; then
clear all tabstops and verify `HT` clamps to the right edge.

Clean design re-review artifacts:

- Prompt: `logs/codex-review/20260601-015043-321806-prompt.md`
- Result: `logs/codex-review/20260601-015043-321806-last-message.md`

Codex found no remaining blockers and approved implementation.

## Result

**Result:** Pass.

Implemented basic default horizontal-tab behavior for the active full-width
stream path.

Stream action changes:

- `Action::HorizontalTab` was added as a private stream action.
- Ground-state `0x09` now dispatches `HorizontalTab`.
- Other C0 controls outside `BS`, `HT`, `LF`, and `CR` remain ignored.
- Pending invalid UTF-8 dispatches `U+FFFD` before an interrupting `HT`.

Terminal and tabstop wiring:

- `TerminalStreamHandler` now borrows immutable `Terminal.tabstops` alongside
  its mutable active-screen borrow.
- The handler does not clone tabstops and does not move ownership out of
  `Terminal`.
- No public API or ABI changed.

Active horizontal-tab behavior:

- It searches for the next tabstop strictly after the current cursor column.
- It moves to that next tabstop when one exists.
- It clamps to `cols - 1` when no later tabstop exists.
- Starting on a tabstop moves to the following tabstop.
- It leaves `cursor.y` unchanged.
- It preserves pending wrap, including at the right edge.
- It does not modify cells.
- It does not dirty rows by itself.

Tested behavior:

- `1\tA` on a 20-column terminal writes `A` at column 8.
- Custom-tabstop coverage clears default tabstops, sets only column 3, and
  verifies `HT` moves there.
- All-tabstops-cleared coverage verifies `HT` clamps to the right edge.
- Repeated `HT` moves to columns 8, 16, then 19 on a 20-column terminal.
- `HT` at the right edge stays there.
- `ABCDE\tX` on a 5-column terminal formats as `ABCDE\nX`, proving `HT`
  preserves pending wrap at the right edge before the next printable byte wraps.
- Dirty-state testing clears prior dirt before issuing `HT` and verifies rows
  remain clean.
- Split-feed horizontal tab behaves the same as same-slice horizontal tab.

This experiment did not implement margins, origin mode, `CBT`, tab-clear/set CSI
behavior, NEL, RI, C1 controls, linefeed-mode changes, scroll regions,
no-scrollback rotation, styles, hyperlinks, wide/Unicode handling, public API,
or public ABI.

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
- `cargo test -p roastty stream` passed 118 tests.
- `cargo test -p roastty terminal_formatter` passed 67 tests.
- `cargo test -p roastty terminal::terminal` passed 107 tests.
- `cargo test -p roastty screen_formatter` passed.
- `cargo test -p roastty page_string` passed.
- `cargo test -p roastty terminal::page_list` passed.
- Full `cargo test -p roastty` passed 1019 unit tests, the ABI harness, and
  doc-tests.

Codex design review passed after the custom-tabstop verification finding was
fixed.

Initial result-review artifacts:

- Prompt: `logs/codex-review/20260601-015411-636447-prompt.md`
- Result: `logs/codex-review/20260601-015411-636447-last-message.md`

Codex found one real upstream-fidelity bug: the first implementation cleared
pending wrap for `HT`, but Ghostty's `horizontalTab()` uses the lower-level
screen cursor movement path and preserves pending wrap at the right edge. The
code, tests, and result language were corrected to preserve pending wrap.

Clean result re-review artifacts:

- Prompt: `logs/codex-review/20260601-015624-288570-prompt.md`
- Result: `logs/codex-review/20260601-015624-288570-last-message.md`

Codex found no remaining blockers and approved the result for commit.

## Conclusion

Roastty now handles basic horizontal tab in the same narrow active full-width
control path as LF, CR, and BS. The implementation uses the existing
`Terminal.tabstops` state, not hard-coded default stops, and preserves the
upstream "move first, then test tabstop" behavior.

The next execute-action experiment should continue through the remaining simple
C0 actions or the next lowest-risk cursor action while keeping margins, origin
mode, CSI tab editing, and reverse-direction tabbing separate.
