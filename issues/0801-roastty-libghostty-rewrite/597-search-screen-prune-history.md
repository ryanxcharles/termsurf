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

# Experiment 597: search ScreenSearch prune_history

## Description

This experiment continues `ScreenSearch` (upstream `terminal/search/screen.zig`)
with `pruneHistory` — the cleanup that drops cached history results whose pages
have been pruned from the scrollback (so the search doesn't return matches that
no longer exist). It is a self-contained method (used by `feed` and `select`),
so it can be ported and tested ahead of those. It extends
`terminal::search::screen` and adds `page_serial_min` accessors on `PageList` /
`Screen`.

## Upstream behavior

```zig
fn pruneHistory(self: *ScreenSearch) void {
    // History results are stored newest-to-oldest. Find the first result whose
    // lowest serial is below the screen's minimum live page serial; everything
    // from there on is older and therefore also pruned.
    for (0..self.history_results.items.len) |i| {
        const hl = &self.history_results.items[i];
        const serials = hl.chunks.items(.serial);
        const lowest = serials[0];
        if (lowest < self.screen.pages.page_serial_min) {
            const alloc = self.allocator();
            for (self.history_results.items[i..]) |*prune_hl| prune_hl.deinit(alloc);
            self.history_results.shrinkAndFree(alloc, i);
            return;
        }
    }
}
```

`page_serial_min` is the `PageList`'s minimum serial of a still-live page; when
pages are pruned from the scrollback it rises. A history result whose first
chunk's serial is below that minimum references a page that is gone. Because the
results are stored newest-to-oldest, the first such result marks the boundary —
everything from index `i` onward is truncated.

## Rust mapping (`roastty/src/terminal/search/screen.rs`)

A `Vec::truncate(i)` replaces the per-element `deinit` + `shrinkAndFree`
(roastty's `Flattened` drops its `Vec<Chunk>` cleanly). The
`screen.pages.page_serial_min` read goes through `Screen::page_serial_min`
(which delegates to `PageList::page_serial_min`), dereferencing the screen
pointer under the screen-alive invariant (the same model as `tick_history`).

```rust
impl ScreenSearch {
    /// Drop cached history results whose pages have been pruned from the scrollback (upstream
    /// `pruneHistory`). History results are stored newest-to-oldest, so the first result whose first
    /// chunk's serial is below the screen's minimum live page serial marks the boundary — it and
    /// everything older are truncated.
    fn prune_history(&mut self) {
        // SAFETY: the screen is alive (the construction-time invariant).
        let min = unsafe { self.screen.as_ref() }.page_serial_min();
        for i in 0..self.history_results.len() {
            let first_chunk_serial = self.history_results[i].chunks[0].serial;
            if first_chunk_serial < min {
                self.history_results.truncate(i);
                self.history_results.shrink_to_fit(); // mirror upstream's `shrinkAndFree`
                return;
            }
        }
    }
}
```

Supporting accessors:

```rust
// page_list.rs
impl PageList {
    /// The minimum serial of a still-live page (upstream `page_serial_min`). Rises as pages are
    /// pruned from the scrollback.
    pub(in crate::terminal) fn page_serial_min(&self) -> u64 {
        self.page_serial_min
    }
}

// screen.rs (the Screen module)
impl Screen {
    /// The minimum serial of a still-live page in this screen's page list.
    pub(in crate::terminal) fn page_serial_min(&self) -> u64 {
        self.pages.page_serial_min()
    }
}
```

## Scope / faithfulness notes

- **Ported**: `pruneHistory` → `prune_history`; plus the
  `PageList::page_serial_min` / `Screen::page_serial_min` accessors.
- **Faithful**: iterating the newest-to-oldest history results; the
  `first_chunk_serial = chunks[0].serial` per-result key; the
  `first_chunk_serial < page_serial_min` staleness test; truncating from the
  first stale result to the end.
- **Faithful adaptation**: the per-element `deinit` + `shrinkAndFree(i)` →
  `Vec::truncate(i)` followed by `shrink_to_fit()` (roastty's `Flattened` owns
  its `Vec<Chunk>` and drops cleanly; `shrink_to_fit` mirrors `shrinkAndFree`'s
  capacity release); `chunks.items(.serial)[0]` → `chunks[0].serial`; the
  `screen.pages.page_serial_min` read goes through the new accessors, with the
  screen-pointer deref under the screen-alive invariant (`prune_history` stays a
  safe fn with an internal `unsafe` block, like `tick_history`). The
  `page_serial_min` read is hoisted out of the loop (it is invariant during the
  prune).
- **Deferred**: `init` / `reloadActive` (construction), `feed`, `searchAll`, and
  `select` / `selectNext` / `selectPrev`; plus `ViewportSearch` and the search
  `Thread`.
- No C ABI/header/ABI-inventory change (internal Rust). Extends
  `terminal::search::screen`; adds two accessors (plus a `#[cfg(test)]` setter).

## Changes

1. `roastty/src/terminal/search/screen.rs`: add `ScreenSearch::prune_history`;
   update the module doc comment.
2. `roastty/src/terminal/page_list.rs`: add `PageList::page_serial_min` and a
   `#[cfg(test)]` `set_page_serial_min_for_tests`.
3. `roastty/src/terminal/screen.rs`: add `Screen::page_serial_min` and a
   `#[cfg(test)]` `set_page_serial_min_for_tests` (delegating to the `PageList`
   helper).
4. Tests (in `search/screen.rs`) — build a real `Screen` (so `page_serial_min`
   is readable) with a test-set `page_serial_min`, and history results whose
   first chunk carries a known serial (the `node` is dangling — `prune_history`
   only reads `serial`):
   - **prunes from the first stale result**: history results (newest-to-oldest)
     with serials `[5, 4, 3, 2]` and `page_serial_min == 4` → after
     `prune_history`, `[5, 4]` remain (serial `3 < 4` marks the boundary).
   - **no pruning when all live**: serials `[5, 4]` with `page_serial_min == 0`
     → unchanged.
   - **empty results**: `prune_history` on empty history is a no-op.
5. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty terminal::search
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/terminal/search roastty/src/terminal/page_list.rs roastty/src/terminal/screen.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `prune_history` reproduces upstream (newest-to-oldest scan; the
  `chunks[0].serial < page_serial_min` boundary; truncate-to-end) — faithful to
  `terminal/search/screen.zig`;
- the tests pass (prunes from first stale / no pruning / empty), and the
  existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the staleness test, the boundary, or the truncation
diverges from upstream, an unrelated item changes, or any public C API/ABI
changes.

## Design Review

Codex reviewed the design and **approved it**, confirming the key questions:
hoisting `page_serial_min` out of the loop is faithful (`prune_history` does not
mutate the screen, so it is invariant during the prune); the safe method with an
internal screen dereference is consistent with the construction invariant used
elsewhere; `chunks[0]` is the right faithful assumption (a real match always has
at least one chunk); and the real-`Screen` + dangling-node `Flattened` test
setup is sound because the prune path reads only serials and drops vectors
without dereferencing chunk nodes. One Optional and one Nit, both adopted:

- **Optional (adopted)**: call `shrink_to_fit()` after `truncate(i)` to mirror
  upstream's `shrinkAndFree` (the visible result set is the same either way;
  this also releases the retained capacity).
