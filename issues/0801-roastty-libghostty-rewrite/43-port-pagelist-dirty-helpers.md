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

# Experiment 43: Port PageList Dirty Helpers

## Description

Port the small upstream PageList dirty-helper surface before moving into larger
PageList operations.

Experiment 42 copied page-level dirty state through `clone_region`, but deferred
upstream `PageList clone full dirty` because PageList-level dirty helpers did
not exist yet. Upstream PageList has a narrow test/debug surface for dirty
state:

- `clearDirty`;
- `isDirty`;
- `markDirty`;
- `Pin.isDirty`;
- `Pin.markDirty`.

This experiment should port only that surface and then enable the deferred clone
dirty test. It must not implement the larger operations that also mark rows
dirty, such as erase, resize, split, compact, row/cell/prompt iterators, or
renderer dirty-region delivery.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/PageList.zig` as the source of truth for:
     - `clearDirty` around the PageList test/debug helpers;
     - `isDirty`;
     - `markDirty`;
     - `Pin.isDirty`;
     - `Pin.markDirty`;
     - `PageList clone full dirty`;
     - dirty assertions in erase/resize/split tests only as future context.
   - Do not modify `vendor/ghostty/`.

2. Add private PageList dirty helpers.
   - Add a private or `pub(super)` `PageList::clear_dirty` helper that:
     - clears every page's page-level dirty bit;
     - clears every row's dirty bit for all live rows in every page.
   - Add a private or `pub(super)` `PageList::is_dirty(point::Point) -> bool`
     helper for tests/debugging.
   - Add a private `PageList::mark_dirty(point::Point)` helper for tests, if the
     clone dirty test needs the same shape as upstream.
   - Invalid points should follow the current PageList point convention: return
     `false` or no-op only if that matches an existing local helper pattern;
     otherwise use `Option` internally and keep tests on valid points.

3. Add Pin dirty helpers.
   - Add `Pin::is_dirty(&self, list: &PageList) -> bool` or the smallest
     equivalent helper that fits Roastty's current ownership model.
   - Add `Pin::mark_dirty(&self, list: &mut PageList)` or use PageList-level
     mutation directly if a Pin method would require awkward aliasing.
   - Preserve upstream semantics:
     - a location is dirty if the containing page dirty bit is true;
     - otherwise it is dirty if the containing row dirty bit is true;
     - marking a pin dirty sets the row dirty bit, not the page dirty bit.
   - Do not add public ABI exposure.

4. Keep Page state helpers narrow.
   - Use existing `Page::is_dirty`, `Page::set_dirty`, and `Row::set_dirty`
     behavior.
   - Do not widen Page methods just for tests unless the dirty helper cannot be
     expressed through existing PageList/Page internals.

5. Add tests.
   - Dirty location test:
     - new PageList starts clean;
     - mark an active point dirty;
     - `is_dirty` returns true for that row and false for neighboring rows.
   - Page-level dirty test:
     - set a page's page-level dirty bit;
     - `is_dirty` returns true for a point on that page.
   - Clear dirty test:
     - create a multi-page PageList;
     - set both page-level and row-level dirty bits on at least two distinct
       pages;
     - call `clear_dirty`;
     - all tested points are clean and all page dirty bits on all pages are
       false.
   - Clone full dirty test:
     - port upstream `PageList clone full dirty`;
     - mark active rows 0, 12, and 23 dirty;
     - clone the full screen region;
     - verify cloned rows 0, 12, and 23 remain dirty while neighboring rows are
       clean.

6. Preserve scope.
   - Do not implement:
     - erase;
     - resize/reflow;
     - split;
     - compact;
     - row/cell/prompt iterators;
     - renderer dirty-region plumbing;
     - screen/parser integration;
     - public C ABI additions.
   - Do not add `ghostty` names except when citing upstream paths or test
     provenance.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - dirty helpers implemented;
     - clone dirty test status;
     - scope boundaries preserved;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- PageList can mark a valid point's row dirty;
- PageList can report dirty state from either page-level or row-level dirty
  bits;
- PageList can clear all page-level and row-level dirty bits across multiple
  pages;
- the deferred upstream clone dirty test is ported and passes;
- no erase, resize, split, compact, iterator, renderer, parser, or ABI work is
  introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- an independent agent reviews the experiment design and completed result and
  approves them, or all real findings are fixed.

The experiment is partial if:

- dirty helpers pass, but the clone dirty test exposes a real clone interaction
  that needs a follow-up experiment.

The experiment fails if:

- dirty state is reported from the wrong row or page;
- `clear_dirty` leaves page-level or row-level dirty state behind;
- clone loses or over-expands row-level dirty state;
- the implementation expands into unrelated PageList operations;
- tests or formatting fail.

## Result

**Result:** Pass

Experiment 43 ported the PageList dirty-helper surface needed by the current
PageList test/debug layer:

- private `Pin::is_dirty` and `Pin::mark_dirty` helpers;
- private `PageList::pin_is_dirty`;
- private `PageList::clear_dirty`;
- private `PageList::is_dirty`;
- private `PageList::mark_dirty`.

The implementation preserves upstream semantics:

- a point is dirty if its containing page is dirty;
- otherwise, a point is dirty if its containing row is dirty;
- marking a point dirty sets the row dirty bit, not the page dirty bit;
- clearing dirty state clears page-level dirty bits and every live row dirty bit
  across every page.

Tests added:

- row-level dirty marking and querying;
- page-level dirty querying;
- multi-page `clear_dirty` covering page-level and row-level dirty bits on
  distinct pages;
- upstream `PageList clone full dirty` behavior, now enabled after Experiment
  42's clone implementation.

No erase, resize/reflow, split, compact, iterator, renderer, parser, or ABI work
was introduced.

Verification:

```bash
cargo fmt
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

Observed result:

- `cargo test -p roastty terminal::page_list`: 111 passed;
- `cargo test -p roastty`: 392 unit tests passed, plus 1 ABI harness test
  passed.

Independent review:

- Design review required multi-page `clear_dirty` coverage; the design was
  updated and approved before implementation.
- Result review found no correctness issues and approved Experiment 43 as ready
  to record as `Pass`.

## Conclusion

PageList now has the dirty test/debug helpers required by upstream clone tests.
The deferred clone dirty case from Experiment 42 is now covered, and dirty state
can be marked, queried, and cleared without expanding into larger PageList
mutation operations.

The next experiment can move to another PageList operation with dirty semantics
available as supporting infrastructure.
