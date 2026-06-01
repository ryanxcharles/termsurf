# Experiment 119: Port CSI Horizontal Tabulation

## Description

Continue the stream/action port by adding Ghostty's counted forward horizontal
tabulation form:

- `CSI I` -> cursor horizontal tabulation (`CHT`)

Upstream Ghostty parses this in `vendor/ghostty/src/terminal/stream.zig`:

- final `I` emits `.horizontal_tab`;
- no params means count `1`;
- one param is passed through;
- more than one param is invalid and dispatches no action;
- intermediates dispatch no action.

Upstream routing in `vendor/ghostty/src/terminal/stream_terminal.zig` calls
`horizontalTab(count)`, which repeats `Terminal.horizontalTab()` up to `count`
times and stops early if the cursor no longer moves. This means explicit
`CSI 0 I` performs zero tab steps; it is not normalized to one like cursor
movement counts.

Roastty already supports a single raw horizontal tab (`HT`, byte `0x09`) through
`Action::HorizontalTab` and `Screen::horizontal_tab_basic()`. This experiment
extends that existing behavior to a counted internal action and routes `CSI I`
through it:

- raw `HT` still performs one horizontal tab;
- `CSI I` performs one horizontal tab;
- `CSI n I` performs up to `n` horizontal tabs;
- `CSI 0 I` performs zero horizontal tabs;
- semicolon-finalized one-param forms follow Ghostty:
  - `CSI ; I` dispatches count `0`;
  - `CSI 3 ; I` dispatches count `3`;
- repeated tabs stop when the cursor no longer moves;
- existing tab stops and right-edge clamping behavior are preserved;
- no cells are written, no rows are dirtied, and pending-wrap behavior remains
  whatever the existing single-tab helper already defines.

