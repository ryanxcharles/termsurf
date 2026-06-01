# Experiment 134: Port OSC 8 Printed Cell Hyperlinks

## Description

Experiment 133 added the first OSC runtime path and intentionally stopped at
active cursor hyperlink state. That was the correct boundary because Roastty's
printed-cell path did not yet have a rollback-safe way to attach the active OSC
8 hyperlink to newly written cells.

This experiment should complete the next OSC 8 slice: when a printable cell is
written while an OSC 8 hyperlink is active, the resulting page cell should store
that hyperlink metadata. When no hyperlink is active, printing should clear any
old hyperlink metadata at the destination cell, matching normal terminal
overwrite behavior.

This experiment is still not a general OSC expansion. It must not implement new
OSC command numbers, palette mutation, clipboard, notifications, semantic
prompts, public ABI, renderer behavior, PTY behavior, or non-macOS behavior.

## Changes

1. Re-read the current source of truth.
   - Use `vendor/ghostty/src/terminal/osc.zig` and
     `vendor/ghostty/src/terminal/stream.zig` only to confirm OSC 8 active-link
     semantics.
   - Use Roastty's existing hyperlink storage in:
     - `roastty/src/terminal/hyperlink.rs`;
     - `roastty/src/terminal/page.rs`;
     - `roastty/src/terminal/page_list.rs`;
     - `roastty/src/terminal/screen.rs`;
     - `roastty/src/terminal/terminal.rs`.
   - Do not modify `vendor/ghostty/`.

2. Add a rollback-safe printed-cell hyperlink write path.
   - Extend the page-list printed-cell write path so it can write:
     - codepoint;
     - style;
     - optional hyperlink.
   - Keep the existing basic/default path fast for cells with no style and no
     active hyperlink.
   - Permit overwriting old cells that contain style and/or hyperlink metadata,
     because this experiment owns replacing those managed fields.
   - Continue rejecting unsupported managed-cell state such as graphemes,
     non-codepoint content tags, wide cells, protected cells, and non-output
     semantic content unless the existing print path already supports it.
   - Add a hyperlink-aware print precheck for the pending-wrap path. The current
     `Screen::print_basic_cell` wrapped-to-cell preflight goes through
     `check_active_cell_for_styled_print`, which rejects existing hyperlink
     cells. Replace or extend that precheck so wrapping onto a cell that has
     only replaceable style and/or hyperlink metadata is allowed, while still
     rejecting unsupported managed-cell state.
   - The helper must be rollback-safe:
     - if style allocation fails, leave the old cell unchanged;
     - if hyperlink insertion fails, release any newly allocated style and leave
       the old cell unchanged;
     - if hyperlink map insertion fails, release any newly inserted hyperlink
       and any newly allocated style, and leave the old cell unchanged;
     - once all fallible allocations/maps succeed, mutate the cell and release
       replaced style/hyperlink references exactly once.
   - Do not naively call `Page::set_hyperlink` / `set_hyperlink_at_offset` as
     the combined-write commit primitive. That helper performs fallible map
     work, mutates map state, and releases the old hyperlink internally. For
     this experiment, add a page-level prepare/commit helper, or otherwise prove
     in code and tests that every fallible operation has completed before any
     old cell state or old refs are mutated.
   - Pin new hyperlink ownership explicitly:
     - `Page::insert_hyperlink` returns one owned ref for the printed cell;
     - do not call `use_hyperlink` for that same newly inserted cell;
     - if the write aborts before commit, release the newly inserted hyperlink
       exactly once;
     - if the write commits, the cell owns that ref and replaced hyperlink refs
       are released exactly once.
   - Maintain row hyperlink flags explicitly after add, replace, and clear. In
     particular, clearing or replacing the last hyperlink in a row must
     recompute the row's `hyperlink` metadata to `false` when no linked cells
     remain.

3. Convert active cursor hyperlink state into page hyperlink data.
   - Add a private conversion from `ScreenCursorHyperlink` to
     `hyperlink::Hyperlink<'_>`.
   - Explicit cursor IDs become
     `hyperlink::HyperlinkId::Explicit(id.as_bytes())`.
   - Implicit cursor IDs become `hyperlink::HyperlinkId::Implicit(id)`.
   - URI strings are stored as raw UTF-8 bytes via `uri.as_bytes()`.
   - Do not percent-decode, validate, normalize, or escape hyperlink IDs or
     URIs.

4. Wire printing through the new helper.
   - Update `Screen::print_basic_cell` so the cell written at the final cursor
     position receives `self.cursor.hyperlink.as_ref()` when present.
   - Existing cursor movement, insert mode, wraparound, scrolling, pending-wrap,
     style, and dirty-row behavior must remain unchanged.
   - Pending-wrap prechecks must use the same replaceability rule as the final
     write path, so wrapping onto an old hyperlink cell works both with and
     without an active cursor hyperlink.
   - If the cursor hyperlink is cleared before printing, the destination cell
     must not keep an old hyperlink from previous content.

5. Add inspection helpers only where needed for tests.
   - Prefer test-only helpers that return a simple snapshot:
     - whether a cell has hyperlink metadata;
     - the stored hyperlink ID shape;
     - the stored URI bytes/string;
     - the stored hyperlink ref count if needed to prove dedup/release.
   - Do not expose hyperlink internals through public API or ABI.

