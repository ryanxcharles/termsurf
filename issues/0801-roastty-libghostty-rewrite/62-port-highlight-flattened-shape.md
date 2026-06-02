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

# Experiment 62: Port Highlight Flattened Shape

## Description

Port the data shape and local methods for upstream `highlight.Flattened`.

Experiment 61 created the `terminal/highlight.rs` module and moved
`highlight.Untracked` there. Upstream consumers show that flattened highlights
are the next structural dependency for search and renderer paths:

- `terminal/render.zig` consumes `[]const highlight.Flattened`;
- `terminal/search/*` produces and stores `highlight.Flattened`;
- selected search results convert `Flattened` back through `untracked().track`.

Tracked highlights depend on Screen-level tracked-pin lifecycle that Roastty has
not ported yet. Flattened highlights are therefore the better next data-model
slice: they establish the serial-stamped page-chunk representation without
adding search, renderer, public API, or tracked-pin behavior.

This experiment should not add the upstream `Flattened.init` page-iterator
constructor yet. In Roastty, `Pin` does not own a PageList iterator the way
upstream Zig pins do, so the constructor needs a PageList-owned design in a
later experiment. This experiment only ports the reusable flattened value shape
and methods that can be implemented locally.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/highlight.zig` for:
     - `Flattened`;
     - nested `Flattened.Chunk`;
     - `empty`;
     - `clone`;
     - `startPin`;
     - `endPin`;
     - `untracked`.
   - Use `vendor/ghostty/src/terminal/search/*` and
     `vendor/ghostty/src/terminal/render.zig` only to confirm why Flattened is
     the next dependency.
   - Do not modify `vendor/ghostty/`.

2. Add `highlight::Flattened`.
   - In `roastty/src/terminal/highlight.rs`, add:

     ```rust
     #[derive(Debug, Clone, PartialEq, Eq)]
     pub(super) struct Flattened {
         pub(super) chunks: Vec<Chunk>,
         pub(super) top_x: CellCountInt,
         pub(super) bot_x: CellCountInt,
     }
     ```

   - Add nested-equivalent `highlight::Chunk`:

     ```rust
     #[derive(Debug, Clone, Copy, PartialEq, Eq)]
     pub(super) struct Chunk {
         pub(super) node: NonNull<Node>,
         pub(super) serial: u64,
         pub(super) start: CellCountInt,
         pub(super) end: CellCountInt,
     }
     ```

   - Use `super::page_list::{Node, Pin}` and `super::size::CellCountInt`.
   - Use `std::ptr::NonNull`.
   - Keep the types `pub(super)`, not public outside `terminal`.

3. Add local methods.
   - Add `Flattened::empty() -> Self` or `Default` matching upstream empty
     values:
     - no chunks;
     - `top_x = 0`;
     - `bot_x = 0`.
   - Add `start_pin(&self) -> Pin`, matching upstream `startPin`.
   - Add `end_pin(&self) -> Pin`, matching upstream `endPin`.
   - Add `untracked(&self) -> Untracked`, matching upstream `untracked`.
   - Preserve upstream's non-empty precondition for `startPin`, `endPin`, and
     `untracked`:
     - do not silently return a dummy pin for an empty flattened highlight;
     - panic with a clear message if called while `chunks` is empty.
   - When constructing `Pin`, set `garbage = false`.
   - Do not implement upstream `Flattened.init` in this experiment.
   - Do not add allocator-specific `deinit`; Rust `Vec` owns the chunks.

4. Adjust visibility narrowly.
   - `highlight.rs` needs to name `Node` and construct non-garbage pins from
     flattened chunk coordinates.
   - Widen only the minimum PageList internals needed within `terminal`:
     - likely `Node` as `pub(super)`;
     - a narrow `Pin` constructor, such as
       `pub(super) fn new(node: NonNull<Node>, y: CellCountInt, x: CellCountInt) -> Self`,
       that always sets `garbage = false`.
   - Keep `Pin` fields private to `page_list.rs`; do not make `node`, `y`, `x`,
     or `garbage` public to sibling modules.
   - Do not expose these outside the `terminal` module.
   - Do not make `PageList`, `Node`, `Pin`, or highlight types crate-public
     unless a compile error proves sibling-module visibility is insufficient.

5. Add tests.
   - Add focused tests, preferably in `page_list.rs` where valid `Pin` and page
     nodes are easy to obtain from real PageList fixtures:
     - `Flattened::empty()` has no chunks and zero x bounds;
     - `start_pin` returns the first chunk's node/start row and `top_x`;
     - `end_pin` returns the last chunk's node, `end - 1` row, and `bot_x`;
     - `untracked` returns the same pins as `start_pin`/`end_pin`;
     - `Clone` preserves chunks and x bounds;
     - empty `start_pin`, `end_pin`, or `untracked` panics with the intended
       precondition message.
   - Use real PageList nodes and split pages where useful so cross-page chunks
     are tested without inventing fake pointers.
   - Existing semantic highlight tests must continue passing.

6. Keep scope narrow.
   - Do not add `Flattened::init` yet.
   - Do not add tracked highlights.
   - Do not add search, renderer, parser, app, public ABI, selection, resize, or
     reflow behavior.
   - Do not change semantic highlight behavior.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Independent review.
   - Before implementation, get an independent agent review of this experiment
     design.
   - After implementation and verification, get an independent result review.
   - Fix all real findings before proceeding.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - Flattened/Chunk fields and methods added;
     - visibility changes;
     - explicit note that `Flattened::init` is deferred;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `highlight::Flattened` exists with chunks, `top_x`, and `bot_x`;
- `highlight::Chunk` stores node pointer, serial, start row, and exclusive end
  row;
- `Flattened::empty` or `Default` matches upstream empty values;
- `start_pin`, `end_pin`, and `untracked` match upstream semantics for nonempty
  flattened highlights;
- empty `start_pin`, `end_pin`, and `untracked` do not silently fabricate pins;
- `Clone` preserves chunks and x bounds;
- visibility is widened only inside `terminal`;
- `Pin` fields remain private and `highlight.rs` constructs pins only through a
  narrow constructor that sets `garbage = false`;
- semantic highlight behavior remains unchanged;
- `Flattened::init`, tracked highlights, search, renderer, parser, app, public
  ABI, selection, resize/reflow, and diagram behavior remain deferred;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- independent design and result reviews approve the experiment, or all real
  findings are fixed before proceeding.

The experiment is partial if:

- the data shape compiles and tests pass, but a clean local method requires one
  follow-up visibility cleanup before `Flattened::init` can be designed.

The experiment fails if:

- flattened highlight methods return incorrect start/end pins;
- empty flattened highlights silently produce dummy pins;
- `Pin`, `Node`, or highlight types become visible outside `terminal`;
- `Pin` fields are made public to sibling modules instead of using a narrow
  constructor;
- semantic highlight behavior regresses;
- `Flattened::init`, tracked highlights, search, renderer, app, public ABI, or
  selection behavior is added prematurely;
- tests or formatting fail.

## Result

**Result:** Pass

Experiment 62 ported the local data shape for upstream `highlight.Flattened`.

Code changes:

- added `highlight::Flattened` with `chunks`, `top_x`, and `bot_x`;
- added `highlight::Chunk` with node pointer, serial, start row, and exclusive
  end row;
- added `Default` and `Flattened::empty`;
- added `start_pin`, `end_pin`, and `untracked`;
- added a clear non-empty precondition panic for `start_pin`, `end_pin`, and
  `untracked`;
- added `Pin::new(node, y, x)`, which sets `garbage = false`;
- widened `Node` only to `pub(super)` so `highlight::Chunk` can name it;
- kept `Pin` fields private.

The experiment deliberately did not add `Flattened::init`, tracked highlights,
search, renderer, parser, app, public ABI, selection, resize/reflow, diagram, or
semantic behavior changes.

Tests added:

- empty flattened highlight has no chunks and zero x bounds;
- `start_pin`, `end_pin`, and `untracked` produce expected screen points across
  split pages;
- cloned flattened highlights preserve chunks and x bounds;
- empty `start_pin`, `end_pin`, and `untracked` panic with the intended
  precondition message.

Verification:

```bash
cargo fmt
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

Results:

- `cargo test -p roastty terminal::page_list`: 269 passed, 0 failed.
- `cargo test -p roastty`: 550 unit tests passed, ABI harness 1 passed,
  doctests 0.

Independent result review approved the experiment as Pass with no required
findings. The reviewer confirmed that `Flattened` and `Chunk` match upstream's
shape, that local methods preserve upstream start/end semantics for this slice,
that empty calls do not fabricate dummy pins, and that visibility stayed narrow:
`Node` is only `pub(super)`, `Pin` fields remain private, and `Pin::new` is the
only constructor exposed to sibling terminal modules.

## Conclusion

Roastty now has the untracked and flattened highlight value shapes needed by
future search and renderer work. The next likely highlight experiment should add
a PageList-owned `Flattened::init` equivalent, because Roastty's `Pin` does not
own page iteration the way upstream Zig pins do.
