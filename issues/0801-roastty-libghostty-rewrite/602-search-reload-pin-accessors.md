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

# Experiment 602: search reload_active pin accessors (pin_before, active_area_top_left)

## Description

These are the last two supporting accessors `ScreenSearch::reload_active` needs
before it can be ported: pin ordering (upstream `Pin.before`, for the
no-scrollback active-area pruning branch — `tl.before(hl.endPin())`) and the
active area's top-left pin (upstream `screen.pages.getTopLeft(.active)`). Both
underlying operations already exist on `PageList` (`pin_before` and the private
`get_top_left`); this experiment **exposes** them through `Screen` (and a
focused `PageList::active_area_top_left` wrapper over the private
`get_top_left`, so its visibility isn't widened). It extends
`terminal::page_list` and `terminal::screen`.

## Upstream behavior

```zig
// reload_active no-scrollback pruning:
const tl = self.screen.pages.getTopLeft(.active);
for (0.., items) |i, *hl| {
    if (!tl.before(hl.endPin())) { /* prune: not in the active area */ }
    ...
}

// Pin.before (PageList.zig):
pub fn before(self: Pin, other: Pin) bool {
    if (self.node == other.node) {
        if (self.y < other.y) return true;
        if (self.y > other.y) return false;
        return self.x < other.x;
    }
    var node_ = self.node.next;
    while (node_) |node| : (node_ = node.next) { if (node == other.node) return true; }
    return false;
}
```

- `Pin.before(other)` is screen-order comparison: same node → compare `y` then
  `x`; different nodes → `self` is before `other` if `other`'s node is reachable
  forward (newer) from `self`'s.
- `getTopLeft(.active)` is the pin at the top-left cell of the active area.

## Rust mapping

roastty already has `PageList::pin_before(pin, other) -> Option<bool>`
(same-node `y`/`x` compare; different node →
`node_index(pin) < node_index(other)`; `None` for an invalid/garbage pin — the
screen stores pages oldest-to-newest, so the index comparison is equivalent to
upstream's forward node walk), the private `PageList::get_top_left(tag) -> Pin`,
**and** an existing `Screen::pin_before` (`pub(super)`, delegating to
`PageList::pin_before`, reachable from the search module). So `pin_before` is
reused as-is; only the active-top-left entry point is added:

```rust
// page_list.rs
impl PageList {
    /// The pin at the top-left cell of the active area (upstream `getTopLeft(.active)`). A focused
    /// wrapper over the private `get_top_left`, for the search subsystem.
    pub(in crate::terminal) fn active_area_top_left(&self) -> Pin {
        self.get_top_left(point::Tag::Active)
    }
}

// screen.rs
impl Screen {
    /// The pin at the top-left cell of the active area (upstream `pages.getTopLeft(.active)`).
    pub(in crate::terminal) fn active_area_top_left(&self) -> Pin {
        self.pages.active_area_top_left()
    }
    // `Screen::pin_before` already exists (`pub(super)`); the search reuses it.
}
```

## Scope / faithfulness notes

- **Ported (exposed)**: `Pin.before` → reuse the existing `Screen::pin_before`
  (over `PageList::pin_before`); `getTopLeft(.active)` →
  `Screen::active_area_top_left` (over a new focused
  `PageList::active_area_top_left` wrapping the private `get_top_left`).
- **Faithful**: `pin_before`'s same-node `y`/`x` ordering and different-node
  ordering (already a faithful port — the oldest-to-newest page index comparison
  matches upstream's forward node walk); `active_area_top_left` returning the
  active area's top-left pin.
- **Faithful adaptation**: `pin_before` returns `Option<bool>` (`None` for an
  invalid/garbage pin) where upstream's `Pin.before` is infallible (it assumes
  valid pins); the search caller treats `None` as "not orderable" (the active
  pruning skips). `get_top_left` stays private; `active_area_top_left` is the
  focused public entry point reload_active needs (only the `.active` tag).
- **Deferred**: `reload_active` itself (which consumes these), `init`, the
  `select` dispatcher, `feed` / `search_all`; plus `ViewportSearch` and the
  search `Thread`.
- No C ABI/header/ABI-inventory change (internal Rust). Adds one `PageList`
  accessor and one `Screen` accessor (`pin_before` already exists).

## Changes

1. `roastty/src/terminal/page_list.rs`: add `PageList::active_area_top_left`
   (wrapping the private `get_top_left(Active)`).
2. `roastty/src/terminal/screen.rs`: add `Screen::active_area_top_left`.
   (`pin_before` already exists.)
3. Tests:
   - **`pin_before` ordering** (in `screen.rs`): a real `Screen`;
     `node = first node`. Same node: `pin_before((node, 0, 0), (node, 0, 1))` is
     `Some(true)` (`x` order); `pin_before((node, 1, 0), (node, 0, 0))` is
     `Some(false)` (`y` order); equal pins → `Some(false)` (not strictly
     before). An invalid pin (`Pin::test_invalid_for_tests`) → `None`.
   - **cross-node `pin_before`** (in `page_list.rs`, where
     `grow_to_two_pages_for_tests` lives — Codex's design-review Optional): a
     two-page list → `pin_before(first-node pin, last-node pin)` is `Some(true)`
     (older page before newer), and the reverse is `Some(false)`.
   - **`active_area_top_left`** (in `screen.rs`): for
     `Screen::init(10, 10, None)`, `active_area_top_left()` is the first node at
     `(x: 0, y: 0)`, and it is `pin_before` a lower row (`(node, 5, 0)`) →
     `Some(true)`.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty terminal::screen
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/terminal/page_list.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Screen::pin_before` reflects screen-order comparison (same-node `y`/`x`,
  different-node ordering, `None` for invalid pins) and
  `Screen::active_area_top_left` returns the active top-left pin — faithful to
  the `Pin.before` / `getTopLeft(.active)` usage in
  `terminal/search/screen.zig`;
- the tests pass (`pin_before` ordering / `active_area_top_left`), and the
  existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the pin ordering or the active top-left diverges
from upstream, an unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed the design and **confirmed the accessor design sound**:
`PageList::pin_before` is the right Rust equivalent of upstream `Pin.before`
(cross-node ordering needs page-list context); `Option<bool>` is the existing
local invalid/garbage-pin adaptation; and a focused
`PageList::active_area_top_left` wrapper around the private
`get_top_left(Tag::Active)` is the right boundary (reload only needs the active
top-left, not generic tag access). One Required and one Optional, both adopted:

- **Required (adopted)**: do **not** add a duplicate `Screen::pin_before` — it
  already exists (`pub(super)`, delegating to `PageList::pin_before`, reachable
  from `terminal::search::screen`). This experiment reuses it and adds only the
  `active_area_top_left` accessors.
- **Optional (adopted)**: add a cross-node ordering assertion (a two-page list,
  older-page pin `<` newer-page pin) to lock in the cross-page mapping that this
  accessor is exposed for. Added in `page_list.rs` (where the two-page test
  helper lives), exercising the same `PageList::pin_before` that
  `Screen::pin_before` delegates to.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d602-prompt.md`
- Result: `logs/codex-review/20260604-d602-last-message.md`

> Note: the accessor was implemented as `active_area_top_left` (not the plan's
> original `active_top_left`) — `PageList::active_top_left` already exists with
> different semantics (it returns the **viewport** pin). The code blocks above
> use the implemented name.

## Result

**Result:** Pass

`PageList` gained `active_area_top_left` (a focused `pub(in crate::terminal)`
wrapper over the private `get_top_left(Tag::Active)`), and `Screen` gained
`active_area_top_left` (delegating to it). The existing `Screen::pin_before`
(`pub(super)`, over `PageList::pin_before`) is reused unchanged.

One name deviation from the plan, validated by the result review: the accessor
is `active_area_top_left`, not `active_top_left` — `PageList::active_top_left`
already exists and returns the **viewport** pin (`&self.viewport_pin`), a
different meaning, so the new active-area accessor took the distinct, clearer
name. Codex confirmed this is the right call.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3304 passed, 0 failed (three new tests; no
  regressions, up from 3301).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps: font/renderer/config + `page_list.rs` +
  lib.rs/header/abi_harness.c clean; this experiment's `screen.rs` additions are
  clean of ghostty names; `git diff --check` clean. (The pre-existing
  `// Upstream Ghostty` comment in `screen.rs` is unrelated to this diff, left
  untouched.)

The three new tests: same-node `pin_before` ordering (column then row, equal not
strictly before, invalid → `None`); cross-page `pin_before` (older page before
newer, and the reverse); and `active_area_top_left` returning the active origin
`(x: 0, y: 0)` that is `pin_before` a lower row.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no Required
or Optional findings** (one Nit: update the doc's stale `active_top_left`
references and add the `## Result` / `## Conclusion` — both done here). Codex
confirmed the rename is the right call (it avoids colliding with the existing
private viewport-pin `active_top_left` and better describes upstream's
`getTopLeft(.active)`), and that the implementation is faithful
(`PageList::active_area_top_left` wraps `get_top_left(Tag::Active)`,
`Screen::active_area_top_left` delegates, and the existing `Screen::pin_before`
remains the correct wrapper over page-list ordering); the tests cover same-node
ordering, invalid pins, cross-page ordering, and the active-area top-left.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r602-prompt.md` (result)
- Result: `logs/codex-review/20260604-r602-last-message.md` (result)

## Conclusion

This experiment exposes the last two `reload_active` pin prerequisites — the
active-area top-left pin (`active_area_top_left`) and pin ordering (the reused
`Screen::pin_before`). With these and the node/scrollback accessors from
Experiment 601, all of `reload_active`'s supporting infrastructure is in place.
The next slice can port `reload_active` itself (the construction/re-search
core), followed by `init`, the `select` dispatcher, and `feed` / `search_all` —
the remaining `ScreenSearch` cluster — and then `ViewportSearch` and the search
`Thread`.
