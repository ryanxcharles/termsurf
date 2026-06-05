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

# Experiment 577: split tree iterator, zoom, and Goto

## Description

This experiment ports the **leaf iterator** of upstream
`datastruct/split_tree.zig` (`iterator` / `Iterator` / `ViewEntry`), the `zoom`
setter, and the `Goto` navigation enum. The iterator walks the node arena
(Experiment 576) yielding each leaf's `(handle, view)` and skipping splits — it
is `f16`-free and self-contained over the arena. `Goto`'s consuming method
(`goto`, which dispatches to the deferred `previous` / `next` / `nearest`) stays
deferred; this lands its enum vocabulary. It extends `terminal::split_tree`.

## Upstream behavior

```zig
pub fn iterator(self) Iterator { return .{ .nodes = self.nodes }; }

pub const ViewEntry = struct { handle: Node.Handle, view: *View };

pub const Iterator = struct {
    i: Node.Handle = .root,
    nodes: []const Node,
    pub fn next(self) ?ViewEntry {
        if (@intFromEnum(self.i) >= self.nodes.len) return null;
        const handle = self.i;
        self.i = @enumFromInt(handle.idx() + 1);
        return switch (self.nodes[handle.idx()]) {
            .leaf => |v| .{ .handle = handle, .view = v },
            .split => self.next(),       // skip splits, advance to the next node
        };
    }
};

pub fn zoom(self, handle: ?Node.Handle) void {
    if (handle) |v| { assert(@intFromEnum(v) >= 0); assert(@intFromEnum(v) < self.nodes.len); }
    self.zoomed = handle;
}

pub const Goto = union(enum) {
    previous, next, previous_wrapped, next_wrapped,
    spatial: Spatial.Direction,
};
```

The iterator visits node indices `0..len` in order, returning a `ViewEntry` for
each **leaf** (`handle` = its index, `view` = its view pointer) and skipping
split nodes. `zoom` records the zoomed handle (asserting it is in range). `Goto`
is the argument to `goto` (deferred): previous/next view (with optional wrap),
or a spatial direction.

## Rust mapping (`roastty/src/terminal/split_tree.rs`)

The iterator becomes a real `Iterator` impl; `ViewEntry` borrows the leaf's
`Rc<V>`; upstream's `.split => self.next()` recursion becomes a `while` loop
(skip splits, advance the index).

```rust
/// A leaf visited by the tree iterator (upstream `ViewEntry`).
pub(crate) struct ViewEntry<'a, V> {
    pub(crate) handle: Handle,
    pub(crate) view: &'a Rc<V>,
}

/// An iterator over the tree's leaf views, in node-arena order (upstream `Iterator`).
pub(crate) struct Iter<'a, V> {
    nodes: &'a [Node<V>],
    i: usize,
}

impl<'a, V> Iterator for Iter<'a, V> {
    type Item = ViewEntry<'a, V>;

    fn next(&mut self) -> Option<ViewEntry<'a, V>> {
        while self.i < self.nodes.len() {
            let handle = Handle::from_index(self.i);
            self.i += 1;
            if let Node::Leaf(view) = &self.nodes[handle.idx()] {
                return Some(ViewEntry { handle, view });
            }
            // split → skip, advance to the next node (upstream's `self.next()` tail recursion).
        }
        None
    }
}

impl<V> SplitTree<V> {
    /// Iterate the tree's leaf views in node-arena order (upstream `iterator`).
    pub(crate) fn iter(&self) -> Iter<'_, V> {
        Iter {
            nodes: &self.nodes,
            i: 0,
        }
    }

    /// Set (or clear) the zoomed node (upstream `zoom`). Asserts the handle is in range.
    pub(crate) fn zoom(&mut self, handle: Option<Handle>) {
        if let Some(h) = handle {
            assert!(h.idx() < self.nodes.len(), "zoom handle out of range");
        }
        self.zoomed = handle;
    }

    /// The currently-zoomed node, if any (upstream's `zoomed` field).
    pub(crate) fn zoomed(&self) -> Option<Handle> {
        self.zoomed
    }
}

/// A navigation target for `goto` (upstream `Goto`): the previous / next view (optionally wrapped),
/// or a spatial direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Goto {
    Previous,
    Next,
    PreviousWrapped,
    NextWrapped,
    Spatial(SpatialDirection),
}
```

## Scope / faithfulness notes

- **Ported**: `iterator` / `Iterator` / `ViewEntry` → `SplitTree::iter` / `Iter`
  / `ViewEntry`; `zoom` (+ a `zoomed` getter); and the `Goto` enum.
