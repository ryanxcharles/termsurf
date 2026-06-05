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

# Experiment 603: search ScreenSearch construction cluster (new / reload_active / select)

## Description

This experiment ports `ScreenSearch`'s construction/dispatch core from upstream
`terminal/search/screen.zig`: `init` (construct + initial `reloadActive`),
`reloadActive` (the re-search engine — re-copy the active area, grow the history
search, and keep the selection valid), and the public `select` dispatcher. These
three are **mutually recursive** (`reloadActive` calls `select(.prev/.next)` to
recover a lost selection; `select` calls `reloadActive`), so they land together.
All prerequisites are now in place: `ActiveSearch` (592), `PageListSearch`
(593), the matcher (587–591), the highlight tracking lifecycle (599),
`select_next` / `select_prev` / `prune_history` / `tick_active` /
`selected_match` (596–600), and the node/scrollback/pin accessors (601–602).
`reloadActive` is decomposed into private helpers (a faithful refactoring, no
behavior change). It completes the incremental, lock-free part of `ScreenSearch`
except `feed` / `search_all`.

## Upstream behavior

```zig
pub fn init(alloc, screen, needle) !ScreenSearch {
    var result: ScreenSearch = .{ .screen=screen, .rows=screen.pages.rows, .cols=screen.pages.cols,
        .active = try .init(alloc, needle), .history=null, .state=.active,
        .active_results=.empty, .history_results=.empty };
    try result.reloadActive();
    return result;
}

pub fn select(self, to: Select) !bool {
    try self.reloadActive();
    self.pruneHistory();
    return switch (to) { .next => try self.selectNext(), .prev => try self.selectPrev() };
}

pub fn reloadActive(self) !void {
    // (A) Selection-garbage recovery: if the selection's pins went garbage, drop it and
    //     (on function exit) re-select the last match.
    const select_prev = ...; defer if (select_prev) _ = self.select(.prev) catch ...;

    // (B) Active update + history growth.
    const list = &self.screen.pages;
    if (try self.active.update(list)) |history_node| history: {
        if (self.screen.no_scrollback) { assert(self.history == null); break :history; }
        // Reset history if its start pin went garbage.
        const history_ = if (self.history) |*h| (if (h.start_pin.garbage) { h.deinit(screen);
            self.history=null; clear history_results; null } else h) else null;
        const history = history_ orelse {  // no history yet → create one
            var search = try PageListSearch.init(alloc, self.needle(), list, history_node);
            const pin = try list.trackPin(.{ .node = history_node });
            self.history = .{ .searcher=search, .start_pin=pin };
            break :history;
        };
        if (history.start_pin.node == history_node) break :history;  // unchanged
        // Grown: re-search [start_pin.node .. history_node] forward, collect (skipping matches that
        //   start on history_node — those are the active area), reverse, prepend to history_results,
        //   advance start_pin.node to history_node, and shift any history-area selection by added_len.
        ...
    } else {
        // No history node → no history. Clear it and move a history-area selection to the active end.
        if (self.history) |*h| { h.deinit(screen); self.history=null; clear history_results; }
        if (self.selected) |*m| if (m.idx >= active_len) { m.deinit(screen); self.selected=null; _=self.select(.prev) ...; };
    }

    // (C) Re-run the active search.
    const old_active_len = self.active_results.items.len;
    const old_selection_idx = if (self.selected) |m| m.idx else null;
    // (errdefer: on failure, reset a now-stale active-area selection via select(.next))
    reset active_results; switch state { .active => tickActive(), else => { keep state; tickActive(); } }

    // (D) No-scrollback active pruning: drop active results not actually in the active area.
    if (self.screen.no_scrollback and active_results.len > 0) { const tl = pages.getTopLeft(.active);
        for (items) |i,*hl| { if (!tl.before(hl.endPin())) { hl.deinit; continue; } prune 0..i; break; } else clear; }

    // (E) Selection fixup: re-resolve the selection's index against the new active results.
    if (old_selection_idx and self.selected) |old_idx, *m| {
        if (old_idx >= old_active_len) { m.idx = m.idx - old_active_len + active_results.len; }
        else { find hl in active_results with m.highlight.{start,end}.eql(hl.untracked().{start,end});
               if found at i: m.idx = active_results.len - 1 - i; else { m.deinit; self.selected=null; _=self.select(.next); } }
    }
}
```

