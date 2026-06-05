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

# Experiment 576: split tree arena (Node, SplitTree, Side, structural queries)

## Description

This experiment lands the **generic node arena** of upstream
`datastruct/split_tree.zig` — the `Node` enum (`leaf` / `split`), the
`SplitTree(V)` container, the `Side` enum, and the `f16`-free structural queries
`isEmpty` / `isSplit` / `deepest` / `dimensions` (plus `init` / `empty` and
`clone` via ref-counting). It builds on the `Handle` / `Layout` / `Direction`
vocabulary (Experiment 572) and the `Split` payload (Experiment 573). The
tree-shaping operations (`split` / `remove` / `goto` / `zoom` / `equalize` /
`resize`), the `iterator`, the formatters, and the `Spatial` container stay
deferred. It extends `terminal::split_tree`.

## Upstream behavior

`SplitTree(V)` is an immutable binary tree stored as a flat arena:

- `Node = union(enum) { leaf: *View, split: Split }` — a node is either a leaf
  holding a (ref-counted) view pointer or an internal `Split`.
- The tree holds `nodes: []const Node` (index 0 is the root) and
  `zoomed: ?Node.Handle`.
- `init(gpa, view)` — a single-leaf tree (`viewRef`s the view). `empty` — zero
  nodes.
- `deinit` — `viewUnref`s every leaf's view (only if non-empty).
- `clone` — duplicates the `nodes` array and `viewRef`s every leaf (`refNodes`).
- `isEmpty()` — `nodes.len == 0`.
- `isSplit()` — non-empty and the root (`nodes[0]`) is a `split`.
- `deepest(side, from)` — walk from `from`, descending into the `left` / `right`
  child per `side` until a leaf, returning its handle.
  `Side = enum { left, right }`.
- `dimensions(from)` — relative dimensions assuming each leaf is `1×1`: a leaf
  is `{1, 1}`; a horizontal split is
  `{ left.w + right.w, max(left.h, right.h) }`; a vertical split is
  `{ max(left.w, right.w), left.h + right.h }` (both `u16`).

Views are **externally ref-counted**: the `View` type provides `ref()` /
`unref()`, and the tree calls them via `viewRef` / `viewUnref`.

## Rust mapping (`roastty/src/terminal/split_tree.rs`)

The leaf holds `Rc<V>`, so Rust's reference counting _is_ upstream's `viewRef` /
`viewUnref`: cloning the tree clones each `Rc` (refcount++ = `refNodes`),
dropping it drops each `Rc` (refcount-- = `deinit`). No explicit `ref` / `unref`
methods are needed.

```rust
use std::rc::Rc;

/// A node in the split tree (upstream `Node`): a leaf holding a (ref-counted) view, or an internal
/// split.
#[derive(Debug, Clone)]
pub(crate) enum Node<V> {
    Leaf(Rc<V>),
    Split(Split),
}

/// Which child to descend into (upstream `Side`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Side {
    Left,
    Right,
}

/// Relative tree dimensions in leaf units (upstream `dimensions`' anonymous return).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Dimensions {
    pub(crate) width: u16,
    pub(crate) height: u16,
}

/// An immutable binary tree of split panes, stored as a flat node arena (upstream `SplitTree(V)`).
/// Index 0 is the root. Cloning the tree clones each leaf's `Rc<V>` (upstream `clone` / `refNodes`);
/// dropping it drops them (upstream `deinit` / `viewUnref`).
#[derive(Debug, Clone)]
pub(crate) struct SplitTree<V> {
    nodes: Vec<Node<V>>,
    zoomed: Option<Handle>,
}

impl<V> SplitTree<V> {
    /// An empty tree with no nodes (upstream `empty`).
    pub(crate) fn empty() -> Self {
        SplitTree {
            nodes: Vec::new(),
            zoomed: None,
        }
    }

    /// A single-leaf tree holding `view` (upstream `init`). The caller's `Rc` is stored (its
    /// refcount is the view's ref).
    pub(crate) fn new(view: Rc<V>) -> Self {
        SplitTree {
            nodes: vec![Node::Leaf(view)],
            zoomed: None,
        }
    }

    /// Whether the tree has no nodes (upstream `isEmpty`).
    pub(crate) fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Whether the root is a split — i.e. the tree has more than one view (upstream `isSplit`).
    pub(crate) fn is_split(&self) -> bool {
        matches!(self.nodes.first(), Some(Node::Split(_)))
    }

    /// The deepest leaf reached by always descending into the `side` child from `from` (upstream
    /// `deepest`).
    pub(crate) fn deepest(&self, side: Side, from: Handle) -> Handle {
        let mut current = from;
        loop {
            match &self.nodes[current.idx()] {
                Node::Leaf(_) => return current,
                Node::Split(s) => {
                    current = match side {
                        Side::Left => s.left,
                        Side::Right => s.right,
                    }
                }
            }
        }
    }

    /// Relative dimensions of the subtree at `from`, in leaf units (upstream `dimensions`).
    pub(crate) fn dimensions(&self, from: Handle) -> Dimensions {
        match &self.nodes[from.idx()] {
            Node::Leaf(_) => Dimensions {
                width: 1,
                height: 1,
            },
            Node::Split(s) => {
                let left = self.dimensions(s.left);
                let right = self.dimensions(s.right);
                match s.layout {
                    Layout::Horizontal => Dimensions {
                        width: left.width + right.width,
                        height: left.height.max(right.height),
                    },
                    Layout::Vertical => Dimensions {
                        width: left.width.max(right.width),
                        height: left.height + right.height,
                    },
                }
            }
        }
    }
}
```

