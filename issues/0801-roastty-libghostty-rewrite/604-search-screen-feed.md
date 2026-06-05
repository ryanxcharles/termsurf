+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 604: search ScreenSearch feed / search_all

## Description

This experiment ports the last of `ScreenSearch`'s incremental surface from
upstream `terminal/search/screen.zig`: `feed` (the lock-holding step that pulls
more history into the searcher and handles a screen resize) and `searchAll` (the
convenience driver that ticks/feeds to completion). With these, `ScreenSearch`
is feature-complete except for any embedding glue. All prerequisites are in
place: the `ScreenSearch` state machine — `tick` / `tick_active` /
`tick_history` / `prune_history` (594–600) — the construction/dispatch cluster
`new` / `reload_active` / `select` / `deinit` (603), and `PageListSearch::feed`
(593).

## Upstream behavior

```zig
pub fn searchAll(self: *ScreenSearch) Allocator.Error!void {
    while (true) {
        self.tick() catch |err| switch (err) {
            error.OutOfMemory => return error.OutOfMemory,
            error.FeedRequired => try self.feed(),
            error.SearchComplete => return,
        };
    }
}

pub fn feed(self: *ScreenSearch) Allocator.Error!void {
    // A resize resets the entire search (no result reflow).
    if (self.screen.pages.rows != self.rows or self.screen.pages.cols != self.cols) {
        const new: ScreenSearch = try .init(self.allocator(), self.screen, self.needle());
        self.deinit();
        self.* = new;
        assert(self.screen.pages.rows == self.rows);
        assert(self.screen.pages.cols == self.cols);
    }

    const history: *PageListSearch = if (self.history) |*h| &h.searcher else {
        self.state = .complete;
        return;
    };

    if (!try history.feed()) {
        self.state = .complete;
        self.pruneHistory();   // reclaim scrollback-pruned results
        return;
    }

    switch (self.state) {
        .active, .history => {},
        .history_feed => self.state = .history,
        .complete => unreachable,
    }
}
```

## Rust mapping (`roastty/src/terminal/search/screen.rs`)