## Rust mapping (`roastty/src/terminal/search/screen.rs`)

`init` becomes an `unsafe fn new` (it stores the `NonNull<Screen>` and
immediately re-derives `&mut Screen` through it). `reload_active` is decomposed
into the orchestrator plus three private helpers — a faithful refactoring of
upstream's labeled blocks:

- `reload_history(&mut self, history_node: Option<NonNull<Node>>)` — phase (B).
- `prune_no_scrollback_active(&mut self)` — phase (D).
- `fixup_selection(&mut self, old_active_len, old_selection_idx)` — phase (E).

The mutual recursion (`reload_active` → `select` → `reload_active`) is just two
methods calling each other (allowed in Rust). The `errdefer`/`defer` recovery
paths (Zig) become explicit calls at the right points (Rust has no `errdefer`;
since allocation is infallible here, the error-recovery `errdefer` of phase (C)
is dropped, and the garbage-recovery `defer` of phase (A) becomes an explicit
`select(Prev)` after the body). Owned `Flattened` is pushed directly (no
re-clone); the `screen` pointer is dereferenced `unsafe` under the
screen-alive + lock invariant. `Pin.before` / `getTopLeft(.active)` use
`Screen::pin_before` / `active_area_top_left` (602); `node.next` uses
`next_node_ptr` (601).

```rust
impl ScreenSearch {
    /// Construct a screen search for `needle` and load the initial active area (upstream `init`).
    ///
    /// # Safety
    /// `screen` must be live and outlive the search; the caller holds the screen lock (no concurrent
    /// access). The search stores the pointer and dereferences it on `reload_active` / `select`.
    pub(in crate::terminal) unsafe fn new(screen: NonNull<Screen>, needle: &[u8]) -> ScreenSearch {
        // SAFETY: caller's contract — `screen` is live.
        let (rows, cols) = { let s = unsafe { screen.as_ref() }; (s.rows(), s.cols()) };
        let mut result = ScreenSearch {
            screen, active: ActiveSearch::new(needle), history: None, state: State::Active,
            selected: None, history_results: Vec::new(), active_results: Vec::new(), rows, cols,
        };
        // SAFETY: see above.
        unsafe { result.reload_active() };
        result
    }

    /// Select the next/previous match, after re-validating the active area and pruning stale history
    /// (upstream `select`). `# Safety`: as `new`.
    pub(in crate::terminal) unsafe fn select(&mut self, to: Select) -> bool {
        unsafe { self.reload_active() };
        self.prune_history();
        match to { Select::Next => self.select_next(), Select::Prev => self.select_prev() }
    }

    /// Re-copy the active area, grow the history search, and keep the selection valid (upstream
    /// `reloadActive`). `# Safety`: as `new`.
    pub(in crate::terminal) unsafe fn reload_active(&mut self) { /* phases A–E, see helpers */ }

    fn reload_history(&mut self, history_node: Option<NonNull<Node>>) { /* phase B */ }
    fn prune_no_scrollback_active(&mut self) { /* phase D */ }
    fn fixup_selection(&mut self, old_active_len: usize, old_selection_idx: Option<usize>) { /* E */ }

    /// Untrack the selection and history pins and drop the searchers (upstream `deinit`). Explicit
    /// (not `Drop`) — it dereferences the `screen` pointer.
    ///
    /// # Safety: as `new`. Call once, before the backing `Screen` is dropped.
    pub(in crate::terminal) unsafe fn deinit(&mut self) {
        // SAFETY: caller's contract — `screen` is live.
        let screen = unsafe { self.screen.as_mut() };
        if let Some(m) = self.selected.take() { m.deinit(screen); }
        if let Some(h) = self.history.take() { h.deinit(screen); }
        // `active` / result vecs drop on their own.
    }
}

