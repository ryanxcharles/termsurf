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

# Experiment 600: search ScreenSearch select_next / select_prev

## Description

This experiment ports `selectNext` and `selectPrev` from upstream
`terminal/search/screen.zig` — the methods that step the currently-selected
match forward (newest→oldest) or backward (oldest→newest), tracking the new
selection's pins so it follows the content. They build on the highlight tracking
lifecycle (Experiment 599). They are self-contained relative to the larger
`reloadActive` / `select` cluster (which calls them). It extends
`terminal::search::screen` and adds `SelectedMatch::deinit`.

## Upstream behavior

```zig
fn selectNext(self) !bool {
    var prev = if (self.selected) |*m| m else {
        // No prior selection: pick the most recent (newest) match.
        const hl = if (active_len > 0) active_results[active_len - 1]
                   else if (history_len > 0) history_results[0]
                   else return false;
        const tracked = try hl.untracked().track(self.screen);
        self.selected = .{ .idx = 0, .highlight = tracked };
        return true;
    };
    const next_idx = if (prev.idx + 1 >= active_len + history_len) 0 else prev.idx + 1;
    const hl = if (next_idx < active_len) active_results[active_len - 1 - next_idx]
               else history_results[next_idx - active_len];
    const tracked = try hl.untracked().track(self.screen);
    prev.deinit(self.screen);
    self.selected = .{ .idx = next_idx, .highlight = tracked };
    return true;
}

fn selectPrev(self) !bool {
    var prev = if (self.selected) |*m| m else {
        // No prior selection: pick the oldest match.
        const hl = if (history_len > 0) history_results[history_len - 1]
                   else if (active_len > 0) active_results[0]
                   else return false;
        const tracked = try hl.untracked().track(self.screen);
        self.selected = .{ .idx = active_len + history_len - 1, .highlight = tracked };
        return true;
    };
    const next_idx = if (prev.idx != 0) prev.idx - 1 else active_len - 1 + history_len;
    const hl = if (next_idx < active_len) active_results[active_len - 1 - next_idx]
               else history_results[next_idx - active_len];
    const tracked = try hl.untracked().track(self.screen);
    prev.deinit(self.screen);
    self.selected = .{ .idx = next_idx, .highlight = tracked };
    return true;
}
```