- **Faithful**: the iterator visits node indices `0..len` in order, yielding a
  `ViewEntry` per leaf (`handle` = its index, `view` = the leaf's view) and
  skipping splits — upstream's `.split => self.next()` tail recursion becomes a
  `while` loop with the same effect. `zoom` records the handle with the in-range
  assertion. `Goto`'s five variants mirror upstream exactly.
- **Faithful adaptation**: `ViewEntry.view` is `&'a Rc<V>` (a borrow of the
  leaf's ref-counted view, the analogue of upstream's `*View`; a consumer can
  `Rc::clone` it to share). The `Iterator` trait impl replaces the hand-written
  `next`. `zoom`'s vacuous `>= 0` assertion (a `usize` is always `>= 0`) is
  dropped. A `zoomed` getter exposes the otherwise-private field.
- **Deferred**: `goto` itself (dispatches to `previous` / `next` /
  `previous_wrapped` / `next_wrapped` and, for `spatial`, builds the `Spatial`
  representation and calls `nearestWrapped`) — it lands with the tree-shaping
  ops and the `Spatial` container.
- No C ABI/header/ABI-inventory change (internal Rust). Extends
  `terminal::split_tree`.

## Changes

1. `roastty/src/terminal/split_tree.rs`: add `ViewEntry<'a, V>`, `Iter<'a, V>`
   (impl `Iterator`), `SplitTree::iter` / `zoom` / `zoomed`, and the `Goto`
   enum.
2. Tests (in `split_tree.rs`):
   - **iterate single leaf**: one entry, `handle == ROOT`, the view matches.
   - **iterate empty tree**: no entries.
   - **iterate a horizontal split of two leaves**: two entries with handles `1`
     and `2`, in order, skipping the split at index `0`; the views match `a`,
     `b`.
   - **iterate a nested tree**: all leaves visited in arena order, splits
     skipped.
   - **zoom**: `zoom(Some(h))` sets `zoomed() == Some(h)`; `zoom(None)` clears
     it; an out-of-range `zoom` panics (`#[should_panic]`).
   - **Goto variants**: the five variants are distinct (incl.
     `Spatial(SpatialDirection::Left)`).
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

- `Iter` visits leaves in node-arena order skipping splits, `zoom` records the
  handle (in-range assert), and `Goto` mirrors upstream's variants — faithful to
  `datastruct/split_tree.zig`;
- the tests pass (single / empty / horizontal / nested iteration / zoom / Goto),
  and the existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the iteration order, the leaf/split handling, the
`zoom` behavior, or the `Goto` variants diverge from upstream, an unrelated item
changes, or any public C API/ABI changes.

## Design Review

Codex reviewed the design and **approved it with no findings**. It confirmed the
iterator mapping is faithful (node-index order, leaf-only output, split
skipping; the `while` loop is observably equivalent to upstream's recursive
skip), that `ViewEntry { view: &Rc<V> }` is the right Rust surface for a
borrowed ref-counted view (callers can `Rc::clone` to retain it), that
`zoom(Some(h))` with the in-range assertion and `zoom(None)` clearing match
upstream (dropping the vacuous nonnegative assertion is correct for `usize`, and
the `zoomed()` getter is a harmless adaptation), that `Goto` mirrors the
upstream variants and deferring `goto()` itself is the right scope, and that the
test plan covers the meaningful behavior.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d577-prompt.md`
- Result: `logs/codex-review/20260604-d577-last-message.md`

## Result

**Result:** Pass

`terminal::split_tree` gained the leaf iterator and navigation vocabulary:
`ViewEntry<'a, V>` (`handle` + `view: &'a Rc<V>`), `Iter<'a, V>` (an `Iterator`
impl walking node indices `0..len`, yielding a `ViewEntry` per leaf and skipping
splits), `SplitTree::iter` / `zoom` / `zoomed`, and the `Goto` enum (`Previous`
/ `Next` / `PreviousWrapped` / `NextWrapped` / `Spatial(SpatialDirection)`). The
module doc comment was updated to reflect the iterator and `zoom` are now
landed.

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3187 passed, 0 failed (seven new tests; no
  regressions, up from 3180).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + terminal/split_tree.rs +
  lib.rs/header/abi_harness.c) clean; `git diff --check` clean.

The seven new tests: single-leaf iteration (`[(0, "v")]`), the empty tree
(`[]`), a horizontal split skipping the split at index 0
(`[(1, "a"), (2, "b")]`), a nested tree visiting all leaves in arena order with
both splits skipped (`[(2, "a"), (3, "b"), (4, "c")]`), `zoom` set/clear, the
out-of-range `zoom` panic, and `Goto` variant distinctness.

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no Required
or Optional findings** (two Nits, both fixed): the module doc comment still
listed `zoom` / the iterator as deferred (updated to leave only the
still-deferred `goto` method, shaping operations, spatial container
normalization/navigation, and formatters), and the `## Result` / `## Conclusion`
sections were not yet in the saved file (added here). Codex confirmed the
implementation matches upstream — arena-order, leaf-only iteration skipping
splits with the same observable behavior as upstream's recursive `next`;
`ViewEntry` correctly borrowing the leaf `Rc`; `zoom` enforcing the in-range
handle and clearing on `None`; and `Goto` mirroring the upstream vocabulary —
and that the tests cover the important cases including nested split skipping and
the zoom panic.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r577-prompt.md` (result)
- Result: `logs/codex-review/20260604-r577-last-message.md` (result)

## Conclusion

This experiment ports the split_tree leaf `iterator` (`Iter` / `ViewEntry`), the
`zoom` setter (and a `zoomed` getter), and the `Goto` navigation enum — the
fifth split_tree slice. The iterator is a faithful arena walk (leaf-only, splits
skipped) expressed as a Rust `Iterator`, and `ViewEntry` borrows the leaf's
`Rc<V>` so a consumer can clone it to share the view. The `Goto` enum's
consuming `goto` method stays deferred (it dispatches to the still-deferred
`previous` / `next` and, for `Spatial`, builds the `Spatial` container and calls
`nearest`). The next split_tree slices are the tree-shaping operations (`split`
/ `remove` — both arena rewrites over `Node<V>`, with `split` also using the
`Direction::split_layout` mapping and `Handle::offset` already ported), the
in-order `previous` / `next` traversal (the backtracking search), then the
`Spatial` container's `spatial` / `fillSpatialSlots` normalization (combining
the `Node` arena with the `Slot` / spatial geometry) and `nearest` /
`nearestWrapped`, then `goto`, and finally the formatters. The other remaining
big-ticket subsystem is the terminal **search subsystem** (coupled to `PageList`
/ `Pin` / `Screen` / `Selection` / `PageFormatter`); the dependency-blocked
helpers persist (regex/oniguruma for `Link::oniRegex`, a URI parser for
`os/uri`, the config-directory naming decision for `file_load` / `edit` /
`loadDefaultFiles`).
