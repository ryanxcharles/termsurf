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

# Experiment 581: split tree split (insert a split node)

## Description

This experiment ports `split` from upstream `datastruct/split_tree.zig` — the
first **tree-shaping** operation. `split` builds a **new** immutable tree by
inserting another tree (`insert`) next to the node `at`, splitting in
`direction` with `ratio`. It reuses the already-ported `Direction::split_layout`
(Experiment 572) and `Handle::offset` (Experiment 572); Rust's `Rc::clone`
provides upstream's `refNodes` view ref-counting. It extends
`terminal::split_tree`.

## Upstream behavior

```zig
pub fn split(self, gpa, at, direction, ratio, insert) !Self {
    const nodes = try alloc.alloc(Node, self.nodes.len + insert.nodes.len + 1);
    if (nodes.len > maxInt(Node.Handle.Backing)) return error.OutOfMemory;  // u16 handle limit

    @memcpy(nodes[0..self.nodes.len], self.nodes);                 // copy self's nodes
    const inserted = nodes[self.nodes.len..][0..insert.nodes.len];
    @memcpy(inserted, insert.nodes);                              // copy insert's nodes...
    for (inserted) |*node| switch (node.*) { .leaf => {},
        .split => |*s| { s.left = s.left.offset(self.nodes.len);  // ...offsetting split handles
                         s.right = s.right.offset(self.nodes.len); } };

    const layout, const left = <direction → (Layout, on-first-side)>;  // = split_layout

    nodes[nodes.len - 1] = nodes[at.idx()];          // relocate the `at` node to the end
    nodes[at.idx()] = .{ .split = .{                 // put a new split where `at` was
        .layout = layout, .ratio = ratio,
        .left  = if (left) self.nodes.len else nodes.len - 1,   // inserted root vs relocated `at`
        .right = if (left) nodes.len - 1 else self.nodes.len,
    } };

    try refNodes(gpa, nodes);                        // ref every view in the new tree
    return .{ .arena = arena, .nodes = nodes, .zoomed = null };   // split resets zoom
}
```

The new tree has `self.nodes.len + insert.nodes.len + 1` nodes: `self`'s nodes
unchanged at the front, `insert`'s nodes next (with their split handles shifted
by `self.nodes.len`), and the node formerly at `at` relocated to the last slot.
A fresh `Split` takes `at`'s slot, pointing at the inserted subtree's root
(`self.nodes.len`) and the relocated original (`nodes.len - 1`), ordered by
whether the new view goes on the first (left/top) side. Every view is ref'd once
(the new tree shares views with `self` and `insert`). `zoomed` resets. A node
count exceeding the `u16` handle range is an error.

## Rust mapping (`roastty/src/terminal/split_tree.rs`)

Building the new `Vec<Node<V>>` by **cloning** each node _is_ `@memcpy` +
`refNodes`: every leaf's `Rc<V>` is cloned (refcount++). Relocating `at` clones
its node to the end; overwriting `at`'s slot with the `Split` drops the old
clone there, leaving each view ref'd exactly once.

```rust
/// The node count of a `split` result would exceed the `u16` handle range (upstream's
/// `error.OutOfMemory` on the handle-limit check).
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct TooManyNodes;

impl<V> SplitTree<V> {
    /// Build a new tree by splitting `at` in `direction` with `ratio`, inserting `insert` on the
    /// new side (upstream `split`). The result shares all views with `self` and `insert`
    /// (ref-counted) and resets the zoom state.
    pub(crate) fn split(
        &self,
        at: Handle,
        direction: Direction,
        ratio: f16,
        insert: &SplitTree<V>,
    ) -> Result<SplitTree<V>, TooManyNodes> {
        let self_len = self.nodes.len();
        let total = self_len + insert.nodes.len() + 1;
        if total > u16::MAX as usize {
            return Err(TooManyNodes);
        }

        let mut nodes: Vec<Node<V>> = Vec::with_capacity(total);
        // self's nodes, unchanged (cloning refs each view).
        nodes.extend(self.nodes.iter().cloned());
        // insert's nodes, with split handles shifted by `self_len`.
        for node in &insert.nodes {
            nodes.push(match node {
                Node::Leaf(v) => Node::Leaf(Rc::clone(v)),
                Node::Split(s) => Node::Split(Split {
                    left: s.left.offset(self_len),
                    right: s.right.offset(self_len),
                    ..*s
                }),
            });
        }

        // Relocate the `at` node to the end, then replace its slot with the new split.
        let relocated_at = total - 1;
        let inserted_root = self_len;
        nodes.push(nodes[at.idx()].clone());

        let (layout, left) = direction.split_layout();
        nodes[at.idx()] = Node::Split(Split {
            layout,
            ratio,
            left: Handle::from_index(if left { inserted_root } else { relocated_at }),
            right: Handle::from_index(if left { relocated_at } else { inserted_root }),
        });

        Ok(SplitTree {
            nodes,
            zoomed: None, // split always resets zoom
        })
    }
}
```

