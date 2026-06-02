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

# Experiment 123: Port CSI Insert Lines

## Description

Continue the stream/action port by adding Ghostty's insert-line form:

- `CSI L` / `CSI 1 L` -> insert one blank line at the cursor row;
- `CSI n L` -> insert `n` blank lines at the cursor row;
- lines from the cursor row through the bottom of the active vertical scrolling
  region shift down;
- lines pushed past the bottom of the active vertical scrolling region are
  discarded;
- newly inserted rows are blank inside the active horizontal margins;
- the cursor moves to the left scrolling margin on the original cursor row.

Upstream Ghostty parses this in `vendor/ghostty/src/terminal/stream.zig`:

- final `L` emits `.insert_lines`;
- no params means count `1`;
- one param emits that exact count, including `0`;
- more than one param is invalid and dispatches no action;
- any private/intermediate form is invalid for this command.

Upstream Ghostty executes this in
`vendor/ghostty/src/terminal/Terminal.zig::insertLines`:

- count `0` is a no-op;
- if the cursor is outside the active top/bottom or left/right scrolling
  margins, the command is a no-op;
- the effective count is clamped to the remaining rows from the cursor through
  the bottom margin;
- shifted rows move downward within the vertical scrolling region;
- if the horizontal margins are full width, row wrap and wrap-continuation
  metadata for shifted source/destination rows are reset;
- if the horizontal margins are not full width, only the cells inside the left /
  right margins shift, and row wrap metadata is preserved;
- inserted cells are cleared to blanks;
- all shifted/cleared rows are dirtied;
- pending wrap is always cleared after an actual insert;
- the cursor is restored to `(left_margin, start_y)`.

This is the first row-shifting CSI mutation in Roastty. It is intentionally
narrower than a full scroll-region rewrite: it ports `CSI L` only, not `CSI M`,
`CSI S`, or `CSI T`. It may add a reusable PageList row-shift-down primitive if
that is the smallest correct foundation for later line deletion and scrolling
experiments, but it must not silently implement those later commands in this
experiment.

Ghostty's `insertLines` calls `rowWillBeShifted`, which adjusts wide-character
heads/tails and spacer cells at scroll-region boundaries. Roastty's current
basic stream print path is still single-cell ASCII-only, and Unicode
width/wide-cell mutation was explicitly deferred by earlier stream experiments.
This experiment should not implement wide-character rendering, width tables, or
wide-boundary parity. It should avoid making future wide support harder, but
wide-boundary adjustment remains deferred until the wide-cell mutation path
exists.

