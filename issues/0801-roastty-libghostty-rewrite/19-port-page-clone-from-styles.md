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

# Experiment 19: Port Page CloneFrom Styles

## Description

Extend the current Page row-copy path to support Ghostty's style branch in
`cloneFrom` / `cloneRowFrom`.

Experiments 17 and 18 built the row-copy foundation for plain rows and
graphemes. The remaining managed-memory guard covers styles and hyperlinks.
Roastty already has style value storage, `style::Set`, Page style storage, and
whole-page style clone. It does not yet have Page hyperlink storage. The next
faithful slice is therefore style migration:

- release destination style references in the copied range before overwrite;
- clone source style values into the destination style set with `add_with_id`;
- preserve source style IDs when possible;
- rewrite copied destination cells to the actual destination style ID when an
  alternate ID is assigned;
- keep row `styled` flags accurate after copy.

Hyperlinks remain out of scope and must still be rejected.

Same-page row copy remains out of scope for this Rust slice. Ghostty has an
`other == self` style fast path, but Roastty's current safe method shape takes
`&mut self` and `&Page`, which excludes safe same-page calls. A later
`clonePartialRowFrom` design can introduce the right Rust API for same-page row
copy without compromising aliasing rules.

## Changes

1. Inspect upstream source.
   - Use `vendor/ghostty/src/terminal/page.zig` as the source of truth.
   - Re-read:
     - style branch of `Page.clonePartialRowFrom`;
     - destination cleanup in `Page.clearCells`;
     - style portions of `Page.exactRowCapacity`;
     - upstream tests around style exact capacity and clone row behavior.
   - There is no single upstream `Page cloneFrom styles` test analogous to the
     grapheme tests. This experiment should port the behavior directly and add
     focused Roastty tests that cover the same style invariants.
   - Do not modify `vendor/ghostty/`.

2. Rename the row-copy method to match the new scope.
   - Rename `clone_rows_from_without_styles_or_hyperlinks` to a scoped name such
     as `clone_rows_from_without_hyperlinks`.
   - Rename the per-row helper similarly.
   - Do not expose the final `clone_from` name yet. Hyperlink migration is still
     missing, so the method remains incomplete relative to Ghostty.

3. Add Page style helper wrappers.
   - Add narrow internal wrappers as needed:
     - `add_style_with_id(style, requested_id) -> Result<style::Id, AddError>`;
     - `release_style` / `use_style` already exist and should be reused;
     - `update_row_styled_flag(row_index)`, mirroring the grapheme flag updater.
   - `add_style_with_id` should preserve `style::Set::add_with_id` semantics:
     - if the requested ID is used, return that requested ID;
     - if an alternate ID is returned, use the alternate ID.
   - Convert `style::Set` `AddError` into the clone error type so style-set
     out-of-memory / rehash requirements are explicit.

4. Add style-aware row copy.
   - Continue preserving Experiments 17 and 18 behavior:
     - upstream `y_start`/`y_end` preconditions;
     - destination rows start at row `0`;
     - destination row `cells` offsets remain destination-owned;
     - copy only the overlapping cell range;
     - preserve trailing destination cells when the source is narrower;
     - clear grow-column trailing spacer heads;
     - migrate graphemes by actual row cell offsets.
   - Replace style rejection with real behavior:
     - source rows with `Row::styled` are allowed;
     - destination rows with `Row::styled` are allowed;
     - source cells with non-default `Cell::style_id()` are allowed;
     - destination cells with non-default `Cell::style_id()` are allowed.
   - Before overwriting each copied destination cell, release any destination
     style reference for that cell and reset its style ID to default.
   - When copying a source styled cell:
     - look up the source style value from the source page;
     - add it to the destination style set with the source style ID as the
       requested ID;
     - set the copied destination cell's style ID to the actual returned
       destination ID;
     - mark the destination row styled.
   - Preserve a valid destination state if style insertion fails:
     - do not leave copied destination cells containing source style IDs that do
       not resolve in the destination style set;
     - either insert the required destination style before writing the copied
       cell's non-default style ID, or copy cells with default style IDs and set
       the destination style ID only after insertion succeeds;
     - return the style insertion error explicitly if capacity or rehash work is
       required.
   - After each copied row, update the destination row styled flag based on the
     whole row, so clearing copied styles does not leave stale row flags and
     preserving trailing styles keeps the flag true.
   - Make style operations use Page style-set base pointers, not heap maps or ad
     hoc side storage.

5. Keep hyperlink guards.
   - Reject source or destination rows with `Row::hyperlink`.
   - Reject copied source or destination cells with `Cell::hyperlink()`.
   - Do not implement hyperlink storage or migration in this experiment.

6. Add focused style clone tests.
   - Style clone:
     - create a source page with a bold style applied across one or more cells;
     - clone into a destination page;
     - verify copied cells use the same style ID as the source when the
       destination can honor the requested ID;
     - verify destination style lookup returns the original style;
     - verify destination style ref count equals copied styled-cell uses;
     - mutate/release source style refs after clone and verify destination
       remains independent.
   - Style cleanup:
     - create destination styled cells in the copied range;
     - clone plain source cells over them;
     - verify copied cells have default style IDs;
     - verify destination row styled flag clears when no styled cells remain;
     - verify destination style count/ref count drops appropriately.
   - Styled-over-styled replacement:
     - destination copied range starts with a different style from the source;
     - clone styled source cells over it;
     - verify the old destination style ref count drops or the style is removed;
     - verify the new copied style ref count is correct and copied cells resolve
       to the source style value.
   - Source-narrower trailing style preservation:
     - source page is narrower than destination;
     - destination has a styled cell beyond the copied range;
     - clone;
     - verify trailing styled cell, style lookup, ref count, and row styled flag
       are preserved.
   - Alternate-ID path:
     - force `style::Set::add_with_id` to return an alternate ID by occupying
       the requested source style ID in the destination with a different style;
     - clone;
     - verify the copied cell uses the alternate destination ID and resolves to
       the source style value.
   - Style insertion failure state:
     - create a destination page whose style set cannot accept the source style;
     - attempt style clone and expect the style insertion error;
     - verify copied destination cells do not retain unresolved source style IDs
       after the failure.
   - Keep Experiment 17/18 plain and grapheme tests green.

