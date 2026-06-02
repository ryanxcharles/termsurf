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

# Experiment 71: Port Selection Tracking Ownership

## Description

Port the ownership behavior of upstream `Selection.track` and `Selection.deinit`
from `Selection.zig`.

Roastty already has the selection value shape and non-owning tracked selection
wrapper, but it does not yet have the PageList-owned operation that converts an
untracked selection into a tracked selection or the corresponding explicit
untrack operation. Upstream stores tracked selection bounds as pointers to
PageList-owned tracked pins and `deinit` releases those pins. Roastty should
mirror that ownership at the `PageList` layer, where tracked pin allocation and
release already live.

This experiment must not add `Drop`-based automatic cleanup, public C ABI,
Screen, selection formatting, gestures, word/line selection, search, renderer,
parser, app, or unrelated terminal mutation behavior.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/Selection.zig` for:
     - `track`;
     - `deinit`;
     - `tracked`.
   - Use existing Roastty code:
     - `roastty/src/terminal/selection.rs`;
     - `roastty/src/terminal/page_list.rs::track_pin`;
     - `roastty/src/terminal/page_list.rs::untrack_pin`;
     - `roastty/src/terminal/page_list.rs::track_highlight`;
     - `roastty/src/terminal/page_list.rs::untrack_highlight`.
   - Do not modify `vendor/ghostty/`.

2. Add PageList-owned selection tracking.
   - In `roastty/src/terminal/page_list.rs`, add private helpers such as:

     ```rust
     fn track_selection(
         &mut self,
         selection: selection::Selection,
     ) -> Option<selection::Selection>

     fn untrack_selection(&mut self, selection: selection::Selection)
     ```

   - `track_selection` should:
     - return `None` if `selection.is_tracked()` is true, matching upstream's
       "must be untracked" precondition without panicking in this private Rust
       helper;
     - return `None` if either untracked endpoint is invalid, missing, garbage,
       or cannot be tracked;
     - check `pin.garbage` explicitly before calling `track_pin`, because
       `track_pin` validates node membership but does not itself reject garbage
       pins;
     - allocate tracked start and end pins with `track_pin`;
     - if tracking the end pin fails after tracking the start pin, untrack the
       start pin before returning `None`;
     - return
       `selection::Selection::tracked(start_ptr, end_ptr, selection.rectangle())`
       on success;
     - preserve the original start/end values and rectangle flag.
   - `untrack_selection` should:
     - untrack both PageList-owned pins for tracked selections;
     - no-op for untracked selections, matching upstream `deinit`;
     - not mutate the PageList if the selection is untracked.
   - Ownership contract:
     - only pass tracked selections returned by `PageList::track_selection` to
       `PageList::untrack_selection`;
     - direct `Selection::tracked(...)` values remain non-owning wrappers around
       caller-provided tracked pins and must not be passed to
       `untrack_selection`;
     - this mirrors the existing tracked-highlight ownership pattern, where the
       PageList helper owns the tracked pins it creates.

3. Preserve the current value type shape.
   - Do not add ownership to `selection::Selection` itself.
   - Do not implement `Drop` for `Selection`; cleanup remains explicit because
     the PageList owns tracked pin storage.
   - Do not expose tracked pointer fields publicly.
   - It is acceptable to add small `selection.rs` private/internal accessors if
     needed to let `PageList` identify tracked pointer storage without unsafe
     pattern matching outside the module.

4. Add tests.
   - Add focused tests proving:
     - tracking an untracked selection allocates exactly two tracked pins;
     - the returned selection is tracked;
     - the returned selection reads the same start/end values and rectangle flag
       as the original;
     - `untrack_selection` releases both tracked pins;
     - `untrack_selection` on an untracked selection is a no-op;
     - tracking a selection that is already tracked returns `None` and does not
       allocate or leak additional tracked pins;
     - invalid start returns `None` without allocating or leaking;
     - invalid end rolls back the already tracked start and returns `None`;
     - missing-node start returns `None` without allocating or leaking;
     - missing-node end rolls back the already tracked start and returns `None`;
     - garbage start or end returns `None` without allocating or leaking;
     - `untrack_selection` is tested only with selections returned by
       `track_selection`, not with manually constructed non-owning
       `Selection::tracked(...)` wrappers;
     - a tracked selection remains connected to PageList-owned tracked pin
       storage across an existing PageList mutation that updates tracked pins;
       after the mutation, `selection.start()` and `selection.end()` should read
       the updated pin values through the stored tracked-pin pointers;
     - selection tracking does not impose ordering: reversed selections can be
       tracked because upstream `Selection.track` tracks stored endpoints as-is.
   - Existing selection ordering, containment, contained-row, adjustment,
     highlight, PageList, and full Roastty tests must continue passing.

5. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list::tests::selection
     cargo test -p roastty terminal::selection
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

6. Independent review.
   - Before implementation, get an independent agent review of this experiment
     design.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get an independent result review.
   - Fix all real findings before proceeding.

7. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - tracking helper behavior;
     - untracking helper behavior;
     - rollback behavior;
     - already-tracked behavior;
     - invalid/garbage behavior;
     - tests added;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- PageList can convert an untracked selection into a tracked selection;
- tracking preserves start, end, and rectangle values;
- tracking allocates exactly two PageList-owned tracked pins;
- untracking a tracked selection releases both tracked pins;
- untracking an untracked selection is a no-op;
- `untrack_selection` is documented and tested as valid only for tracked
  selections returned by `PageList::track_selection`; direct non-owning
  `Selection::tracked(...)` wrappers are outside its ownership contract;
- tracking an already tracked selection returns `None` without allocating;
- invalid, missing-node, or garbage endpoints return `None` without leaks;
- end-tracking failure rolls back start-tracking allocation;
- returned tracked selections remain connected to PageList-owned tracked pin
  storage across PageList mutation;
- reversed stored endpoints can be tracked as-is;
- no `Drop` cleanup, public C ABI, Screen, formatting, gestures, word/line
  selection, search, renderer, parser, app, or unrelated terminal mutation
  behavior is introduced;
- `cargo fmt`, targeted selection/PageList tests, and full
  `cargo test -p roastty` pass;
- independent design and result reviews approve the experiment, or all real
  findings are fixed before proceeding.

The experiment is partial if:

- tracking works, but explicit untracking or rollback behavior needs a
  follow-up.

The experiment fails if:

- tracked selections do not use PageList-owned tracked pins;
- tracking leaks pins on invalid input or partial failure;
- untracking an untracked selection mutates PageList state;
- the design or implementation treats manually constructed non-owning
  `Selection::tracked(...)` wrappers as PageList-owned selections;
- reversed stored endpoints are rejected solely because of order;
- ownership is moved into `Selection` with `Drop` cleanup prematurely;
- public ABI, Screen, formatting, gestures, word/line selection, search,
  renderer, parser, app, or unrelated behavior is added prematurely;
- tests or formatting fail.

## Design Review

Codex reviewed the initial design and required three changes before
implementation:

- record the design-review outcome inside this experiment file;
- add a test proving a returned tracked selection remains connected to
  PageList-owned tracked pin storage across PageList mutation;
- state that `track_selection` must reject garbage pins explicitly instead of
  relying on `track_pin`.

Those changes are incorporated above. A follow-up Codex review must approve this
updated design before implementation begins.

Follow-up Codex review approved the updated design for implementation. No
remaining blockers were found.

## Result

**Result:** Pass

Implemented PageList-owned selection tracking for Roastty:

- added `PageList::track_selection`;
- added `PageList::untrack_selection`;
- added an internal `Selection::tracked_pins` accessor so PageList can release
  tracked selection pins without exposing pointer storage publicly;
- kept `Selection::tracked(...)` as a non-owning wrapper;
- kept ownership and cleanup at the PageList layer;
- did not add `Drop`, public C ABI, Screen integration, formatting, gestures,
  word/line selection, search, renderer, parser, app, or unrelated terminal
  behavior.

Behavior covered by tests:

- tracking an untracked selection allocates exactly two PageList-owned tracked
  pins;
- returned tracked selections preserve start, end, and rectangle values;
- `untrack_selection` releases both PageList-owned pins for selections returned
  by `track_selection`;
- untracking an untracked selection is a no-op;
- already tracked input returns `None` without allocation or leaks;
- invalid start returns `None` without leaks;
- invalid end rolls back the already tracked start;
- missing-node start returns `None` without leaks;
- missing-node end rolls back the already tracked start;
- garbage start or end returns `None` before calling `track_pin`;
- reversed stored endpoints are tracked as-is;
- duplicate start/end endpoints create two distinct tracked pins;
- tracked selections remain connected to PageList-owned tracked pin storage
  across `PageList::split`.

Verification:

```bash
cargo fmt
cargo test -p roastty terminal::page_list::tests::selection
cargo test -p roastty page_list_track_selection
cargo test -p roastty terminal::selection
cargo test -p roastty
cargo test -p roastty track_selection
```

Results:

- `cargo test -p roastty terminal::page_list::tests::selection`: 32 passed. This
  historical filter does not include the new `page_list_track_selection_*`
  tests.
- `cargo test -p roastty page_list_track_selection`: 10 passed.
- `cargo test -p roastty terminal::selection`: 12 passed.
- `cargo test -p roastty`: 620 unit tests passed, ABI harness passed, doctests
  passed.
- `cargo test -p roastty track_selection`: 11 passed, covering all new
  tracking/untracking tests.

Codex reviewed the implementation and found no code blockers. The only review
requirements were to record this result, update the README status to `Pass`, and
include a verification command that actually ran the new tracking tests. Those
requirements are reflected here.

## Conclusion

Experiment 71 completes the upstream `Selection.track` / `Selection.deinit`
ownership slice at the PageList layer. Roastty now has the same explicit
tracked-selection lifecycle shape as Ghostty: untracked bounds can be converted
into PageList-owned tracked pins, those tracked pins follow PageList mutation,
and explicit untracking releases them.

The next experiment should continue from the remaining upstream selection or
terminal behavior not yet ported, rather than revisiting selection tracking
ownership.