## Scope / faithfulness notes

- **Ported**: the node arena of `split_tree` →
  `terminal::split_tree::{Node, SplitTree, Side, Dimensions}` with `empty` /
  `new` / `is_empty` / `is_split` / `deepest` / `dimensions`, and tree `clone`
  (the derived `Clone`).
- **Faithful**: `Node` (`Leaf` / `Split`), the root-at-index-0 arena, `is_empty`
  / `is_split`, `deepest`'s side descent, and `dimensions`' leaf=`{1,1}` /
  horizontal=`{l.w+r.w, max(h)}` / vertical=`{max(w), l.h+r.h}` recursion are
  reproduced exactly (the `u16` adds use plain `+`, as upstream does — split
  trees are tiny). `Side` is the upstream enum.
- **Faithful adaptation**: upstream's externally ref-counted `*View` (`View.ref`
  / `View.unref`, called via `viewRef` / `viewUnref`) becomes `Rc<V>`: cloning
  the tree clones each leaf's `Rc` (refcount++ = `refNodes` / `clone`), and
  dropping it drops them (refcount-- = `viewUnref` / `deinit`), so no explicit
  ref/unref methods or allocator are needed. The empty tree is just an empty
  `Vec` (no "undefined arena" special case). `dimensions`' anonymous struct
  becomes the named `Dimensions`.
- **Deferred** (the rest of the tree, still multi-experiment): the shaping
  operations (`split` / `remove` / `goto` / `previous` / `next` / `zoom` /
  `equalize` / `resize`), the `iterator`, the `Spatial` container (`spatial` /
  `fillSpatialSlots`, which use `Slot`) and `nearest` / `nearestWrapped`, and
  the formatters. The GTK gobject glue is out of scope (libroastty is not GTK).
- No C ABI/header/ABI-inventory change (internal Rust). Extends
  `terminal::split_tree`.

## Changes

1. `roastty/src/terminal/split_tree.rs`: add `use std::rc::Rc;`, `Node<V>`,
   `Side`, `Dimensions`, `SplitTree<V>` with `empty` / `new` / `is_empty` /
   `is_split` / `deepest` / `dimensions`.
2. Tests (in `split_tree.rs`), building trees by hand (since `split` is
   deferred):
   - **single leaf**: `new(view)` is not empty, not a split; `deepest` of either
     side is the root; `dimensions(root) == {1, 1}`.
   - **empty**: `empty()` is empty and not a split.
   - **horizontal split of two leaves**: `is_split`; `deepest(Left, root)` /
     `deepest(Right, root)` are the two leaf handles;
     `dimensions(root) == {2, 1}`.
   - **vertical split**: `dimensions(root) == {1, 2}`.
   - **nested tree**: a split whose left child is itself a split —
     `deepest(Left, root)` reaches the deepest-left leaf; `dimensions`
     sums/maxes correctly.
   - **clone ref-counts the views**: cloning a tree raises each leaf view's
     `Rc::strong_count`; dropping the clone lowers it.
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

- `Node` / `SplitTree` / `Side` / `Dimensions` reproduce upstream's arena,
  `is_empty` / `is_split`, `deepest`, and `dimensions`, with `Rc<V>` providing
  the view ref-count lifecycle — faithful to `datastruct/split_tree.zig`;
- the tests pass (single leaf / empty / horizontal / vertical / nested / clone
  ref-count), and the existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the arena structure, the structural queries, the