impl HistorySearch {
    /// Untrack the start pin and drop the page-list searcher (upstream `HistorySearch.deinit`).
    fn deinit(self, screen: &mut Screen) {
        // SAFETY: the searcher and pin belong to the alive screen.
        let mut searcher = self.searcher;
        unsafe { searcher.deinit() };
        screen.untrack_pin(self.start_pin);
    }
}
```

The history `start_pin` is tracked at `Pin::new(history_node, 0, 0)` (mirroring
upstream's `.{ .node = history_node }` zero-defaults). `Screen::rows` / `cols`
accessors (delegating to the page list) are added for `new`'s dimension capture
(none currently exist).

The subtle pieces, made explicit:

- **History growth re-search** (phase B): a forward `SlidingWindow` over the
  page range `(prior start_pin.node ..= history_node)`, walked with
  `next_node_ptr`, collecting `window.next()` results that do **not** start on
  `history_node` (those belong to the active area), reversed, then prepended to
  `history_results`; `start_pin` advanced to `history_node`; and any
  history-area selection's `idx += added_len`.
- **No-scrollback active prune** (phase D): keep only results at/after the
  active top-left (`Screen::active_area_top_left` `pin_before` the result's end
  pin) — results are sorted, so the first in-active-area result marks the
  boundary and everything before it is dropped; none in-area → clear.
- **Selection fixup** (phase E): a history-area selection
  (`old_idx >= old_active_len`) shifts by the active-length delta
  (`idx - old_active_len + active_results.len`); an active-area selection is
  re-found by comparing the selection's **current tracked pin values**
  (`Screen::tracked_pin_value` of `m.highlight.start` / `end`, since the tracked
  pins may have moved) against each candidate's `hl.untracked()` start/end
  (`idx = active_results.len - 1 - i`); if either tracked pin is
  missing/garbage, or no candidate matches, the selection is dropped +
  `select(Next)` (the not-found path).

## Scope / faithfulness notes

- **Ported**: `init` → `new`; `reloadActive` → `reload_active` (+ the
  `reload_history` / `prune_no_scrollback_active` / `fixup_selection` helpers);
  `select` → `select`.
- **Faithful**: the construction; the selection-garbage recovery; the active
  update + history create/reset/grow with the re-search and selection shift; the
  active re-search reset + `tick_active` (preserving non-active state); the
  no-scrollback active prune; and the selection fixup (index shift / re-find /
  drop-and-reselect).
- **Faithful adaptation**: `init`/`select`/`reload_active` are `unsafe fn`
  (deref the stored `NonNull<Screen>`, under the screen-alive + lock contract —
  the `PageListSearch` pointer model); the `errdefer`/`defer` recovery becomes
  explicit calls (no `errdefer`; the OOM-only phase-(C) `errdefer` is dropped as
  allocation is infallible); owned `Flattened` is pushed without re-cloning; the
  monolithic `reloadActive` is split into helpers (no behavior change);
  `node.next` / `Pin.before` / `getTopLeft(.active)` use the new accessors.
- **Deferred**: `feed` / `search_all` (the next slice; `feed` reinits via `new`
  on resize, advances the history search, and prunes on completion); plus
  `ViewportSearch` and the search `Thread`.
- No C ABI/header/ABI-inventory change (internal Rust). Extends
  `terminal::search::screen`; may add small `Screen` accessors (`rows` / `cols`)
  if not already present.

## Changes

1. `roastty/src/terminal/search/screen.rs`: add `ScreenSearch::new`,
   `reload_active`, `reload_history`, `prune_no_scrollback_active`,
   `fixup_selection`, `select`, `ScreenSearch::deinit`, and
   `HistorySearch::deinit`; update the module doc comment.
2. `roastty/src/terminal/screen.rs`: add `Screen::rows` / `Screen::cols`
   accessors (delegating to the page list — none currently exist; needed for
   `new`'s dimension capture and `feed`'s resize check). Adds `PageList::cols`
   if absent (`active_rows` already gives rows).
3. Tests (in `screen.rs`) — over a real `Screen` (so the active update and pin
   tracking work):
   - **`new` loads the active area**: a screen with two `"Fizz"` lines and
     `ScreenSearch::new(screen, b"Fizz")` (single page, no history) →
     `matches_len() == 2` after construction (the initial `reload_active` ran
     `tick_active`); `deinit` afterward (so tracked pins are released).
   - **`select(Next)` / `select(Prev)`**: after `new`, `select(Next)` selects a
     match (`selected_match()` is `Some`), and repeated `select(Next)` /
     `select(Prev)` step through with wraparound (asserting `selected_match`'s
     `top_x`/row); `select` on a no-match screen returns `false`.
   - **`reload_active` is idempotent on a static screen**: calling it again does
     not change `matches_len()` and keeps a selection valid (and the tracked-pin
     count stays stable).
   - **history-growth helper test** (Codex's design-review Optional): a manually
     seeded two-page `ScreenSearch` with a tracked `HistorySearch.start_pin`
     pointing at an older node and content on both pages → `reload_active`'s
     `reload_history` re-searches the grown range, prepending the older page's
     matches to `history_results` and advancing `start_pin` to the new
     active-covering node. (Asserts `history_results` grew and the start node
     moved.)
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty terminal::search
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/terminal/search && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `new` / `reload_active` / `select` reproduce upstream (construction; the
  active update + history grow + selection recovery; the dispatcher) — faithful
  to `terminal/search/screen.zig`;
- the tests pass (new-loads-active / select-step / idempotent-reload), and the
  existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the construction, the history grow re-search, the
no-scrollback prune, the selection fixup, or the dispatch diverges from
upstream, an unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed the design and **confirmed the prioritized questions**: the
helper decomposition is fine (probably preferable); the deferred `select(Prev)`
recursion is faithful and bounded (the outer reload clears/deinits the garbage
selection before setting the flag, so the recursive reload does not re-hit the
garbage-selection branch — depth ~2); dropping the phase-C `errdefer` is
faithful (the Rust active-result rebuild is infallible; the phase-E fixup
remains); and the history-growth algorithm matches upstream, including deduping
matches whose first chunk starts on the new active-covering history node.
**Three Required findings** and one Optional and one Nit, all adopted:

- **Required (adopted)**: add the dimension accessors — `Screen::rows` / `cols`
  (delegating to the page list; none exist) — so `new` can initialize the `rows`
  / `cols` fields (and `feed` can compare them on resize).
- **Required (adopted)**: phase E must compare the selection's **current tracked
  pin values**, not pointer identity — upstream `eql`s the dereferenced `*Pin`
  values. roastty reads them via
  `Screen::tracked_pin_value(m.highlight.start / end)` before comparing to each
  candidate's `untracked()` start/end; a missing/garbage tracked pin takes the
  not-found path (drop selection + `select(Next)`). (Corrected the fixup
  design.)
- **Required (adopted)**: add the lifecycle helpers this slice needs —
  `HistorySearch::deinit(self, screen)` (untrack `start_pin` +
  `PageListSearch::deinit`, used by `reload_active`'s history-reset paths) and
  `ScreenSearch::deinit` (untrack the selection + history pins; since `new`
  creates tracked pins it needs a paired top-level cleanup API).
- **Optional (adopted)**: add a direct helper-level history-growth test (a
  manually-seeded two-page `ScreenSearch` with a tracked `start_pin`), since the
  forward re-search / skip-on-history-node / reverse / prepend is the riskiest
  part and doesn't need a full terminal scenario.
- **Nit (adopted)**: the history `start_pin` is tracked at
  `Pin::new(history_node, 0, 0)` (mirroring upstream's
  `.{ .node = history_node }` zero-defaults).

Review artifacts:

- Prompt: `logs/codex-review/20260604-d603-prompt.md`
- Result: `logs/codex-review/20260604-d603-last-message.md`
