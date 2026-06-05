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

# Experiment 572: split tree foundations (Handle, Layout, Direction)

## Description

This experiment begins the port of upstream `datastruct/split_tree.zig` — a
large (2517-line) immutable binary tree of terminal split panes — by landing its
**`f16`-free foundational vocabulary**: the node `Handle` (a `u16`-backed
index), the `Split.Layout` and `Split.Direction` enums, and the direction →
(layout, first-side) mapping from `split`. The tree machinery itself (the arena
of nodes, view ref-counting, and the spatial / ratio / resize logic) is
**deferred**: much of it is parameterized over `f16` coordinates and a
`ratio: f16`, which Rust has no stable type for — the same float block as
`background-image-opacity`. This first slice establishes the module and the
split direction/layout logic without touching `f16`. It lands at
`terminal::split_tree` (roastty homes its data structures under `terminal::`).

## Upstream behavior

`datastruct/split_tree.zig`'s `SplitTree(V)` (a generic over the view type)
contains, among much else:

- `Node.Handle` — a `u16`-backed handle into the `nodes` array
  (`enum(u16) { root = 0, _ }`), so nodes are referenced by 16-bit indices
  rather than pointers. Methods: `idx()` (the `usize` index) and `offset(v)`
  (the handle plus `v`, asserting the result stays below `maxInt(u16)`).
- `Split.Layout` — `enum { horizontal, vertical }`: the orientation of a split.
- `Split.Direction` — `enum { left, right, down, up }`: the direction a new view
  is split off in.
- In `split(...)`, the direction maps to a `(Layout, left)` pair, where `left`
  is whether the new view goes on the first (left / top) side:
  ```zig
  const layout: Split.Layout, const left: bool = switch (direction) {
      .left  => .{ .horizontal, true },
      .right => .{ .horizontal, false },
      .up    => .{ .vertical, true },
      .down  => .{ .vertical, false },
  };
  ```

(There is also a separate `Spatial.Direction` enum with the same four variants,
used for 2D navigation over the `f16`-normalized spatial representation; it is
deferred with that spatial logic.)

## Rust mapping (`roastty/src/terminal/split_tree.rs`)

`Handle` becomes a `u16` newtype; the enums port directly; the direction switch
becomes a `Direction::split_layout` method returning `(Layout, bool)`.

```rust
//! Foundational types for the split-pane tree (port of the `f16`-free vocabulary of upstream
//! `datastruct/split_tree`). The tree itself — the node arena, view ref-counting, and the
//! `f16`-based spatial / ratio / resize logic — is deferred (Rust has no stable `f16`).

/// A handle into the tree's `nodes` array (upstream `Node.Handle`): a `u16`-backed index, so nodes
/// are referenced by 16-bit handles rather than pointers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Handle(u16);

impl Handle {
    /// The root node's handle (index 0) (upstream `.root`).
    pub(crate) const ROOT: Handle = Handle(0);

    /// Build a handle from an index (upstream `@enumFromInt`). The full `u16` range is valid —
    /// upstream's `enum(u16)` can represent `u16::MAX`, which the tree iterator uses as an
    /// end sentinel (`@enumFromInt(handle.idx() + 1)`).
    pub(crate) fn from_index(index: usize) -> Handle {
        assert!(index <= u16::MAX as usize, "split tree handle out of range");
        Handle(index as u16)
    }

    /// The index this handle refers to (upstream `idx`).
    pub(crate) fn idx(self) -> usize {
        self.0 as usize
    }

    /// Offset the handle by `v` (upstream `offset`), asserting the result stays below `u16::MAX`
    /// (matching upstream's `final < maxInt(Backing)`).
    pub(crate) fn offset(self, v: usize) -> Handle {
        let result = (self.0 as usize)
            .checked_add(v)
            .expect("split tree handle offset overflow");
        assert!(result < u16::MAX as usize, "split tree handle overflow");
        Handle(result as u16)
    }
}

/// The orientation of a split (upstream `Split.Layout`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Layout {
    Horizontal,
    Vertical,
}

/// The direction a new view is split off in (upstream `Split.Direction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    Left,
    Right,
    Down,
    Up,
}