`dimensions` recursion, or the ref-count lifecycle diverges from upstream, an
unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed the design and **approved it with no findings**. It confirmed
`Rc<V>` is a faithful Rust mapping for this slice (clone increments each leaf
reference, drop decrements it, and the structural queries need only read access
to the leaf payload — a future view needing mutation can use interior mutability
inside `V`), and that the arena/query behavior matches upstream: `is_empty`,
root-only `is_split`, `deepest` returning `from` when it starts on a leaf, and
`dimensions` recursion with leaf `{1, 1}`, horizontal width-sum / max-height,
and vertical max-width / height-sum (plain `u16` addition acceptable for the
tiny split trees, matching the upstream expression shape). The deferral scope is
correct, building tests by hand until `split` lands is fine, and the
`Rc::strong_count` clone/drop test is the right way to validate the refcount
adaptation.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d576-prompt.md`
- Result: `logs/codex-review/20260604-d576-last-message.md`

## Result

**Result:** Pass

`terminal::split_tree` gained the node arena: `Node<V>` (`Leaf(Rc<V>)` /
`Split`), `Side`, `Dimensions`, and `SplitTree<V>` (`nodes: Vec<Node<V>>`,
`zoomed: Option<Handle>`, deriving `Clone`) with `empty` / `new(Rc<V>)` /
`is_empty` / `is_split` / `deepest` / `dimensions`. The leaf's `Rc<V>` supplies
the view ref-count lifecycle: cloning the tree refs each view (upstream
`refNodes`), dropping it unrefs them (upstream `viewUnref` / `deinit`). The
module doc comment was updated to reflect that the arena and ref-counting are
now landed.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3180 passed, 0 failed (six new tests; no regressions,
  up from 3174).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + terminal/split_tree.rs +
  lib.rs/header/abi_harness.c) clean; `git diff --check` clean.

The six new tests: single-leaf queries, the empty tree, a horizontal split
(`is_split`, `deepest` left/right = the two leaf handles,
`dimensions == {2, 1}`), a vertical split (`{1, 2}`), a nested tree (a
horizontal split of a vertical 1×2 column and a single leaf → `deepest` left
reaches the deep leaf, right reaches the leaf, `dimensions == {2, 2}`), and the
`Rc::strong_count` clone/drop lifecycle (`2 → 3` on clone, `→ 2` on dropping the
clone, `→ 1` on dropping the tree).

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no Required
or Optional findings** (two Nits, both fixed): the module doc comment still
listed the arena / ref-counting as deferred (updated to reflect they are landed,
narrowing the deferral to the tree-shaping / spatial / formatter work), and the
`## Result` / `## Conclusion` sections were not yet in the saved file (added
here). Codex confirmed the implementation matches upstream for the arena model,
root-at- index-0, `is_empty`, root `is_split`, `deepest`, and `dimensions` —
verifying the nested `{2, 2}` case (a vertical `{1, 2}` left subtree plus a
`{1, 1}` right leaf under a horizontal split gives width `2`, height `2`) — and
that the `Rc` clone/drop strong-count test soundly validates the refcount
adaptation.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r576-prompt.md` (result)
- Result: `logs/codex-review/20260604-r576-last-message.md` (result)

## Conclusion

This experiment lands the **generic `Node<V>` arena** of `datastruct/split_tree`
— the fourth split_tree slice (after the vocabulary, the `Split` / `Slot`
payloads, and the spatial geometry). The defining adaptation is the view
lifecycle: upstream's externally ref-counted `*View` (`View.ref` / `View.unref`
via `viewRef` / `viewUnref`) becomes `Rc<V>`, so the tree's `clone` and drop
_are_ the ref / unref — no allocator, no explicit ref methods, validated by the
`Rc::strong_count` test. With the arena in place, the next split_tree slices are
the tree-shaping operations (`split` / `remove` / `goto` / `previous` / `next` /
`zoom`) and the `iterator` (all arena walks over `Node<V>`), then the `Spatial`
container's normalization (`spatial` / `fillSpatialSlots`, which combine the
`Node` arena with the `Slot` / spatial geometry already ported) and the
arena-coupled `nearest` / `nearestWrapped`, and finally the formatters. The
other remaining big-ticket subsystem is the terminal **search subsystem**
(coupled to `PageList` / `Pin` / `Screen` / `Selection` / `PageFormatter`); the
dependency-blocked helpers persist (regex/oniguruma for `Link::oniRegex`, a URI
parser for `os/uri`, the config-directory naming decision for `file_load` /
`edit` / `loadDefaultFiles`).
