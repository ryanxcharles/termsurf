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

# Experiment 118: Port CSI Cursor Position

## Description

Continue the stream/action port by adding Ghostty's two-axis cursor-positioning
forms:

- `CSI H` -> cursor position (`CUP`)
- `CSI f` -> horizontal and vertical position (`HVP`)

Upstream Ghostty parses these in `vendor/ghostty/src/terminal/stream.zig`:

- final `H` and final `f` emit `.cursor_pos`;
- no params means row `1`, column `1`;
- one param means that row and column `1`;
- two params mean row and column;
- more than two params are invalid and dispatch no action;
- intermediates dispatch no action.

Upstream routing in `vendor/ghostty/src/terminal/stream_terminal.zig` maps
`.cursor_pos` to `Terminal.setCursorPos(value.row, value.col)`. `setCursorPos`
in `vendor/ghostty/src/terminal/Terminal.zig` resolves row or column `0` to `1`,
converts the 1-indexed values to zero-indexed cursor coordinates, clamps to the
screen bounds, and clears pending wrap.

Roastty does not yet have origin mode, scrolling-region-aware positioning, or
full `setCursorPos` parity. This experiment ports the parser boundary and basic
full-screen behavior only:

- `CSI H` and `CSI f` set row and column together;
- missing params resolve to row `1`, column `1`;
- leading and interior empty params are represented as explicit zero and
  resolved by terminal positioning;
- trailing separators do not append a final empty param, matching Ghostty's
  parser finalization rule;
- explicit zero is preserved by the parser;
- oversized values clamp to the screen bottom/right edges;
- pending wrap is cleared;
- no cells are written, no rows are dirtied, and no scrolling occurs.

