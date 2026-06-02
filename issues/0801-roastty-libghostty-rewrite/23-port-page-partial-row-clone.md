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

# Experiment 23: Port Page Partial Row Clone

## Description

Port upstream `Page.clonePartialRowFrom` to Roastty.

Roastty can now clone full rows across plain cells, graphemes, styles, and
hyperlinks. Experiment 22 added exact row capacity, which is one of the
supporting pieces used by upstream page splitting and reflow paths. The next
missing Page operation is partial row cloning: copying only a selected column
range from one row into another while preserving destination cells and metadata
outside that range.

This experiment should add an internal `Page::clone_partial_row_from` method and
make `clone_row_from` call it for full-row clones. It should not add reflow,
screen splitting, parser integration, terminal scrollback behavior, or public
ABI.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/page.zig` as the source of truth for:
     - `Page.clonePartialRowFrom`;
     - `Page.cloneRowFrom`;
     - tests named `Page cloneRowFrom partial ...`.
   - Preserve upstream semantics where Roastty's already-ported storage model
     supports them.
   - Do not modify `vendor/ghostty/`.

2. Add `Page::clone_partial_row_from`.
   - Add an internal method:

     ```rust
     fn clone_partial_row_from(
         &mut self,
         other: &Page,
         dst_y: usize,
         src_y: usize,
         x_start: usize,
         x_end_req: usize,
     ) -> Result<(), CloneFromError>
     ```

   - This method is for cross-page copies and for callers that have distinct
     source and destination pages. It must not be used with aliasing `self` and
     `other`; safe Rust should make that impossible.
   - Match upstream bounds behavior:
     - `dst_y < self.size.rows`;
     - `src_y < other.size.rows`;
     - `x_start <= x_end`;
     - `x_end = min(x_end_req, min(self.size.cols, other.size.cols))`.
   - Copy only cells in `x_start..x_end`.
   - Preserve destination cells outside `x_start..x_end`.

3. Add an explicit same-page partial clone path.
   - Add an internal same-page helper, for example:

     ```rust
     fn clone_partial_row_within_page(
         &mut self,
         dst_y: usize,
         src_y: usize,
         x_start: usize,
         x_end_req: usize,
     ) -> Result<(), CloneFromError>
     ```

   - This helper exists because Rust cannot safely call
     `page.clone_partial_row_from(&page, ...)` with both `&mut self` and
     `&Page`.
   - Preserve upstream same-page semantics:
     - snapshot the source cells and managed-memory IDs/slices before clearing
       destination cells;
     - for copied same-page hyperlinks, reuse the existing hyperlink ID and
       increment its refcount;
     - for copied same-page styles, reuse the existing style ID and increment
       its refcount;
     - do not convert same-page copies into cross-page clones by cloning the
       whole page, because that would skip the upstream same-page refcount
       behavior this experiment needs to port.
   - The same-page helper may share lower-level migration code with the
     cross-page helper, but the design must keep the same-page aliasing problem
     explicit and safe.

4. Make full-row clone use partial clone.
   - Change `clone_row_from` to call `clone_partial_row_from` with:

     ```rust
     x_start = 0
     x_end_req = self.size.cols
     ```

   - Preserve all existing full-row clone behavior and tests.
   - Do not duplicate separate full-row and partial-row implementations after
     this experiment. The shared method should be the source of truth.

5. Preserve destination metadata outside partial ranges.
   - When the copied range is smaller than the destination row width:
     - preserve destination `wrap`;
     - preserve destination `wrap_continuation`;
     - preserve destination row `grapheme`, `hyperlink`, and `styled` flags for
       non-copied cells;
     - combine dirty state so an already-dirty destination row stays dirty.
   - After copying, recompute row managed-memory flags from actual cells. This
     is required because copied cells and preserved cells can both contribute to
     row-level flags.
   - If the copy range covers the full destination width, the source row
     metadata should replace the destination row metadata except for the
     destination cell offset.

6. Clear destination managed memory only inside the copied range.
   - Before overwriting copied cells, release graphemes, hyperlinks, and styles
     attached to destination cells in `x_start..x_end`.
   - Do not release managed memory for destination cells outside
     `x_start..x_end`.
   - Preserve refcounts and allocations for non-copied destination graphemes,
     styles, and hyperlinks.

7. Reset copied cell managed-memory markers before fallible migration.
   - After copying source cell values into the destination range, immediately
     reset copied destination cells to a locally valid, non-managed state before
     any fallible insertion:
     - set `style_id` to `style::DEFAULT_ID`;
     - set `hyperlink` to `false`;
     - if the copied content tag is grapheme-backed, reset it to ordinary
       codepoint content before re-adding graphemes through Page helpers.
   - This mirrors upstream's failure-safety pattern and Roastty's current
     full-row clone behavior.
   - On any later allocation/refcount error, copied cells must not be left with
     source-page style IDs, hyperlink bits, or grapheme tags that do not resolve
     in the destination page.
   - Recompute row-level managed-memory flags before returning errors.

8. Copy source managed memory inside the copied range.
   - Plain cells:
     - copy directly after clearing destination managed-memory markers.
   - Graphemes:
     - copy grapheme codepoint slices for copied cells only;
     - source graphemes outside the range must not be copied;
     - destination graphemes outside the range must remain intact.
   - Hyperlinks:
     - copy hyperlinks for copied cells only;
     - support same-page copying by reusing the existing ID and increasing its
       refcount;
     - support cross-page copying by cloning or reusing equivalent hyperlink
       entries through existing hyperlink helpers;
     - surface existing `CloneFromError` variants when hyperlink set, map, or
       string storage is insufficient.
   - Styles:
     - copy non-default style IDs for copied cells only;
     - support same-page copying by increasing the existing style refcount;
     - support cross-page copying through existing style clone helpers;
     - preserve destination styles outside the range.

9. Handle spacer-head cleanup.
   - Preserve the existing full-row behavior for growing-column clones: if the
     destination is wider than the source and the last copied source cell is a
     spacer head, clear that spacer head to narrow.
   - For partial clones, apply that cleanup only when:
     - `self.size.cols > other.size.cols`; and
     - the copied range includes the source-edge cell at `other.size.cols - 1`.
   - Do not clear spacer heads outside the copied range for partial clones.
   - Add a targeted test if the existing full-row spacer-head test does not
     cover this path after refactoring.

10. Add focused success tests.

- Port or create Roastty equivalents for upstream partial clone tests:
  - plain partial row copy copies only `x_start..x_end`;
  - source graphemes outside the copied range are omitted;
  - destination graphemes outside the copied range survive;
  - same-page hyperlink inside the copied range is copied and refcounted;
  - same-page hyperlink outside the copied range is omitted from the destination
    copy.
- Add style-specific partial clone coverage:
  - source style inside the copied range is copied;
  - source style outside the copied range is omitted;
  - destination style outside the copied range survives.
- Add cross-page hyperlink and style coverage when current helper behavior makes
  this practical without duplicating earlier tests.
- Ensure all existing full-row clone tests still pass through the new shared
  method.

11. Add focused failure-path tests.

- Add targeted partial-row failure tests for the managed-memory paths that can
  fail in the copied range:
  - grapheme allocation failure;
  - style set out of memory or needs-rehash, using the existing style error
    setup patterns from earlier clone experiments;
  - hyperlink map out of memory;
  - hyperlink set out of memory or needs-rehash where practical;
  - string allocation failure for explicit hyperlink IDs or URIs where
    practical.
- Each failure test should verify:
  - cells outside `x_start..x_end` remain unchanged;
  - destination managed memory outside `x_start..x_end` remains live;
  - copied-range cells are left in a valid destination-page state after the
    error;
  - row-level `grapheme`, `hyperlink`, and `styled` flags are recomputed;
  - the operation returns the expected `CloneFromError` variant.
- If one failure mode cannot be induced cleanly with the current test helpers,
  document the exact reason in the result and keep the rest of the failure
  coverage.

12. Preserve scope.

- Do not implement:
  - terminal reflow;
  - screen splitting;
  - parser/screen integration;
  - scrollback behavior;
  - public ABI or app-facing APIs.
- Do not change Page layout constants or allocator semantics except where a
  helper is needed to avoid duplicating existing clear/copy logic.

13. Verify.
    - Run:

      ```bash
      cargo fmt
      cargo test -p roastty terminal::page
      cargo test -p roastty
      ```

    - `cargo fmt` output must be accepted as-is.

14. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - partial-row API added;
      - same-page helper shape and why it exists;
      - how full-row clone now routes through partial-row clone;
      - managed-memory preservation and refcount behavior;
      - failure-state behavior after allocation/refcount errors;
      - tests added;
      - any deferred upstream Page behavior;
      - verification command output summary.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `Page::clone_partial_row_from` copies only the requested column range;
- `clone_row_from` uses the partial clone implementation for full-row copies;
- same-page partial clone is supported through a Rust-safe helper that preserves
  upstream same-page refcount semantics;
- destination cells and managed-memory allocations outside the copied range are
  preserved;
- destination cells inside the copied range release previous styles, graphemes,
  and hyperlinks before being overwritten;
- copied-range cells reset source-page managed-memory markers before fallible
  migration, so errors cannot leave invalid source IDs or flags in the
  destination;
- copied graphemes, styles, and hyperlinks inside the range match full-row clone
  semantics;
- same-page hyperlink and style copies increment refcounts correctly;
- cross-page managed-memory copies continue to use the existing allocation and
  error paths;
- row-level `grapheme`, `hyperlink`, and `styled` flags reflect the final mixed
  row contents after partial copies;
- source managed-memory cells outside the copied range are not copied;
- allocation/refcount failure tests prove partial-copy errors leave the
  destination page internally valid and preserve out-of-range managed memory;
- existing full-row clone tests do not regress;
- `cargo fmt`, targeted Page tests, and full `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- plain partial row copies work, but one managed-memory type still needs follow
  up;
