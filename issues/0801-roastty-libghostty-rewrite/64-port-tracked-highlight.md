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

# Experiment 64: Port Tracked Highlight

## Description

Port upstream `highlight.Tracked` in a Roastty-shaped way.

Upstream Ghostty stores a tracked highlight as two tracked `PageList.Pin`
pointers. `Tracked.init(screen, start, end)` asks `screen.pages` to track both
pins, unwinds the first pin if tracking the second fails, and `Tracked.deinit`
untracks both pins through the same page list. `Tracked.initAssume(start, end)`
wraps already tracked pin pointers without taking ownership.

Roastty does not have the upstream `Screen` layer yet, but `PageList` already
owns tracked pin allocation, tracked pin storage, reset/remap behavior, and
untracking. Therefore this experiment should keep tracking ownership on
`PageList` and add only the tracked highlight shape plus PageList-owned
construction/destruction helpers.

This experiment must not add selection, search, renderer, app, public ABI,
parser, resize/reflow, terminal screen, or semantic behavior changes.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/highlight.zig` for:
     - `Tracked`;
     - `Tracked.init`;
     - `Tracked.initAssume`;
     - `Tracked.deinit`;
     - `Untracked.track`.
   - Use `roastty/src/terminal/page_list.rs` for existing:
     - `track_pin`;
     - `untrack_pin`;
     - `count_tracked_pins`;
     - `tracked_pins`;
     - tracked pin reset/remap tests;
     - `highlight_pin_order_key` from Experiment 63.
   - Do not modify `vendor/ghostty/`.

2. Add the tracked highlight shape.
   - In `roastty/src/terminal/highlight.rs`, add:

     ```rust
     pub(super) struct Tracked {
         pub(super) start: NonNull<Pin>,
         pub(super) end: NonNull<Pin>,
     }
     ```

   - Derive only traits that are safe for pointer identity, likely `Debug`,
     `Clone`, `Copy`, `PartialEq`, and `Eq`.
   - Add a narrow `Tracked::init_assume(start, end) -> Tracked` helper matching
     upstream `initAssume`.
   - `init_assume` is a non-owning wrapper around pins that are already tracked
     by some caller. A highlight created with `init_assume` must not be passed
     to `PageList::untrack_highlight`, because this experiment did not acquire
     ownership of those tracked pins.
   - Do not add public ABI or app-facing selection types.

3. Add PageList-owned tracked highlight helpers.
   - Add a private helper such as:

     ```rust
     fn track_highlight(
         &mut self,
         highlight: highlight::Untracked,
     ) -> Option<highlight::Tracked>
     ```

   - The helper should:
     - reject invalid pins using the same validity/order basis as Experiment 63;
     - reject garbage pins;
     - reject reversed ranges;
     - track the start pin first;
     - track the end pin second;
     - if end tracking fails, untrack the already tracked start pin before
       returning `None`;
     - return `Some(highlight::Tracked)` only when both tracked pins are owned
       by this `PageList`.
   - Add a private helper such as:

     ```rust
     fn untrack_highlight(&mut self, highlight: highlight::Tracked)
     ```

   - `untrack_highlight` should untrack both pins through
     `PageList::untrack_pin`. It must not dereference the pointers.
   - `untrack_highlight` is only valid for owned tracked highlights returned by
     `PageList::track_highlight`. It must not be used with
     `Tracked::init_assume` values.

4. Preserve ownership boundaries.
   - Do not let `highlight::Tracked` allocate or free pins directly.
   - Do not import a new `Screen` abstraction.
   - Do not make tracked highlight helpers visible outside `terminal`.
   - Do not expose `Pin` fields publicly.
   - Do not alter existing tracked pin behavior except through the new narrow
     helpers.

5. Add tests.
   - Add focused PageList/highlight tests:
     - `Tracked::init_assume` stores the two pointer identities without changing
       PageList tracked pin counts, and the test must not pass that non-owning
       value to `untrack_highlight`;
     - tracking a valid untracked highlight increments PageList tracked pin
       count by exactly two;
     - the tracked pointers appear in `PageList::tracked_pins`;
     - the tracked start/end pointer values dereference to pins equivalent to
       the original untracked start/end pins while tracked;
     - untracking a tracked highlight removes exactly those two tracked pins and
       restores the prior count;
     - invalid start pins return `None` and do not change the count;
     - invalid end pins return `None` and do not leak the already tracked start
       pin;
     - garbage pins return `None` and do not change the count;
     - reversed same-page ranges return `None` and do not change the count;
     - reversed cross-page ranges return `None` and do not change the count.
   - Existing tracked pin, semantic highlight, flattened highlight, and PageList
     integrity tests must continue passing.

6. Keep scope narrow.
   - Do not add selection ownership on `Screen`.
   - Do not add selection string extraction.
   - Do not add search.
   - Do not add renderer or app behavior.
   - Do not add public C ABI.
   - Do not change semantic highlighting.

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
     - tracked highlight shape;
     - PageList tracking/untracking behavior;
     - invalid/reversed/garbage behavior;
     - leak/unwind behavior when end tracking fails;
     - tests added;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `highlight::Tracked` stores start/end tracked pin pointers;
- `Tracked::init_assume` wraps existing pointers without allocating, untracking,
  or changing PageList tracked pin counts;
- `Tracked::init_assume` is documented and tested as non-owning, and
  `untrack_highlight` is constrained to values returned by `track_highlight`;
- `PageList::track_highlight` or equivalent tracks both pins for valid ordered
  untracked highlights;
- tracking a valid highlight increments the tracked pin count by exactly two;
- `PageList::untrack_highlight` or equivalent removes both tracked pins and
  restores the prior count;
- invalid, garbage, and reversed pins return `None`;
- failed tracking does not leak a partially tracked start pin;
- same-page and cross-page reversed ranges are both rejected;
- no selection, search, renderer, app, public ABI, parser, resize/reflow,
  terminal screen, or semantic behavior changes are introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- independent design and result reviews approve the experiment, or all real
  findings are fixed before proceeding.

The experiment is partial if:

- the tracked shape exists and valid tracking works, but invalid/reversed
  cleanup or leak coverage needs a follow-up.

The experiment fails if:

- tracked highlights expose public ABI or app-facing selection behavior;
- tracking leaks pins on failure;
- untracking removes the wrong pins or leaves tracked highlight pins behind;
- invalid, garbage, or reversed highlights become tracked;
- existing tracked pin reset/remap behavior regresses;
- semantic or flattened highlight behavior regresses;
- tests or formatting fail.

## Result

**Result:** Pass

Experiment 64 added the tracked highlight shape and PageList-owned tracking
helpers without introducing Screen, selection, search, renderer, app, public
ABI, parser, resize/reflow, or semantic behavior.

The implementation added `highlight::Tracked` as two tracked pin pointers:
`start: NonNull<Pin>` and `end: NonNull<Pin>`. It also added
`Tracked::init_assume(start, end)`, matching upstream's non-owning wrapper
semantics: it stores pointer identity only and does not allocate, untrack, or
mutate PageList state.

PageList now has private tracked-highlight helpers:

- `track_highlight(Untracked) -> Option<Tracked>` validates both endpoints,
  rejects garbage pins, rejects invalid pins, rejects same-page and cross-page
  reversed ranges, tracks the start pin, tracks the end pin, and unwinds the
  start pin if end tracking fails.
- `untrack_highlight(Tracked)` untracks both pointers through `untrack_pin`
  without dereferencing them. Per the design, it is only for owned tracked
  highlights returned by `track_highlight`, not for non-owning `init_assume`
  values.

The tests added coverage for:

- `Tracked::init_assume` preserving pointer identity without changing tracked
  pin counts;
- valid owned highlight tracking increasing the tracked pin count by exactly
  two;
- tracked pointers appearing in `PageList::tracked_pins`;
- tracked pointer values dereferencing to the original start/end pins while
  still owned by PageList;
- owned highlight untracking removing both tracked pins and restoring the prior
  count;
- invalid start pins returning `None` without changing the count;
- invalid end pins returning `None` without leaking a partially tracked start;
- garbage pins returning `None` without changing the count;
- same-page reversed ranges returning `None`;
- cross-page reversed ranges returning `None`.

Verification:

```bash
cargo fmt
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

Results:

- `cargo fmt` passed.
- `cargo test -p roastty terminal::page_list` passed: 284 tests, 0 failed.
- `cargo test -p roastty` passed: 565 unit tests plus 1 ABI harness test, 0
  failed.

Independent result review approved the experiment with no blocking findings. The
reviewer confirmed the ownership model matches upstream, `init_assume` is
non-owning, `track_highlight` uses the Experiment 63 validity/order basis,
`untrack_highlight` does not dereference pointers, and there was no scope drift.

## Conclusion

Roastty now has the full core highlight shape set from upstream `highlight.zig`:
untracked, tracked, flattened, and PageList-owned construction for tracked and
flattened highlights. The next experiment can move beyond the standalone
highlight shapes and choose the next terminal-core subsystem slice that depends
on them.