## Scope / faithfulness notes

- **Ported**: `split` → `SplitTree::split`.
- **Faithful**: the new node count (`self_len + insert_len + 1`), `self`'s nodes
  copied at the front, `insert`'s nodes copied next with their split handles
  `offset` by `self_len`, the direction → `(layout, on-first-side)` mapping
  (`split_layout`), the relocation of the `at` node to the last slot, the new
  `Split` at `at`'s slot pointing at the inserted root vs the relocated original
  (ordered by `left`), the `u16`-handle-range error, and the zoom reset are all
  reproduced.
- **Faithful adaptation**: upstream's `@memcpy` (no refcount change) + a single
  `refNodes` (ref all) become per-node `Rc::clone` while building the `Vec` —
  every leaf view ends up ref'd once. The `at`-relocation clones the node and
  the subsequent slot overwrite drops the stale clone, so the relocated view is
  ref'd exactly once (matching upstream). The combined allocator-OOM /
  handle-overflow `error.OutOfMemory` becomes a dedicated `TooManyNodes` error
  (Rust has no fallible `Vec` alloc here; only the handle-range overflow is a
  real error). `at` may be any node (leaf or split): a relocated split keeps its
  child handles, which still index `self`'s preserved front nodes.
- **Deferred**: `remove` / `equalize` / `resize` (the other shaping ops) and the
  formatters.
- No C ABI/header/ABI-inventory change (internal Rust). Extends
  `terminal::split_tree`.

## Changes

1. `roastty/src/terminal/split_tree.rs`: add `TooManyNodes` and
   `SplitTree::split`.
2. Tests (in `split_tree.rs`):
   - **split single + single, `Right`**: a leaf split right by another leaf → a
     horizontal split with the original on the left and the inserted on the
     right; `is_split`, `dimensions == {2,1}`, and `spatial` places them
     left/right.
   - **split single + single, `Left`**: the inserted goes on the left, original
     on the right.
   - **split single + a 2-leaf subtree**: the inserted split's child handles are
     offset correctly (verified via iteration order and `dimensions`).
   - **vertical direction (`Down` / `Up`)**: a vertical split with the inserted
     below / above.
   - **`at` is a split node**: split at an internal split — the relocated split
     keeps its child handles (pointing into the preserved front nodes) and the
     new parent split points to the inserted root plus the relocated split,
     verified via iteration order / `dimensions`.
   - **overflow**: a tree with `u16::MAX` nodes split by a single leaf (`total`
     = `u16::MAX + 2`) returns `Err(TooManyNodes)`.
   - **ref-counting**: after `split`, every distinct view's `Rc::strong_count`
     rises by one (the new tree shares them).
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

- `split` builds the new tree with the correct node layout (self front, offset
  insert, relocated `at`), the new `Split` ordered by direction, the `u16`
  overflow error, and the shared-view ref-counting and zoom reset — faithful to
  `datastruct/split_tree.zig`;
- the tests pass (single+single right/left / subtree-offset / vertical /
  ref-count), and the existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the node layout, the handle offsetting, the new
split's orientation / child order, the overflow error, or the ref-counting
diverges from upstream, an unrelated item changes, or any public C API/ABI
changes.

## Design Review

Codex reviewed the design and found **no Required findings**, with two Optionals
— both adopted:

- **Optional (adopted)**: add a test where `at` is itself a split node (an
  explicit design edge: the relocated split keeps its child handles into the
  preserved front nodes, and the new parent split points to the inserted root
  plus the relocated split).
- **Optional (adopted)**: add a `TooManyNodes` boundary test — a tree with
  `u16::MAX` nodes split by a single leaf makes `total` exceed `u16::MAX`,
  exercising the dedicated overflow error.

