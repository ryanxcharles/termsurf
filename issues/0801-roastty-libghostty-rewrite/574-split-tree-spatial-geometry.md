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

# Experiment 574: split tree spatial geometry (SpatialDirection, slot direction/distance/wrap)

## Description

This experiment ports the **pure `f16` geometry core** of upstream
`datastruct/split_tree.zig`'s spatial navigation (`nearest` / `nearestWrapped`):
the `Spatial.Direction` enum, the "is this slot in a given direction of the
target" predicate, the euclidean distance between two slots, and the wrap-around
target shift. These are self-contained over `Slot`s (Experiment 573) and do not
need the `Node` arena — only the arena-dependent leaf filtering and slot
iteration of `nearest` itself stay deferred. It extends `terminal::split_tree`.

## Upstream behavior

In `nearest(sp, from, direction, target)`, for each candidate `slot` upstream:

1. **direction test** — keep the candidate only if it lies in `direction`
   relative to `target`:
   ```zig
   switch (direction) {
       .left  => slot.maxX() <= target.x,
       .right => slot.x >= target.maxX(),
       .up    => slot.maxY() <= target.y,
       .down  => slot.y >= target.maxY(),
   }
   ```
2. **distance** — euclidean distance from the candidate to the target, used to
   pick the nearest:
   ```zig
   const dx = slot.x - target.x;
   const dy = slot.y - target.y;
   const distance = @sqrt(dx * dx + dy * dy);
   ```

In `nearestWrapped`, when no in-direction candidate exists, the target is
shifted by one full normalized grid in the opposite-of-travel sense and
`nearest` is retried (the grid itself is not modified):

```zig
switch (direction) {
    .left  => target.x += 1,
    .right => target.x -= 1,
    .up    => target.y += 1,
    .down  => target.y -= 1,
}
```

`Spatial.Direction` is `enum { left, right, down, up }` — a _separate_ type from
`Split.Direction` (same variants), used specifically for this visual/spatial
navigation.

## Rust mapping (`roastty/src/terminal/split_tree.rs`)

A separate `SpatialDirection` enum (mirroring upstream's separate
`Spatial.Direction`), plus three `Slot` methods over `half::f16`.

```rust
/// A spatial navigation direction — the nearest surface visually in this direction (upstream
/// `Spatial.Direction`; a separate type from `Direction`, with the same variants).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpatialDirection {
    Left,
    Right,
    Down,
    Up,
}

impl Slot {
    /// Whether `self` (a candidate slot) lies in `direction` relative to `target` (upstream
    /// `nearest`'s direction switch).
    pub(crate) fn is_in_direction(self, target: Slot, direction: SpatialDirection) -> bool {
        match direction {
            SpatialDirection::Left => self.max_x() <= target.x,
            SpatialDirection::Right => self.x >= target.max_x(),
            SpatialDirection::Up => self.max_y() <= target.y,
            SpatialDirection::Down => self.y >= target.max_y(),
        }
    }

    /// The euclidean distance from `self` to `target` (upstream `nearest`'s
    /// `@sqrt(dx*dx + dy*dy)`). The `dx`/`dy`/products/sum are computed in `f16` (matching
    /// upstream's per-op binary16 arithmetic); the square root widens the `f16` sum to `f64`, takes
    /// the root there, and rounds back to `f16` (Rust's `half` has no `f16` sqrt). The wide `f64`
    /// intermediate makes this a single effective rounding, matching Zig's `@sqrt` on `f16` (the
    /// correctly-rounded binary16 result) for normal split-tree coordinates.
    pub(crate) fn distance_to(self, target: Slot) -> f16 {
        let dx = self.x - target.x;
        let dy = self.y - target.y;
        let sum = dx * dx + dy * dy;
        f16::from_f64(sum.to_f64().sqrt())
    }

    /// `self` shifted by one full normalized (1×1) grid for wrap-around in `direction` (upstream
    /// `nearestWrapped`'s target shift). Shifts in the opposite sense of travel so the nearest
    /// search re-finds across the wrap boundary.
    pub(crate) fn wrapped_for(self, direction: SpatialDirection) -> Slot {
        let one = f16::from_f32(1.0);
        match direction {
            SpatialDirection::Left => Slot {
                x: self.x + one,
                ..self
            },
            SpatialDirection::Right => Slot {
                x: self.x - one,
                ..self
            },
            SpatialDirection::Up => Slot {
                y: self.y + one,
                ..self
            },
            SpatialDirection::Down => Slot {
                y: self.y - one,
                ..self
            },
        }
    }
}
```