- same-page copies work, but cross-page partial copies expose an allocator or
  refcount issue that needs a smaller next experiment;
- full-row clone behavior remains correct, but partial clone requires another
  pass for a specific metadata flag or error path.

The experiment fails if:

- full-row clone regresses;
- destination cells outside the copied range are cleared or refcount-released;
- source managed-memory outside the copied range is copied;
- an allocation/refcount error leaves copied destination cells with stale
  source-page style IDs, hyperlink bits, or grapheme tags;
- hyperlink/style/grapheme refcounts become inconsistent;
- the implementation expands into reflow, parser/screen integration, public ABI,
  or unrelated terminal behavior.

## Result

**Result:** Pass

Roastty now has internal partial-row clone support for Page storage.

Implementation details:

- `clone_row_from` now routes through `clone_partial_row_from` with a full-row
  column range, so full-row and partial-row clones share one implementation
  path.
- `clone_partial_row_from` supports cross-page row-range copying with
  `x_start..x_end_req` bounds matching upstream's min-with-source/destination
  width behavior.
- `clone_partial_row_within_page` provides the Rust-safe same-page copy shape
  that upstream gets from `other == self`. It snapshots source cells before
  mutation and temporarily holds same-page style/hyperlink refs so overlapping
  copies cannot drop an ID before it is reused.