- **Nit (adopted)**: name the local `first_chunk_serial` (not `lowest`) — the
  expression is specifically `chunks[0].serial`, and the clearer name makes the
  invariant easier to read.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d597-prompt.md`
- Result: `logs/codex-review/20260604-d597-last-message.md`

## Result

**Result:** Pass

`ScreenSearch` gained `prune_history`, which reads the screen's
`page_serial_min` once, scans `history_results` newest-to-oldest, and at the
first result whose first chunk's serial is below that minimum truncates from
there to the end (`truncate(i)` + `shrink_to_fit()`). Supporting
`PageList::page_serial_min` and `Screen::page_serial_min` accessors (plus
`#[cfg(test)]` setters) were added.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3287 passed, 0 failed (three new tests; no
  regressions, up from 3284).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps: `roastty/src/terminal/search` and
  `roastty/src/terminal/page_list.rs` clean; `git diff --check` clean.
- **Scoped no-name-gate exception**: grepping the whole
  `roastty/src/terminal/screen.rs` (a large pre-existing file touched only to
  add the `page_serial_min` accessors) surfaces one **pre-existing** comment
  (`// Upstream Ghostty currently uses this broad range`, present in `HEAD`, not
  in this experiment's diff). This experiment's additions to `screen.rs` contain
  no ghostty names; the pre-existing comment was left untouched per the "no
  unrequested changes to unrelated code" rule. Codex confirmed leaving it was
  the right call.

The three new tests build a real `Screen` (so `page_serial_min` is readable)
with a test-set minimum and history results whose first chunk carries a known
serial (the `node` is dangling — `prune_history` reads only the serial): pruning
from the first stale result (`[5, 4, 3, 2]`, `min == 4` → `[5, 4]`), no pruning
when all live (`[5, 4]`, `min == 0` → unchanged), and the empty no-op.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no Required
or Optional findings** (one Nit: the `## Result` / `## Conclusion` sections were
not yet saved, and to note the scoped no-name-gate exception clearly — both done
here). Codex confirmed the port is faithful: `prune_history` reads the live
minimum serial once, scans newest-to-oldest, uses `chunks[0].serial < min` as
the stale boundary, and truncates from that result through the older tail;
`truncate(i)` correctly drops the owned `Flattened` values and the adopted
`shrink_to_fit()` mirrors upstream's capacity release; the tests are sound (a
real `Screen` for `page_serial_min`, dangling chunk nodes never dereferenced on
this path); and leaving the pre-existing `screen.rs` comment untouched was the
right call under the "no unrequested changes" rule.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r597-prompt.md` (result)
- Result: `logs/codex-review/20260604-r597-last-message.md` (result)

## Conclusion

This experiment ports `prune_history` — the cleanup that drops cached history
results whose scrollback pages have been pruned (keyed on the page serial
falling below the screen's `page_serial_min`). It is a self-contained piece used
by `feed` and `select`. The remaining `ScreenSearch` work is the construction
(`init` / `reload_active` — the trickiest, setting up the `HistorySearch` with
its tracked `start_pin` and handling active-area growth into history), `feed`
(advance the history searcher, reinit on resize, prune on completion), and
`select` / `select_next` / `select_prev` (step the tracked selected match).
After `ScreenSearch`, `ViewportSearch` (`search/viewport.zig`) and the search
`Thread` remain.
