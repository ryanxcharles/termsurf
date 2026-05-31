# Experiment 24: Port Page Move Cells

## Description

Port upstream `Page.moveCells` to Roastty.

`moveCells` moves a contiguous cell range from one row location to another,
clears the source cells, and preserves managed-memory ownership without
allocating. Upstream uses this for terminal edit operations such as inserting,
deleting, and shifting cells. Roastty now has enough Page managed-memory
machinery to port this operation: cells, graphemes, styles, hyperlinks, row
flags, clear/release helpers, and partial row clone behavior are all present.

This experiment should add internal Page cell-moving support only. It should not
add `swapCells`, terminal parser/screen edit operations, reflow, scrollback, or
public ABI.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/page.zig` as the source of truth for:
     - `Page.moveCells`;
     - helper behavior around `clearCells`, `moveGrapheme`, and `moveHyperlink`;
     - tests named `Page moveCells ...`.
   - Preserve upstream's no-allocation move semantics.
   - Do not modify `vendor/ghostty/`.

2. Add an internal move API.
   - Add an internal method such as:

     ```rust
     fn move_cells(
         &mut self,
         src_y: usize,
         src_left: usize,
         dst_y: usize,
         dst_left: usize,
         len: usize,
     )
     ```

   - Match upstream preconditions:
     - source and destination row indexes are valid;
     - `src_left + len <= self.size.cols`;
     - `dst_left + len <= self.size.cols`.
     - same-row overlapping ranges are rejected with an assertion.
   - The operation must not allocate and must not return an allocation error.
   - Exact self-moves (`src_y == dst_y`, `src_left == dst_left`, `len > 0`) are
     also rejected as overlapping ranges. Upstream does not document or test
     this case, and supporting it would require a separate contract.

3. Clear destination cells first.
   - Before moving source cells, clear managed memory in destination cells
     `dst_left..dst_left + len`:
     - release destination graphemes;
     - release destination hyperlinks;
     - release destination styles.
   - Clear only the destination range, not the whole row.
   - Preserve destination cells outside the range.

4. Move plain cells directly.
   - If the source row has no managed memory, copy the source cells into the
     destination range and then zero the source range.
   - Same-row overlapping ranges are out of scope and must assert before any
     destination clearing or source mutation happens.

5. Move grapheme map entries.
   - For source cells with grapheme data:
     - move the grapheme map entry from the source cell offset to the
       destination cell offset;
     - do not allocate a new grapheme slice;
     - reset the source cell to non-grapheme;
     - mark the destination cell and row as grapheme-bearing.
   - Source grapheme counts and allocator used bytes should remain unchanged
     after a successful move.

6. Move hyperlink map entries.
   - For source cells with hyperlink data:
     - move the hyperlink map entry from the source cell offset to the
       destination cell offset;
     - do not change the hyperlink set refcount for the moved link;
     - reset the source cell hyperlink bit;
     - mark the destination cell and row as hyperlink-bearing.
   - Source hyperlink counts should remain unchanged after a successful move,
     except for destination links that were cleared before the move.

7. Preserve style refcounts while moving.
   - Styles are stored by cell ID and refcounted by number of using cells.
   - Moving a styled cell from source to destination should not change its style
     refcount, because one using cell is moved rather than cloned.
   - Clearing the destination range must release any previous destination style
     refs before the moved source cell overwrites them.
   - Zeroing source cells must not release moved source styles a second time.

8. Update row flags.
   - After the move:
     - source row flags must reflect remaining source cells;
     - destination row flags must reflect destination cells after the move;
     - if the entire source row range was moved and the row has no remaining
       managed cells, source row flags should clear.
   - Recompute `grapheme`, `hyperlink`, and `styled` flags from row contents
     instead of depending on incremental assumptions.

9. Add focused tests.
   - Port or create Roastty equivalents for upstream move tests:
     - text-only full-row move copies text into the destination and blanks the
       source;
     - grapheme full-row move preserves grapheme slices and count while blanking
       the source.
   - Add managed-memory coverage beyond upstream's current tests:
     - styled cells move without changing source style refcounts except for
       clearing destination styles;
     - hyperlink cells move without changing moved hyperlink refcounts except
       for clearing destination hyperlinks;
     - destination grapheme/style/hyperlink data in the target range is released
       before the move;
     - cells outside source and destination ranges are preserved;
     - partial-range move between distinct non-overlapping ranges works;
     - same-row overlapping ranges, including exact self-move, panic before
       modifying the page.

10. Preserve scope.
    - Do not implement:
      - `swapCells`;
      - terminal edit commands;
      - parser/screen integration;
      - reflow or scrollback behavior;
      - public ABI or app-facing APIs.
    - Do not change Page layout constants or allocator semantics.

11. Verify.
    - Run:

      ```bash
      cargo fmt
      cargo test -p roastty terminal::page
      cargo test -p roastty
      ```

    - `cargo fmt` output must be accepted as-is.

12. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - move API added;
      - same-row overlap rejection behavior;
      - managed-memory movement strategy;
      - refcount behavior for styles and hyperlinks;
      - tests added;
      - any deferred upstream Page methods such as `swapCells`;
      - verification command output summary.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `Page::move_cells` moves the requested source range to the destination range
  and blanks the source range;
- destination managed memory in the target range is released before the move;
- source managed memory is moved, not cloned or leaked;
- grapheme count and allocator usage remain stable for moved graphemes;
- moved hyperlink refcounts remain stable except for destination links cleared
  before the move;
- moved style refcounts remain stable except for destination styles cleared
  before the move;
- source and destination row flags are correct after full and partial moves;
- same-row overlapping moves, including exact self-move, are rejected before
  modifying the page;
- existing clone, partial clone, exact capacity, style, grapheme, and hyperlink
  tests do not regress;
- `cargo fmt`, targeted Page tests, and full `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- text and grapheme moves work, but one managed-memory type needs a focused
  follow-up;
- non-overlapping moves work, but a future caller requires same-row overlap and
  needs a separate experiment with a precise contract;
- move semantics are correct, but row-flag recomputation exposes a pre-existing
  helper gap that needs a smaller preparatory experiment.

The experiment fails if:

- moved cells are cloned instead of moved;
- source managed-memory refs are released after being moved;
- destination managed-memory refs leak;
- overlapping moves mutate the page instead of asserting before mutation;
- row flags become inconsistent with row contents;
- the implementation expands into `swapCells`, parser/screen integration,
  reflow, scrollback, public ABI, or unrelated behavior.
