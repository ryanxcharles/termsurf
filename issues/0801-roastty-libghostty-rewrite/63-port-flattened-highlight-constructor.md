# Experiment 63: Port Flattened Highlight Constructor

## Description

Port the constructor behavior of upstream `highlight.Flattened.init` in a
Roastty-shaped way.

Upstream Zig implements `Flattened.init(alloc, start, end)` by asking `start`
for a page iterator and collecting each page chunk with its node pointer, page
serial, start row, and exclusive end row. In Roastty, `Pin` does not own
iteration; `PageList` owns the page storage and page iterators. Therefore the
constructor should live on `PageList`, likely as
`PageList::flatten_highlight(start, end) -> Option<highlight::Flattened>`.

Experiment 62 added the `Flattened` and `Chunk` shapes but deliberately deferred
construction. This experiment should add construction only. It must not add
tracked highlights, search, renderer, app, public ABI, selection, parser,
resize/reflow, or semantic behavior changes.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/highlight.zig` for:
     - `Flattened.init`;
     - chunk fields collected from page iteration;
     - x-bound assignment from start/end pins.
   - Use `roastty/src/terminal/page_list.rs` for existing:
     - `PageIterator`;
     - `page_iterator`;
     - `row_iterator_from_pin`;
     - `node_for_ptr`;
     - `node_index`;
     - split/cross-page tests.
   - Do not modify `vendor/ghostty/`.

2. Add PageList-owned flattening.
   - Add a private helper such as:

     ```rust
     fn flatten_highlight(&self, start: Pin, end: Pin) -> Option<highlight::Flattened>
     ```

   - The helper should:
     - reject invalid pins;
     - reject garbage pins;
     - reject reversed ranges where `end` is before `start` in PageList order,
       including ranges reversed across pages;
     - iterate rows/pages from `start` to `end` in `Direction::RightDown`;
     - collect every produced `PageChunk` into `highlight::Chunk`;
     - copy each chunk's node pointer, node serial, start row, and exclusive end
       row;
     - set `top_x = start.x`;
     - set `bot_x = end.x`;
     - return `None` if iteration produces no chunks.
   - Prefer returning `Option` over panicking for invalid/reversed input. Empty
     flattened values remain possible through `Flattened::empty`, but this
     constructor should only return nonempty flattened highlights.
   - Treat a pin as invalid if:
     - `pin.garbage` is true;
     - its node is not present in the PageList;
     - `pin.y >= node.page.size_rows()`;
     - `pin.x >= self.cols`.

3. Keep existing visibility discipline.
   - Do not expose `PageIterator`, `PageChunk`, `Node`, `Pin`, or highlight
     types outside `terminal`.
   - Do not make `Pin` fields public.
   - If `highlight::Chunk` construction needs a constructor to keep fields
     private, add one narrowly inside `highlight.rs`; otherwise keep
     `pub(super)` fields as established by Experiment 62.

4. Add tests.
   - Add focused PageList tests:
     - single-page flattening produces one chunk with the expected node serial,
       start row, end row, `top_x`, and `bot_x`;
     - cross-page flattening after `PageList::split` produces multiple chunks in
       top-to-bottom order;
     - the flattened `start_pin`, `end_pin`, and `untracked` values map back to
       the original start/end screen points;
     - same-page reversed ranges return `None`;
     - cross-page reversed ranges return `None`;
     - garbage pins return `None`;
     - missing-node pins return `None`;
     - out-of-bounds row pins return `None`;
     - out-of-bounds column pins return `None`;
     - a range ending on the same row includes that row by using an exclusive
       `end = end.y + 1` chunk bound.
   - Existing semantic highlight and flattened shape tests must continue
     passing.

5. Keep scope narrow.
   - Do not add tracked highlights.
   - Do not add search, renderer, parser, app, public ABI, selection, resize, or
     reflow behavior.
   - Do not change semantic highlight behavior.
   - Do not make `flatten_highlight` public outside `PageList` yet.

6. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

7. Independent review.
   - Before implementation, get an independent agent review of this experiment
     design.
   - After implementation and verification, get an independent result review.
   - Fix all real findings before proceeding.

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - constructor behavior;
     - invalid/reversed/garbage behavior;
     - tests added;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `PageList::flatten_highlight` or equivalent constructs nonempty
  `highlight::Flattened` values from valid ordered pins;
- produced chunks preserve node pointer, page serial, start row, and exclusive
  end row;
- `top_x` comes from the start pin and `bot_x` comes from the end pin;
- same-page and cross-page ranges produce correct chunks;
- same-row ranges include the endpoint row;
- `start_pin`, `end_pin`, and `untracked` map back to the original range;
- invalid, garbage, and reversed pins return `None`;
- invalid-pin coverage includes missing node, out-of-bounds row, and
  out-of-bounds column cases;
- reversed-range coverage includes both same-page and cross-page reversed
  ranges;
- no public API, tracked highlight, search, renderer, parser, app, selection,
  resize/reflow, diagram, or semantic behavior changes are introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- independent design and result reviews approve the experiment, or all real
  findings are fixed before proceeding.

The experiment is partial if:

- flattening works for same-page ranges, but cross-page ordering or invalid
  range handling needs a follow-up.

The experiment fails if:

- flattened chunks have wrong serials or row bounds;
- reversed or invalid pins produce a flattened highlight;
- same-row ranges omit the endpoint row;
- `top_x`/`bot_x` are swapped or normalized incorrectly;
- PageList or highlight internals become visible outside `terminal`;
- tracked highlights, search, renderer, app, public ABI, or selection behavior
  is added prematurely;
- semantic highlight behavior regresses;
- tests or formatting fail.
