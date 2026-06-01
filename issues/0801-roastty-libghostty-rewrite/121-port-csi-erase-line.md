# Experiment 121: Port CSI Erase Line

## Description

Continue the stream/action port by adding Ghostty's erase-line forms:

- `CSI K` / `CSI 0 K` -> erase from the cursor through the right edge of the
  cursor row;
- `CSI 1 K` -> erase from the left edge of the cursor row through the cursor;
- `CSI 2 K` -> erase the complete cursor row;
- the same forms with `?` after `CSI` -> protected erase-line requests.

Upstream Ghostty parses this in `vendor/ghostty/src/terminal/stream.zig`:

- final `K` emits one erase-line action;
- no params means `.right`;
- one param is accepted only if the numeric value is less than `3`;
- therefore accepted numeric modes are `0`, `1`, and `2`;
- semicolon-finalized one-param forms are still one-param forms, so `CSI ; K`
  behaves like `CSI 0 K` and `CSI 1 ; K` behaves like `CSI 1 K`;
- more than one param is invalid and dispatches no action;
- no protected marker means ordinary erase;
- one `?` marker means DEC protected erase;
- any other private/intermediate form is invalid and dispatches no action.

`vendor/ghostty/src/terminal/csi.zig::EraseLine` contains
`.right_unless_pending_wrap = 4`, and `stream.zig` has a handler arm for it, but
the current upstream stream parser rejects mode `4` before the switch because it
requires `input.params[0] < 3`. This experiment should match the parser
behavior: do not accept `CSI 4 K` unless fresh source inspection proves the
upstream parser changed.

Roastty can reuse most of the clearing machinery added in Experiment 120:

- `Page::clear_cells()` releases graphemes, styles, hyperlinks, and cell
  contents;
- `Page::clear_unprotected_cells()` skips protected cells while releasing
  managed memory for cleared cells;
- `PageList::clear_active_cells()` already handles active-row range clearing,
  dirty marking, protected skipping, and full-row metadata reset.

`CSI K` cannot blindly reuse `PageList::clear_active_cells()` for every case:
that helper resets row metadata for unprotected full-row clears, which is
correct for Experiment 120's active-screen erase-display behavior, but upstream
Ghostty explicitly keeps row soft-wrap metadata for `CSI 2 K`. This experiment
needs either a second clear helper or an option on the existing helper so
erase-line complete can clear every cell while preserving row metadata.

This experiment connects those pieces for erase-line. It must not add unrelated
insert/delete line, scroll-region, margin, origin-mode, SGR, Unicode-width,
wide-character rendering, public API, or ABI work.

Two upstream details are intentionally scoped:

- Ghostty adjusts erase-line ranges around wide-character heads/tails. Roastty's
  current basic stream print path is still single-cell ASCII-only, and Unicode
  width/wide-cell mutation was explicitly deferred by earlier stream
  experiments. This experiment should not implement wide-character rendering or
  width tables. It should avoid making wide support worse, and it should record
  wide erase adjustment as deferred until the wide-cell mutation path exists.
