# Experiment 18: Port Page CloneFrom Graphemes

## Description

Extend Experiment 17's row-copy foundation to support Ghostty's grapheme
branches in `Page.cloneFrom`.

Experiment 17 deliberately rejected all managed-memory rows and cells. That was
the right temporary guard for the plain-row fast path, but Roastty already has
working Page grapheme storage. The next useful managed-memory slice is therefore
to replace the grapheme part of that guard with real migration:

- source grapheme cells are copied into destination grapheme storage;
- destination graphemes in the copied range are freed before overwrite;
- cloning plain rows over destination grapheme rows clears the copied grapheme
  cells and updates row flags.

This experiment should still reject styles and hyperlinks. Style row-copy can
follow later because it needs `style::Set::add_with_id` wiring from Page.
Hyperlinks must wait until Page hyperlink storage exists.

## Changes

1. Inspect upstream source.
   - Use `vendor/ghostty/src/terminal/page.zig` as the source of truth.
   - Re-read:
     - `Page.cloneFrom`
     - `Page.cloneRowFrom`
     - grapheme portions of `Page.clonePartialRowFrom`
     - upstream tests:
       - `Page cloneFrom graphemes`
       - `Page cloneFrom frees dst graphemes`
   - Read the style and hyperlink branches only to keep guards correct. Do not
     implement them here.
   - Do not modify `vendor/ghostty/`.

2. Rename the temporary clone method if needed.
   - Experiment 17's `clone_plain_rows_from` name is accurate for plain rows but
     becomes misleading once graphemes are supported.
   - Either:
     - rename it to a still-scoped internal name such as
       `clone_rows_from_without_styles_or_hyperlinks`, or
     - keep `clone_plain_rows_from` as a wrapper around a new internal
       implementation used by grapheme tests.
   - Do not expose a complete `clone_from` name yet unless the method still
     returns explicit unsupported errors for styles and hyperlinks and the name
     is documented as incomplete. Prefer the scoped name to avoid implying full
     Ghostty parity too early.

3. Add grapheme-aware row copy.
   - Continue preserving Experiment 17 behavior:
     - upstream `y_start`/`y_end` preconditions;
     - destination rows start at row `0`;
     - destination row `cells` offsets remain destination-owned;
     - copy only the overlapping cell range;
     - preserve trailing destination cells when the source is narrower;
     - clear grow-column trailing spacer heads.
   - Replace the source/destination grapheme rejection with real behavior:
     - source rows with `Row::grapheme` are allowed;
     - destination rows with `Row::grapheme` are allowed;
     - source cells with `Cell::has_grapheme()` are allowed;
     - destination cells with `Cell::has_grapheme()` are allowed.
   - Before overwriting each copied destination cell, clear any destination
     grapheme storage for that cell.
   - When copying a source grapheme cell:
     - copy the base cell content;
     - reset the copied destination cell's content tag to `Codepoint` before
       inserting new grapheme storage;
     - look up the source grapheme codepoints from the source page;
     - append those codepoints into the destination page's grapheme storage;
     - preserve the source cell's base codepoint.
   - Grapheme lookups, clears, and inserts must be keyed by the actual
     source/destination cell offsets from each row's `cells` slice, matching
     Ghostty's pointer/offset-keyed model. Do not use the current
     `*_grapheme_at(x, y)` helpers for clone-row migration unless they are first
     refactored to accept row-derived cell offsets. This matters for future
     `cloneRowFrom`/partial-row work, where a row reference is the source of
     truth.
   - After each copied row, update the destination row's grapheme flag based on
     the whole row, so clearing copied graphemes does not leave stale row flags.
   - Convert `GraphemeError` into the clone error type so out-of-memory remains
     explicit.

4. Keep style and hyperlink guards.
   - Reject source or destination rows with:
     - `Row::styled`
     - `Row::hyperlink`
   - Reject copied source or destination cells with:
     - non-default `Cell::style_id()`
     - `Cell::hyperlink()`
   - Do not reject grapheme rows/cells anymore.
   - Add tests proving style and hyperlink markers are still rejected.

5. Port upstream grapheme tests.
   - Port `Page cloneFrom graphemes`:
     - write codepoint `y + 1` in source column `1`;
     - append grapheme `0x0A`;
     - clone all rows;
     - verify destination codepoints, row grapheme flags, cell grapheme tags,
       and looked-up grapheme slices;
     - clear source graphemes and source codepoints;
     - verify the destination remains unchanged.
   - Port `Page cloneFrom frees dst graphemes`:
     - source page has plain codepoints and no graphemes;
     - destination page starts with matching codepoints plus graphemes;
     - clone source into destination;
     - verify destination cells are plain, row grapheme flags are clear, and
       `Page::grapheme_count()` is `0`.
   - Add a multi-codepoint grapheme clone test:
     - append at least two grapheme codepoints to one source cell;
     - clone;
     - verify the destination lookup returns the exact full slice, not only the
       first appended codepoint.
   - Add a trailing-destination preservation test:
     - source page is narrower than the destination page;
     - destination has a grapheme cell beyond the copied range;
     - clone;
     - verify the trailing cell, grapheme map entry, and row grapheme flag are
       preserved.
   - Keep Experiment 17 plain-row tests green.

6. Preserve scope.
   - Do not implement:
     - style migration;
     - hyperlink storage or migration;
     - `clearCells` as a general public/internal API beyond local grapheme
       cleanup needed for row clone;
     - `clonePartialRowFrom`;
     - `exactRowCapacity`;
     - full integrity checking.
   - Do not change whole-page `Page::clone_page`.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - final method naming;
     - grapheme migration approach;
     - destination grapheme cleanup approach;
     - remaining unsupported style/hyperlink cases;
     - upstream tests ported;
     - verification command output summary.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- source grapheme cells clone into independent destination grapheme storage;
- clearing or mutating source graphemes after clone does not affect the
  destination;
- destination grapheme cells in the copied range are freed when overwritten by
  plain source cells;
- destination grapheme cells outside the copied range are preserved when the
  source is narrower than the destination;
- multi-codepoint grapheme slices clone exactly;
- grapheme migration is keyed by actual row cell offsets, not by a canonical
  `y * cols + x` calculation that ignores `Row::cells`;
- destination row grapheme flags are correct after cloning;
- `Page::grapheme_count()` returns `0` after cloning plain rows over all
  destination graphemes;
- Experiment 17's plain-row clone tests still pass;
- style and hyperlink row/cell markers are still rejected rather than silently
  copied;
- no full `clone_from`, style migration, hyperlink migration, partial-row copy,
  exact-capacity, or integrity behavior is introduced;
- `cargo fmt`, targeted Page tests, and full `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- source grapheme cloning works, but destination grapheme cleanup requires a
  focused `clearCells` prerequisite before it can be faithful.

The experiment fails if:

- destination grapheme allocations leak after plain-source clone;
- cloned destination graphemes alias source storage;
- source grapheme cells are copied without destination grapheme-map entries;
- stale row grapheme flags remain after cleanup;
- style or hyperlink markers are copied without migrating their backing storage;
- existing plain-row clone, whole-page clone, style, or layout behavior
  regresses.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or implementing the
slice.
