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

# Experiment 120: Port CSI Erase Display

## Description

Continue the stream/action port by adding Ghostty's erase-display forms:

- `CSI J` / `CSI 0 J` -> erase from cursor through the end of the active screen;
- `CSI 1 J` -> erase from the start of the active screen through the cursor;
- `CSI 2 J` -> erase the complete active screen;
- `CSI 3 J` -> erase scrollback/history;
- `CSI 22 J` -> Kitty scroll-complete clear.

Upstream Ghostty parses this in `vendor/ghostty/src/terminal/stream.zig`:

- final `J` emits one of five erase-display actions;
- no params means `.below`;
- one param is converted through `csi.EraseDisplay`;
- valid numeric modes are `0`, `1`, `2`, `3`, and `22`;
- semicolon-finalized one-param forms are still one-param forms, so `CSI ; J`
  behaves like `CSI 0 J` and `CSI 1 ; J` behaves like `CSI 1 J`;
- more than one param is invalid and dispatches no action;
- no protected marker means ordinary erase;
- one `?` marker means DEC protected erase;
- any other private/intermediate form is invalid and dispatches no action.

Roastty already has the ingredients needed for a first faithful terminal-level
port:

- the stream parser can carry up to two params, which is enough for `CSI J`;
- `Page::clear_cells()` already releases graphemes, styles, hyperlinks, and cell
  contents;
- `PageList` already has row/history erase helpers and `scroll_clear()`;
- `Screen` already tracks cursor position, pending wrap, current style, and
  protected cursor state.

This experiment connects those pieces for erase-display. It should preserve
Ghostty's user-visible behavior where Roastty already has equivalent state. It
must not add unrelated `CSI K`, line insertion/deletion, scroll-region,
semantic-prompt, Kitty graphics, public API, or ABI work.

Two upstream details are intentionally handled conservatively:

- Ghostty's complete erase may heuristically turn into `scroll_complete` when
  the primary screen appears to be at a prompt. Roastty does not yet have the
  semantic prompt machinery in the mutation path, so this experiment implements
  the direct complete-clear behavior and documents the prompt heuristic as
  deferred.
