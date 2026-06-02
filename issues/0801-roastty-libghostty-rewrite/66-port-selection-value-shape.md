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

# Experiment 66: Port Selection Value Shape

## Description

Port the value-shape portion of upstream `Selection.zig`.

Upstream `Selection` starts as a small value object: it stores either untracked
start/end pins or tracked start/end pin pointers, plus a `rectangle` flag. It
also exposes basic operations that do not require screen geometry:

- initialize an untracked selection;
- read the unordered start/end pins;
- read mutable start/end pin pointers;
- report whether the selection is tracked;
- compare selections by start, end, and rectangle flag.

The rest of upstream `Selection.zig` depends on `Screen` and PageList coordinate
conversion: ordering, top-left/bottom-right normalization, containment,
contained row extraction, adjustment, word selection, line selection, and
formatting. Those are intentionally out of scope for this experiment.

This experiment should add only the selection value shape and tests. It must not
add Screen, selection ordering, selection containment, adjustment, selection
string extraction, gestures, formatter behavior, C ABI, search, renderer,
parser, app, or terminal mutation behavior.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/Selection.zig` for:
     - `bounds`;
     - `rectangle`;
     - `Bounds`;
     - `init`;
     - `eql`;
     - `startPtr`;
     - `endPtr`;
     - `start`;
     - `end`;
     - `tracked`.
   - Do not port `deinit`, `track`, `topLeft`, `bottomRight`, `order`,
     `ordered`, `contains`, `containedRow`, `containedRowCached`, `Adjustment`,
     or `adjust` in this experiment.
   - Do not modify `vendor/ghostty/`.

2. Add a Roastty module.
   - Add `roastty/src/terminal/selection.rs`.
   - Add it to `roastty/src/terminal/mod.rs` using the same internal module
     style as other terminal modules.
   - Define a terminal-internal selection value:

     ```rust
     pub(super) struct Selection {
         bounds: Bounds,
         rectangle: bool,
     }
     ```

   - Define a terminal-internal bounds enum:

     ```rust
     enum Bounds {
         Untracked { start: Pin, end: Pin },
         Tracked { start: NonNull<Pin>, end: NonNull<Pin> },
     }
     ```

   - Use Roastty/Rust naming:
     - `Selection::new(start, end, rectangle)`;
     - `Selection::tracked(start, end, rectangle)`;
     - `Selection::start`;
     - `Selection::end`;
     - `Selection::start_mut`;
     - `Selection::end_mut`;
     - `Selection::is_tracked`.

3. Preserve upstream semantics.
   - `Selection::new` creates untracked bounds.
   - `Selection::tracked` wraps already tracked pointer bounds without
     allocating or taking ownership. It is a value-shape constructor only, not a
     PageList ownership helper.
   - `start()` and `end()` return the unordered start/end pins. They must not
     normalize, sort, or call PageList coordinate conversion.
   - `start_mut()` and `end_mut()` return mutable references to the underlying
     pins:
     - for untracked selections, references into the enum storage;
     - for tracked selections, references to the tracked pin pointers.
   - `is_tracked()` reflects the active bounds variant.
   - equality compares current `start()`, `end()`, and `rectangle`, matching
     upstream `eql`.
   - If `PartialEq`/`Eq` is implemented, equality must be implemented manually.
     Do not derive equality for `Selection` or `Bounds`, because derived
     equality would compare tracked pointer identity or enum variants instead of
     upstream's dereferenced pin values.

4. Ownership and safety.
   - `Selection::tracked` is non-owning. It must not allocate, untrack, or imply
     ownership of tracked pins.
   - Do not add `Drop` for `Selection`.
   - Do not call `PageList::track_pin`, `PageList::untrack_pin`,
     `PageList::track_highlight`, or `PageList::untrack_highlight` from this
     module.
   - Any unsafe dereference for tracked pin access must be localized and
     documented with a safety comment. Tests must only dereference tracked
     pointers while the owning `PageList` still owns them.

5. Add tests.
   - Add focused tests in `selection.rs` proving:
     - `Selection::new` creates an untracked selection with the original
       unordered start/end pins and rectangle flag;
     - `Selection::tracked` creates a tracked selection that reports tracked
       state and reads the current pointed-to start/end pin values;
     - `Selection::tracked` does not change PageList tracked pin counts;
     - `start_mut()` and `end_mut()` mutate untracked selections in place;
     - `start_mut()` and `end_mut()` mutate tracked pin storage when used on a
       tracked selection;
     - equality includes start, end, and rectangle flag;
     - an untracked selection and a tracked selection with the same dereferenced
       pin values compare equal when their rectangle flag matches;
     - two tracked selections with different tracked pointer identities but the
       same dereferenced start/end pin values compare equal;
     - tracked selections with different dereferenced pin values compare
       unequal;
     - reversed start/end order is preserved and not normalized.
   - Existing highlight, PageList, and selection-codepoint tests must continue
     passing.

6. Keep scope narrow.
   - Do not add `Selection::deinit`.
   - Do not add `Selection::track`.
   - Do not add `Order`, `Adjustment`, `top_left`, `bottom_right`, `ordered`,
     `contains`, contained-row helpers, word selection, line selection,
     selection formatting, selection gestures, C ABI, Screen, search, renderer,
     parser, app, or terminal mutation behavior.
   - Do not expose selection through crate-public API or C ABI.
   - Do not change PageList tracked pin behavior.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::selection
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
     - selection value shape;
     - tracked/untracked behavior;
     - non-ownership behavior;
     - tests added;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `roastty/src/terminal/selection.rs` exists;
- `terminal::selection` is registered in `terminal/mod.rs`;
- `Selection` stores untracked and tracked bounds plus rectangle state;
- `Selection::new` creates untracked selections;
- `Selection::tracked` wraps already tracked pins without allocation,
  untracking, or ownership transfer;
- `start`, `end`, `start_mut`, `end_mut`, `is_tracked`, and equality match the
  upstream value-shape semantics;
- equality is manual/value-based, not derived enum or pointer identity equality;
- reversed start/end order is preserved;
- tests prove untracked access/mutation, tracked access/mutation, equality,
  tracked-vs-untracked equality by pin value, tracked-vs-tracked equality by
  dereferenced pin value rather than pointer identity, rectangle comparison,
  non-ownership, and no normalization;
- no Screen, ordering, containment, adjustment, formatter, selection gesture, C
  ABI, search, renderer, parser, app, PageList tracking behavior, or terminal
  mutation behavior is introduced;
- `cargo fmt`, targeted selection tests, and full `cargo test -p roastty` pass;
- independent design and result reviews approve the experiment, or all real
  findings are fixed before proceeding.

The experiment is partial if:

- the untracked value shape works, but tracked pointer access or mutation needs
  a follow-up.

The experiment fails if:

- selection start/end pins are normalized or sorted;
- tracked selections allocate, untrack, or take ownership;
- equality compares tracked pointer identity or bounds variants instead of
  dereferenced start/end pin values;
- equality ignores the rectangle flag;
- `start_mut`/`end_mut` mutate copies rather than underlying stored pins;
- Screen, ordering, containment, adjustment, formatter, selection gesture, C
  ABI, search, renderer, parser, app, or unrelated behavior is added
  prematurely;
- tests or formatting fail.

## Result

**Result:** Pass

Experiment 66 added `roastty/src/terminal/selection.rs` and registered it in
`roastty/src/terminal/mod.rs`.

The new module ports the value-shape portion of upstream `Selection.zig`:

- `Selection` stores bounds plus a rectangle flag.
- `Bounds` supports untracked start/end pins and tracked start/end pin pointers.
- `Selection::new` creates untracked selections.
- `Selection::tracked` wraps already tracked pin pointers without allocating,
  untracking, taking ownership, or changing PageList tracked pin counts.
- `start()` and `end()` return unordered pin values.
- `start_mut()` and `end_mut()` mutate the stored untracked pins or the pointed
  tracked pin storage.
- `is_tracked()` reports the active bounds variant.
- `rectangle()` exposes the rectangle flag.
- `PartialEq`/`Eq` is implemented manually by comparing `start()`, `end()`, and
  `rectangle`, matching upstream `eql`.

Tracked pointer access is localized to `selection.rs` and documented with safety
comments. `Selection` has no `Drop` and does not call PageList tracking or
untracking helpers.

The tests added coverage for:

- untracked construction preserving unordered bounds and rectangle state;
- tracked construction reading pointed-to bounds;
- untracked `start_mut`/`end_mut` mutating enum storage;
- tracked `start_mut`/`end_mut` mutating pointed-to pin storage;
- equality comparing start, end, and rectangle;
- untracked-vs-tracked equality by dereferenced pin values;
- tracked-vs-tracked equality by dereferenced pin values rather than pointer
  identity;
- tracked selections with different dereferenced pin values comparing unequal;
- reversed start/end order being preserved;
- real PageList tracked pins wrapped by `Selection::tracked` without changing
  tracked pin counts.

Verification:

```bash
cargo fmt
cargo test -p roastty terminal::selection
cargo test -p roastty selection_tracked_wraps_page_list_pins_without_ownership_change
cargo test -p roastty
```

Results:

- `cargo fmt` passed.
- `cargo test -p roastty terminal::selection` passed: 12 tests, 0 failed.
- `cargo test -p roastty selection_tracked_wraps_page_list_pins_without_ownership_change`
  passed: 1 test, 0 failed.
- `cargo test -p roastty` passed: 578 unit tests plus 1 ABI harness test, 0
  failed.

Independent result review approved the experiment with no blocking findings. The
reviewer confirmed the value shape matches upstream, equality is manual and
value-based, tracked selections remain non-owning, unsafe pointer access is
localized, and no Screen, ordering, containment, adjustment, formatting,
selection gesture, C ABI, search, renderer, parser, app, or terminal mutation
behavior was introduced.

## Conclusion

Roastty now has the core selection value object: untracked bounds, non-owning
tracked bounds, rectangle state, unordered accessors, mutable accessors, tracked
state, and upstream-compatible value equality. The next selection experiment can
build on this shape by adding the first PageList/Screen-dependent behavior in a
separate reviewed slice.