Both become `unsafe fn` — they dereference the `screen` pointer (and, in `feed`,
the history searcher's `NonNull<PageList>` into that screen). `tick` stays safe
(no screen access) and already returns the `Tick` enum (`Progressed` /
`FeedRequired` / `Complete`), so `search_all` matches on it instead of catching
errors. `Allocator.Error` is infallible in roastty, so the OOM arm disappears.

```rust
/// Tick and feed until the search is complete (upstream `searchAll`).
///
/// # Safety
/// As `feed` — the caller holds the screen lock and the screen outlives the search.
pub(in crate::terminal) unsafe fn search_all(&mut self) {
    loop {
        match self.tick() {
            Tick::Progressed => {}
            // SAFETY: caller's contract.
            Tick::FeedRequired => unsafe { self.feed() },
            Tick::Complete => return,
        }
    }
}

/// Pull one page of history into the searcher, resetting on a resize (upstream `feed`).
///
/// # Safety
/// The caller holds the screen lock; the screen is live and outlives the search.
pub(in crate::terminal) unsafe fn feed(&mut self) {
    // (1) On a resize we can't reflow cached results, so reset the whole search.
    let (cur_rows, cur_cols) = {
        // SAFETY: screen alive.
        let s = unsafe { self.screen.as_ref() };
        (s.rows(), s.cols())
    };
    if cur_rows != self.rows || cur_cols != self.cols {
        let screen = self.screen;
        let needle = self.needle().to_vec();
        // SAFETY: screen alive. `new` runs `reload_active`, creating fresh tracked pins; the old
        // pins (still tracked) are released by `deinit` immediately below — they briefly coexist,
        // exactly as upstream's `init` then `self.deinit()`.
        let new = unsafe { ScreenSearch::new(screen, &needle) };
        // SAFETY: screen alive; release the old pins before overwriting.
        unsafe { self.deinit() };
        *self = new;
        debug_assert!(self.rows == cur_rows && self.cols == cur_cols);
    }

    // (2) No history searcher → nothing left to feed.
    if self.history.is_none() {
        self.state = State::Complete;
        return;
    }

    // (3) Feed one page. No `&mut Screen` is held across this — `PageListSearch::feed`
    // dereferences its own `NonNull<PageList>` into the same screen (cf. `HistorySearch::deinit`).
    // SAFETY: screen alive; the searcher's list is the live screen's page list.
    let fed = unsafe { self.history.as_mut().unwrap().searcher.feed() };
    if !fed {
        // No more data → complete; reclaim scrollback-pruned history results.
        self.state = State::Complete;
        self.prune_history();
        return;
    }

    // (4) A successful feed while waiting resumes the history search; active/history are unchanged.
    match self.state {
        State::Active | State::History => {}
        State::HistoryFeed => self.state = State::History,
        State::Complete => unreachable!("a complete search's feed returns no data"),
    }
}
```

### Notes / deviations

- **No `&mut Screen` across `searcher.feed()`.** The resize check's `&Screen`
  (step 1) is dropped before step 3, and `prune_history` (step 3's no-data path)
  re-derives its own `&Screen` after the searcher borrow ends. This mirrors the
  Exp 603 completion-review fix to `HistorySearch::deinit`: never hold a
  `&mut Screen` while a searcher dereferences its raw `NonNull<PageList>`.
- **Reinit falls through (does not return).** Upstream's resize branch resets
  the search and then continues into the history-feed logic against the _new_
  state (whose `state` after `init` is `History`); the port keeps that
  fall-through.
- **`Tick::Complete` from `tick`** corresponds to upstream's
  `error.SearchComplete`; `Tick::FeedRequired` to `error.FeedRequired`.

## Verification

- `cargo build -p roastty` — no warnings.
- `cargo test -p roastty` — no regressions; new tests:
  - `search_all_reaches_complete_and_keeps_active_matches` — a single-page
    screen with two needle hits: `new`, `search_all`, then `tick()` is
    `Complete` and `matches_len() == 2`; `deinit` returns the tracked-pin count
    to baseline.
  - `feed_repeatedly_reaches_complete` — `new`, then repeated `feed()` drives
    the state to complete (`tick()` is `Complete`) without losing the active
    matches.
- `cargo fmt -p roastty -- --check` — clean.
- no-ghostty grep on touched source — clean.
- `git diff --check` — clean.

Pass = `search_all` terminates at `Complete` with the active matches intact, and
`feed` advances the state machine and prunes on exhaustion, with no tracked-pin
leaks.

## Design Review

Codex reviewed the design and **APPROVED** it with **no Required findings**. It
confirmed: the resize branch falls through like upstream; `new`-before-`deinit`
briefly coexisting pins matches upstream `init` then `self.deinit()`, with the
old owned Rust fields dropped after explicit pin cleanup; the aliasing plan is
sound (the resize-check `&Screen` is scoped before `PageListSearch::feed`, and
`prune_history` re-borrows after); `search_all` over the `Tick` enum is the
right adaptation of upstream's error-union loop with the OOM arm removed; and
the `Complete`-arm `unreachable!` matches upstream.

- **Optional (partially adopted)**: add a resize-branch test if a practical
  screen-resize test helper exists — the reset path is the most stateful part.
  Recorded as the main residual coverage gap (resizing a `Screen` mid-test is
  involved); the constituent pieces (`new`, `deinit`) are independently tested.
- **Optional (adopted)**: a direct `HistoryFeed` → `History` transition unit.
  Added as a third test (`feed_from_history_feed_resumes_history`).
- **Nit (fixed)**: stale prerequisite experiment numbers in the Description —
  corrected to the `ScreenSearch` state-machine range (594–600).

Review artifacts:

- Prompt: `logs/codex-review/20260605-d604-prompt.md`
- Result: `logs/codex-review/20260605-d604-last-message.md`

## Result

**Result:** Pass

Implemented `search_all` and `feed` in `roastty/src/terminal/search/screen.rs`,
faithfully porting upstream's `searchAll` / `feed`:

- `unsafe fn search_all(&mut self)` — loops on `tick()`: `Progressed` →
  continue, `FeedRequired` → `feed()`, `Complete` → return (the `Tick`-enum form
  of upstream's error-union loop, minus the infallible OOM arm).
- `unsafe fn feed(&mut self)` — resize → reinit (`new` + `deinit` +
  `*self = new`) with fall-through; `history.is_none()` → `Complete`;
  `!searcher.feed()` → `Complete` + `prune_history`; otherwise the state switch
  (`HistoryFeed` → `History`, `active`/`history` unchanged, `Complete`
  unreachable). No `&mut Screen` is held across `PageListSearch::feed` (the
  resize-check `&Screen` is scoped to step 1; `prune_history` re-borrows after).

Three tests added: `search_all_reaches_complete_and_keeps_active_matches`
(single-page drive to `Complete` with matches intact),
`feed_repeatedly_reaches_complete` (single-page exhaustion is idempotent), and
`feed_from_history_feed_resumes_history_search` (a two-page screen exercises the
`HistoryFeed` → `History` resume path where `searcher.feed()` returns data) —
the resize-branch test from the design's first Optional remains the residual
coverage gap. All three confirm `deinit` returns the tracked-pin count to
baseline.

Gates: `cargo fmt --check` clean, `cargo build -p roastty` no warnings,
`cargo test -p roastty` **3310 passed / 0 failed** (3307 → 3310, +3), no-ghostty
grep clean (only the pre-existing `// Upstream Ghostty` comment in `screen.rs`,
untouched), `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **APPROVED** it with **no Required
and no Optional findings**, confirming: `search_all` loops/feeds/returns
correctly; `feed` preserves the resize rebuild-then-fall-through; the no-history
and exhausted-history paths match upstream (`Complete`, plus `prune_history` on
exhaustion); the `HistoryFeed` → `History` transition and the `Complete`
`unreachable!` are correct; the borrow scoping is sound (no `&mut Screen` across
`PageListSearch::feed`); and the reinit pin lifecycle matches upstream with no
double-free. The one Nit (record `## Result` / `## Conclusion`) is addressed by
this section.

Review artifacts:

- Prompt: `logs/codex-review/20260605-r604-prompt.md`
- Result: `logs/codex-review/20260605-r604-last-message.md`

## Conclusion

`ScreenSearch` is now feature-complete: the read accessors, the full `tick` /
`feed` / `search_all` state machine, `prune_history`, the construction/dispatch
cluster, selection stepping, and teardown all port faithfully. The incremental,
lock-aware screen search is done. The remaining search subsystem is
`ViewportSearch` (a thin viewport-scoped wrapper over `ScreenSearch`) and the
search `Thread` (the background driver that owns a `ScreenSearch` and pumps
`tick` / `feed` off the render thread). The next experiment should port
`ViewportSearch`, after which the `Thread` completes the subsystem.
