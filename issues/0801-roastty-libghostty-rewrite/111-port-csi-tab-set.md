# Experiment 111: Port CSI Tab Set

## Description

Continue the stream/action port by adding the CSI cursor tabulation control
forms that Ghostty maps to horizontal tab set:

- `CSI W`
- `CSI 0 W`

Experiment 110 already added the private `Action::TabSet` and terminal behavior
for setting a tabstop at the active cursor column through `ESC H`. Upstream
Ghostty treats `CSI W` and `CSI 0 W` as the same `.tab_set` action. Roastty
currently consumes all CSI finals without dispatching any action, and has a test
that keeps `CSI W` unsupported. This experiment replaces that deliberate gap
with upstream-compatible tab-set behavior.

This experiment is intentionally narrow. It does not implement `CSI 2 W`
(`tab_clear_current`), `CSI 5 W` (`tab_clear_all`), `CSI ? 5 W` (`tab_reset`),
horizontal-tab-back, or a general CSI parameter/action system.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/stream.zig` for the
     `Cursor Tabulation Control` branch:
     - `W` with no intermediates and no parameters emits `.tab_set`;
     - `W` with no intermediates and one parameter `0` emits `.tab_set`;
     - `W` with a private marker/intermediate or other parameters does not emit
       `.tab_set` for this experiment.
   - Use the upstream `stream CSI W tab set` tests as the parity target for
     `CSI W`, `CSI 0 W`, `CSI > W`, and `CSI 99 W`.
   - Do not modify `vendor/ghostty/`.

2. Extend CSI tracking only as far as this action requires.
   - Replace the bare `EscapeState::Csi` marker with a small private CSI state.
   - Track whether the in-progress CSI remains a valid tab-set candidate:
     - no parameters is valid for `W`;
     - exactly one numeric parameter whose value is zero is valid for `W`;
     - private markers such as `?` or `>`, intermediates, semicolons, non-zero
       digits, multiple parameters, overflow, or unsupported bytes make the
       sequence invalid for tab-set dispatch.
   - Keep unsupported CSI finals consumed and ignored, preserving the existing
     no-leak behavior for sequences like `ESC [ C`.
   - Do not introduce a public CSI parser API.
   - Do not implement any non-`W` CSI action.

3. Dispatch `TabSet` for valid `W` finals.
   - On CSI final byte `W`, dispatch the existing private `Action::TabSet` only
     when the CSI state is either empty-param or single-zero-param.
   - Set the stream parser back to ground before invoking the handler, so a
     handler error cannot leave the parser stuck in CSI state.
   - Preserve pending invalid UTF-8 behavior: if an incomplete invalid UTF-8
     sequence is interrupted by `CSI W` or `CSI 0 W`, dispatch `U+FFFD` before
     the CSI sequence dispatches `TabSet`.

4. Reuse existing terminal tab-set behavior.
   - Keep `TerminalStreamHandler`'s existing `Action::TabSet` path from
     Experiment 110.
   - Do not add new tabstop helpers unless implementation proves the existing
     `Screen::tab_set_basic()` helper is insufficient.
   - Do not modify cells, cursor position, dirty rows, pending wrap, public API,
     or public ABI.

5. Add tests.
   - Stream parser tests:
     - `A\x1b[WB` dispatches print, tab-set, print in order;
     - `A\x1b[0WB` dispatches print, tab-set, print in order;
     - split-feed `CSI W` and `CSI 0 W` dispatch `TabSet`;
     - pending invalid UTF-8 dispatches `U+FFFD` before same-slice and
       split-feed `CSI W`;
     - a handler error from `TabSet` leaves the parser in ground state for the
       next byte;
     - `CSI > W`, `CSI ? W`, `CSI 99 W`, `CSI 1 W`, and `CSI 0 ; 0 W` do not
       dispatch `TabSet`;
     - deferred upstream neighbor forms `CSI 2 W`, `CSI 5 W`, and `CSI ? 5 W` do
       not dispatch `TabSet` or any new action in this experiment;
     - an overflowing numeric parameter before `W` does not dispatch `TabSet`
       and the parser recovers for the next byte;
     - unsupported non-`W` CSI finals still do not leak printable bytes.
   - Terminal tests:
     - after clearing default tabstops, printing three cells and receiving
       `CSI W` sets a tabstop at column 3;
     - after clearing default tabstops, printing three cells and receiving
       `CSI 0 W` sets a tabstop at column 3;
     - a later `HT` can use a tabstop set by `CSI W`;
     - `CSI W` leaves cursor position, pending wrap, dirty rows, and cells
       unchanged by itself.
   - Existing printable, pending-wrap, wrap-scroll, LF/CR, VT/FF, backspace,
     horizontal-tab, `ESC H`, formatter, PageList, and stream tests must keep
     passing.

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
     - the CSI tracking shape;
     - exact accepted and rejected CSI `W` forms;
     - stream parser state behavior on handler error;
     - terminal tabstop behavior;
     - what remains deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- the stream parser dispatches `CSI W` as `Action::TabSet`;
- the stream parser dispatches `CSI 0 W` as `Action::TabSet`;
- split-feed `CSI W` and `CSI 0 W` dispatch the same action;
- unsupported CSI `W` variants such as `CSI > W`, `CSI ? W`, `CSI 99 W`,
  `CSI 1 W`, and `CSI 0 ; 0 W` remain ignored;
- deferred upstream neighbor forms `CSI 2 W`, `CSI 5 W`, and `CSI ? 5 W` remain
  ignored in this experiment;
- overflowing numeric parameters before `W` remain ignored and the parser
  recovers for the next byte;
- unsupported non-`W` CSI finals remain ignored and do not leak bytes;
- pending invalid UTF-8 emits `U+FFFD` before CSI tab-set dispatch;
- a handler error from CSI tab-set leaves the parser in ground state for the
  next byte;
- `CSI W` and `CSI 0 W` set a tabstop at the current active cursor column;
- subsequent `HT` can use the tabstop set by CSI tab-set;
- CSI tab-set leaves cursor position and pending wrap unchanged;
- CSI tab-set does not dirty rows or modify cells by itself;
- no tab clear/reset, horizontal-tab-back, margins, origin mode, no-scrollback
  rotation, styles, hyperlinks, wide/Unicode handling, public API, or public ABI
  are added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- `CSI W` dispatches correctly, but the minimal CSI state proves too limited and
  a broader parser structure is needed before `CSI 0 W` can be safely accepted.

The experiment fails if:

- `CSI W` or `CSI 0 W` remains silently ignored;
- CSI bytes leak as printable text;
- unsupported CSI `W` variants dispatch `TabSet`;
- deferred tab clear/reset forms dispatch `TabSet` or any new action;
- overflowing numeric parameters dispatch `TabSet` or leave the parser stuck;
- CSI tab-set sets the wrong column;
- CSI tab-set moves the cursor, clears pending wrap, dirties rows, or modifies
  cells;
- tab clear/reset behavior is accidentally implemented without its own reviewed
  experiment;
- public API or ABI changes are added.

## Design Review

Codex reviewed this design before implementation.

Initial review artifacts:

- Prompt: `logs/codex-review/20260601-021821-558795-prompt.md`
- Result: `logs/codex-review/20260601-021821-558795-last-message.md`

Codex found two real design gaps: deferred upstream neighbor forms (`CSI 2 W`,
`CSI 5 W`, and `CSI ? 5 W`) needed explicit negative tests, and the overflow
invalid-state rule needed an explicit recovery test. The design was updated for
both.

Clean design re-review artifacts:

- Prompt: `logs/codex-review/20260601-021944-123196-prompt.md`
- Result: `logs/codex-review/20260601-021944-123196-last-message.md`

Codex found no remaining blockers and approved implementation.
