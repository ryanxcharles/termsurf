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

# Experiment 34: Port PageList Points

## Description

Port the first PageList coordinate conversion layer:

- `getTopLeft`
- `getBottomRight`
- `pin`
- `pointFromPin`
- basic `Pin` row movement needed by those conversions

Experiment 33 gave Roastty an initialized PageList with stable node and viewport
pin handles. This experiment should make PageList able to translate between
tagged terminal points and stable page pins for the currently initialized page
set. That is the next dependency for selection, viewport, rendering, and later
scrolling work.

This experiment should not implement tracked pin creation/removal beyond the
existing viewport pin, scrolling, grow/prune, reset, erase, resize, split,
iterators, selection, highlighting, or screen/parser integration.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/PageList.zig` as the source of truth for:
     - `pin`;
     - `pointFromPin`;
     - `getTopLeft`;
     - `getBottomRight`;
     - `Pin.down`;
     - `Pin.up`;
     - `Pin.downOverflow`;
     - `Pin.upOverflow`.
   - Use upstream tests for guidance:
     - `PageList pointFromPin active no history`;
     - the initialized-state parts of later point/pin tests.
   - Do not modify `vendor/ghostty/`.

2. Add PageList node traversal helpers for the Rust storage shape.
   - Keep the current `Vec<Box<Node>>` storage from Experiment 33.
   - Add helpers to map a `NonNull<Node>` back to its index in `pages`.
   - Add helpers to get first and last node pointers.
   - Add helpers for neighboring nodes by index rather than adding intrusive
     `prev` / `next` links in this experiment.
   - Document that this is faithful for immutable initialized lists; later
     mutation experiments may replace or extend this with explicit links if
     split/erase/grow need cheaper mutation.

3. Add `Pin` movement basics.
   - Add `Pin::down`, `Pin::up`, or equivalent PageList methods that can move a
     pin across initialized pages.
   - Implement enough movement for `pin`, `point_from_pin`, `get_top_left`, and
     `get_bottom_right`.
   - Preserve upstream overflow semantics internally:
     - moving past the last page returns `None`;
     - moving before the first page returns `None`;
     - movement within a page preserves x.
   - Do not implement left/right wrapping, iterators, dirty marking, cell
     access, style lookup, or grapheme lookup in this experiment.

4. Port `get_top_left`.
   - Add a method equivalent to upstream `getTopLeft(tag)`.
   - Semantics:
     - `screen` and `history` top-left are the first page at x/y 0;
     - `viewport` delegates to active/top/pin viewport state;
     - `active` walks backward from the last page by `self.rows` to find the
       active area's top-left.
   - Because scrolling is out of scope, `Viewport::Pin` should only use the
     already-owned viewport pin if tests construct that state directly.

5. Port `get_bottom_right`.
   - Add a method equivalent to upstream `getBottomRight(tag)`.
   - Semantics:
     - `screen` and `active` return the last active cell of the last page;
     - `viewport` starts at viewport top-left and moves down `rows - 1`;
     - `history` returns `None` when there is no history before active.
   - For this slice, test the no-history case and the initialized
     single/multi-page active case. Defer scrollback/grow cases.

6. Port `pin`.
   - Add `PageList::pin(point::Point) -> Option<Pin>`.
   - Reject points whose x coordinate is greater than or equal to `self.cols`.
   - Start from the tagged top-left, move down by the point's y coordinate, set
     x, and return `None` if movement is out of bounds.
   - Returned pins are untracked and only valid until PageList mutation,
     matching upstream.

7. Port `point_from_pin`.
   - Add `PageList::point_from_pin(point::Tag, Pin) -> Option<point::Point>`.
   - Start at the tag's top-left and traverse pages until the pin's node is
     found.
   - Match upstream's forward-traversal semantics exactly:
     - return `None` if the pin is before the tag top-left on the same node;
     - return `None` if the pin's node is not reachable by walking forward from
       the tag top-left;
     - do not additionally enforce a bottom bound with `get_bottom_right`.
   - Preserve the non-obvious initialized-list history behavior from upstream:
     `history` top-left is the first page, so
     `point_from_pin(Tag::History, first_page_pin)` can produce a history point
     even when `get_bottom_right(Tag::History)` returns `None` because no
     history exists before active.
   - Build the correct `point::Point` variant for the requested tag.

8. Add tests.
   - Port `PageList pointFromPin active no history`:
     - first cell converts to `Point::active((0, 0))`;
     - a same-page cell such as x=4/y=2 converts to the matching active point.
   - Add Rust tests for:
     - `pin(Point::active(...))` returning the expected first-page pin;
     - out-of-bounds x returning `None`;
     - out-of-bounds y returning `None`;
     - `pin(Point::viewport(...))` and `point_from_pin(Tag::Viewport, ...)`
       preserving viewport tags when the viewport is still active;
     - `pin(Point::history((0, 0)))` and
       `point_from_pin(Tag::History, first_page_pin)` preserving upstream's
       initialized no-history traversal semantics;
     - `get_top_left(Tag::Active)` on a multi-page initialized list returning
       the first active row, which may be inside the first page or a later page
       depending on active rows;
     - `point_from_pin(Tag::Screen, pin_on_second_page)` producing accumulated y
       across pages;
     - `get_bottom_right(Tag::History)` returning `None` when no history exists;
     - `get_bottom_right(Tag::Active)` returning the last page's last active
       cell.

9. Preserve scope.
   - Do not implement:
     - `trackPin` / `untrackPin` for arbitrary pins;
     - scrolling or viewport offset caches;
     - scrollbar;
     - reset, grow, resize, split, compact, erase, clone, or iterators;
     - cell reads, style reads, selection, or highlighting;
     - screen/parser integration;
     - public C ABI additions.
   - Do not add `ghostty` names except when citing upstream paths or test
     provenance.

10. Verify.
    - Run:

      ```bash
      cargo fmt
      cargo test -p roastty terminal::page_list
      cargo test -p roastty
      ```

    - `cargo fmt` output must be accepted as-is.

11. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - node traversal shape chosen;
      - point/pin APIs added;
      - tests added;
      - verification command output summary.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `get_top_left`, `get_bottom_right`, `pin`, and `point_from_pin` work for
  initialized single-page and multi-page PageLists;
- point conversion preserves the active/viewport/screen/history tag;
- out-of-bounds x/y point lookups return `None`;
- no arbitrary tracked-pin lifecycle, scroll, resize, grow, reset, erase,
  screen/parser behavior, or public ABI is introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- an independent agent reviews the experiment design and completed result and
  approves them, or all real findings are fixed.

The experiment is partial if:

- initialized-list point conversion works, but a later mutable-node experiment
  needs to replace the node traversal helper with explicit links;
- one history/viewport assertion is deferred because real scrollback is not in
  scope yet.

The experiment fails if:

- active top-left is computed from the first page unconditionally instead of
  walking backward from the active area;
- `point_from_pin` rejects pins reachable by upstream's forward traversal from
  the requested tag top-left, or accepts pins before that top-left;
- `pin` returns pins for out-of-bounds coordinates;
- the implementation expands into scrolling, arbitrary tracked pins, resize,
  grow, erase, screen/parser behavior, or public ABI;
- tests or formatting fail.

## Result

**Result:** Pass

Implemented PageList point/pin conversion in
`roastty/src/terminal/page_list.rs`.

The node traversal shape stays aligned with the Rust storage chosen in
Experiment 33:

- pages remain `Vec<Box<Node>>`;
- pins keep stable `NonNull<Node>` handles;
- traversal maps a node pointer back to its vector index when walking across
  pages;
- no intrusive `prev` / `next` links were added in this slice.

Added PageList methods for:

- first/last node lookup;
- node-index lookup from a `NonNull<Node>`;
- `get_top_left`;
- `get_bottom_right`;
- `pin`;
- `point_from_pin`;
- `pin_down`;
- `pin_up`.

The implementation preserves upstream `pointFromPin` traversal semantics:
conversion starts at the requested tag's top-left and walks forward until the
pin's node is found. It rejects pins before that top-left or on unreachable
nodes, and it does not add a bottom-bound check with `get_bottom_right`. This
keeps upstream's initialized no-history behavior where `history` top-left is the
first page even though `get_bottom_right(history)` is `None`.

Added tests for:

- upstream `PageList pointFromPin active no history`;
- `pin(Point::active(...))`;
- out-of-bounds x and y returning `None`;
- viewport point/pin conversion preserving viewport tags;
- history point/pin conversion preserving upstream initialized no-history
  traversal semantics;
- active top-left on a multi-page initialized list;
- screen point conversion accumulating rows across pages;
- active bottom-right returning the last active cell;
- rejecting a pin before the active top-left.

Verification passed:

```bash
cargo fmt
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

The targeted PageList suite reported 26 passing tests. The full `roastty` suite
reported 307 unit tests, the ABI harness, and doc tests passing.

## Conclusion

Roastty PageList can now translate initialized terminal points into page pins
and page pins back into tagged points across active, viewport, screen, and
history coordinate spaces. This is the first read-only coordinate layer needed
before viewport scrolling, rendering iteration, selection, and later mutable
PageList operations.