This experiment is intentionally narrow. It does not implement `CSI Z`
back-tabulation, origin-mode/left-right-margin tab behavior beyond the current
full-screen helper, erase-display (`CSI J`), public API, or ABI behavior.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/stream.zig` for `CSI I` dispatch:
     - no params -> count `1`;
     - one param -> that count, including explicit zero;
     - more than one param -> invalid, no action;
     - intermediates -> no action.
   - Use `vendor/ghostty/src/terminal/stream_terminal.zig::horizontalTab` for
     counted routing.
   - Use `vendor/ghostty/src/terminal/Terminal.zig::horizontalTab` for the
     single-step tab behavior.
   - Do not modify `vendor/ghostty/`.

2. Extend the private stream action.
   - Change or extend the private horizontal-tab action so it can carry a count:
     - preferred: `Action::HorizontalTab { count: u16 }`;
     - acceptable alternative: add a separate private counted action if that
       keeps the diff smaller.
   - Raw `HT` should dispatch count `1`.
   - Keep the action internal to the terminal module.
   - Do not add public API or ABI surface.

3. Extend CSI dispatch.
   - Add a count helper for tabulation params that preserves explicit zero:
     - no params -> `1`;
     - one param -> param value as-is;
     - explicit zero -> `0`.
   - Do not reuse `movement_count()`, because cursor movement treats explicit
     zero as one.
   - Dispatch `CSI I` when there is no private marker, no colon/mixed separator,
     no intermediate byte, and zero or one semicolon-finalized numeric param.
   - Semicolon-finalized one-param forms are valid for this new final:
     - `CSI ; I` -> count `0`;
     - `CSI 3 ; I` -> count `3`.
   - Colon params, mixed separators, private markers, intermediates, and real
     multi-param forms dispatch no action and should not leak printable final
     bytes.
   - Preserve all existing `CSI A/B/C/D/E/F/G/H/k/a/j/backtick/d/e/f` and
     `CSI W` behavior. The only new accepted final is `I`.
   - Keep parser transition to ground before invoking the handler, so handler
     errors cannot leave the stream stuck in CSI state.
   - Preserve pending invalid UTF-8 behavior: if an incomplete invalid UTF-8
     sequence is interrupted by `CSI I`, dispatch `U+FFFD` before the
     horizontal-tab action.

4. Add counted terminal behavior.
   - Route raw `HT` as one horizontal tab.
   - Route `CSI I` through the same single-step helper in a loop:
     - repeat at most `count` times;
     - if the cursor x coordinate does not change after a step, stop early;
     - count `0` should do nothing.
   - Preserve existing tab-stop behavior:
     - jump to the next custom/default tabstop when one exists;
     - clamp to the right edge when no later tabstop exists;
     - starting on a tabstop still moves to the next tabstop.
   - Preserve existing pending-wrap and dirty/cell behavior for a single
     horizontal tab. Do not add clearing, writing, dirtying, or scrolling.
   - Do not implement reverse tabulation or margin-aware tabulation in this
     experiment.

5. Add tests.
   - Stream parser tests:
     - raw `HT` dispatches `HorizontalTab { count: 1 }`;
     - `A\x1b[IB` dispatches print `A`, horizontal tab count `1`, print `B`;
     - `CSI 3 I` dispatches count `3`;
     - `CSI 0 I` dispatches count `0`;
     - `CSI ; I` dispatches count `0`;
     - `CSI 3 ; I` dispatches count `3`;
     - overflowing count dispatches `u16::MAX`;
     - split-feed `CSI I` and `CSI 3 I` dispatch correctly;
     - pending invalid UTF-8 emits `U+FFFD` before same-slice and split-feed
       `CSI I`;
     - invalid private, intermediate-bearing, colon-param, mixed-separator, and
       real multi-param forms dispatch no action and do not leak printable final
       bytes;
     - direct C1 CSI byte `0x9b` followed by `I` remains out of scope and
       dispatches `U+FFFD` plus printable `I`, not horizontal tabulation;
     - handler errors from counted horizontal tab leave the parser in ground
       state;
     - existing cursor, positioning, line, and tab CSI behavior still behaves as
       before.
   - Terminal tests:
     - raw `HT` still moves to the next tabstop as before;
     - `CSI I` moves one tab stop;
     - `CSI 0 I` leaves the cursor unchanged;
     - `CSI ; I` leaves the cursor unchanged;
     - `CSI n I` moves across multiple tab stops;
     - `CSI n ; I` moves across `n` tab stops;
     - counted tabbing stops at the right edge if no more tab stops exist;
     - custom tab stops are honored;
     - starting on a tab stop still moves to the next tab stop;
     - pending-wrap behavior is preserved for right-edge tabbing;
     - counted tabbing does not modify cells, dirty rows, or scroll;
     - split-feed `CSI I` mutates terminal state correctly.
   - Existing stream, movement, positioning, tabstop, formatter, PageList, and
     ABI tests must keep passing.

6. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty stream
     cargo test -p roastty terminal::terminal
     cargo test -p roastty terminal_formatter
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
     - accepted `CSI I` forms;
     - rejected `CSI I` forms;
     - parser behavior for missing, explicit zero, and overflowing counts;
     - parser state behavior on handler errors;
     - terminal repeated-tab, early-stop, pending-wrap, dirty-row, no-scroll,
       and no-cell-write behavior;
     - confirmation that existing raw `HT`, cursor, positioning, line, and tab
       CSI behavior did not regress;
     - what remains deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- raw `HT` dispatches and behaves as one horizontal tab;
- `CSI I` dispatches counted horizontal tabulation;
- missing param dispatches count `1`;
- explicit zero dispatches count `0` and moves zero tab stops;
- one numeric param dispatches that count;
- semicolon-finalized one-param forms dispatch the same count Ghostty would
  produce;
- overflowing numeric params saturate to `u16::MAX`;
- invalid private, intermediate, colon-param, mixed-separator, and real
  multi-param forms dispatch no horizontal-tab action and do not leak printable
  bytes;
- direct C1 CSI byte `0x9b` remains out of scope and follows current raw-C1
  UTF-8 replacement behavior instead of dispatching horizontal tabulation;
- pending invalid UTF-8 emits `U+FFFD` before the horizontal-tab action;
- handler errors leave the parser in ground state;
- counted terminal behavior repeats the existing single-tab helper, stops early
  if the cursor no longer moves, honors custom tab stops, clamps to the right
  edge, and preserves existing pending-wrap behavior;
- counted terminal behavior does not scroll, dirty rows, or write cells;
- existing `CSI A/B/C/D/E/F/G/H/k/a/j/backtick/d/e/f` cursor/positioning
  behavior remains unchanged;
- existing `CSI W` tab behavior remains unchanged;
- no `CSI Z`, reverse tabulation, margin-aware tabulation, erase-display, public
  API, or ABI behavior is added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- stream parsing works, but the current single-tab helper needs a small
  prerequisite refactor before counted routing can be implemented without
  changing existing pending-wrap or dirty/cell behavior.

The experiment fails if:

- `CSI I` remains silently ignored;
- final `I` leaks as printable text;
- explicit zero is normalized to one;
- invalid variants dispatch horizontal-tab actions;
- counted tabbing skips tab stops, fails to stop at the right edge, writes
  cells, dirties rows, or scrolls;
- existing raw `HT`, cursor, positioning, line, or `CSI W` behavior regresses;
- reverse tabulation, margin-aware tabulation, erase-display, public API, or ABI
  behavior is accidentally added.

## Design Review

Codex reviewed this design before implementation.

Initial review artifacts:

- Prompt: `logs/codex-review/20260601-034920-497098-prompt.md`
- Result: `logs/codex-review/20260601-034920-497098-last-message.md`

Codex found that the first draft incorrectly rejected semicolon-finalized
one-param `CSI I` forms. Ghostty accepts those because `CSI I` dispatch only
checks the finalized parameter count. The design was updated so:

- `CSI ; I` dispatches count `0`;
- `CSI 3 ; I` dispatches count `3`;
- colon/mixed and real multi-param forms remain invalid;
- `I` is no longer listed as existing behavior.

Final review artifacts:

- Prompt: `logs/codex-review/20260601-035109-564585-prompt.md`
- Result: `logs/codex-review/20260601-035109-564585-last-message.md`

Codex approved the corrected design with no findings and said it is ready to
commit before implementation.

## Result

**Result:** Pass

Experiment 119 ports `CSI I` / CHT into Roastty's current full-screen terminal
model.

Accepted forms:

- raw `HT` still dispatches `Action::HorizontalTab { count: 1 }`;
- `CSI I` dispatches count `1`;
- `CSI n I` dispatches count `n`;
- `CSI 0 I` dispatches count `0`;
- semicolon-finalized one-param forms match Ghostty:
  - `CSI ; I` dispatches count `0`;
  - `CSI 3 ; I` dispatches count `3`;
- oversized counts saturate to `u16::MAX`.

Rejected forms:

- private variants such as `CSI ? 3 I`;
- non-standard private variants such as `CSI > 3 I`;
- colon or mixed-separator variants such as `CSI 1 : 2 I` and `CSI 1 ; 2 : 3 I`;
- real multi-param variants such as `CSI 5 ; 4 I`;
- intermediate-bearing forms;
- raw C1 `0x9b` followed by `I`, which remains out of scope and keeps the
  current replacement-character behavior.

The implementation changes the private `HorizontalTab` stream action to carry a
count. Raw `HT` dispatches count `1`, while `CSI I` uses an explicit-zero
preserving tabulation helper. The helper accepts semicolon-finalized one-param
forms for this new final without broadening prior one-param CSI behavior.

Terminal routing now repeats the existing single-step `horizontal_tab_basic()`
helper up to the requested count and stops early if a step does not move the
cursor. This preserves existing tab-stop behavior, right-edge clamping, and
pending-wrap behavior. Count `0` performs no movement. Counted tabulation does
not write cells, dirty rows, or scroll.

Parser error behavior was preserved: if a handler fails on counted
`HorizontalTab`, the stream is already back in ground state and the next
printable byte is parsed normally. Pending invalid UTF-8 still emits `U+FFFD`
before same-slice and split-feed `CSI I` actions.

Existing raw `HT`, cursor movement, cursor positioning, line movement, and
`CSI W` tab-stop behavior continued to pass. This experiment did not add
`CSI Z`, reverse tabulation, margin-aware tabulation, erase-display, public API,
or ABI behavior.

Verification passed:

```text
cargo fmt
cargo test -p roastty stream
cargo test -p roastty terminal::terminal
cargo test -p roastty terminal_formatter
cargo test -p roastty screen_formatter
cargo test -p roastty page_string
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

The full package test run passed with 1174 unit tests and the ABI harness.

Codex reviewed the completed implementation with no findings.

Result review artifacts:

- Prompt: `logs/codex-review/20260601-035542-503389-prompt.md`
- Result: `logs/codex-review/20260601-035542-503389-last-message.md`

## Conclusion

Roastty now has counted forward horizontal tabulation for the current basic
terminal model. The important parser distinction from this slice is that a new
CSI final can opt into semicolon-finalized one-param forms without changing the
previously ported one-param cursor actions.

Reverse tabulation (`CSI Z`) and margin-aware tab movement remain deferred.