6. Add tests.
   - Add page/page-list tests proving the new printed-cell helper:
     - writes an implicit hyperlink to a plain cell;
     - writes an explicit hyperlink to a plain cell;
     - writes style and hyperlink metadata together;
     - deduplicates repeated identical links through the page hyperlink set;
     - clears/replaces an old hyperlink when printing without an active link;
     - replaces an old hyperlink with a different active link and releases old
       refs;
     - clears the last linked cell in a row and clears the row hyperlink flag;
     - rejects unsupported managed-cell state without mutation;
     - rolls back on style allocation failure;
     - rolls back on hyperlink string or set insertion failure;
     - rolls back on hyperlink map insertion failure where feasible at the page
       helper level. If the terminal-level path cannot force a specific failure
       because no injection hook exists, document that limitation in the result.
     - proves exact hyperlink ref ownership:
       - one printed linked cell has ref count 1;
       - repeated identical links across multiple cells have the expected ref
         count;
       - replacement decrements the old link and increments/owns the new link;
       - clearing decrements the old link and removes the map entry.
   - Add terminal stream tests proving:
     - `OSC 8 start` + text + `OSC 8 end` stores hyperlinks only on printed
       cells inside the active range;
     - text printed after `OSC 8 end` has no hyperlink;
     - explicit IDs are stored exactly;
     - implicit links share the same implicit ID across all cells printed while
       that OSC 8 link is active;
     - starting a second implicit link gets a different implicit ID;
     - SGR styled printing and OSC 8 hyperlinks compose on the same cells;
     - pending-wrap overwrites a destination hyperlink cell correctly, both with
       and without an active cursor hyperlink;
     - insert mode shifts existing hyperlink metadata correctly or fails with a
       documented existing unsupported managed-cell error;
     - wrap and scroll preserve stored hyperlink metadata through the existing
       page movement paths.
   - Keep Experiment 133's active cursor hyperlink formatter tests passing.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal_stream_osc
     cargo test -p roastty terminal_stream_sgr
     cargo test -p roastty terminal_formatter
     cargo test -p roastty terminal::page
     cargo test -p roastty terminal::page_list
     cargo test -p roastty terminal::screen
     cargo test -p roastty terminal::terminal
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix all real design findings before implementation.
   - Record the design-review outcome in this experiment file before
     implementation.
   - Commit the approved experiment design before implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real result findings before proceeding.
   - Commit the recorded experiment result separately from the design commit.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - the final page-list write helper shape;
     - the exact rollback guarantees implemented;
     - explicit and implicit OSC 8 storage behavior;
     - how old hyperlink/style refs are released;
     - any unsupported managed-cell cases left intentionally unchanged;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- cells printed while an OSC 8 link is active store hyperlink metadata;
- cells printed after OSC 8 end do not store hyperlink metadata;
- printing over an old hyperlink without an active link clears and releases the
  old hyperlink metadata;
- printing over an old hyperlink with a new active link replaces and releases
  refs correctly;
- style and hyperlink metadata can coexist on the same printed cell;
- pending-wrap prechecks allow overwriting replaceable hyperlink cells and still
  reject unsupported managed-cell state;
- implicit hyperlink IDs remain stable for one active OSC 8 range and differ
  across separate implicit OSC 8 ranges;
- explicit hyperlink IDs and URI bytes are stored exactly as received;
- row hyperlink metadata is correct after add, replace, and clearing the final
  link in a row;
- exact hyperlink ref counts are proven for one-cell, multi-cell dedup,
  replacement, and clear cases;
- fallback and failure paths do not leak style or hyperlink refs;
- unsupported managed-cell states remain rejected without mutation;
- existing print, SGR, OSC parser, active cursor hyperlink formatter, page,
  page-list, screen, terminal, formatter, and ABI behavior remains intact;
- no unrelated OSC protocols, public API, public ABI, renderer, app callback,
  PTY, or non-macOS behavior is added;
- `cargo fmt`, targeted tests, and full `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- active OSC 8 links can be written to plain cells, but style+hyperlink
  composition exposes a missing prerequisite that needs a separate print-helper
  refactor;
- hyperlink writes work for direct printing, but insert/wrap/scroll propagation
  reveals a pre-existing managed-cell movement gap that needs a follow-up
  subsystem experiment.

The experiment fails if:

- OSC 8 active cursor state updates but printed cells still do not store
  hyperlink metadata;
- hyperlink writes are not rollback-safe;
- old hyperlink refs leak or are released twice;
- style metadata regresses when hyperlinks are active;
- hyperlink bytes are normalized, escaped, or otherwise changed before storage;
- existing OSC parsing/termination behavior regresses;
- unrelated terminal protocols or public API/ABI are added.

## Design Review

Codex reviewed the initial design and agreed this is the right next experiment
after Experiment 133, but found five real design gaps:
`logs/codex-review/20260601-081324-802508-last-message.md`.

The design was updated to:

- require a hyperlink-aware pending-wrap precheck so wrapped printing can
  overwrite replaceable hyperlink cells;
- require a page-level prepare/commit strategy instead of naively using
  `Page::set_hyperlink` as the combined write primitive;
- pin ownership of the newly inserted hyperlink ref and forbid extra
  `use_hyperlink` calls for the same printed cell;
- require row hyperlink flag maintenance after add, replace, and clear;
- strengthen rollback and exact ref-count verification.

Codex re-reviewed the updated design and found no remaining blocking issues:
`logs/codex-review/20260601-081538-077871-last-message.md`.

The design is approved for implementation.