The key implementation risk is PageList row movement across page boundaries.
Roastty already has upward row-shift machinery for erase-row behavior, but
insert-lines needs the opposite direction and must preserve managed
style/grapheme/hyperlink ownership. If the safe row-shift-down primitive cannot
be implemented and tested in this experiment, stop and record Partial rather
than adding placeholder insert-line behavior.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/stream.zig` for `CSI L` parsing:
     - no params -> count `1`;
     - one param -> exact count;
     - multiple params -> invalid;
     - private/intermediate forms -> invalid.
   - Use `vendor/ghostty/src/terminal/Terminal.zig::insertLines` for terminal
     semantics.
   - Use upstream tests around `Terminal: insertLines` as the behavior
     checklist, especially simple insertion, zero count, cursor outside region,
     top/bottom margins, high counts, pending wrap, wrap reset, managed
     metadata, and left/right margins.
   - Do not modify `vendor/ghostty/`.

2. Extend the private stream action.
   - Add `Action::InsertLines { count }` in `roastty/src/terminal/stream.rs`.
   - Keep the action internal to the terminal module.
   - Do not add public API or ABI surface.

3. Extend CSI dispatch for final `L`.
   - Add an `insert_lines_action()` helper.
   - Accept:
     - `CSI L`;
     - `CSI 0 L`;
     - `CSI ; L`;
     - `CSI 1 L`;
     - `CSI 1 ; L`;
     - larger single numeric params, clamped to `u16::MAX` by the current
       parser's numeric accumulator behavior.
   - Reject and dispatch no action for:
     - private forms such as `CSI ? L` and `CSI > L`;
     - real multi-param forms such as `CSI 1 ; 2 L` and `CSI ;; L`;
     - colon/mixed separators;
     - any byte that the current parser treats as invalid;
     - direct C1 CSI byte `0x9b`, which remains out of scope and follows the
       current UTF-8 replacement behavior.
   - Preserve parser ground-state behavior on handler errors.
   - Preserve pending invalid UTF-8 behavior: if an incomplete invalid UTF-8
     sequence is interrupted by `CSI L`, dispatch `U+FFFD` before the
     insert-lines action.

4. Add a safe active-region insert-lines primitive.
   - Add the narrow Page/PageList helper needed to shift active rows downward
     from `cursor_y` through `bottom_margin`.
   - The helper should:
     - operate only inside the active screen domain;
     - clamp count to `bottom_margin - cursor_y + 1`;
     - traverse from bottom to top so source rows are copied before they are
       overwritten;
     - move full rows cheaply when left/right margins are full width;
     - copy only the `[left_margin, right_margin + 1)` cell range when
       left/right margins are active;
     - clear inserted rows inside the same horizontal range;
     - release managed memory for overwritten and cleared cells;
     - preserve moved grapheme/style/hyperlink ownership correctly;
     - preserve non-managed metadata such as protected bits on moved cells;
     - clear inserted/vacated cells to default cells with default style under
       the current basic model;
     - mark every shifted or cleared row dirty;
     - preserve scrollback row count and content.
   - Prefer reusing existing `Page::move_cells()`, `Page::clone_row_from()`,
     `Page::clear_cells()`, and existing PageList pin/page-boundary patterns. Do
     not weaken existing Page invariants to make this work.
   - Add PageList tests before relying on the helper from `Screen`.
   - Stop and record Partial if managed-memory movement across page boundaries
     cannot be made integrity-safe in this experiment.

5. Add screen/terminal insert-lines behavior.
   - Add
     `Screen::insert_lines_basic(count, rows, cols, top_margin, bottom_margin, left_margin, right_margin)`.
   - Pass the existing `Terminal::scrolling_region` bounds into the helper.
   - If `count == 0`, do nothing:
     - no dirty rows;
     - no pending-wrap change;
     - no cursor movement.
   - If the cursor is outside the active top/bottom or left/right scrolling
     margins, do nothing and preserve pending wrap.
   - Clamp count to the remaining rows from cursor through bottom margin.
   - Shift rows downward and blank inserted rows inside the horizontal margins.
   - After an actual insert:
     - clear pending wrap;
     - move the cursor to `(left_margin, original_cursor_y)`;
     - for full-width horizontal margins, clear wrap and wrap-continuation
       metadata on shifted/cleared rows in the affected vertical range;
     - for left/right horizontal margins, preserve row wrap metadata.
   - Do not mutate rows above `cursor_y` or below `bottom_margin`.
   - Do not mutate scrollback.
   - Current-SGR blank-cell coloring remains deferred because Roastty's current
     basic print path does not yet write cells with cursor style. Add tests
     proving default blank behavior now, and document SGR-preserving blanks as
     deferred until the SGR mutation path exists.

6. Route terminal actions.
   - In `TerminalStreamHandler`, route `Action::InsertLines` to the new helper.
   - Reuse the existing error conversion style.
   - Existing print, linefeed, cursor, positioning, tab, erase-display,
     erase-line, delete-character, formatter, PageList, and ABI behavior must
     keep passing unchanged.

7. Add tests.
   - Stream parser tests:
     - `A\x1b[LB` dispatches print `A`, insert-lines count `1`, print `B`;
     - `CSI L` dispatches count `1`;
     - `CSI 0 L` and `CSI ; L` dispatch count `0`;
     - `CSI 1 L` and `CSI 1 ; L` dispatch count `1`;
     - larger single params dispatch their parsed/clamped value;
     - real multi-param, colon-param, mixed-separator, and invalid-private forms
       dispatch no action and do not leak printable final bytes;
     - split-feed `CSI L` and `CSI 3 L` dispatch correctly;
     - pending invalid UTF-8 emits `U+FFFD` before same-slice and split-feed
       `CSI L`;
     - direct C1 CSI byte `0x9b` followed by `L` remains out of scope and
       dispatches `U+FFFD` plus printable `L`;
     - handler errors from insert-lines leave the parser in ground state;
     - existing cursor, positioning, line, tab, erase-display, erase-line,
       delete-character, and `CSI I` behavior remains unchanged.
   - PageList tests:
     - full-width single-page insert shifts rows down and clears inserted rows;
     - full-width insert across a page boundary preserves page-list integrity
       and managed metadata;
     - left/right margin insert copies only the bounded cell range and preserves
       cells outside the margins;
     - moved style/grapheme/hyperlink metadata remains owned exactly once;
     - protected bits and other non-managed metadata move with shifted cells;
     - overwritten/cleared cells release managed memory;
     - inserted/cleared blank cells are default cells with default style under
       the current basic model;
     - scrollback row count and content are unchanged;
     - all shifted/cleared active rows are dirty, and unrelated rows are not.
   - Terminal tests:
     - simple insert at a middle row produces Ghostty's `ABC\n\nDEF\nGHI` shape;
     - count `0` is a no-op and preserves pending wrap;
     - count larger than remaining region clamps;
     - cursor outside top/bottom margins is a no-op;
     - cursor outside left/right margins is a no-op;
     - top/bottom scrolling region constrains the shifted/cleared rows;
     - left/right scrolling region shifts and blanks only bounded columns;
     - cursor moves to the left margin on the original row after an actual
       insert;
     - actual insert clears pending wrap;
     - full-width insert resets wrap metadata for affected rows;
     - left/right-margin insert preserves wrap metadata;
     - rows above the cursor and below the bottom margin are not mutated;
     - scrollback is not mutated;
     - inserted/cleared blank cells are rendered as ordinary default blanks in
       current formatter output;
     - unsupported/invalid `CSI L` forms do not mutate terminal state;
     - split-feed `CSI L` mutates terminal state correctly.
   - Existing stream, cursor movement, positioning, tabstop, erase-display,
     erase-line, delete-character, formatter, PageList, and ABI tests must keep
     passing.

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
      - accepted `CSI L` forms;
      - rejected `CSI L` forms;
      - terminal behavior for count `0`, count `1`, larger counts, clamping,
        margins, pending wrap, cursor restoration, and row metadata;
      - PageList row-shift and page-boundary behavior;
      - managed-memory safety evidence;
      - dirty-row behavior;
      - deferred SGR blank-cell coloring and wide-boundary behavior, if still
        deferred;
      - confirmation that existing raw print, cursor, positioning, line, tab,
        erase-display, erase-line, delete-character, formatter, PageList, and
        ABI behavior did not regress;
      - verification command output summary;
      - Codex design-review outcome;
      - Codex result-review outcome.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Design Review

Codex reviewed the initial design and found two real issues:
`logs/codex-review/20260601-045628-466529-last-message.md`.

- The design mentioned wide-boundary behavior only in the result checklist. The
  design now explicitly defers Ghostty's `rowWillBeShifted` wide-head/tail and
  spacer-cell boundary adjustment until Roastty has Unicode-width and wide-cell
  mutation support, and it forbids claiming wide-boundary parity in this
  experiment.
- The design said inserted blanks should use current default blank behavior, but
  the test checklist did not require proving it. The design now requires
  PageList, terminal, and pass-criteria coverage that inserted/cleared blank
  cells are default cells with default style under the current basic model.

Codex re-reviewed the updated design and reported no findings:
`logs/codex-review/20260601-045804-240451-last-message.md`.

The second review confirmed the prior findings were fixed and the design is
ready to commit before implementation.

## Verification

The experiment passes if:

- `CSI L` dispatches and performs insert-lines count `1`;
- `CSI 0 L` and `CSI ; L` dispatch count `0` and no-op at terminal level;
- single-count `CSI n L` forms dispatch and perform the corresponding clamped
  active-region insert;
- invalid/private/colon/mixed/multi-param forms dispatch no insert-lines action
  and do not leak printable bytes;
- direct C1 CSI byte `0x9b` remains out of scope and follows current raw-C1
  UTF-8 replacement behavior;
- pending invalid UTF-8 emits `U+FFFD` before the insert-lines action;
- handler errors leave the parser in ground state;
- insert-lines shifts only the intended rows and columns inside the active
  scrolling margins;
- cursor position is restored to the left margin on the original row after an
  actual insert;
- actual inserts clear pending wrap;
- full-width inserts reset affected row wrap metadata;
- left/right-margin inserts preserve row wrap metadata;
- count `0` and outside-margin no-ops preserve pending wrap;
- rows above the cursor, rows below the bottom margin, and scrollback are not
  mutated;
- shifted/cleared rows become dirty and unaffected rows do not;
- managed grapheme/style/hyperlink movement and cleanup remains integrity-safe;
- non-managed cell metadata such as the protected bit moves with shifted cells;
- inserted/cleared blank cells are default cells with default style under the
  current basic model;
- existing raw print, linefeed, cursor, positioning, tabstop, erase-display,
  erase-line, delete-character, PageList, formatter, and ABI behavior remains
  unchanged;
- no delete-line, scroll up/down, SGR, Unicode-width, wide-character rendering,
  public API, ABI, or non-macOS behavior is added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- stream dispatch is correct but active-region row shifting needs a separate
  PageList primitive design;
- full-width insert works but left/right-margin behavior needs a separate
  bounded-copy primitive;
- plain-cell insert works but managed grapheme/style/hyperlink movement cannot
  be made integrity-safe in this slice;
- behavior is correct for cells but dirty tracking or row metadata requires an
  additional narrow primitive before the result can be called complete.

The experiment fails if:

- it changes unrelated cursor, tab, print, erase-display, erase-line,
  delete-character, formatter, or ABI behavior;
- it treats `CSI 0 L` as count `1`;
- it mutates cells outside the active scrolling margins;
- it mutates scrollback;
- it corrupts PageList integrity or leaks managed memory;
- it silently implements incompatible placeholder insert-line semantics;
- it adds unrelated delete-line, scroll up/down, public API, ABI, or non-macOS
  behavior.

## Result

**Result:** Pass

Roastty now parses and executes `CSI L` / Insert Lines for the current basic
cell model.

The stream parser accepts the Ghostty-compatible forms for this slice:

- `CSI L` dispatches `InsertLines { count: 1 }`;
- `CSI 0 L` and `CSI ; L` dispatch `InsertLines { count: 0 }`;
- `CSI 1 L` and `CSI 1 ; L` dispatch count `1`;
- larger single numeric params dispatch their parsed `u16` count.

The parser rejects private, real multi-param, colon-param, mixed-separator, and
invalid forms without leaking printable final bytes or mutating terminal state.
Direct raw-C1 `0x9b` remains out of scope: it does not dispatch `InsertLines`,
and it follows the current replacement-plus-printable-final behavior (`U+FFFD`
then `L`). Pending invalid UTF-8 still dispatches `U+FFFD` before the
insert-lines action, including split-feed cases, and handler errors leave the
parser in ground state.

The terminal implementation now:

- treats count `0` as a true no-op;
- no-ops when the cursor is outside the active vertical or horizontal scrolling
  margins;
- clamps larger counts to the remaining rows from the cursor through the bottom
  margin;
- shifts rows downward only inside the active margins;
- clears inserted cells to default blank/default style cells under the current
  basic model;
- restores the cursor to the left scrolling margin on the original cursor row
  after an actual insert;
- clears pending wrap after an actual insert and preserves it for no-ops;
- resets affected row wrap metadata for full-width inserts;
- preserves row wrap metadata for left/right-margin inserts;
- leaves rows above the cursor, rows below the bottom margin, and scrollback
  unchanged.

The PageList helper shifts active rows from bottom to top so source rows are
copied before overwrite. It handles same-page and cross-page copies, preserves
managed style/grapheme/hyperlink ownership, preserves non-managed metadata such
as protected bits, releases overwritten/cleared managed cells through the
existing page cell-clear path, dirties affected rows, and preserves scrollback
row count/content.

Current-SGR blank-cell coloring remains deferred because Roastty's current basic
print path does not yet write cells with cursor style. Ghostty's
`rowWillBeShifted` wide-head/tail and spacer-cell boundary behavior also remains
deferred until Roastty has Unicode-width and wide-cell mutation support. This
experiment deliberately did not add delete-line, scroll up/down, SGR,
Unicode-width, wide-character rendering, public API, ABI, or non-macOS behavior.

Verification passed:

```text
cargo fmt
cargo test -p roastty stream
cargo test -p roastty terminal::page_list
cargo test -p roastty terminal::terminal
cargo test -p roastty terminal_formatter
cargo test -p roastty screen_formatter
cargo test -p roastty page_string
cargo test -p roastty
```

The full `cargo test -p roastty` run passed with 1255 unit tests, the ABI
harness passed, and doctests had no tests to run. Existing raw print, linefeed,
cursor, positioning, tabstop, erase-display, erase-line, delete-character,
formatter, PageList, and ABI behavior remained green.

Codex design review found two real issues in the initial plan and approved the
updated design after they were fixed:

- `logs/codex-review/20260601-045628-466529-last-message.md`
- `logs/codex-review/20260601-045804-240451-last-message.md`

Codex result review found two real issues:
`logs/codex-review/20260601-050942-959179-last-message.md`.

- The result text overclaimed raw-C1 behavior. The wording now states the actual
  behavior: direct `0x9b` does not dispatch `InsertLines`, and it follows the
  current replacement-plus-printable-final path.
- Scrollback preservation needed content coverage, not just row-count coverage.
  The implementation now has explicit PageList and terminal tests proving
  scrollback content is unchanged.

Codex re-reviewed the fixed result and reported no blocking findings:
`logs/codex-review/20260601-051342-362780-last-message.md`.

## Conclusion

`CSI L` is now ported for Roastty's current basic cell engine. The
implementation adds the first downward active-region row-shift primitive without
weakening page integrity or expanding public surface area. The next experiment
can continue the same row-mutation family, most likely with delete-line or
scroll-region behavior that reuses the new PageList directionality work.
