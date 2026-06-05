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

# Experiment 579: split tree nearest / nearestWrapped (spatial navigation)

## Description

This experiment ports `nearest` and `nearestWrapped` from upstream
`datastruct/split_tree.zig` — the spatial navigation that finds the nearest leaf
in a direction over the `Spatial` representation (Experiment 578), using the
`Slot` geometry helpers (Experiment 574) and the `Node<V>` arena
(Experiment 576) to filter to leaves. It is the consumer that ties those three
slices together. The `goto` dispatch (which also needs the deferred in-order
`previous` / `next`) stays deferred; this lands its spatial core. It extends
`terminal::split_tree`.

## Upstream behavior

```zig
fn nearest(self, sp: Spatial, from: Node.Handle, direction: Spatial.Direction,
           target: Spatial.Slot) ?Node.Handle {
    var result: ?struct { handle: Node.Handle, distance: f16 } = null;
    for (sp.slots, 0..) |slot, handle| {
        if (handle == from.idx()) continue;                  // never match ourself
        switch (self.nodes[handle]) { .leaf => {}, .split => continue }  // only leaves
        if (!<slot in `direction` of target>) continue;      // direction test
        const distance = @sqrt(dx*dx + dy*dy);               // distance to target
        if (result) |n| if (distance >= n.distance) continue; // strictly-closer wins (first on tie)
        result = .{ .handle = @enumFromInt(handle), .distance = distance };
    }
    return if (result) |n| n.handle else null;
}

fn nearestWrapped(self, sp, from, direction) ?Node.Handle {
    var target = sp.slots[from.idx()];
    if (self.nearest(sp, from, direction, target)) |v| return v;   // no-wrap result wins
    assert(target.x >= 0 and target.y >= 0);
    assert(target.maxX() <= 1 and target.maxY() <= 1);
    switch (direction) { .left => target.x += 1, .right => target.x -= 1,
                         .up => target.y += 1, .down => target.y -= 1 }
    return self.nearest(sp, from, direction, target);          // retry with the wrapped target
}
```

`nearest` scans every slot, skipping the `from` node and any non-leaf node,
keeping only candidates in the given `direction` of `target`, and returns the
one with the smallest euclidean distance (ties resolved to the first found).
`nearestWrapped` tries `nearest` against `from`'s own slot first; if nothing is
found, it shifts the target by one normalized grid (the wrap) and retries.

## Rust mapping (`roastty/src/terminal/split_tree.rs`)

The direction test, distance, and wrap shift are the `Slot` helpers already
ported in Experiment 574 (`is_in_direction` / `distance_to` / `wrapped_for`).
The slot iteration carries the node index (which equals the slot index — slots
match the arena 1:1) to filter leaves and build the handle.

```rust
impl<V> SplitTree<V> {
    /// The nearest leaf to `target` in `direction`, or `None` if there is none (upstream
    /// `nearest`). Scans the spatial slots, skipping `from` and non-leaf nodes, and returns the
    /// closest in-direction leaf (ties to the first found).
    fn nearest(
        &self,
        sp: &Spatial,
        from: Handle,
        direction: SpatialDirection,
        target: Slot,
    ) -> Option<Handle> {
        let mut result: Option<(Handle, f16)> = None;
        for (handle, &slot) in sp.slots().iter().enumerate() {
            if handle == from.idx() {
                continue; // never match ourself
            }
            if !matches!(self.nodes[handle], Node::Leaf(_)) {
                continue; // only leaves
            }
            if !slot.is_in_direction(target, direction) {
                continue; // must be in the proper direction
            }
            let distance = slot.distance_to(target);
            if let Some((_, best)) = result {
                if distance >= best {
                    continue; // an existing nearest must be strictly closer
                }
            }
            result = Some((Handle::from_index(handle), distance));
        }
        result.map(|(handle, _)| handle)
    }

    /// Like `nearest`, but wraps around the 1×1 grid if there is no in-direction leaf (upstream
    /// `nearestWrapped`).
    fn nearest_wrapped(
        &self,
        sp: &Spatial,
        from: Handle,
        direction: SpatialDirection,
    ) -> Option<Handle> {
        let target = sp.slots()[from.idx()];
        if let Some(v) = self.nearest(sp, from, direction, target) {
            return Some(v);
        }

        // No in-direction leaf: shift the target one full grid (the wrap) and retry. The grid is
        // normalized to 1×1, so this models wrapping without modifying the representation.
        let zero = f16::from_f32(0.0);
        let one = f16::from_f32(1.0);
        assert!(target.x >= zero && target.y >= zero);
        assert!(target.max_x() <= one && target.max_y() <= one);
        let wrapped = target.wrapped_for(direction);
        self.nearest(sp, from, direction, wrapped)
    }
}
```

## Scope / faithfulness notes

- **Ported**: `nearest` / `nearestWrapped` → `SplitTree::nearest` /
  `nearest_wrapped`.
- **Faithful**: the slot scan (skip `from`, skip non-leaf nodes via the arena,
  direction filter, euclidean min-distance with first-on-tie), and
  `nearestWrapped`'s try-then-wrap-then-retry (including the `0 ≤ x,y` /
  `maxX,maxY ≤ 1` bounds assertions) are reproduced exactly. The direction test,
  distance, and wrap reuse the Experiment-574 `Slot` helpers, which already
  match upstream.
- **Faithful adaptation**: the running-minimum struct becomes an
  `Option<(Handle, f16)>`; the `distance >= best` tie rule is preserved
  (strictly-closer replaces, so the first-found wins ties). The slot index
  doubles as the node index (slots match the arena 1:1, as `spatial`
  guarantees).
