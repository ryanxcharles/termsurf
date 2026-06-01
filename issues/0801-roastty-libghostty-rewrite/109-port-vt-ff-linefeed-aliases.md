# Experiment 109: Port VT and FF Linefeed Aliases

## Description

Continue the C0 execute-action port by adding vertical tab (`VT`, `0x0b`) and
form feed (`FF`, `0x0c`) as linefeed/index aliases in the stream path.

Experiment 106 added basic LF (`0x0a`) and CR (`0x0d`). Experiment 108 added HT.
Roastty still ignores `VT` and `FF`. In Ghostty's stream path, the parser maps
LF, VT, and FF to the same `.linefeed` action, and the stream handler
deliberately calls `Terminal.index()` for that action. That means stream `VT`
and `FF` should have the same index-style behavior as stream `LF`:

- move down one row;
- preserve the current column;
- clear pending wrap through the existing index/LF movement;
- scroll at the bottom row;
- do not apply `Mode::Linefeed` automatic carriage return in the stream path.

This experiment ports only those stream aliases. `Terminal.linefeed()` mode
semantics, CSI mode set/reset parsing, newline preprocessing, NEL, IND, RI,
margins, origin mode, and public API/ABI remain separate experiments.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/termio/stream_handler.zig` for `.linefeed`.
   - Confirm that the stream handler calls `Terminal.index()`, not
     `Terminal.linefeed()`.
   - Use `vendor/ghostty/src/terminal/Terminal.zig` for the existing index/LF
     movement behavior already ported in Experiment 106.
   - Do not modify `vendor/ghostty/`.

2. Extend stream dispatch.
   - Keep private `Action::LineFeed` unchanged.
   - In ground state, dispatch `0x0b` (`VT`) and `0x0c` (`FF`) as
     `Action::LineFeed`.
   - Keep LF (`0x0a`) dispatch unchanged.
   - Keep other C0 controls outside `BS`, `HT`, `LF`, `VT`, `FF`, and `CR`
     ignored.
   - Preserve Experiment 102/106/107/108 behavior: if a pending invalid UTF-8
     sequence is interrupted by `VT` or `FF`, dispatch `U+FFFD` before
     dispatching `LineFeed`.

3. Reuse existing terminal behavior.
   - Do not add a new terminal action.
   - Do not add a new screen helper.
   - `VT` and `FF` should reach the same `TerminalStreamHandler::line_feed()`
     path as `LF`.
   - Do not read or change `Mode::Linefeed`; Ghostty's stream linefeed path
     bypasses `Terminal.linefeed()` and uses index semantics.
   - Do not add public API or ABI.

4. Add tests.
   - Stream parser tests:
     - `A\x0bB\x0cC` dispatches print, linefeed, print, linefeed, print in
       order;
     - other C0 controls besides `BS`, `HT`, `LF`, `VT`, `FF`, and `CR` remain
       ignored;
     - pending invalid UTF-8 dispatches `U+FFFD` before an interrupting `VT`;
     - pending invalid UTF-8 dispatches `U+FFFD` before an interrupting `FF`.
   - Terminal tests:
     - `A\x0bB` behaves like `A\nB`, preserving column for `B`;
     - `A\x0cB` behaves like `A\nB`, preserving column for `B`;
     - with `Mode::Linefeed` enabled, `A\x0bB` still formats as `A\n B`, proving
       stream `VT` bypasses automatic CR;
     - with `Mode::Linefeed` enabled, `A\x0cB` still formats as `A\n B`, proving
       stream `FF` bypasses automatic CR;
     - `VT` and `FF` clear pending wrap without soft-wrapping first, matching
       the existing LF/index behavior;
     - `VT` and `FF` scroll at the bottom row through the same path as LF;
     - split-feed `VT` and `FF` work when printable bytes and the control bytes
       arrive in separate `next_slice` calls.
   - Update existing ignored-control tests so they no longer use `VT` or `FF` as
     ignored examples.
   - Existing printable, pending-wrap, wrap-scroll, LF/CR, backspace,
     horizontal-tab, formatter, PageList, and stream tests must keep passing.

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
   - Commit the approved design before implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real result findings before proceeding.
   - Commit the approved result separately from the design commit.

7. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - stream action changes;
     - VT/FF alias behavior;
     - pending-wrap behavior;
     - bottom-row scroll behavior;
     - what remains deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- the stream parser dispatches `VT` and `FF` as `Action::LineFeed`;
- other C0 controls outside `BS`, `HT`, `LF`, `VT`, `FF`, and `CR` remain
  ignored;
- pending invalid UTF-8 emits `U+FFFD` before interrupting `VT` and `FF` bytes;
- `VT` and `FF` preserve the current column the same way stream `LF` does;
- `VT` and `FF` preserve the current column even when `Mode::Linefeed` is
  enabled, proving the stream path bypasses automatic CR;
- `VT` and `FF` clear pending wrap without soft-wrapping first through the
  existing LF/index movement;
- `VT` and `FF` scroll at the bottom row through the same path as LF;
- split-feed `VT` and `FF` behave the same as same-slice input;
- no `Mode::Linefeed` behavior, CSI mode set/reset parsing, DEC private modes,
  newline preprocessing, NEL, IND, RI, margins, origin mode, no-scrollback
  rotation, styles, hyperlinks, wide/Unicode handling, public API, or public ABI
  are added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- `VT` and `FF` dispatch as stream actions, but a later refactor is needed to
  share all LF/index behavior cleanly.

The experiment fails if:

- `VT` or `FF` remains silently ignored;
- `VT` or `FF` moves to column 0 instead of preserving the current column;
- `VT` or `FF` applies `Mode::Linefeed` behavior in the stream path;
- `VT` or `FF` soft-wraps pending wrap before the linefeed/index movement;
- `VT` or `FF` fails to scroll at the bottom row;
- CSI mode parsing or unrelated mode behavior is added without a separate
  reviewed experiment;
- public API or ABI changes are added.

## Design Review

Codex first reviewed an earlier linefeed-mode design for Experiment 109.

Rejected design-review artifacts:

- Prompt: `logs/codex-review/20260601-015841-958217-prompt.md`
- Result: `logs/codex-review/20260601-015841-958217-last-message.md`

Codex found a real upstream-fidelity issue: Ghostty's stream linefeed action
calls `Terminal.index()` directly, not `Terminal.linefeed()`, so making incoming
stream LF consult `Mode::Linefeed` would be a divergence. The experiment was
re-scoped to the upstream stream behavior that remains missing in Roastty:
dispatching `VT` and `FF` through the same linefeed/index path as `LF`.

Revised design-review artifacts:

- Prompt: `logs/codex-review/20260601-020131-640695-prompt.md`
- Result: `logs/codex-review/20260601-020131-640695-last-message.md`

Codex found two real design issues in the revised VT/FF plan: it needed tests
that enable `Mode::Linefeed` and prove `VT`/`FF` still bypass automatic CR, and
it needed to acknowledge existing ignored-control tests must stop using `VT` or
`FF` as ignored examples. The design was updated for both.

Clean design re-review artifacts:

- Prompt: `logs/codex-review/20260601-020258-599631-prompt.md`
- Result: `logs/codex-review/20260601-020258-599631-last-message.md`

Codex found no remaining blockers and approved implementation.