The index counts from the end of the combined newest-to-oldest list (see
Experiment 598's `selected_match`). With no prior selection, `selectNext` picks
index 0 (the newest match) and `selectPrev` picks the last index (the oldest).
Otherwise each steps the index by one with wraparound, looks up the result at
the new index (the `selected_match` indexing), tracks its pins, deinits the
previous selection's tracked pins, and stores the new selection. `return false`
only when there are no matches at all.

## Rust mapping (`roastty/src/terminal/search/screen.rs`)

The result lookup at an index
(`idx < active_len ? active[active_len-1-idx] : history[idx-active_len]`) is
factored into a private `result_at(idx)` helper (the `selected_match` indexing
without the bounds check, since the callers guarantee an in-range index).
`hl.untracked().track(screen)` returns `Option<Tracked>` (Experiment 599); if
`None` (a pin couldn't be tracked) the step returns `false` (it could not
establish a selection). The `self.screen` pointer is dereferenced to
`&mut Screen` under the screen-alive + caller-holds-lock invariant (upstream
documents that `select` requires read/write screen access); the methods stay
safe fns with an internal `unsafe` block, like `prune_history`.

```rust
impl SelectedMatch {
    /// Untrack the selection's pins (upstream `SelectedMatch.deinit`). Takes `self` by value (the
    /// lifecycle style of `Tracked::deinit`; the caller owns the previous selection via `take`).
    fn deinit(self, screen: &mut Screen) {
        self.highlight.deinit(screen);
    }
}

impl ScreenSearch {
    /// The cached result at `idx` (the `selected_match` indexing; the caller guarantees
    /// `idx < active_len + history_len`).
    fn result_at(&self, idx: usize) -> Flattened {
        let active_len = self.active_results.len();
        if idx < active_len {
            self.active_results[active_len - 1 - idx].clone()
        } else {
            self.history_results[idx - active_len].clone()
        }
    }

    /// Select the next match (newest→oldest, wrapping), upstream `selectNext`. `false` only if there
    /// are no matches.
    fn select_next(&mut self) -> bool {
        let active_len = self.active_results.len();
        let history_len = self.history_results.len();
        let total = active_len + history_len;

        let next_idx = match &self.selected {
            None => {
                if total == 0 {
                    return false;
                }
                // The newest match is index 0.
                0
            }
            Some(m) => {
                if m.idx + 1 >= total {
                    0
                } else {
                    m.idx + 1
                }
            }
        };

        self.set_selection(next_idx);
        true
    }

    /// Select the previous match (oldest→newest, wrapping), upstream `selectPrev`.
    fn select_prev(&mut self) -> bool {
        let active_len = self.active_results.len();
        let history_len = self.history_results.len();
        let total = active_len + history_len;

        let next_idx = match &self.selected {
            None => {
                if total == 0 {
                    return false;
                }
                // The oldest match is the last index.
                total - 1
            }
            Some(m) => {
                if m.idx != 0 {
                    m.idx - 1
                } else {
                    total - 1
                }
            }
        };

        self.set_selection(next_idx);
        true
    }

    /// Track the result at `next_idx`, deinit any previous selection, and store the new one. Shared
    /// by `select_next` / `select_prev`; the caller guarantees `next_idx` is in range.
    fn set_selection(&mut self, next_idx: usize) {
        let hl = self.result_at(next_idx);
        // SAFETY: the screen is alive and exclusively accessed (the caller holds the screen lock —
        // upstream's `select` read/write contract).
        let screen = unsafe { self.screen.as_mut() };
        // Track first, so a (non-)failure leaves the previous selection intact. A `None` here is an
        // invariant violation (a valid cached match must have trackable pins), not a "no match" —
        // upstream's `try` propagates the error rather than returning `false`.
        let tracked = hl
            .untracked()
            .track(screen)
            .expect("selected match pins must be trackable");
        if let Some(prev) = self.selected.take() {
            prev.deinit(screen);
        }
        self.selected = Some(SelectedMatch {
            idx: next_idx,
            highlight: tracked,
        });
    }
}
```

The `set_selection` helper unifies the two methods' common tail (the no-prior
and the stepped cases both reduce to "track the result at `next_idx`, deinit the
previous, store"): upstream's no-prior branch is the same as the stepped branch
except it skips the `prev.deinit` (there is no previous) — `set_selection`'s
`if let Some(prev) = self.selected.take()` handles both uniformly. `select_next`
/ `select_prev` return `false` only for the no-match case (handled before
`set_selection`) and `true` otherwise.

## Scope / faithfulness notes

- **Ported**: `selectNext` / `selectPrev` → `select_next` / `select_prev`;
  `SelectedMatch.deinit` → `SelectedMatch::deinit`.
- **Faithful**: the no-prior-selection first pick (newest = index 0 for next,
  oldest = last index for prev); the wrapping step (`+1` wrap to 0 for next,
  `-1` wrap to `total-1` for prev); the result lookup at the new index; tracking
  the new pins, deiniting the previous selection, storing the new one; `false`
  only when there are no matches.
- **Faithful adaptation**: `!bool` → `bool` (the alloc error vanishes; `false`
  means "no matches" only); a failed `track` (`None`) is an invariant violation
  (a valid cached match must have trackable pins) and `expect`s — upstream's
  `try` propagates the error rather than returning `false`, so this keeps
  tracking failure distinct from "no match"; the `try hl.untracked().track` +
  later `prev.deinit` ordering is preserved (track first, then deinit the
  previous, then store — so a track failure leaves the previous selection
  intact); the common tail is unified into `set_selection`;
  `SelectedMatch::deinit` takes `self` by value (the `Tracked::deinit` lifecycle
  style); the `self.screen` deref is an internal `unsafe` `as_mut` under the
  screen-alive + lock invariant (safe fns, like `prune_history`).
- **Deferred**: `select` (the public dispatcher that calls `reloadActive` +
  `pruneHistory` then these), `init` / `reloadActive` (construction), `feed`,
  `searchAll`; plus `ViewportSearch` and the search `Thread`.
- No C ABI/header/ABI-inventory change (internal Rust). Extends
  `terminal::search::screen`.

## Changes

1. `roastty/src/terminal/search/screen.rs`: add `SelectedMatch::deinit`,
   `ScreenSearch::result_at`, `select_next`, `select_prev`, and the
   `set_selection` helper; update the module doc comment.
2. Tests (in `screen.rs`) — these track pins on a **real** `Screen`, so the test
   results carry **valid** pins (a `Flattened` with one chunk on the screen's
   first node, `start: 0` / `end: 1`, `top_x` / `bot_x` `0`, so its
   `untracked()` pins are valid):
   - **first select_next picks the newest**: two trackable active results, no
     prior selection → `select_next` returns `true`, `selected.idx == 0`, and
     the screen's tracked-pin count rises by `2`.
   - **select_next steps and wraps**: with 2 results, successive `select_next`
     calls move `idx` `0 → 1 → 0` (wrap), each keeping the tracked-pin count at
     `2` (the previous selection is deinited).
   - **first select_prev picks the oldest**: no prior selection → `select_prev`
     selects `idx == total - 1`.
   - **select_prev steps and wraps**: `idx` moves
     `(total-1) → … → 0 → (total-1)`.
   - **no matches**: empty results → `select_next` / `select_prev` return
     `false` and leave `selected` `None`.

   Each test drops the `ScreenSearch` before the `Screen` (so the dangling
   tracked pins are never dereferenced), and where a selection remains it is
   cleaned up by the screen's drop.

3. Format and test (`cargo fmt`, accept output).

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

- `select_next` / `select_prev` reproduce upstream (the first pick, the wrapping
  step, the track/deinit/store, the no-match `false`) — faithful to
  `terminal/search/screen.zig`;
- the tests pass (first-next / step-wrap / first-prev / prev-wrap / no-match),
  and the existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the first pick, the wrapping step, the
track/deinit/store ordering, or the no-match handling diverges from upstream, an
unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed the design and **confirmed the rest faithful** (the
`set_selection` factoring is sound — no-prior and stepped selection differ only
in index choice and whether a previous selection exists; the `select_next` /
`select_prev` index math matches upstream including wrapping and the no-prior
newest/oldest choices; keeping the methods safe with an internal
`NonNull<Screen>::as_mut()` is consistent with the earlier `ScreenSearch`
slices, given construction establishes the screen-alive/exclusive-access
invariant). One Required and one Optional, both adopted:

- **Required (adopted)**: do **not** return `false` when `track` fails for an
  in-range match. Upstream's `false` means "no matches"; a tracking failure is
  an error path (`try`), not a no-match result, and in Rust a `None` from
  `track` means an invalid cached result / invariant failure. `set_selection`
  now `expect`s the `track` (preserving the track-first / then-deinit-previous /
  then-store ordering, so the previous selection stays intact and an invariant
  failure is visible rather than conflated with "no matches"); `select_next` /
  `select_prev` return `false` only for the genuine no-match case.
- **Optional (adopted)**: `SelectedMatch::deinit` takes `self` by value
  (matching upstream's lifecycle style and `Tracked::deinit(self, ...)`;
  `self.selected.take()` already yields ownership).

Review artifacts:

- Prompt: `logs/codex-review/20260604-d600-prompt.md`
- Result: `logs/codex-review/20260604-d600-last-message.md`