- Ghostty's protected erase respects ISO protected mode or an explicit DEC
  protected request. Roastty currently has per-cell protection bits and cursor
  protected state but no full protected-mode state machine in the stream path.
  This experiment should support the explicit `?J` request and any already
  stored protected cell bits, while avoiding a fake global protected-mode model.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/csi.zig::EraseDisplay` for mode values:
     - `0` below;
     - `1` above;
     - `2` complete;
     - `3` scrollback;
     - `22` scroll complete.
   - Use `vendor/ghostty/src/terminal/stream.zig` for `CSI J` parsing:
     - no params -> below;
     - one param -> enum lookup;
     - semicolon-finalized one-param forms -> enum lookup of the finalized
       value, so `CSI ; J` is below and `CSI 1 ; J` is above;
     - multiple params -> invalid;
     - no protected marker -> unprotected request;
     - `?` marker -> protected request;
     - any other private/intermediate form -> invalid.
   - Use `vendor/ghostty/src/terminal/Terminal.zig::eraseDisplay` for terminal
     semantics.
   - Do not modify `vendor/ghostty/`.

2. Extend the private stream action.
   - Add a private `EraseDisplay` mode enum in `roastty/src/terminal/stream.rs`
     with variants for `Below`, `Above`, `Complete`, `Scrollback`, and
     `ScrollComplete`.
   - Add `Action::EraseDisplay { mode, protected }`.
   - Keep the action internal to the terminal module.
   - Do not add public API or ABI surface.

3. Extend CSI dispatch for final `J`.
   - Add an `erase_display_action()` helper.
   - Accept:
     - `CSI J`;
     - `CSI 0 J`;
     - `CSI ; J`;
     - `CSI 1 J`;
     - `CSI 1 ; J`;
     - `CSI 2 J`;
     - `CSI 3 J`;
     - `CSI 22 J`;
     - the same forms with `?` after `CSI`, such as `CSI ? 2 J`.
   - Reject and dispatch no action for:
     - private markers other than `?`;
     - any byte that the current parser treats as invalid;
     - colon/mixed separators;
     - real multi-param forms such as `CSI 1 ; 2 J` and `CSI ;; J`;
     - unsupported numeric modes such as `CSI 4 J`;
     - direct C1 CSI byte `0x9b`, which remains out of scope and follows the
       current UTF-8 replacement behavior.
   - In Roastty's current parser, the leading `?` is represented as
     `CsiState.private == Some(b'?')`, not as general CSI intermediate support.
     Use that existing private marker for `CSI ? J`. Do not add general CSI
     intermediate parsing in this experiment.
   - Preserve parser ground-state behavior on handler errors.
   - Preserve pending invalid UTF-8 behavior: if an incomplete invalid UTF-8
     sequence is interrupted by `CSI J`, dispatch `U+FFFD` before the
     erase-display action.

4. Add screen/page erase helpers.
   - Add `Screen::erase_display_basic(mode, protected, rows, cols)`.
   - Add or expose only the minimal `PageList` helpers needed to clear:
     - a cell range on one active row;
     - complete active rows;
     - active rows above/below the cursor;
     - scrollback/history;
     - scroll-complete clear.
   - Clearing a range should:
     - clear cells to empty/default cells;
     - release associated grapheme/style/hyperlink managed memory;
     - mark affected rows dirty;
     - clear soft-wrap metadata for rows that become fully cleared if the
       existing primitives already do that;
     - preserve cursor position for below, above, complete, and scrollback;
     - clear pending wrap for below, above, complete, and scroll complete;
     - leave scrollback-only pending-wrap behavior matched to upstream or
       explicitly documented if the current basic model differs.
   - Implement ranges using the active cursor:
     - below: cursor cell through right edge on cursor row, then every row
       below;
     - above: left edge through cursor cell on cursor row, then every row above;
     - complete: every active row;
     - scrollback: history only, not active screen cells;
     - scroll complete: move visible non-empty active rows into scrollback via
       existing `scroll_clear()`, then leave the active screen empty.
   - Treat scroll complete separately from ordinary erase cursor preservation.
     Upstream `Screen.scrollClear()` reloads the cursor after moving rows; if
     the cursor pin is no longer in the active area it lands at the active
     top-left. Roastty does not yet track cursor pins, so this experiment should
     define and test the basic-equivalent result explicitly: after
     scroll-complete clear, active cells are empty, pending wrap is false, and
     the cursor is at `(0, 0)` unless implementation evidence shows a more
     faithful cursor-preserving path exists in the current model.
   - Keep this in the basic full-screen coordinate model already used by the
     current cursor experiments. Do not introduce origin mode, scroll regions,
     left/right margins, alternate-screen differences, or prompt heuristics.

5. Handle protected erase honestly.
   - If `protected == false`, clear all targeted cells.
   - If `protected == true`, skip cells whose per-cell protection bit is set.
   - Do not invent global ISO/DEC protected-mode behavior in this experiment.
   - Add tests proving protected cells survive explicit `CSI ? J` forms while
     unprotected cells in the same target range are cleared.
   - Seed protected-cell tests with a narrow test-only helper or fixture that
     marks specific active cells protected before erase. Do not rely on basic
     print to create protected cells, because the current basic print path does
     not yet propagate cursor protection into written cells.
   - If current Page/PageList APIs cannot clear only unprotected cells without
     risking managed-memory leaks, stop and record a Partial result rather than
     clearing protected cells incorrectly.

6. Route terminal actions.
   - In `TerminalStreamHandler`, route `Action::EraseDisplay` to the new
     `Screen::erase_display_basic()` helper.
   - Convert any PageList allocation/cell errors to the existing terminal stream
     error style or add a narrow internal error variant if needed.
   - Existing print, linefeed, cursor, positioning, tab, and formatter behavior
     must keep passing unchanged.

7. Add tests.
   - Stream parser tests:
     - `A\x1b[JB` dispatches print `A`, erase below unprotected, print `B`;
     - `CSI J`, `CSI 0 J`, and `CSI ; J` dispatch below;
     - `CSI 1 J`, `CSI 1 ; J`, `CSI 2 J`, `CSI 3 J`, and `CSI 22 J` dispatch
       their modes;
     - `CSI ? J`, `CSI ? 0 J`, `CSI ? 1 J`, `CSI ? 2 J`, `CSI ? 3 J`, and
       `CSI ? 22 J` dispatch the same modes with `protected = true`;
     - protected semicolon-finalized forms such as `CSI ? ; J` and `CSI ? 1 ; J`
       dispatch their modes with `protected = true`;
     - unsupported mode, real multi-param, colon-param, mixed-separator, and
       invalid-private forms dispatch no action and do not leak printable final
       bytes;
     - split-feed `CSI J` and `CSI 22 J` dispatch correctly;
     - pending invalid UTF-8 emits `U+FFFD` before same-slice and split-feed
       `CSI J`;
     - direct C1 CSI byte `0x9b` followed by `J` remains out of scope and
       dispatches `U+FFFD` plus printable `J`;
     - handler errors from erase-display leave the parser in ground state;
     - existing cursor, positioning, line, tab, and `CSI I` behavior remains
       unchanged.
   - Terminal tests:
     - erase below clears the cursor cell, the rest of that row, and all rows
       below while preserving cells above/left of the cursor;
     - erase above clears rows above and the cursor row through the cursor cell
       while preserving cells below/right of the cursor;
     - complete clears all active cells and preserves cursor position;
     - scrollback erases history without changing active cells;
     - scroll complete clears active visible cells through the existing
       `scroll_clear()` behavior, clears pending wrap, and has the explicitly
       tested cursor result from step 4;
     - erase below/above/complete clear pending wrap;
     - scrollback-only pending-wrap behavior is either matched to upstream or
       explicitly documented if the current model differs;
     - dirty state is set on affected active rows and not on unaffected rows;
     - style, grapheme, and hyperlink managed-memory cleanup is covered by
       either PageList tests or targeted terminal tests;
     - explicit protected erase preserves protected cells and clears unprotected
       cells;
     - unsupported/invalid `CSI J` forms do not mutate terminal state;
     - split-feed `CSI J` mutates terminal state correctly.
   - Existing stream, movement, positioning, tabstop, formatter, PageList, and
     ABI tests must keep passing.

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
      - accepted `CSI J` forms;
      - rejected `CSI J` forms;
      - terminal behavior for below, above, complete, scrollback, and scroll
        complete;
      - protected erase behavior and any deferred protected-mode limitations;
      - prompt-heuristic and Kitty-graphics behavior that remains deferred;
      - pending-wrap and dirty-row behavior;
      - confirmation that existing raw print, cursor, positioning, line, tab,
        `CSI I`, formatter, PageList, and ABI behavior did not regress;
      - verification command output summary;
      - Codex design-review outcome;
      - Codex result-review outcome.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Design Review

Codex reviewed the revised design and reported no findings:
`logs/codex-review/20260601-040515-057643-last-message.md`.

The first review found four real issues, all fixed before this approval:

- semicolon-finalized one-param erase-display forms such as `CSI ; J` and
  `CSI 1 ; J` must be accepted;
- scroll-complete cursor and pending-wrap behavior needed to be specified
  separately from ordinary erase modes;
- Roastty's parser should map protected `CSI ? J` through the existing private
  marker rather than adding general CSI intermediate parsing;
- protected-cell tests need a concrete test-only seeding path because basic
  print does not yet create protected cells.

## Verification

The experiment passes if:

- `CSI J` dispatches and performs erase-display below;
- `CSI 0/1/2/3/22 J` dispatch and perform the corresponding Ghostty
  erase-display modes;
- semicolon-finalized one-param forms such as `CSI ; J` and `CSI 1 ; J` dispatch
  like their finalized single param;
- `CSI ? J` and `CSI ? 0/1/2/3/22 J` dispatch protected erase requests;
- invalid/private/colon/mixed/multi-param/unsupported numeric forms dispatch no
  erase-display action and do not leak printable bytes;
- direct C1 CSI byte `0x9b` remains out of scope and follows current raw-C1
  UTF-8 replacement behavior;
- pending invalid UTF-8 emits `U+FFFD` before the erase-display action;
- handler errors leave the parser in ground state;
- below, above, and complete erase only the intended active-screen cells;
- scrollback erase removes history without active-screen mutation;
- scroll-complete uses the existing scroll-clear path and leaves active visible
  cells empty;
- cursor position is preserved for below, above, complete, and scrollback;
- scroll-complete cursor behavior is explicitly tested and documented;
- pending-wrap behavior matches the documented scope;
- affected rows become dirty and unaffected rows do not;
- explicit protected erase preserves protected cells while clearing unprotected
  cells in the same range;
- managed grapheme/style/hyperlink cleanup remains integrity-safe;
- existing raw print, linefeed, cursor, positioning, tabstop, `CSI I`, PageList,
  formatter, and ABI behavior remains unchanged;
- no `CSI K`, insert/delete line, scroll-region, margin, semantic-prompt,
  Kitty-graphics, public API, or ABI behavior is added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- stream dispatch is correct but terminal mutation exposes a missing PageList
  primitive that should be designed separately;
- unprotected erase works but protected erase cannot be implemented without a
  safer Page/PageList primitive;
- below/above/complete work but scrollback or scroll-complete needs a separate
  experiment;
- behavior is correct for active cells but managed memory or dirty tracking
  needs additional support before the result can be called complete.

The experiment fails if:

- it changes unrelated cursor, tab, print, formatter, or ABI behavior;
- it clears protected cells during an explicit protected erase;
- it corrupts PageList integrity or leaks managed memory;
- it silently implements incompatible placeholder erase semantics;
- it adds unrelated `CSI K`, line insertion/deletion, public API, ABI, or
  non-macOS behavior.

## Result

**Result:** Pass

Implemented Roastty's private `CSI J` erase-display path for the current basic
full-screen terminal model.

Accepted stream forms:

- `CSI J`, `CSI 0 J`, and `CSI ; J` dispatch erase below;
- `CSI 1 J` and `CSI 1 ; J` dispatch erase above;
- `CSI 2 J` dispatches complete erase;
- `CSI 3 J` dispatches scrollback erase;
- `CSI 22 J` dispatches scroll-complete clear;
- the same forms with `?` after `CSI` dispatch protected erase requests.

Rejected forms include unsupported modes, real multi-param forms, colon/mixed
separators, invalid private markers, and raw C1 CSI. Invalid forms do not leak
their final byte as printable text. Pending invalid UTF-8 emits `U+FFFD` before
the erase-display action, including split-feed cases. Handler errors restore the
parser to ground state before the next byte.

Terminal behavior now covers:

- erase below: cursor cell through the end of the cursor row, plus all rows
  below;
- erase above: all rows above, plus start of cursor row through cursor cell;
- complete: all active rows;
- scrollback: history only, preserving active cells;
- scroll complete: existing `scroll_clear()` path, empty active screen, cursor
  reset to `(0, 0)`, pending wrap cleared.

Unprotected full-row erases now reset row metadata while preserving the row's
cell offset, matching Ghostty's `clearRows` behavior. This clears stale
soft-wrap metadata on complete erase and on fully-cleared rows below/above the
cursor. Protected erase skips cells with the per-cell protection bit and
preserves row metadata, matching the conservative protected-clear scope from the
design.

Implemented code changes:

- `roastty/src/terminal/stream.rs`
  - added private `EraseDisplayMode`;
  - added `Action::EraseDisplay { mode, protected }`;
  - added `CSI J` dispatch with Ghostty mode values and semicolon-finalized
    one-param behavior.
- `roastty/src/terminal/page.rs`
  - added `clear_unprotected_cells()`;
  - added `reset_cleared_row_metadata()`;
  - added managed-memory/protected-cell tests.
- `roastty/src/terminal/page_list.rs`
  - added active-cell clear helper;
  - added narrow wrappers for history erase and scroll clear;
  - added test-only protected-cell and scrollback helpers.
- `roastty/src/terminal/screen.rs`
  - added `erase_display_basic()`;
  - added test helpers for protected cells, row wrap metadata, and scrollback
    count.
- `roastty/src/terminal/terminal.rs`
  - routed erase-display actions;
  - added terminal tests for every implemented erase-display mode and regression
    surface.

Verification passed after `cargo fmt`:

```bash
cargo test -p roastty stream
cargo test -p roastty terminal::terminal
cargo test -p roastty terminal::page_list
cargo test -p roastty terminal::page::tests::page_clear_unprotected_cells_skips_protected_and_releases_managed_memory
cargo test -p roastty terminal_formatter
cargo test -p roastty screen_formatter
cargo test -p roastty page_string
cargo test -p roastty
```

The final full package run passed `1192` unit tests plus the ABI harness.

Codex result review:

- First result review:
  `logs/codex-review/20260601-041610-790107-last-message.md`
  - Found one real blocker: full-row unprotected erase cleared cells but left
    soft-wrap row metadata behind.
  - Fixed by adding `Page::reset_cleared_row_metadata()` and using it for
    unprotected full-row active clears.
- Second result review:
  `logs/codex-review/20260601-041846-662471-last-message.md`
  - Reported no findings.
  - Confirmed the result is good enough to record as Pass.

Deferred:

- `CSI K` erase-line;
- line insertion/deletion and scroll commands after `CSI J`;
- scroll-region/origin/margin-aware erase behavior;
- semantic prompt heuristic for complete erase;
- Kitty graphics deletion side effects;
- global ISO/DEC protected-mode stream semantics beyond explicit protected `?J`
  requests and already-stored per-cell protection bits.

## Conclusion

Experiment 120 successfully ports the next Ghostty stream action, `CSI J`, into
Roastty's current basic terminal model. The implementation is intentionally
private/internal, covered by parser and terminal tests, and keeps the broader
libghostty rewrite moving without adding public API or ABI surface.

The next experiment should continue in upstream stream order with `CSI K` erase
line, reusing the same Page/PageList clearing primitives where appropriate.