7. Preserve scope.
   - Do not implement:
     - hyperlink storage or migration;
     - same-page row copy;
     - general `clearCells` beyond local style/grapheme cleanup needed for row
       clone;
     - `clonePartialRowFrom`;
     - `exactRowCapacity`;
     - full integrity checking.
   - Do not change whole-page `Page::clone_page`.

8. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page
     cargo test -p roastty terminal::style
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - final method naming;
     - style migration approach;
     - destination style cleanup approach;
     - remaining unsupported hyperlink cases;
     - tests added;
     - verification command output summary.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- source styled cells clone into independent destination style storage;
- copied destination cells use the actual destination style ID returned by
  `add_with_id`;
- empty-destination clone preserves source style IDs when `add_with_id` can
  honor the requested ID;
- releasing or mutating source style refs after clone does not affect the
  destination;
- destination style refs in the copied range are released when overwritten by
  plain source cells;
- destination style refs in the copied range are released when overwritten by
  other styled source cells;
- style insertion failures leave copied destination cells in a valid state with
  no unresolved source style IDs;
- destination styled cells outside the copied range are preserved when the
  source is narrower than the destination;
- row styled flags are correct after cloning;
- the alternate-ID path is tested and works;
- Experiments 17 and 18 plain/grapheme clone tests still pass;
- hyperlink row/cell markers are still rejected rather than silently copied;
- no full `clone_from`, hyperlink migration, partial-row copy, exact-capacity,
  same-page row copy, or integrity behavior is introduced;
- `cargo fmt`, targeted Page/style tests, and full `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- basic source style cloning works, but destination style cleanup or alternate
  ID assignment exposes a focused prerequisite in `style::Set` or Page style
  helpers.

The experiment fails if:

- copied style cells keep source IDs that do not resolve in the destination
  style set;
- a style insertion failure leaves source style IDs dangling in destination
  cells;
- destination style refs leak after plain-source clone;
- destination style refs leak after styled-source replacement;
- cloned destination styles alias source storage;
- stale row styled flags remain after cleanup;
- hyperlink markers are copied without migrating their backing storage;
- existing plain-row, grapheme, whole-page clone, style set, or layout behavior
  regresses.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or implementing the
slice.

## Result

**Result:** Pass

Experiment 19 replaced the style portion of the temporary row-copy managed
memory guard with real style migration. The row-copy method is now:

- `Page::clone_rows_from_without_hyperlinks`
- `Page::clone_row_from_without_hyperlinks`

The name remains scoped because hyperlink migration is still missing.

### Style Migration

The row-copy path now:

- allows source and destination styled rows;
- allows copied source and destination cells with non-default style IDs;
- releases destination style references in the copied range before overwrite;
- copies source cells into the destination with default style IDs first;
- inserts source styles into the destination style set with
  `style::Set::add_with_id`;
- preserves the source style ID when the requested ID is available;
- rewrites copied destination cells to the alternate destination ID when
  `add_with_id` cannot honor the requested ID;
- updates the destination row styled flag after copy.

Style insertion failures are handled without leaving dangling source style IDs
in destination cells. If insertion fails after cells have been copied, the cells
remain valid with either default style IDs or already-inserted destination style
IDs, and the row styled flag is recomputed before the error is returned.

### Remaining Unsupported Cases

The row-copy path still rejects hyperlinks:

- source or destination rows with `Row::hyperlink`;
- copied source or destination cells with `Cell::hyperlink()`.

Same-page row copy also remains out of scope. Roastty's current safe method
shape uses `&mut self` plus `&Page`, which excludes the Ghostty `other == self`
style fast path for now. That should be designed with `clonePartialRowFrom` or a
dedicated same-page Rust API later.

### Tests Added

Added Page tests:

- `page_clone_from_styles_preserves_requested_id`
- `page_clone_from_plain_source_releases_destination_styles`
- `page_clone_from_replaces_destination_style_refs`
- `page_clone_from_preserves_trailing_destination_style`
- `page_clone_from_style_alternate_id`
- `page_clone_from_style_insert_failure_leaves_valid_cells`

The existing hyperlink rejection test was updated for the new scope:

- `page_clone_from_rejects_hyperlink_rows_and_cells`

Experiment 17 and 18 plain/grapheme row-copy tests remain green.

### Verification

Commands run:

```bash
cargo fmt
cargo test -p roastty terminal::page
cargo test -p roastty terminal::style
cargo test -p roastty
```

Results:

- `cargo test -p roastty terminal::page`: 69 passed.
- `cargo test -p roastty terminal::style`: 21 passed.
- `cargo test -p roastty`: 178 Rust unit tests passed, ABI harness passed, doc
  tests passed.

## Conclusion

Roastty's Page row-copy path now supports plain rows, graphemes, and styles. The
remaining managed-memory gap for full `cloneFrom` is hyperlinks. Since Page
hyperlink storage and string allocation are not implemented yet, the next work
should likely build the hyperlink substrate before attempting hyperlink row-copy
or full `cloneFrom` naming.