Codex confirmed the node layout is faithful (self front, offset inserted
subtree, relocated `at` at the end, the new split at `at`, child order from
`Direction::split_layout`), that the refcount trace is correct for `at` as a
leaf (cloning self gives the new tree one ref; pushing the relocated clone
temporarily adds another; overwriting `at` drops the stale clone, leaving
exactly one new-tree ref per view), and that for `Right` the original stays left
and the inserted goes right (matching upstream).

Review artifacts:

- Prompt: `logs/codex-review/20260604-d581-prompt.md`
- Result: `logs/codex-review/20260604-d581-last-message.md`

## Result

**Result:** Pass

`terminal::split_tree` gained `TooManyNodes` and `SplitTree::split` — the first
tree-shaping operation. `split` builds a new tree (`self`'s nodes cloned at the
front, `insert`'s nodes cloned next with their split handles `offset` by
`self_len`, the `at` node relocated to the final slot, and a fresh `Split` at
`at`'s slot whose child order follows `Direction::split_layout`), erroring with
`TooManyNodes` when the node count exceeds the `u16` handle range, and resetting
`zoomed`.

One structural fix was required during implementation: the derived `Clone` on
`Node<V>` and `SplitTree<V>` added a spurious `V: Clone` bound (a `derive`-macro
limitation), but `Rc<V>` is `Clone` for any `V` and `split` is generic with no
`Clone` bound. The derived `Clone` was replaced with **manual** `Clone` impls
for both (`Node::clone` → `Rc::clone` / copy the `Copy` `Split`;
`SplitTree::clone` → clone the node `Vec` + copy `zoomed`), with no `V` bound;
`#[derive(Debug)]` was kept. The module doc comment was updated to mark `split`
landed.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3206 passed, 0 failed (seven new tests; no
  regressions, up from 3199).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + terminal/split_tree.rs +
  lib.rs/header/abi_harness.c) clean; `git diff --check` clean.

The seven new tests: split single + single `Right` (original left, inserted
right; `dimensions == {2,1}`, spatial placement), `Left` (mirror), vertical
`Down` (inserted below, `{1,2}`), inserting a 2-leaf subtree (its child handles
offset by `self_len`, `{3,1}`), splitting **at a split node** (the relocated
split keeps its child handles, views `a, b, c`), the `u16`-overflow
`TooManyNodes` error (a `u16::MAX`-node tree), and the `Rc::strong_count`
ref-counting (each view `2 → 3` on split, `→ 2` on drop).

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no Required
or Optional findings** (two Nits, both fixed): the module doc comment still
listed `split` as deferred (updated) and the `## Result` / `## Conclusion`
sections were not yet in the saved file (added here). Codex confirmed the
implementation matches upstream (self nodes cloned at the front, insert nodes
cloned with handles offset by `self_len`, `at` relocated to the final slot, the
replacement split's child order following `Direction::split_layout`, `zoomed`
reset, and the `TooManyNodes` guard covering the `u16` limit before `offset` can
panic), that the manual `Clone` impls are sound and better than derive here (no
`V: Clone` needed, the `at`-relocation refcount trace correct), and that the
tests cover the shape, offset, split-at-split, overflow, spatial placement, and
refcount cases.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r581-prompt.md` (result)
- Result: `logs/codex-review/20260604-r581-last-message.md` (result)

## Conclusion

This experiment ports `split` — the ninth split_tree slice and the **first
tree-shaping operation**. `split` builds a new immutable tree by inserting a
subtree next to a node, reusing the already-ported `Direction::split_layout` and
`Handle::offset`, with Rust's `Rc::clone` providing upstream's `refNodes` view
ref-counting for free. A notable side-effect was switching `Node<V>` /
`SplitTree<V>` to **manual `Clone`** impls (the derived bound `V: Clone` was
wrong for an `Rc<V>`-holding type) — which also makes the tree `clone`able for
any view type, as upstream's is. The remaining split_tree work is the other
shaping operations — `remove` (the inverse of `split`), `equalize`, and `resize`
(the `f16`-ratio rebalancers) — and the `formatText` / `formatDiagram`
formatters. The other remaining big-ticket subsystem is the terminal **search
subsystem** (coupled to `PageList` / `Pin` / `Screen` / `Selection` /
`PageFormatter`); the dependency-blocked helpers persist (regex/oniguruma for
`Link::oniRegex`, a URI parser for `os/uri`, the config-directory naming
decision for `file_load` / `edit` / `loadDefaultFiles`).