impl Direction {
    /// The split layout and whether the new view goes on the first (left / top) side, for a split
    /// in this direction (upstream `split`'s direction switch).
    pub(crate) fn split_layout(self) -> (Layout, bool) {
        match self {
            Direction::Left => (Layout::Horizontal, true),
            Direction::Right => (Layout::Horizontal, false),
            Direction::Up => (Layout::Vertical, true),
            Direction::Down => (Layout::Vertical, false),
        }
    }
}
```

## Scope / faithfulness notes

- **Ported**: the `f16`-free foundational types of `datastruct/split_tree` →
  `terminal::split_tree` — `Handle` (`ROOT` / `from_index` / `idx` / `offset`),
  `Layout`, `Direction`, and `Direction::split_layout`.
- **Faithful**: the `u16`-backed handle with its `root` value, `idx`, and
  `offset` (including the `< maxInt(u16)` overflow assertion); the two enums;
  and the direction → `(layout, left)` mapping (`left`/`right` ⇒ horizontal,
  `up`/`down` ⇒ vertical; `left`/`up` ⇒ first side) reproduced exactly.
- **Faithful adaptation**: upstream's `enum(u16) { root = 0, _ }` (a
  non-exhaustive integer enum) becomes a `u16` newtype `Handle`, with
  `from_index` standing in for upstream's `@enumFromInt` (the construction used
  elsewhere in the tree). `from_index` accepts the full `u16` range
  (`index <= u16::MAX`) since upstream's enum can represent `u16::MAX` (the
  iterator's end sentinel); only `offset` keeps the strict `result < u16::MAX`
  assertion, matching upstream's `final < std.math.maxInt(Backing)`. `offset`
  uses `checked_add` before that assertion so a pathological `v` cannot wrap
  past the check in optimized builds.
- **Deferred** (the bulk of `split_tree.zig`, a multi-experiment subsystem):
  - the generic immutable tree itself (the arena of `Node`s — `leaf: *View` /
    `split: Split` — and view ref-counting, `init` / `clone` / `deinit` /
    `iterator` / `zoom` / `goto` / `deepest` / `split` / `remove` / `equalize` /
    `resize` / formatters);
  - all `f16`-based logic: `Split.ratio`, the `Spatial` representation
    (`Slot { x, y, width, height }`, normalization), `Spatial.Direction`
    navigation, and the resize math — blocked on the lack of a stable Rust `f16`
    (the same block as `background-image-opacity`).
- No C ABI/header/ABI-inventory change (internal Rust). Adds
  `terminal::split_tree`.

## Changes

1. `roastty/src/terminal/split_tree.rs` (new): `Handle`, `Layout`, `Direction`,
   `Direction::split_layout`.
2. `roastty/src/terminal/mod.rs`: add `#[allow(dead_code)] mod split_tree;`
   (alphabetical).
3. Tests (in `split_tree.rs`):
   - **handle root / idx / offset**: `ROOT.idx() == 0`;
     `from_index(5).idx() == 5`; `ROOT.offset(3).idx() == 3`;
     `from_index(2).offset(4).idx() == 6`.
   - **from_index allows u16::MAX**:
     `from_index(u16::MAX as usize).idx() == u16::MAX as usize` (the
     end-sentinel value); `from_index(u16::MAX as usize + 1)` panics
     (`#[should_panic]`).
   - **offset overflow panics**: an `offset` whose result reaches `u16::MAX`
     (`ROOT.offset(u16::MAX as usize)`) panics (`#[should_panic]`);
     `ROOT.offset(u16::MAX as usize
     - 1).idx() == u16::MAX as usize - 1` succeeds.
   - **split_layout mapping**: each of the four directions maps to the correct
     `(Layout, bool)`.
   - **enum distinctness**: `Horizontal` ≠ `Vertical`; the four directions are
     distinct.
4. Format and test (`cargo fmt`, accept output).

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

- `Handle` reproduces upstream's `u16`-backed index (`root` / `idx` / `offset`
  with the overflow assert), and `Direction::split_layout` reproduces the
  direction → `(layout, first-side)` mapping — faithful to
  `datastruct/split_tree.zig`;
- the tests pass (handle root/idx/offset / overflow / split_layout /
  distinctness), and the existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the handle semantics, the overflow assertion, or the
direction mapping diverge from upstream, an unrelated item changes, or any
public C API/ABI changes.

## Design Review

Codex reviewed the design and found **two Required** findings, both adopted:

- **Required (fixed)**: `from_index` (the `@enumFromInt` adapter) must allow
  `u16::MAX`, not reject it — upstream's `enum(u16) { root = 0, _ }` can
  represent `65535`, which the tree iterator creates as an end sentinel via
  `@enumFromInt(handle.idx() + 1)`. Changed `from_index` to assert
  `index <= u16::MAX`; only `offset` keeps the strict `< u16::MAX` assert
  (upstream asserts that explicitly). The overflow test was split:
  `from_index(u16::MAX)` now succeeds, while `from_index(u16::MAX + 1)` and an
  `offset` reaching `u16::MAX` panic.
- **Required (fixed)**: `offset` should use **checked addition** before the
  range assertion — `self.0 as usize + v` can wrap in optimized builds for a
  pathological `v`, after which the `< u16::MAX` assert could incorrectly pass.
  Changed to `checked_add(v).expect(...)` then the strict `< u16::MAX` check.

Codex confirmed the `Direction::split_layout` mapping is faithful, the enum
vocabulary is fine, and deferring the `f16`-heavy tree / spatial logic is
appropriately scoped.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d572-prompt.md`
- Result: `logs/codex-review/20260604-d572-last-message.md`

## Result

**Result:** Pass

`terminal::split_tree` was added with the `f16`-free foundational vocabulary of
upstream `datastruct/split_tree`: `Handle(u16)` (`ROOT` = index 0; `from_index`
accepting the full `u16` range as the `@enumFromInt` adapter; `idx`; `offset`
using `checked_add` then the strict `< u16::MAX` assert), `Layout` (`Horizontal`
/ `Vertical`), `Direction` (`Left` / `Right` / `Down` / `Up`), and
`Direction::split_layout() -> (Layout, bool)` reproducing the upstream `split`
direction switch. Registered via `#[allow(dead_code)] mod split_tree;` in
`terminal/mod.rs`. The tree machinery and all `f16` logic are deferred.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3166 passed, 0 failed (seven new tests; no
  regressions, up from 3159).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + terminal/split_tree.rs +
  lib.rs/header/abi_harness.c) clean; `git diff --check` clean.

The seven new tests: handle root / `idx` / `offset` composition, `from_index`
allowing `u16::MAX` (the end sentinel) while `from_index(u16::MAX + 1)` panics,
`offset` succeeding at `u16::MAX - 1` and panicking at `u16::MAX`, the
four-direction `split_layout` mapping, and enum distinctness.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no Required
or Optional findings** (one Nit: the `## Result` / `## Conclusion` sections were
not yet in the saved file — added here as part of result recording). Codex
confirmed the fixed `Handle` semantics line up with upstream — `from_index`
allows the full `u16` range for `@enumFromInt`-style construction, `offset` uses
checked addition and keeps the strict `< u16::MAX` assertion, and the
direction-to-layout/first-side mapping is exact — that the `f16` / tree deferral
is appropriately scoped, and that the tests cover the handle boundaries and the
mapping.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r572-prompt.md` (result)
- Result: `logs/codex-review/20260604-r572-last-message.md` (result)

## Conclusion

This experiment opens the `terminal::split_tree` module with the `f16`-free
foundational vocabulary of upstream's 2517-line `datastruct/split_tree` — the
`u16` node `Handle`, the `Layout` / `Direction` enums, and the direction →
`(layout, first-side)` mapping. It is the first slice of what will be a
multi-experiment subsystem: the immutable node arena, view ref-counting, and the
tree-shaping operations (`split` / `remove` / `goto` / `zoom` / `equalize`)
follow, but the **`f16`-parameterized** parts (`Split.ratio`, the normalized
`Spatial` representation and its resize math) are blocked on the lack of a
stable Rust `f16` — the same block as `background-image-opacity`. The remaining
big-ticket subsystem is the terminal **search subsystem** (coupled to `PageList`
/ `Pin` / `Screen` / `Selection` / `PageFormatter`), and the dependency-blocked
helpers persist (regex/oniguruma for `Link::oniRegex`, a URI parser for
`os/uri`, the config-directory naming decision for `file_load` / `edit` /
`loadDefaultFiles`). The `f16` block now spans `background-image-opacity` and
the `split_tree` spatial/ratio logic; a future slice could introduce a shared
half-precision-float representation to unblock both.