## Scope / faithfulness notes

- **Ported**: the pure-geometry core of `split_tree`'s spatial navigation →
  `terminal::split_tree::SpatialDirection` + `Slot::is_in_direction` /
  `distance_to` / `wrapped_for`.
- **Faithful**: the direction predicate (`max_x`/`max_y` comparisons, exactly as
  upstream's switch), the distance (`dx`/`dy`/products/sum in `f16`), and the
  one-grid wrap shift are reproduced. `SpatialDirection` is a separate enum
  mirroring upstream's separate `Spatial.Direction`.
- **Faithful adaptation**: the square root widens the `f16` sum to `f64`, roots
  it there, and rounds to `f16` (Rust's `half` has no `f16` `sqrt`). The wide
  `f64` intermediate makes this a single effective rounding, matching Zig's
  `@sqrt` on `f16` (the correctly-rounded binary16 result) for normal split-tree
  coordinates; a theoretical double-rounding edge case is possible but would not
  affect a min-distance _comparison_ in practice. The methods live on `Slot`
  (the candidate is `self`, the from-slot is `target`); `one` is
  `f16::from_f32(1.0)`.
- **Deferred**: `nearest` / `nearestWrapped` themselves — they iterate the
  `Spatial` slots, skip non-leaf nodes via the `Node` arena, and track the
  running minimum; they land with the `Node` arena and the `Spatial` container.
  The `goto` dispatch (`previous` / `next` / `spatial`) also follows with the
  arena.
- No C ABI/header/ABI-inventory change (internal Rust). Extends
  `terminal::split_tree`.

## Changes

1. `roastty/src/terminal/split_tree.rs`: add `SpatialDirection`, and
   `Slot::is_in_direction` / `distance_to` / `wrapped_for`.
2. Tests (in `split_tree.rs`), using binary-exact `f16` values:
   - **is_in_direction**: for a target rect, a candidate fully to its left
     satisfies `Left` (and not `Right`); symmetric checks for right / up / down;
     an overlapping candidate satisfies none.
   - **is_in_direction boundary-touch** (inclusive `<=` / `>=`): a candidate
     with `candidate.max_x() == target.x` counts as `Left`;
     `candidate.x == target.max_x()` as `Right`; and likewise the touching cases
     for `Up` / `Down` — locking in the inclusive comparison.
   - **distance_to**: a `(dx, dy) = (0.75, 1.0)` separation gives distance
     `1.25` (a binary-exact 3-4-5 triangle: `0.75² + 1.0² = 1.5625 = 1.25²`); a
     zero separation gives `0`; an axis-aligned `dy = 0.5` gives `0.5`.
   - **wrapped_for**: each direction shifts the correct axis by `1.0` in the
     correct sense, leaving the other fields unchanged.
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

- `SpatialDirection` and `Slot::is_in_direction` / `distance_to` / `wrapped_for`
  reproduce upstream's direction predicate, euclidean distance, and wrap shift
  with faithful `f16` semantics — faithful to `datastruct/split_tree.zig`;
- the tests pass (direction / distance / wrap), and the existing tests still
  pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the direction predicate, the distance computation,
or the wrap shift diverges from upstream, an unrelated item changes, or any
public C API/ABI changes.

## Design Review

Codex reviewed the design and found **no Required findings**, with one Optional
and one Nit, both adopted:

- **Optional (adopted)**: add boundary-touching direction tests. Upstream's
  comparisons are inclusive (`<=` / `>=`), so a candidate with
  `candidate.max_x() == target.x` should count as `Left`,
  `candidate.x == target.max_x()` as `Right`, etc. Added edge-touch tests to
  lock in the inclusive behavior.
- **Nit (adopted)**: temper the "can never change a comparison" claim on the
  `f32`-rounded sqrt, and optionally widen the sqrt intermediate. Switched the
  sqrt to widen the `f16` sum to **`f64`**
  (`f16::from_f64(sum.to_f64().sqrt())`) for a single effective rounding, and
  softened the wording to "would not affect a min-distance comparison in
  practice."

Codex confirmed the direction predicates, the target-wrapping directions, the
separate `SpatialDirection` enum, and the deferral of the arena-coupled
`nearest` logic are all faithful.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d574-prompt.md`
- Result: `logs/codex-review/20260604-d574-last-message.md`

## Result

**Result:** Pass

`terminal::split_tree` gained the spatial-geometry core: the `SpatialDirection`
enum (`Left` / `Right` / `Down` / `Up`, a separate type from `Direction`), and
three `Slot` methods — `is_in_direction` (the candidate-vs-target `max_x` /
`max_y` direction predicate, inclusive), `distance_to` (the euclidean distance
with `dx` / `dy` / products / sum in `f16` and the square root widened through
`f64`), and `wrapped_for` (the one-grid wrap shift of the target slot).

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3173 passed, 0 failed (four new tests; no
  regressions, up from 3169).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + terminal/split_tree.rs +
  lib.rs/header/abi_harness.c) clean; `git diff --check` clean.

The four new tests: `is_in_direction` basics (each direction plus the
overlapping-target none-of-them case), the inclusive boundary-touch cases
(`candidate.max_x() == target.x` ⇒ `Left`, etc.), `distance_to` (zero, an
axis-aligned `0.5`, and the binary-exact 3-4-5 diagonal `0.75`/`1.0` ⇒ `1.25`),
and `wrapped_for` (each direction shifts the correct axis by `1.0` leaving the
others unchanged).

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no Required
or Optional findings** (one Nit: the `## Result` / `## Conclusion` sections were
not yet in the saved file — added here as part of result recording). Codex
confirmed `is_in_direction` matches upstream's candidate-vs-target comparisons
including the inclusive boundary behavior, `distance_to` keeps the per-op half
arithmetic through the sum and uses a wider sqrt intermediate before returning
to `f16`, `wrapped_for` shifts the target by one normalized unit in the correct
axis/direction, `SpatialDirection` stays properly separate from the split
`Direction`, and the tests cover the core geometry and boundary cases.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r574-prompt.md` (result)
- Result: `logs/codex-review/20260604-r574-last-message.md` (result)

## Conclusion

This experiment ports the pure-geometry core of split_tree's spatial navigation
— the `SpatialDirection` enum and the `Slot` direction predicate, euclidean
distance, and wrap shift — all over `half::f16` (Experiment 573). It is the
third split_tree slice (after the `Handle` / `Layout` / `Direction` vocabulary
and the `Split` / `Slot` structs). What remains for the spatial side is the
arena-coupled `nearest` / `nearestWrapped` (which iterate the `Spatial` slots,
skip non-leaf nodes via the `Node` arena, and track the running minimum using
exactly these helpers), plus the `Spatial` container and its `spatial()` /
`fillSpatialSlots()` / `dimensions()` construction — all gated on the
**`Node`-over-`View`-generic arena and ref-counting**, which is the next
split_tree design question. The remaining big-ticket subsystem is the terminal
**search subsystem** (coupled to `PageList` / `Pin` / `Screen` / `Selection` /
`PageFormatter`), and the dependency-blocked helpers persist (regex/oniguruma
for `Link::oniRegex`, a URI parser for `os/uri`, the config-dir naming decision
for `file_load` / `edit` / `loadDefaultFiles`). With `f16` now in place,
`background-image-opacity`'s float formatter is also unblocked as its own config
slice.
