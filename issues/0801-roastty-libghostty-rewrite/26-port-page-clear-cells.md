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

# Experiment 26: Port Page Clear Cells

## Description

Port upstream `Page.clearCells` to Roastty as a standalone Page primitive.

Experiment 24 introduced `clear_cells_range` as a helper for `move_cells`, but
that helper has not yet been treated as the faithful Rust adaptation of upstream
`clearCells`. Upstream `clearCells` clears a row range, releases any
managed-memory ownership in that range, zeros the cell payloads, and updates row
flags differently for full-row versus partial-row clears.

This experiment should formalize and test that behavior. It should not add
terminal edit commands, parser/screen integration, reflow, scrollback, public
ABI, or app-facing APIs.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/page.zig` as the source of truth for:
     - `Page.clearCells`;
     - `Page.clearGrapheme`;
     - `Page.clearHyperlink`;
     - row flag update behavior after full-row and partial-row clears.
   - Upstream does not have a direct `Page clearCells` test block, so create
     Roastty tests that prove the behavior directly.
   - Do not modify `vendor/ghostty/`.

2. Promote the clear helper to the Page primitive.
   - Keep or rename the existing helper to an internal Page method such as:

     ```rust
     fn clear_cells(&mut self, row_index: usize, left: usize, end: usize)
     ```

   - Update `move_cells` destination clearing to use the canonical method.
   - Do **not** use `clear_cells` for `move_cells` source blanking. Upstream
     explicitly avoids this because the source managed memory has already been
     moved; clearing the source through `clearCells` would release moved
     graphemes, hyperlinks, or styles.
   - `move_cells` source blanking must remain raw `Cell::default()` writes with
     no managed-memory release.
   - Match upstream bounds behavior:
     - `row_index < self.size.rows`;
     - `left <= end`;
     - `end <= self.size.cols`.
   - Empty ranges are allowed and should leave the page unchanged.

3. Release managed memory in the clear range.
   - For every cleared cell in `left..end`:
     - if the cell has a grapheme, remove its grapheme map entry and free its
       grapheme slice;
     - if the cell has a hyperlink, remove its hyperlink map entry and release
       the hyperlink set ref;
     - if the cell has a non-default style ID, release the style ref.
   - Release only cells inside the clear range.
   - Do not release or alter managed memory outside the clear range.

4. Zero cleared cells.
   - After managed memory for a cell is released, set that cell to
     `Cell::default()`.
   - The operation must not allocate and must not return allocation errors.

5. Update row flags.
   - Match upstream semantics:
     - after full-row clear, `grapheme`, `hyperlink`, and `styled` flags must be
       false;
     - after partial-row clear, flags must reflect any remaining cells outside
       the cleared range.
   - Recomputing the row flags from cells is acceptable for both full and
     partial clears.
   - Preserve unrelated row metadata such as wrap/dirty/semantic prompt.

6. Add focused tests.
   - Plain clear:
     - clearing a range zeros only that range;
     - cells outside the range are preserved;
     - empty range is a no-op.
   - Grapheme clear:
     - clearing grapheme cells removes map entries and frees allocator bytes;
     - partial clear preserves graphemes outside the range and keeps the row
       flag true;
     - full-row clear clears the row flag.
   - Hyperlink clear:
     - clearing linked cells removes map entries and releases hyperlink refs;
     - partial clear preserves links outside the range and keeps the row flag
       true;
     - full-row clear clears the row flag.
   - Style clear:
     - clearing styled cells releases style refs;
     - partial clear preserves styled cells outside the range and keeps the row
       flag true;
     - full-row clear clears the row flag.
   - Mixed managed-memory clear:
     - one range containing style, grapheme, and hyperlink data releases all
       three kinds of managed memory and zeros cells.
   - Row metadata:
     - wrap/dirty/semantic prompt metadata survives cell clearing.
   - Call-site behavior:
     - `move_cells` still preserves moved grapheme/style/hyperlink ownership
       after routing destination cleanup through `clear_cells`;
     - source blanking in `move_cells` does not release moved managed memory.

7. Route partial clone destination cleanup through `clear_cells`.
   - Upstream `clonePartialRowFrom` also uses `clearCells` for destination range
     cleanup.
   - Update Roastty's partial clone cleanup to use the canonical clear helper
     for `x_start..x_end`.
   - Preserve the failure-safety reset behavior added in Experiment 23: copied
     source cell payloads must still reset style IDs, hyperlink bits, and
     grapheme tags before fallible managed-memory migration.
   - Existing partial clone success and failure tests must continue passing.

8. Preserve scope.
   - Do not implement:
     - terminal edit commands;
     - parser/screen integration;
     - reflow or scrollback behavior;
     - public ABI or app-facing APIs.
   - Do not change Page layout constants or allocator semantics.
   - Do not change `move_cells` behavior except to route destination cleanup
     through the canonical clear helper.
   - Do not change `move_cells` source blanking to use the canonical clear
     helper.
   - Do not change `swap_cells` behavior.

9. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

10. Record the result.

- Append `## Result` and `## Conclusion` to this file.
- Include:
  - clear API name and call-site routing;
  - confirmation that `move_cells` source blanking does not use `clear_cells`;
  - partial clone cleanup routing;
  - managed-memory release behavior;
  - full-row and partial-row flag behavior;
  - tests added;
  - any deferred upstream Page methods;
  - verification command output summary.
