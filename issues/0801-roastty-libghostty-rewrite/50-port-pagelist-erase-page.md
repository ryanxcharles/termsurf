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

# Experiment 50: Port PageList Erase Page

## Description

Port the upstream PageList `erasePage` helper.

Experiments 48 and 49 implemented the row-shifting erase primitives. The next
higher-level erase path, `eraseRows`, removes full pages from the front or back
of the list. Upstream delegates that full-page removal to `erasePage`, which has
important side effects: it updates serial invalidation for first-page deletion,
updates tracked pins that pointed into the deleted page, removes the page from
the list, destroys the page backing memory, and updates byte accounting through
the page-destruction path.

This experiment should add the page-deletion helper only. It must not implement
`eraseRows`, `eraseHistory`, `eraseActive`, active regrowth, partial range
erasure, resize/reflow, scrollClear, parser retry loops, renderer delivery, or
public ABI.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/PageList.zig` as the source of truth for:
     - `PageList.erasePage`;
     - `PageList.destroyNode`;
     - `PageList.destroyNodeExt`;
     - full-page deletion behavior used by `eraseRows`.
   - Do not modify `vendor/ghostty/`.

2. Add a Rust `PageList::erase_page` helper.
   - Input should be a `NonNull<Node>` or page index, whichever matches current
     Rust PageList internals most cleanly.
   - The target page must be at the front or back of the PageList, never in the
     middle.
   - The target page must not be the final remaining page.
   - If either invariant is violated, return a narrow error or panic only if
     that matches current internal-helper style. The chosen behavior must be
     explicit in the result.
   - When deleting the first page, update `page_serial_min` to the next page's
     serial, matching upstream's external-reference invalidation model.
   - Update tracked pins that point into the deleted page:
     - move them to the previous page if one exists;
     - otherwise move them to the next page;
     - set `y = 0` and `x = 0`;
     - do not mark them garbage, matching upstream.
   - Remove the page from `self.pages`.
   - Subtract the deleted page's backing length from `page_size`.
   - Do not change `total_rows`; upstream `erasePage` leaves row accounting to
     its caller.
   - Preserve the remaining page order, remaining page serials, `page_serial`,
     `rows`, `cols`, and max-size fields.
   - Do not call full `PageList::verify_integrity` after successful deletion,
     because current Rust integrity checks require the page row sum to match
     `total_rows`, and upstream `erasePage` intentionally leaves `total_rows`
     caller-accounted until `eraseRows` finishes.
   - Instead, add a narrow structural check or direct assertions that prove no
     pins reference the removed page and that remaining page serials, order,
     byte accounting, and pointer identities are valid. Reject/no-mutation paths
     should still remain fully integrity-checkable.

3. Handle viewport pin behavior explicitly.
   - The viewport pin is included in `tracked_pins`, so if it points into the
     deleted page it should move with the same tracked-pin rules.
   - If this creates a pinned viewport whose cached offset is now invalid, clear
     `viewport_pin_row_offset`.
   - Do not call `fixup_viewport`; upstream `eraseRows` performs viewport fixup
     after it knows how many rows were erased.
   - Add tests proving viewport pin movement and cache invalidation.

4. Preserve Rust memory safety.
   - Removing a `Box<Node>` from `Vec<Box<Node>>` drops the Node allocation; all
     pins into that Node must be remapped before removal.
   - Remaining `NonNull<Node>` pointers should stay valid because moving
     `Box<Node>` values in the Vec does not move the boxed Nodes themselves.
   - Tests should cover existing pins into pages before and after the removed
     page so pointer stability is exercised.

5. Add tests.
   - Delete first page:
     - create a multi-page list;
     - record first and second page serials and page sizes;
     - track a pin in the first page and a pin in the second page;
     - delete the first page;
     - verify `page_serial_min` becomes the former second page's serial;
     - verify the first-page pin moves to the new first page at `(0, 0)`;
     - verify the second-page pin remains pointed at the same Node allocation;
     - verify `page_size` decreases by the deleted page backing length;
     - verify `total_rows` is unchanged by the helper.
     - verify no tracked pin or viewport pin still references the removed page;
     - verify full `verify_integrity` would pass after the test temporarily
       subtracts the deleted page's row count from `total_rows`, proving the
       remaining structure is sound.
   - Delete last page:
     - create a multi-page list;
     - track a pin in the last page and a pin in the previous page;
     - delete the last page;
     - verify the last-page pin moves to the previous page at `(0, 0)`;
     - verify `page_serial_min` is unchanged;
     - verify `page_size` decreases and `total_rows` is unchanged.
     - verify no tracked pin or viewport pin still references the removed page;
     - verify full `verify_integrity` would pass after the test temporarily
       subtracts the deleted page's row count from `total_rows`, proving the
       remaining structure is sound.
   - Reject middle page:
     - create at least three pages;
     - attempt to delete the middle page;
     - verify the helper refuses and the list remains unchanged.
   - Reject only page:
     - create a one-page list;
     - attempt to delete the only page;
     - verify the helper refuses and the list remains unchanged.
   - Viewport pin:
     - set viewport to `Pin` with a cached offset pointing into the page being
       deleted;
     - delete that page;
     - verify the viewport pin moves like any tracked pin and
       `viewport_pin_row_offset` is cleared.
   - Accounting/order:
     - verify remaining page order, remaining serials, `page_serial`, `rows`,
       `cols`, max-size fields, and `total_rows` are preserved.

6. Preserve scope.
   - Do not implement:
     - `eraseRows`;
     - `eraseHistory` or `eraseActive`;
     - partial range erasure;
     - active regrowth;
     - resize/reflow;
     - scrollClear;
     - row/cell/prompt iterators;
     - parser retry loops;
     - renderer or app integration;
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
     - page deletion behavior implemented;
     - front/back deletion behavior;
     - rejection behavior;
     - tracked-pin and viewport-pin behavior;
     - accounting and serial behavior;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- front-page deletion removes the page, updates `page_serial_min`, remaps pins
  from the deleted page, decreases `page_size`, preserves remaining page order,
  and leaves `total_rows` unchanged;
- last-page deletion removes the page, remaps pins from the deleted page,
  decreases `page_size`, preserves `page_serial_min`, and leaves `total_rows`
  unchanged;
- middle-page deletion is rejected without mutation;
- only-page deletion is rejected without mutation;
- viewport pins into deleted pages are remapped and cached viewport offsets are
  invalidated;
- pins into remaining pages stay valid;
- remaining page serials, `page_serial`, `rows`, `cols`, max-size fields, and
  `total_rows` are preserved;
- no range erase, history/active erase, active regrowth, resize/reflow,
  scrollClear, iterator, parser, renderer, app, or ABI work is introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- an independent agent reviews the experiment design and completed result and
  approves them, or all real findings are fixed.

The experiment is partial if:

- front/back page removal works, but viewport pin/cache interactions expose a
  missing viewport primitive that should be designed separately.

The experiment fails if:

- deleting a page leaves dangling pins;
- deleting a page changes `total_rows`;
- first-page deletion fails to update `page_serial_min`;
- middle or only-page deletion mutates the list;
- remaining page order, serials, or pointer identity are corrupted;
- the implementation expands into range erase, history/active erase, active
  regrowth, parser, renderer, app, or ABI work;
- tests or formatting fail.

## Result

**Result:** Pass

`PageList::erase_page` is implemented as an internal helper for front/back page
deletion. It accepts a `NonNull<Node>` target and returns a narrow
`ErasePageError` for invalid, middle-page, or only-page requests. Middle-page
and only-page rejection paths leave the list unchanged and remain fully
integrity-checkable.

Front-page deletion removes the first page, updates `page_serial_min` to the
former second page serial, remaps pins that pointed into the removed page to the
replacement page at `(0, 0)`, subtracts the removed backing length from
`page_size`, and preserves `total_rows` for the future caller to account.
Last-page deletion removes the last page, remaps removed-page pins to the
previous page at `(0, 0)`, preserves `page_serial_min`, subtracts the removed
backing length from `page_size`, and likewise leaves `total_rows` unchanged.

The viewport pin is covered by the same tracked-pin remapping path. If the
viewport is pinned, `viewport_pin_row_offset` is cleared because page removal
invalidates the cached absolute offset. Successful deletion intentionally does
not run full `verify_integrity` inside the helper because current integrity
checks require `total_rows` to already match the remaining page row sum, while
upstream `erasePage` leaves row accounting to `eraseRows`. Tests temporarily
apply the caller's expected row accounting to prove the remaining structure is
sound after deletion.

Verification:

- `cargo fmt -- roastty/src/terminal/page_list.rs`
- `cargo test -p roastty terminal::page_list` — 174 PageList tests passed, ABI
  harness filtered out
- `cargo test -p roastty` — 455 unit tests passed, ABI harness passed, doc-tests
  passed

Independent result review approved the experiment as a Pass with no required
findings. The review specifically accepted the caller-accounted `total_rows`
behavior, adjusted integrity proof, pin remapping before dropping the removed
`Box<Node>`, viewport cache invalidation, front/back/reject behavior,
`page_serial_min`/`page_size` accounting, and the clean scope boundary.

## Conclusion

Experiment 50 completed the full-page deletion primitive needed by the future
`eraseRows` port. The helper now has the same important side effects as
upstream's `erasePage` without expanding into range erase or public terminal
behavior. The next experiment can build on this by designing the higher-level
row-range erase path that decides when to erase whole pages, when to shift rows,
and when to perform caller row accounting and viewport fixup.