- Ghostty's protected erase also respects ISO protected mode. Roastty currently
  supports explicit protected erase requests against already-stored per-cell
  protection bits, but does not yet have the full protected-mode stream state
  machine. This experiment should support explicit `?K` requests and existing
  per-cell protection bits only.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/csi.zig::EraseLine` for mode names:
     - `0` right;
     - `1` left;
     - `2` complete;
     - `4` exists in the enum but is not accepted by the current parser.
   - Use `vendor/ghostty/src/terminal/stream.zig` for `CSI K` parsing:
     - no params -> right;
     - one param less than `3` -> enum lookup;
     - semicolon-finalized one-param forms -> enum lookup of the finalized
       value, so `CSI ; K` is right and `CSI 1 ; K` is left;
     - multiple params -> invalid;
     - no protected marker -> unprotected request;
     - `?` marker -> protected request;
     - any other private/intermediate form -> invalid.
   - Use `vendor/ghostty/src/terminal/Terminal.zig::eraseLine` for terminal
     semantics.
   - Do not modify `vendor/ghostty/`.

2. Extend the private stream action.
   - Add a private `EraseLineMode` enum in `roastty/src/terminal/stream.rs` with
     variants for `Right`, `Left`, and `Complete`.
   - Add `Action::EraseLine { mode, protected }`.
   - Keep the action internal to the terminal module.
   - Do not add public API or ABI surface.

3. Extend CSI dispatch for final `K`.
   - Add an `erase_line_action()` helper beside `erase_display_action()`.
   - Accept:
     - `CSI K`;
     - `CSI 0 K`;
     - `CSI ; K`;
     - `CSI 1 K`;
     - `CSI 1 ; K`;
     - `CSI 2 K`;
     - the same forms with `?` after `CSI`, such as `CSI ? 2 K`.
   - Reject and dispatch no action for:
     - `CSI 3 K`, `CSI 4 K`, and other unsupported numeric modes;
     - real multi-param forms such as `CSI 1 ; 2 K` and `CSI ;; K`;
     - colon/mixed separators;
     - private markers other than `?`;
     - any byte that the current parser treats as invalid;
     - direct C1 CSI byte `0x9b`, which remains out of scope and follows the
       current UTF-8 replacement behavior.
   - Use the existing `CsiState.private == Some(b'?')` representation for
     protected `CSI ? K`. Do not add general CSI intermediate parsing.
   - Preserve parser ground-state behavior on handler errors.
   - Preserve pending invalid UTF-8 behavior: if an incomplete invalid UTF-8
     sequence is interrupted by `CSI K`, dispatch `U+FFFD` before the erase-line
     action.

4. Add screen erase-line behavior.
   - Add `Screen::erase_line_basic(mode, cols, protected)`.
   - Implement ranges using the active cursor:
     - right: cursor cell through `cols`;
     - left: column `0` through the cursor cell, inclusive;
     - complete: column `0` through `cols`.
   - Preserve cursor position for all erase-line modes.
   - Clear `pending_wrap` for every valid erase-line mode, matching Ghostty's
     `eraseLine()` behavior.
   - Reset cursor-row soft-wrap metadata for right erase only. Ghostty's
     `eraseLine(.right, ...)` calls `cursorResetWrap()`, which clears
     `pending_wrap`, clears the cursor row's `wrap` flag if set, and clears the
     next active row's `wrap_continuation` flag when there is a next row.
   - Do not reset cursor-row soft-wrap metadata for left or complete erase.
     Ghostty's `eraseLine(.complete, ...)` explicitly notes that xterm does not
     reset the line's soft-wrap state, and its clear primitive does not modify
     row wrap metadata.
   - Add a narrow PageList/Screen helper if needed to clear a full active row
     while preserving row metadata for `CSI 2 K`. Do not reuse Experiment 120's
     full-row metadata reset behavior for erase-line complete.
   - Mark the affected row dirty.
   - Do not mutate rows above or below the cursor.
   - Do not mutate scrollback.

5. Handle protected erase honestly.
   - If `protected == false`, clear all targeted cells.
   - If `protected == true`, skip cells whose per-cell protection bit is set.
   - Do not invent global ISO/DEC protected-mode behavior in this experiment.
   - Add tests proving protected cells survive explicit `CSI ? K` forms while
     unprotected cells in the same target range are cleared.
   - Seed protected-cell tests with the existing test-only protected-cell helper
     from Experiment 120. Do not rely on basic print to create protected cells.

6. Route terminal actions.
   - In `TerminalStreamHandler`, route `Action::EraseLine` to the new
     `Screen::erase_line_basic()` helper.
   - Reuse the existing error conversion style from erase-display.
   - Existing print, linefeed, cursor, positioning, tab, erase-display,
     formatter, PageList, and ABI behavior must keep passing unchanged.

7. Add tests.
   - Stream parser tests:
     - `A\x1b[KB` dispatches print `A`, erase right unprotected, print `B`;
     - `CSI K`, `CSI 0 K`, and `CSI ; K` dispatch right;
     - `CSI 1 K`, `CSI 1 ; K`, and `CSI 2 K` dispatch left/complete;
     - `CSI ? K`, `CSI ? 0 K`, `CSI ? 1 K`, and `CSI ? 2 K` dispatch the same
       modes with `protected = true`;
     - protected semicolon-finalized forms such as `CSI ? ; K` and `CSI ? 1 ; K`
       dispatch their modes with `protected = true`;
     - unsupported modes `3`, `4`, and large values dispatch no action;
     - real multi-param, colon-param, mixed-separator, and invalid-private forms
       dispatch no action and do not leak printable final bytes;
     - split-feed `CSI K` and `CSI 2 K` dispatch correctly;
     - pending invalid UTF-8 emits `U+FFFD` before same-slice and split-feed
       `CSI K`;
     - direct C1 CSI byte `0x9b` followed by `K` remains out of scope and
       dispatches `U+FFFD` plus printable `K`;
     - handler errors from erase-line leave the parser in ground state;
     - existing cursor, positioning, line, tab, erase-display, and `CSI I`
       behavior remains unchanged.
   - Terminal tests:
     - erase right clears the cursor cell through the row end while preserving
       cells to the left;
     - erase left clears row start through the cursor cell while preserving
       cells to the right;
     - complete clears the entire cursor row while preserving cursor position;
     - no mode mutates rows above, rows below, or scrollback;
     - every valid mode clears pending wrap;
     - right erase clears cursor-row `wrap` and the next active row's
       `wrap_continuation` when applicable;
     - left erase preserves row soft-wrap metadata;
     - complete erase preserves row soft-wrap metadata, even for unprotected
       full-row clears;
     - affected row becomes dirty;
     - unaffected rows do not become dirty;
     - explicit protected erase preserves protected cells and clears unprotected
       cells in the same row range;
     - unsupported/invalid `CSI K` forms do not mutate terminal state;
     - split-feed `CSI K` mutates terminal state correctly.
   - Existing stream, cursor movement, positioning, tabstop, erase-display,
     formatter, PageList, and ABI tests must keep passing.

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
      - accepted `CSI K` forms;
      - rejected `CSI K` forms, especially unsupported `3`/`4` modes;
      - terminal behavior for right, left, and complete erase;
      - protected erase behavior and any deferred protected-mode limitations;
      - pending-wrap and row soft-wrap metadata behavior;
      - dirty-row behavior;
      - confirmation that existing raw print, cursor, positioning, line, tab,
        erase-display, formatter, PageList, and ABI behavior did not regress;
      - verification command output summary;
      - Codex design-review outcome;
      - Codex result-review outcome.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Design Review

Codex reviewed the design and reported no findings:
`logs/codex-review/20260601-042458-449270-last-message.md`.

The review confirmed:

- the design matches upstream Ghostty's current `CSI K` parser, including
  rejecting `CSI 4 K` despite the enum variant;
- terminal semantics match `Terminal.zig::eraseLine()`, including the important
  distinction that right erase resets soft-wrap metadata while complete erase
  preserves it;
- the planned PageList extension is narrow and avoids reusing Experiment 120's
  full-row metadata reset where it would be wrong;
- verification covers parser, terminal mutation, protected cells, dirty rows,
  pending wrap, row metadata, and existing `CSI J` regression surfaces.

## Verification

The experiment passes if:

- `CSI K` dispatches and performs erase-line right;
- `CSI 0/1/2 K` dispatch and perform the corresponding Ghostty erase-line modes;
- semicolon-finalized one-param forms such as `CSI ; K` and `CSI 1 ; K` dispatch
  like their finalized single param;
- `CSI ? K` and `CSI ? 0/1/2 K` dispatch protected erase requests;
- invalid/private/colon/mixed/multi-param/unsupported numeric forms dispatch no
  erase-line action and do not leak printable bytes;
- `CSI 4 K` is explicitly rejected unless the current upstream parser is found
  to accept it;
- direct C1 CSI byte `0x9b` remains out of scope and follows current raw-C1
  UTF-8 replacement behavior;
- pending invalid UTF-8 emits `U+FFFD` before the erase-line action;
- handler errors leave the parser in ground state;
- right, left, and complete erase only the intended cursor-row cells;
- cursor position is preserved;
- all valid erase-line modes clear pending wrap;
- right erase resets cursor-row soft-wrap metadata and next-row continuation
  metadata when applicable;
- left and complete erase preserve row soft-wrap metadata;
- affected row becomes dirty and unaffected rows do not;
- explicit protected erase preserves protected cells while clearing unprotected
  cells in the same target range;
- managed grapheme/style/hyperlink cleanup remains integrity-safe through the
  reused Page/PageList clear primitives;
- existing raw print, linefeed, cursor, positioning, tabstop, erase-display,
  PageList, formatter, and ABI behavior remains unchanged;
- no insert/delete line, scroll-region, margin, origin-mode, SGR, Unicode-width,
  wide-character rendering, public API, ABI, or non-macOS behavior is added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- stream dispatch is correct but terminal mutation exposes a missing Screen or
  PageList primitive that should be designed separately;
- unprotected erase works but protected erase cannot be implemented without a
  safer Page/PageList primitive;
- right/left/complete cell ranges work but the metadata-preserving full-row
  clear needed by `CSI 2 K` requires a separate helper before the result can be
  called complete;
- behavior is correct for active cells but managed memory or dirty tracking
  needs additional support before the result can be called complete.

The experiment fails if:

- it changes unrelated cursor, tab, print, erase-display, formatter, or ABI
  behavior;
- it accepts `CSI 4 K` without proving current upstream accepts it;
- it clears protected cells during an explicit protected erase;
- it corrupts PageList integrity or leaks managed memory;
- it silently implements incompatible placeholder erase semantics;
- it adds unrelated insert/delete line, scroll-region, public API, ABI, or
  non-macOS behavior.
