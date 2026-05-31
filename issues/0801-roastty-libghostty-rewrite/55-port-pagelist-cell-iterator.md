# Experiment 55: Port PageList Cell Iterator

## Description

Port the upstream PageList cell-iteration layer.

Experiment 54 added `RowIterator`, which is the base traversal primitive used by
upstream `CellIterator`. This experiment should add the missing cell iterator
above that row iterator. The iterator should yield `Pin` values for every cell
in a region, preserve the caller's starting column for the first row, advance
through complete rows via `RowIterator`, and stop at the same row-region bounds
as upstream.

This is still PageList-only traversal work. It must not port prompt iteration,
semantic highlighting, diagrams, selection/search behavior, parser behavior,
renderer delivery, app behavior, or public ABI.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/PageList.zig` for:
     - `CellIterator`;
     - `PageList.cellIterator`;
     - `Pin.cellIterator`;
     - the dependency on `RowIterator`.
   - Do not modify `vendor/ghostty/`.

2. Add an internal Rust `CellIterator<'a>`.
   - Store:
     - `row_it: RowIterator<'a>`;
     - `cell: Option<Pin>`.
   - Implement `Iterator<Item = Pin>`.
   - Keep the type private/internal for now.
   - Do not expose mutable row, cell, page, or PageList access through the
     iterator.

3. Preserve upstream direction semantics.
   - For `Direction::RightDown`:
     - yield the current cell;
     - if `x + 1 < cols`, advance `x` by one;
     - otherwise advance to the next row from `RowIterator`;
     - the next row starts at `x = 0` because `RowIterator` yields row pins.
   - For `Direction::LeftUp`:
     - yield the current cell;
     - if `x > 0`, decrement `x`;
     - otherwise advance to the previous row from `RowIterator`;
     - when a previous row exists, set `x = cols - 1`.
   - Use the page's current column count from the resolved row pin's owning
     page; do not hard-code `PageList.cols` if the page object is the clearer
     source of truth.

4. Add cell-iterator constructors.
   - Add a private helper equivalent to upstream `Pin.cellIterator`, shaped as a
     `PageList` helper if that is cleaner in Rust.
   - Add `PageList::cell_iterator(direction, top_left, bottom_left)`.
   - Match the existing `PageList::row_iterator` point handling:
     - resolve `top_left` with `pin`;
     - resolve explicit `bottom_left` with `pin`;
     - use `get_bottom_right(top_left.tag())` when no bottom-left point is
       supplied;
     - return an empty iterator if either endpoint cannot be pinned.
   - For `RightDown`, iterate from top-left toward bottom-left.
   - For `LeftUp`, iterate from bottom-left toward top-left.
   - Explicit bounds constrain the row range through `RowIterator`. They do not
     horizontally clip the final row. The bound pin's `x` is only relevant when
     that bound is also the starting pin for the chosen direction.

5. Add tests.
   - Single-row `RightDown` iteration starts at the caller's top-left `x`, walks
     to the end of that row, and uses the explicit bottom bound only as a row
     bound.
   - Single-row `LeftUp` iteration starts at the caller's bottom-left `x`, walks
     left to `x = 0`, and uses the explicit top bound only as a row bound.
   - `RightDown` explicit-limit coverage must prove `bottom_left.x` does not
     clip the final row: use a low `bottom_left.x` and assert yielded cells
     still reach `cols - 1`.
   - `LeftUp` explicit-limit coverage must prove `top_left.x` does not clip the
     final/top row: use a nonzero `top_left.x` and assert yielded cells still
     reach `x = 0`.
   - Multi-row `RightDown` iteration preserves the caller's starting column only
     for the first yielded row; subsequent rows start at `x = 0`.
   - Multi-row `LeftUp` iteration preserves the caller's bottom column only for
     the first yielded row; earlier rows start at `x = cols - 1`.
   - Cross-page iteration works in both directions.
   - Active partial starts across page boundaries work in both directions.
   - History iteration in both directions stops before active rows.
   - Invalid/unpinnable endpoints return an empty iterator.
   - Yielded cell pins convert back to expected points with `point_from_pin`
     where appropriate.

6. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

7. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - iterator shape;
     - direction behavior;
     - first-row/next-row column behavior;
     - boundary/limit behavior;
     - tests added;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `CellIterator` yields cells in upstream order for both directions;
- first-row starting columns and subsequent-row reset columns match upstream;
- cell iteration crosses rows and pages using `RowIterator`;
- explicit and implicit bounds match upstream behavior;
- explicit bound `x` values are not treated as horizontal clipping limits except
  when the bound is also the starting pin for the chosen direction;
- history cell iteration stops before active rows in both directions;
- invalid endpoints produce an empty iterator instead of panics;
- no prompt iterator, diagram, semantic highlighting, parser, renderer, app,
  public ABI, resize/reflow, selection, or search work is introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- an independent agent reviews the experiment design and completed result and
  approves them, or all real findings are fixed.

The experiment is partial if:

- basic cell iteration works, but a corner case around first-row column
  handling, implicit bounds, or history/active coordinate conversion needs a
  follow-up experiment.

The experiment fails if:

- cell iteration duplicates page traversal logic instead of building on
  `RowIterator`;
- cell iteration yields cells in the wrong direction or skips/duplicates
  boundary cells;
- explicit bound `x` values incorrectly clip final rows;
- invalid points panic;
- first-row starting columns or subsequent-row reset columns diverge from
  upstream;
- the implementation expands into prompt iteration, diagram output, semantic
  highlighting, parser, renderer, app, ABI, resize/reflow, selection, or search
  work;
- tests or formatting fail.
