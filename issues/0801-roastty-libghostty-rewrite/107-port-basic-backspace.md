# Experiment 107: Port Basic Backspace

## Description

Continue the C0 execute-action port by adding basic backspace (`BS`, `0x08`).

Experiment 106 added LF and CR, so normal `hello\r\nworld` text now works. The
next smallest upstream behavior is backspace. In Ghostty, stream `backspace`
calls `Terminal.backspace()`, which calls `cursorLeft(1)`. In the default
non-reverse-wrap mode, `cursorLeft(1)` clamps at column 0, clears pending wrap,
and does not dirty rows by itself because moving the cursor does not change
rendered content.

This experiment ports only that default active full-width behavior:

- dispatch `0x08` as a private `Backspace` action;
- move left by one column, clamped at active column 0;
- clear pending wrap;
- do not modify cells;
- do not dirty rows just because the cursor moved.

Reverse wrap, reverse-wrap-extended, cursor-left CSI, delete/erase behavior,
wide characters, and tabs remain separate experiments.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/termio/stream_handler.zig` for `.backspace`.
   - Use `vendor/ghostty/src/terminal/Terminal.zig` for:
     - `backspace()`;
     - `cursorLeft()`;
     - `Terminal: backspace`;
     - `Terminal: cursorLeft unsets pending wrap state`;
     - `Terminal: cursorLeft unsets pending wrap state with longer jump`.
   - Do not modify `vendor/ghostty/`.

2. Extend the stream action surface privately.
   - Add private `Action::Backspace`.
   - In ground state, dispatch `0x08` as `Backspace`.
   - Keep other C0 controls outside `BS`, `LF`, and `CR` ignored.
   - Preserve Experiment 102/106 behavior: if a pending invalid UTF-8 sequence
     is interrupted by `BS`, dispatch `U+FFFD` before dispatching `Backspace`.

3. Add active full-width backspace behavior.
   - Add a private `Screen` helper:
     - clear `pending_wrap`;
     - if `cursor.x > 0`, decrement `cursor.x` by one;
     - if `cursor.x == 0`, leave it at zero;
     - do not change `cursor.y`;
     - do not dirty rows;
     - do not modify cells.
   - Wire `TerminalStreamHandler::vt(Action::Backspace)` to the helper.
   - Do not add public API or ABI.

4. Add tests.
   - Stream parser tests:
     - `A\x08B` dispatches print, backspace, print in order;
     - other C0 controls besides `BS`, `LF`, and `CR` remain ignored;
     - pending invalid UTF-8 dispatches `U+FFFD` before `BS`.
   - Terminal tests:
     - `hello\x08y` formats as `helly`;
     - backspace at column 0 stays at column 0 and does not modify content;
     - backspace clears pending wrap, so `ABCDE\x08X` on a 5-column terminal
       formats as `ABCXE` rather than wrapping first, matching upstream's
       `Terminal: cursorLeft unsets pending wrap state` test;
     - the pending-wrap test also asserts cursor position after `BS` or after
       the following printable byte;
     - backspace does not dirty rows by itself, verified by clearing dirty state
       before issuing `BS`;
     - split-feed backspace works when the printable bytes and `BS` arrive in
       separate `next_slice` calls.
   - Existing printable, LF/CR, pending-wrap, wrap-scroll, formatter, PageList,
     and stream tests must keep passing.

5. Verify.
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

6. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix all real design findings before implementation.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real result findings before proceeding.

7. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - stream action changes;
     - active backspace behavior;
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

- the stream parser dispatches `BS` in order with printable actions;
- other C0 controls outside `BS`, `LF`, and `CR` remain ignored;
- pending invalid UTF-8 emits `U+FFFD` before an interrupting `BS`;
- `hello\x08y` formats as `helly`;
- `BS` at column 0 stays at column 0;
- `BS` clears pending wrap without soft-wrapping first;
- `BS` does not dirty rows or modify cells by itself;
- split-feed `BS` behaves the same as same-slice `BS`;
- no reverse wrap, reverse-wrap-extended, tabs, NEL, RI, C1 controls,
  linefeed-mode changes, margins, scroll regions, no-scrollback rotation,
  styles, hyperlinks, wide/Unicode handling, public API, or public ABI are
  added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- the default no-reverse-wrap behavior works in `Screen`, but a stream action
  refactor is needed before dispatching `BS` safely.

The experiment fails if:

- `BS` remains silently ignored;
- `BS` at column 0 underflows or moves to a previous row;
- `BS` soft-wraps pending wrap before clearing it;
- `BS` dirties rows or modifies cells without a following printable write;
- reverse-wrap behavior is added without a separate reviewed experiment;
- public API or ABI changes are added.

## Design Review

Codex reviewed this design before implementation.

Initial review artifacts:

- Prompt: `logs/codex-review/20260601-014053-149154-prompt.md`
- Result: `logs/codex-review/20260601-014053-149154-last-message.md`

Codex found one real design issue: the pending-wrap test expected `ABCDE\x08X`
to format as `ABCDX`, but upstream Ghostty expects `ABCXE` for the default
no-reverse-wrap path. The design was corrected to expect `ABCXE` and to assert
cursor position around the backspace/following-print path.

Clean design re-review artifacts:

- Prompt: `logs/codex-review/20260601-014149-223678-prompt.md`
- Result: `logs/codex-review/20260601-014149-223678-last-message.md`

Codex found no remaining design blockers and approved implementation.