- **Deferred**: `goto` itself — it dispatches `previous` / `next` /
  `previous_wrapped` / `next_wrapped` (the deferred in-order backtracking
  traversal) and, for `spatial`, builds the `Spatial` and calls
  `nearest_wrapped`; it lands with `previous` / `next`. The tree-shaping ops and
  the formatters remain deferred.
- No C ABI/header/ABI-inventory change (internal Rust). Extends
  `terminal::split_tree`.

## Changes

1. `roastty/src/terminal/split_tree.rs`: add `SplitTree::nearest` and
   `SplitTree::nearest_wrapped` (private; called in-module and, later, by
   `goto`).
2. Tests (in `split_tree.rs`), building a tree, computing its `spatial`, and
   navigating:
   - **horizontal split, no wrap**: from the left leaf, `Right` → the right
     leaf; from the right leaf, `Left` → the left leaf.
   - **no in-direction leaf**: from the left leaf, `Left` → `None` (via
     `nearest` with its own target).
   - **wrap-around**: `nearest_wrapped` from the left leaf, `Left` → the right
     leaf (wraps); from the right leaf, `Right` → the left leaf.
   - **2×2 grid (nested tree)**: from a corner leaf, each of `Left` / `Right` /
     `Up` / `Down` finds the adjacent leaf (and wraps to the far side when there
     is none).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty terminal::split_tree
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/terminal/split_tree.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `nearest` returns the closest in-direction leaf (skipping `from` and splits,
  ties to the first), and `nearest_wrapped` tries-then-wraps-then-retries —
  faithful to `datastruct/split_tree.zig`;
- the tests pass (horizontal nav / no-direction / wrap / grid), and the existing
  tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the slot scan, the direction/distance/tie logic, the
wrap retry, or the leaf filtering diverges from upstream, an unrelated item
changes, or any public C API/ABI changes.

## Design Review

Codex reviewed the design and **approved it with no findings**. It confirmed
`nearest` matches upstream — it scans slots in arena order, skips `from`,
filters to leaves through `self.nodes`, applies the direction predicate, and
preserves the strict-closer rule with `distance >= best` so ties keep the first
candidate (and the `spatial()` invariant makes the slot index equal the node
index, so the handle reconstruction is sound) — and that `nearest_wrapped`
matches too (tries the unwrapped target first, asserts the normalized bounds
only when wrapping, shifts the target with the already-reviewed `wrapped_for`,
then retries against the same spatial representation). Borrowing `sp.slots()`
while reading `self.nodes` is clean (separate immutable data paths), and the
test plan covers no-wrap / no-result / wrap-around / grid navigation.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d579-prompt.md`
- Result: `logs/codex-review/20260604-d579-last-message.md`

## Result

**Result:** Pass

`terminal::split_tree` gained the spatial navigation: `SplitTree::nearest` (scan
the `Spatial` slots by index, skip `from` and non-leaf nodes via the arena,
filter by `is_in_direction`, keep the strictly-closest by `distance_to`,
returning the handle) and `SplitTree::nearest_wrapped` (try `from`'s own slot
first; otherwise assert the normalized bounds, shift the target with
`wrapped_for`, and retry). The module doc comment was updated to mark the
navigation as landed.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3196 passed, 0 failed (three new tests; no
  regressions, up from 3193).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + terminal/split_tree.rs +
  lib.rs/header/abi_harness.c) clean; `git diff --check` clean.

The three new tests: horizontal no-wrap (`Right`/`Left` find the adjacent leaf,
`Left` from the leftmost is `None`), horizontal wrap (`Left` from the leftmost
wraps to the rightmost, and vice versa), and a 2×2 grid (`TL` `Right`→`TR`, `TL`
`Down`→`BL`, `BR` `Left`→`BL`, `BR` `Up`→`TR`, and `TL` `Left` wrapping to the
same row on the right).

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no Required
or Optional findings** (one Nit: the `## Result` / `## Conclusion` sections were
not yet in the saved file — added here). Codex confirmed `nearest` matches
upstream — arena-order scan, skip `from`, leaf-only filtering, the direction
predicate, the `distance >= best` comparison preserving first-on-tie, and handle
reconstruction from the slot index — and that `nearest_wrapped` matches
(unwrapped attempt first, normalized-bounds assertions before wrapping, shift
target, retry), and that the 2×2 expectations are sound.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r579-prompt.md` (result)
- Result: `logs/codex-review/20260604-r579-last-message.md` (result)

## Conclusion

This experiment ports the split_tree spatial navigation — the seventh split_tree
slice — the consumer that **ties together** the `Node<V>` arena (Experiment
576), the `Slot` geometry helpers (Experiment 574), and the `Spatial` container
(Experiment 578): `nearest` scans a `Spatial`'s slots, filters to in-direction
leaves via the arena, and returns the closest; `nearest_wrapped` wraps the
target by one grid when there is no neighbor ahead. With the spatial core
complete, the remaining split_tree work is the in-order `previous` / `next`
backtracking traversal, the `goto` dispatch (which combines `previous` / `next`
with `nearest_wrapped`), the tree-shaping operations (`split` / `remove` /
`equalize` / `resize`), and the formatters. The other remaining big-ticket
subsystem is the terminal **search subsystem** (coupled to `PageList` / `Pin` /
`Screen` / `Selection` / `PageFormatter`); the dependency-blocked helpers
persist (regex/oniguruma for `Link::oniRegex`, a URI parser for `os/uri`, the
config-directory naming decision for `file_load` / `edit` / `loadDefaultFiles`).