- Update the Issue 801 README experiment index from `Designed` to `Pass`,
  `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `Page::clear_cells` clears only `left..end` in the selected row;
- empty ranges are no-ops;
- cleared cells are zeroed;
- grapheme map entries and slices inside the range are removed/freed;
- hyperlink map entries inside the range are removed and hyperlink refs are
  released;
- style refs inside the range are released;
- managed memory outside the range is preserved;
- full-row clears leave row `grapheme`, `hyperlink`, and `styled` flags false;
- partial-row clears leave row flags matching remaining cells;
- unrelated row metadata is preserved;
- `move_cells` uses `clear_cells` only for destination cleanup and does not
  release source managed memory after moving it;
- partial clone destination cleanup routes through `clear_cells` without
  regressing Experiment 23 failure-safety behavior;
- existing clone, partial clone, move, swap, exact capacity, style, grapheme,
  and hyperlink tests do not regress;
- `cargo fmt`, targeted Page tests, and full `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- plain clears work, but one managed-memory type needs a focused follow-up;
- managed-memory clearing works, but row metadata preservation exposes an
  unrelated pre-existing gap.

The experiment fails if:

- clearing a range releases or zeros cells outside the requested range;
- managed-memory refs or allocations leak;
- row flags become inconsistent with row contents;
- unrelated row metadata is lost;
- the implementation expands into parser/screen integration, terminal edit
  commands, reflow, scrollback, public ABI, or unrelated behavior.

## Result

**Result:** Pass

Roastty now has canonical internal Page cell-clearing support.

Implementation details:

- promoted the previous `clear_cells_range` helper to `Page::clear_cells`;
- routed `move_cells` destination cleanup through `clear_cells`;
- preserved `move_cells` source blanking as raw `Cell::default()` writes with no
  managed-memory release;
- routed partial clone destination cleanup through `clear_cells`;
- preserved Experiment 23's failure-safety behavior by still resetting copied
  source cell managed-memory markers before fallible migration;
- `clear_cells` releases grapheme slices/map entries, hyperlink map entries and
  refs, and style refs only inside the requested range;
- cleared cells are zeroed;
- row `grapheme`, `hyperlink`, and `styled` flags are recomputed after clearing;
- unrelated row metadata such as wrap, wrap-continuation, dirty, and semantic
  prompt survives clearing.

Tests added:

- plain range clear and empty-range no-op;
- partial and full grapheme clear with allocator bytes and row flags checked;
- partial and full hyperlink clear with map count/refcount and row flags
  checked;
- partial and full style clear with refcounts and row flags checked;
- mixed managed-memory clear across style, grapheme, and hyperlink cells;
- unrelated row metadata preservation.

The existing `move_cells` and partial clone tests continue to verify the two
important call-site invariants: destination cleanup can use `clear_cells`, while
`move_cells` source blanking must not release moved managed memory.

The experiment did not implement terminal edit commands, parser/screen
integration, reflow, scrollback, public ABI, or app-facing APIs.

Verification run:

```bash
cargo fmt
cargo test -p roastty terminal::page
cargo test -p roastty
```

Results:

- `cargo test -p roastty terminal::page`: 128 passed.
- `cargo test -p roastty`: 237 unit tests passed; ABI harness passed; doc tests
  passed.

## Conclusion

Experiment 26 successfully ports upstream `Page.clearCells` behavior into
Roastty's Page storage model and removes the last ad hoc destination cleanup in
partial row clone. Page clearing is now a standalone primitive with focused
coverage for plain cells, all currently ported managed-memory types, full-row
and partial-row flag behavior, and metadata preservation.

The next experiment should continue through the remaining upstream Page surface
after re-reading current upstream call sites.