- copied destination cells clear existing grapheme, hyperlink, and style state
  only inside the copied range;
- destination cells outside the copied range remain untouched, including their
  managed-memory refs;
- copied cells reset style IDs, hyperlink bits, and grapheme tags before
  fallible managed-memory migration, so error returns cannot leave source-page
  IDs or flags in destination cells;
- row-level `grapheme`, `hyperlink`, and `styled` flags are recomputed on both
  success and partial-copy error paths;
- spacer-head cleanup remains limited to the growing-columns case and only runs
  when the copied range includes the source-edge cell.

Tests added:

- plain partial row copy;
- source graphemes outside the copied range are omitted;
- destination graphemes outside the copied range survive;
- same-page partial hyperlink copy and omit cases;
- same-page partial style reuse/refcount case;
- cross-page partial style copy with destination style preservation and
  alternate-ID behavior;
- grapheme-map OOM leaves copied cells valid and preserves outside cells;
- style-set OOM leaves copied cells valid and preserves outside cells;
- hyperlink-map OOM preserves out-of-range destination links and refs;
- hyperlink-string OOM leaves copied cells valid and rolls back link state;
- hyperlink-set OOM frees duplicated strings and leaves copied cells valid.

The experiment did not implement terminal reflow, screen splitting,
parser/screen integration, scrollback behavior, public ABI, or app-facing APIs.
Those remain later Issue 801 slices.

Verification run:

```bash
cargo fmt
cargo test -p roastty terminal::page
cargo test -p roastty
```

Results:

- `cargo test -p roastty terminal::page`: 105 passed.
- `cargo test -p roastty`: 214 unit tests passed; ABI harness passed; doc tests
  passed.

## Conclusion

Experiment 23 successfully ports upstream Page partial-row clone semantics into
Roastty's current storage model. Full-row clone now shares the partial-row
implementation, and the new same-page helper covers the upstream `other == self`
refcount behavior without unsafe aliasing.

The next experiment can move to the next upstream Page operation that depends on
exact capacity and partial row clone, likely page splitting/reflow support, but
that should be designed only after re-reading the upstream call sites.
