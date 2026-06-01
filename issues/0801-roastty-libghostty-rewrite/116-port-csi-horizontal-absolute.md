# Experiment 116: Port CSI Horizontal Absolute

## Description

Continue the stream/action port by adding Ghostty's horizontal absolute cursor
positioning forms:

- `CSI G` -> cursor horizontal position absolute (`CHA` / `HPA`)
- ``CSI ` `` -> horizontal position absolute alias

Upstream Ghostty parses these in `vendor/ghostty/src/terminal/stream.zig`:

- final `G` and final `` ` `` emit `.cursor_col`;
- no params means column `1`;
- one param is passed through;
- more than one param is invalid and dispatches no action;
- intermediates dispatch no action.

Upstream routing in `vendor/ghostty/src/terminal/stream_terminal.zig` maps
`.cursor_col` to `Terminal.setCursorPos(current_y + 1, value)`. That means the
row is unchanged and the requested column is 1-indexed. `Terminal.setCursorPos`
then treats column `0` as column `1`, clamps oversized columns to the right
edge, and clears pending wrap.

Roastty does not yet have origin mode, left/right margins, or full
`setCursorPos` parity. This experiment ports the parser boundary and basic
full-screen terminal behavior only:

- row stays unchanged;
- column is 1-indexed;
- missing and zero columns resolve to the left edge;
- oversized columns clamp to the right edge;
- pending wrap is cleared;
- no cells are written, no rows are dirtied, and no scrolling occurs.

This experiment is intentionally narrow. It does not implement `CSI H` / `CSI f`
cursor positioning, `CSI d` / `CSI e` row movement, `CSI a` already handled in
Experiment 114, multi-parameter CSI parsing beyond rejecting it, origin mode,
left/right margins, scrolling-region-aware positioning, direct C1 CSI bytes,
public API, or ABI changes.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/stream.zig` for CSI dispatch:
     - final `G` emits `.cursor_col`;
     - final `` ` `` emits `.cursor_col`;
     - no params means column `1`;
     - one param is passed through;
     - more than one param is invalid and dispatches no action;
     - intermediates dispatch no action.
   - Use `vendor/ghostty/src/terminal/stream_terminal.zig` for routing:
     `.cursor_col` keeps the current row and calls `setCursorPos` with the
     requested column.
   - Use `vendor/ghostty/src/terminal/Terminal.zig` for basic column semantics:
     column `0` resolves to `1`, columns are 1-indexed, oversized columns clamp,
     and positioning clears pending wrap.
   - Do not modify `vendor/ghostty/`.

2. Extend the private stream action surface.
   - Add a private action:
     - `Action::CursorColumn { col: u16 }`
   - Keep it internal to the terminal module.
   - Do not add public API or ABI surface.

3. Extend CSI dispatch.
   - Reuse the existing single numeric param parser from Experiment 114.
   - Dispatch `CSI G` and ``CSI ` `` only when there is no private marker, no
     intermediate byte, and zero or one numeric param.
   - Missing param should dispatch `col: 1`.
   - Explicit `0` should dispatch `col: 0`; terminal behavior should convert
     that to the left edge. This preserves the upstream parser/terminal split.
   - Do not reuse the relative-movement `movement_count()` helper for this
     action. Relative movement treats zero as one in the parser, but absolute
     positioning must preserve explicit zero until the terminal positioning
     helper resolves it to the left edge.
   - Numeric overflow should continue saturating at `u16::MAX`, then terminal
     positioning clamps to the right edge.
   - Private, intermediate-bearing, semicolon-param, colon-param, and other
     invalid forms should dispatch no action and should not leak printable final
     bytes.
   - Keep the parser transition to ground before invoking the handler, so a
     handler error cannot leave it stuck in CSI state.
   - Preserve pending invalid UTF-8 behavior: if an incomplete invalid UTF-8
     sequence is interrupted by `CSI G` or ``CSI ` ``, dispatch `U+FFFD` before
     the cursor-column action.
   - Preserve all existing `CSI A/B/C/D/E/F/k/a/j` and `CSI W` behavior.