This experiment is intentionally narrow. It does not implement origin mode,
scrolling-region-aware positioning, direct C1 CSI bytes, public API, ABI
changes, or broad multi-parameter support for unrelated CSI sequences.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/stream.zig` for `CSI H` / `CSI f`
     dispatch:
     - no params -> `{ row = 1, col = 1 }`;
     - one param -> `{ row = param0, col = 1 }`;
     - two params -> `{ row = param0, col = param1 }`;
     - more than two params -> invalid, no action;
     - intermediates -> no action.
   - Use `vendor/ghostty/src/terminal/stream_terminal.zig` for routing.
   - Use `vendor/ghostty/src/terminal/Terminal.zig::setCursorPos` for basic
     full-screen semantics.
   - Do not modify `vendor/ghostty/`.

2. Extend the private stream action surface.
   - Add a private action:
     - `Action::CursorPosition { row: u16, col: u16 }`
   - Keep it internal to the terminal module.
   - Do not add public API or ABI surface.

3. Refactor CSI parameter storage narrowly.
   - Replace the current single-parameter `CsiState` storage with enough
     structure to represent zero, one, or two numeric params.
   - Match Ghostty's semicolon parameter-finalization rule from
     `vendor/ghostty/src/terminal/Parser.zig`: separators store the current
     accumulator and reset it, but final dispatch only appends the current
     accumulator if digits were seen after the last separator.
   - Leading and interior empty params are represented as explicit zero.
     Trailing omitted params are not appended.
   - Required examples:
     - `CSI H` -> zero params;
     - `CSI 5 H` -> one param `[5]`;
     - `CSI 5 ; 6 H` -> two params `[5, 6]`;
     - `CSI ; H` -> one param `[0]`;
     - `CSI 5 ; H` -> one param `[5]`;
     - `CSI ; 7 H` -> two params `[0, 7]`.
     - `CSI ; ; H` -> two params `[0, 0]`.
     - `CSI 5 ; ; H` -> two params `[5, 0]`.
     - `CSI 5 ; 6 ; H` -> two params `[5, 6]`.
   - Numeric overflow must keep saturating at `u16::MAX` per parameter.
   - A third stored parameter, such as `CSI 5 ; 6 ; 7 H`, must mark the CSI
     invalid for this experiment.
   - Colon or mixed separators remain invalid for `CSI H` / `CSI f`. Ghostty's
     parser rejects colon or mixed separators for every CSI final except `m`
     before dispatch reaches `stream.zig`.
   - Private markers in unsupported locations and intermediates remain invalid
     for these positioning forms.
   - Preserve existing behavior for single-parameter actions:
     - `CSI A/B/C/D/E/F/G/k/a/j/backtick/d/e` still accept zero or one numeric
       param as before;
     - separator-bearing forms remain invalid and dispatch no action for those
       one-param actions, even when Ghostty's lower-level parser would finalize
       them as one stored param;
     - `CSI W` behavior remains unchanged, including `CSI ? 5 W`.
   - This deliberately avoids broadening prior experiments' accepted input
     surface. Upstream parity for trailing separators on one-param CSI actions
     can be handled later as its own explicit experiment.
   - Keep parser transition to ground before invoking the handler, so handler
     errors cannot leave the stream stuck in CSI state.
   - Preserve pending invalid UTF-8 behavior: if an incomplete invalid UTF-8
     sequence is interrupted by `CSI H` or `CSI f`, dispatch `U+FFFD` before the
     cursor-position action.

4. Dispatch `CSI H` and `CSI f`.
   - Add a cursor-position dispatch helper that only accepts no private marker,
     no invalid state, and zero through two params.
   - Missing values from the no-param and one-param cases should be filled at
     dispatch time as Ghostty does:
     - no params -> row `1`, col `1`;
     - one param -> row `param0`, col `1`.
   - Empty leading and interior params created by separators are explicit zero
     and must not be rewritten to `1` by the parser.
   - Trailing separators do not create a final empty param.
   - More than two params dispatch no action and should not leak printable final
     bytes.

5. Add basic terminal positioning behavior.
   - Add a private full-screen helper on `Screen` or `Terminal` for two-axis
     cursor positioning.
   - It should:
     - treat row `0` and column `0` as row `1` / column `1` through
       saturating-sub semantics;
     - convert 1-indexed row/column to zero-indexed cursor coordinates;
     - clamp oversized row/column to the bottom/right screen edges;
     - clear pending wrap;
     - avoid writing cells, dirtying rows, or scrolling.
   - Existing `cursor_column_basic()` and `cursor_row_basic()` may delegate to
     the new helper if that keeps the source simpler, but their externally
     tested behavior must not change.
   - Do not implement origin-mode or scrolling-region-aware behavior in this
     experiment.

6. Add tests.
   - Stream parser tests:
     - `A\x1b[HB` dispatches print `A`, `CursorPosition { row: 1, col: 1 }`,
       print `B`;
     - `A\x1b[fB` dispatches the same action;
     - `CSI 5 H` dispatches row `5`, col `1`;
     - `CSI 5 ; 6 H` and `CSI 5 ; 6 f` dispatch row `5`, col `6`;
     - `CSI 0 ; 0 H`, `CSI ; H`, `CSI 5 ; H`, `CSI ; 7 H`, `CSI ; ; H`, and
       `CSI 5 ; ; H` match Ghostty's parameter finalization semantics;
     - overflowing row and column params dispatch `u16::MAX`;
     - split-feed `CSI H` and `CSI f` dispatch correctly;
     - pending invalid UTF-8 emits `U+FFFD` before same-slice and split-feed
       cursor-position actions;
     - invalid private, intermediate-bearing, colon-param, mixed-separator, and
       real three-param forms dispatch no action and do not leak printable final
       bytes;
     - direct C1 CSI byte `0x9b` followed by `H` or `f` remains out of scope and
       dispatches `U+FFFD` plus printable `H` / `f`, not cursor positioning;
     - handler errors from `CursorPosition` leave the parser in ground state;
     - existing cursor, vertical/horizontal positioning, line, and tab CSI
       behavior still behaves as before.
   - Terminal tests:
     - `CSI H` and `CSI f` move to the top-left cell with default params;
     - one-param forms set row and default column `1`;
     - two-param forms set the requested 1-indexed row/column converted to
       zero-indexed cursor coordinates;
     - explicit zero row/column values and empty leading/interior params move to
       the top/left edges;
     - trailing omitted params use Ghostty's one-param or two-param defaults;
     - oversized row/column values clamp to bottom/right edges;
     - positioning clears pending wrap;
     - positioning does not modify cells, dirty rows, or scroll;
     - split-feed `CSI H` / `CSI f` mutates terminal state correctly.
   - Existing stream, movement, tabstop, formatter, PageList, and ABI tests must
     keep passing.

7. Verify.
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

8. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix all real design findings before implementation.
   - Record the design-review outcome in this experiment file before
     implementation.
   - Commit the approved design before implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real result findings before proceeding.
   - Commit the approved result separately from the design commit.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - accepted `CSI H` / `CSI f` forms;
     - rejected `CSI H` / `CSI f` forms;
     - parser representation of leading, interior, and trailing empty params;
     - parser representation of explicit zero;
     - count parsing and clamping behavior;
     - parser state behavior on handler errors;
     - terminal pending-wrap, dirty-row, no-scroll, and two-axis clamping
       behavior;
     - confirmation that existing cursor, positioning, line, and tab CSI
       behavior did not regress;
     - what remains deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `CSI H` and `CSI f` dispatch `CursorPosition`;
- no params dispatch row `1`, col `1`;
- one param dispatches that row and default col `1`;
- two params dispatch row and col;
- leading and interior empty separator params are represented as explicit zero;
- trailing separators do not append a final empty param;
- explicit zero dispatches value `0`;
- terminal positioning resolves zero row/column to top/left edges;
- overflowing numeric params saturate to `u16::MAX` and clamp at the terminal
  bottom/right edges;
- invalid private, intermediate, colon-param, mixed-separator, and real
  three-param forms dispatch no cursor-position action and do not leak printable
  bytes;
- separator-bearing forms remain invalid for CSI actions that only accept zero
  or one param;
- direct C1 CSI byte `0x9b` remains out of scope and follows current raw-C1
  UTF-8 replacement behavior instead of dispatching cursor actions;
- pending invalid UTF-8 emits `U+FFFD` before cursor-position actions;
- handler errors leave the parser in ground state;
- existing `CSI A/B/C/D/E/F/G/k/a/j/backtick/d/e` cursor behavior remains
  unchanged;
- existing `CSI W` tab behavior remains unchanged;
- terminal behavior clamps to full-screen bounds, clears pending wrap, and does
  not scroll, dirty rows, or write cells;
- no origin-mode, scrolling-region, direct C1 CSI, public API, or ABI behavior
  is added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- `CSI H` / `CSI f` stream parsing works, but the CSI parameter refactor reveals
  that a smaller prerequisite parameter parser experiment is needed before the
  terminal behavior can be safely routed.

The experiment fails if:

- `CSI H` or `CSI f` remains silently ignored;
- either final leaks as printable text;
- invalid variants dispatch cursor-position actions;
- the CSI parameter refactor regresses existing one-param or tab CSI behavior;
- handler errors leave the parser stuck in CSI state;
- terminal behavior fails to clear pending wrap, fails to clamp to the screen,
  writes cells, dirties rows, or scrolls;
- origin-mode, scrolling-region, direct C1 CSI, public API, or ABI behavior is
  accidentally added.

## Design Review

Codex reviewed this design before implementation.

Initial review artifacts:

- Prompt: `logs/codex-review/20260601-033311-851946-prompt.md`
- Result: `logs/codex-review/20260601-033311-851946-last-message.md`

Codex found that the first draft incorrectly treated trailing semicolon params
as explicit empty params. Ghostty's parser only appends the final accumulator
when digits were seen after the last separator. The design was updated so
leading and interior empty params become explicit zero, while trailing omitted
params are not appended.

Second review artifacts:

- Prompt: `logs/codex-review/20260601-033616-047222-prompt.md`
- Result: `logs/codex-review/20260601-033616-047222-last-message.md`

Codex then found that colon and mixed separators must remain invalid for `CSI H`
/ `CSI f`, because Ghostty rejects colon or mixed separators for every CSI final
except `m` before dispatch. The design was updated to reject those forms, and to
explicitly preserve the prior one-param CSI behavior for separator-bearing forms
outside `H` / `f`.

Final review artifacts:

- Prompt: `logs/codex-review/20260601-033741-119221-prompt.md`
- Result: `logs/codex-review/20260601-033741-119221-last-message.md`

Codex approved the corrected design with no findings and said it is ready to
commit before implementation.

## Result

**Result:** Pass

Experiment 118 ports the basic full-screen forms of `CSI H` / CUP and `CSI f` /
HVP into Roastty.

Accepted forms:

- `CSI H` and `CSI f` dispatch `Action::CursorPosition { row: 1, col: 1 }`.
- `CSI n H` dispatches row `n`, column `1`.
- `CSI row ; col H` and `CSI row ; col f` dispatch the explicit row and column.
- Explicit zero params are preserved by the parser and resolved by terminal
  positioning to the top/left edges.
- Leading and interior empty semicolon params are represented as explicit zero.
- Trailing semicolon params are omitted, matching Ghostty's parser finalization:
  - `CSI ; H` dispatches row `0`, col `1`;
  - `CSI 5 ; H` dispatches row `5`, col `1`;
  - `CSI ; 7 H` dispatches row `0`, col `7`;
  - `CSI ; ; H` dispatches row `0`, col `0`;
  - `CSI 5 ; ; H` dispatches row `5`, col `0`;
  - `CSI 5 ; 6 ; H` dispatches row `5`, col `6`.
- Oversized row and column params saturate to `u16::MAX` in the parser and clamp
  to the bottom/right edges in the terminal.

Rejected forms:

- private variants such as `CSI ? 3 H` / `CSI ? 3 f`;
- non-standard private variants such as `CSI > 3 H` / `CSI > 3 f`;
- colon or mixed-separator variants such as `CSI 1 : 2 H` and `CSI 1 ; 2 : 3 H`;
- real three-param variants such as `CSI 5 ; 6 ; 7 H`;
- intermediate-bearing forms;
- raw C1 `0x9b` followed by `H` or `f`, which remains out of scope and keeps the
  current replacement-character behavior.

The implementation replaces the old single-parameter `CsiState` storage with a
narrow two-parameter parser that records semicolon-finalized params,
leading/interior empty params, trailing omitted params, and separator presence.
One-param CSI actions still use `single_param(false)`, so separator-bearing
forms outside `H` / `f` remain invalid for this experiment. `CSI W` keeps its
existing behavior, including `CSI ? 5 W`.

Terminal behavior is routed through a private `cursor_position_basic()` helper.
It clears pending wrap, resolves zero via saturating-sub semantics, clamps to
the full-screen row/column bounds, and does not write cells, dirty rows, or
scroll.

Parser error behavior was preserved: if a handler fails on `CursorPosition`, the
stream is already back in ground state and the next printable byte is parsed
normally. Pending invalid UTF-8 still emits `U+FFFD` before same-slice and
split-feed `CSI H` / `CSI f` actions.

Existing behavior for `CSI A/B/C/D/E/F/G/k/a/j/backtick/d/e` and `CSI W`
continued to pass. This experiment did not add origin mode,
scrolling-region-aware positioning, direct C1 CSI, public API, or ABI behavior.

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

The full package test run passed with 1158 unit tests and the ABI harness.

Codex reviewed the completed implementation with no findings.

Result review artifacts:

- Prompt: `logs/codex-review/20260601-034516-060667-prompt.md`
- Result: `logs/codex-review/20260601-034516-060667-last-message.md`

## Conclusion

Roastty now has Ghostty's basic two-axis cursor-positioning forms for the
current full-screen terminal model. The useful foundation from this experiment
is the CSI parameter parser shape: it can now represent the subset needed for
cursor positioning while still preserving the intentionally narrow input surface
for earlier one-param CSI slices.

Remaining cursor-positioning parity still depends on future origin-mode and
scrolling-region-aware `setCursorPos` behavior.