4. Add basic terminal positioning behavior.
   - Add a private full-screen helper on `Screen` or `Terminal` for horizontal
     absolute positioning.
   - The helper should:
     - keep the current row unchanged;
     - treat column `0` as column `1`;
     - convert the 1-indexed column to a zero-indexed cursor `x`;
     - clamp oversized columns to the right edge;
     - clear pending wrap;
     - avoid writing cells, dirtying rows, or scrolling.
   - Do not implement origin-mode or left/right-margin behavior in this
     experiment.

5. Add tests.
   - Stream parser tests:
     - `A\x1b[GB` dispatches print `A`, `CursorColumn { col: 1 }`, print `B`;
     - bytes `A`, `ESC`, `[`, `` ` ``, `B` dispatch print `A`,
       `CursorColumn { col: 1 }`, print `B`;
     - explicit counts such as `CSI 5 G` and ``CSI 6 ` `` dispatch those column
       values;
     - explicit zero dispatches `col: 0`;
     - overflowing columns dispatch `col: u16::MAX`;
     - split-feed `CSI G` and ``CSI ` `` dispatch correctly;
     - pending invalid UTF-8 emits `U+FFFD` before same-slice and split-feed
       cursor-column actions;
     - invalid private, intermediate-bearing, semicolon-param, colon-param, and
       multi-param forms dispatch no action and do not leak printable final
       bytes;
     - direct C1 CSI byte `0x9b` followed by `G` or `` ` `` remains out of scope
       and dispatches `U+FFFD` plus printable `G` / `` ` ``, not cursor
       positioning;
     - handler errors from `CursorColumn` leave the parser in ground state;
     - existing `CSI A/B/C/D/E/F/k/a/j` cursor behavior and `CSI W` tab actions
       still behave as before.
   - Terminal tests:
     - `CSI G` and ``CSI ` `` move to column zero with default param and keep
       the row unchanged;
     - explicit columns move to the requested 1-indexed column converted to
       zero-indexed cursor `x`;
     - explicit zero moves to the left edge;
     - oversized columns clamp to the right edge;
     - horizontal absolute positioning clears pending wrap;
     - positioning does not modify cells, dirty rows, or scroll;
     - split-feed `CSI G` / ``CSI ` `` mutates terminal state correctly.
   - Existing stream, movement, tabstop, formatter, PageList, and ABI tests must
     keep passing.

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
     - accepted `CSI G` / ``CSI ` `` forms;
     - rejected `CSI G` / ``CSI ` `` forms;
     - parser/terminal split for explicit column `0`;
     - count parsing and clamping behavior;
     - parser state behavior on handler errors;
     - terminal pending-wrap, dirty-row, no-scroll, and row-preservation
       behavior;
     - confirmation that existing cursor and tab CSI behavior did not regress;
     - what remains deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `CSI G` dispatches `CursorColumn`;
- ``CSI ` `` dispatches `CursorColumn`;
- missing params dispatch column `1`;
- explicit zero dispatches column `0` and terminal behavior moves to the left
  edge;
- one numeric param dispatches that column;
- overflowing numeric params saturate to `u16::MAX` and clamp at the terminal
  right edge;
- invalid private, intermediate, semicolon-param, colon-param, and multi-param
  forms dispatch no cursor-column action and do not leak printable bytes;
- direct C1 CSI byte `0x9b` remains out of scope and follows current raw-C1
  UTF-8 replacement behavior instead of dispatching cursor actions;
- pending invalid UTF-8 emits `U+FFFD` before cursor-column actions;
- handler errors leave the parser in ground state;
- existing `CSI A/B/C/D/E/F/k/a/j` cursor behavior remains unchanged;
- existing `CSI W` tab behavior remains unchanged;
- terminal behavior keeps the row unchanged, clamps to full-screen horizontal
  bounds, clears pending wrap, and does not scroll, dirty rows, or write cells;
- no `CSI H` / `CSI f` / two-parameter cursor positioning, row positioning,
  scrolling-region, origin-mode, direct C1 CSI, public API, or ABI behavior is
  added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- stream parsing works, but the current screen cursor helpers need a small
  prerequisite refactor before horizontal absolute positioning can be routed
  cleanly without mutating cells or dirty rows.

The experiment fails if:

- `CSI G` or ``CSI ` `` remains silently ignored;
- either final leaks as printable text;
- invalid variants dispatch cursor-column actions;
- handler errors leave the parser stuck in CSI state;
- existing cursor or tab CSI behavior regresses;
- terminal behavior changes the row, fails to clear pending wrap, fails to clamp
  to the screen, writes cells, dirties rows, or scrolls;
- `CSI H` / `CSI f`, row positioning, origin-mode, scrolling-region, direct C1
  CSI, public API, or ABI behavior is accidentally added.

## Design Review

Codex reviewed this design before implementation.

Initial review artifacts:

- Prompt: `logs/codex-review/20260601-031312-596670-prompt.md`
- Result: `logs/codex-review/20260601-031312-596670-last-message.md`

The helper reported that the stored Codex session was full and automatically
started a fresh session, as required by the review skill.

Codex found no blocking design issues and approved `CSI G` / ``CSI ` `` as the
right next slice. It confirmed that explicit zero should be preserved by the
parser and resolved to the left edge only by terminal positioning, matching
Ghostty's parser/terminal split.

Codex noted two non-blocking cleanups before the design commit: the backtick
test bullet had ambiguous Markdown, and the implementation constraint should
explicitly warn against reusing the relative `movement_count()` helper because
relative movement and absolute positioning have different zero semantics. The
design was updated for both findings.

## Result

**Result:** Pass

Implemented the horizontal absolute cursor-positioning slice:

- `CSI G` dispatches private `Action::CursorColumn { col }`.
- ``CSI ` `` dispatches private `Action::CursorColumn { col }`.

The parser uses a separate absolute-column helper instead of the relative
`movement_count()` helper. This preserves Ghostty's parser/terminal split:

- missing param dispatches `col: 1`;
- explicit `0` dispatches `col: 0`;
- numeric overflow saturates at `u16::MAX`;
- terminal positioning resolves `0` to the left edge and clamps oversized
  columns to the right edge.

Accepted forms:

- `CSI G` and ``CSI ` ``;
- `CSI 0 G` and ``CSI 0 ` ``;
- `CSI n G` and ``CSI n ` ``.

Rejected forms:

- private forms such as `CSI ? 3 G` and ``CSI ? 3 ` ``;
- unsupported private markers such as `CSI > 3 G` and ``CSI > 3 ` ``;
- semicolon params such as `CSI 5 ; 4 G` and ``CSI 5 ; 4 ` ``;
- colon params such as `CSI 1 : 2 G` and ``CSI 1 : 2 ` ``;
- intermediate-bearing forms such as `CSI SP G` and ``CSI SP ` ``.

Rejected forms dispatch no cursor-column action and do not leak printable final
bytes. Direct C1 CSI byte `0x9b` remains out of scope and follows the current
raw-C1 UTF-8 replacement behavior.

Terminal behavior:

- keeps the current row unchanged;
- converts 1-indexed columns to zero-indexed cursor `x`;
- treats explicit zero as the left edge;
- clamps oversized columns to the right edge;
- clears pending wrap;
- does not write cells, dirty rows, or scroll.

Existing `CSI A/B/C/D/E/F/k/a/j` cursor behavior and `CSI W` tab behavior did
not regress. `CSI H` / `CSI f`, row positioning, two-param parsing, origin-mode,
scrolling-region-aware positioning, direct C1 CSI, public API, and ABI behavior
remain deferred.

Verification:

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

All commands passed. The full `cargo test -p roastty` run reported 1128 unit
tests passed, the ABI harness passed, and doc-tests had zero tests.

Codex result review artifacts:

- Prompt: `logs/codex-review/20260601-031707-728435-prompt.md`
- Result: `logs/codex-review/20260601-031707-728435-last-message.md`

Codex found no implementation blockers and approved recording a Pass. It noted
one non-blocking coverage gap: split-feed pending invalid UTF-8 covers `CSI G`
but not the backtick alias. Same-slice pending invalid UTF-8 and ordinary
split-feed tests cover the alias, so no implementation change was required.

## Conclusion

Experiment 116 completed the next upstream CSI cursor-positioning branch without
pulling in full two-parameter cursor positioning. Roastty now parses `CSI G` and
``CSI ` `` with the correct absolute-column zero semantics and basic full-screen
terminal behavior. The next positioning slice can add row/column positioning
with two CSI params (`CSI H` / `CSI f`) or first add the remaining single-axis
forms (`CSI d` and `CSI e`) depending on which parser refactor is most useful.
