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

# Experiment 564: the doubly-linked list (arena port of the intrusive list)

## Description

This experiment ports upstream `datastruct/intrusive_linked_list.zig` —
`DoublyLinkedList`, an intrusive doubly-linked list adapted from Zig's standard
library (with the unused functionality removed). roastty's sixth `datastruct/`
port. It lands at `terminal::intrusive_linked_list` (the upstream file name, for
traceability), as an **arena-owned** list — the intrusive raw-pointer design
does not translate to safe Rust, so the faithful equivalent owns its nodes in an
arena and hands out `usize` handles (the idiom already used inside `Lru`).

## Upstream behavior

`datastruct/intrusive_linked_list.zig` — `DoublyLinkedList(T)`. The node type
**is** `T`, which must have `prev: ?*T` and `next: ?*T` fields; the list itself
holds only `first: ?*Node` and `last: ?*Node`. The caller owns each node's
storage; the list only rewires the pointers.

- `insertAfter(node, new_node)`: link `new_node` after `node` (updating
  `node.next`, the old successor's `prev`, or `list.last` when `node` was last).
- `insertBefore(node, new_node)`: link `new_node` before `node` (symmetric,
  updating `list.first` when `node` was first).
- `append(new_node)`: `insertAfter(last, …)`, or `prepend` when the list is
  empty.
- `prepend(new_node)`: `insertBefore(first, …)`, or (empty list) set
  `first = last = new_node` with null `prev`/`next`.
- `remove(node)`: rewire the neighbors past `node` (updating `list.first` /
  `list.last` at the ends). It does **not** free the node — the caller still
  owns it.
- `pop()`: remove and return `last` (or `null` if empty).
- `popFirst()`: remove and return `first` (or `null` if empty).

The upstream test builds `{1, 2, 3, 4, 5}` via `append(2)`, `append(5)`,
`prepend(1)`, `insertBefore(5, 4)`, `insertAfter(2, 3)`, traverses forwards
(`first` → `next`) checking `1, 2, 3, 4, 5`, traverses backwards (`last` →
`prev`) checking `5, 4, 3, 2, 1`, then `popFirst()` → `{2, 3, 4, 5}`, `pop()` →
`{2, 3, 4}`, `remove(3)` → `{2, 4}`, and asserts `first == 2`, `last == 4`.

## Rust mapping (`roastty/src/terminal/intrusive_linked_list.rs`)

The arena (`Vec<Option<Node<T>>>` + a `free: Vec<usize>` list, with a
`NIL = usize::MAX` sentinel) owns the nodes; the list holds `first` / `last`
indices. Handles are `usize` indices (the analogue of upstream's `*Node`).
Because the caller no longer allocates nodes separately, `append` / `prepend` /
`insert_after` / `insert_before` both **allocate and link**, returning the new
node's handle; `remove` / `pop` / `pop_first` **unlink and free**, returning the
node's data. Traversal mirrors the upstream `first`/`last`/`next`/`prev` walk
via `Option<usize>` handles (`None` hides the `NIL` sentinel from callers).

```rust
//! A doubly-linked list (arena port of upstream `datastruct/intrusive_linked_list`).
//!
//! Upstream's list is *intrusive*: the node type is `T` itself (with `prev`/`next` pointers) and
//! the caller owns each node. That raw-pointer design does not translate to safe Rust, so this
//! port owns its nodes in an arena (`Vec<Option<Node>>` + a free list, `NIL` sentinel) and hands
//! out `usize` handles — the analogue of upstream's `*Node`. The list order and operations match
//! upstream exactly.

const NIL: usize = usize::MAX;

struct Node<T> {
    data: T,
    prev: usize, // NIL = none
    next: usize, // NIL = none
}

/// A doubly-linked list owning its nodes in an arena (upstream `DoublyLinkedList`). Handles are
/// `usize` indices returned by the insert methods.
pub(crate) struct DoublyLinkedList<T> {
    nodes: Vec<Option<Node<T>>>,
    free: Vec<usize>,
    first: usize,
    last: usize,
}

impl<T> DoublyLinkedList<T> {
    pub(crate) fn new() -> Self {
        Self { nodes: Vec::new(), free: Vec::new(), first: NIL, last: NIL }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.first == NIL
    }

    fn node(&self, idx: usize) -> &Node<T> {
        self.nodes[idx].as_ref().expect("occupied slot")
    }
    fn node_mut(&mut self, idx: usize) -> &mut Node<T> {
        self.nodes[idx].as_mut().expect("occupied slot")
    }

    /// Allocate a node (reusing a freed slot if available), returning its handle.
    fn alloc(&mut self, node: Node<T>) -> usize {
        match self.free.pop() {
            Some(idx) => {
                self.nodes[idx] = Some(node);
                idx
            }
            None => {
                self.nodes.push(Some(node));
                self.nodes.len() - 1
            }
        }
    }

    /// Insert `data` after the node at `handle`, returning the new handle (upstream `insertAfter`).
    pub(crate) fn insert_after(&mut self, handle: usize, data: T) -> usize {
        let next = self.node(handle).next;
        let idx = self.alloc(Node { data, prev: handle, next });
        if next != NIL {
            self.node_mut(next).prev = idx;
        } else {
            self.last = idx;
        }
        self.node_mut(handle).next = idx;
        idx
    }

    /// Insert `data` before the node at `handle`, returning the new handle (upstream
    /// `insertBefore`).
    pub(crate) fn insert_before(&mut self, handle: usize, data: T) -> usize {
        let prev = self.node(handle).prev;
        let idx = self.alloc(Node { data, prev, next: handle });
        if prev != NIL {
            self.node_mut(prev).next = idx;
        } else {
            self.first = idx;
        }
        self.node_mut(handle).prev = idx;
        idx
    }

    /// Append `data` at the end, returning its handle (upstream `append`).
    pub(crate) fn append(&mut self, data: T) -> usize {
        if self.last != NIL {
            self.insert_after(self.last, data)
        } else {
            self.prepend(data)
        }
    }

    /// Prepend `data` at the beginning, returning its handle (upstream `prepend`).
    pub(crate) fn prepend(&mut self, data: T) -> usize {
        if self.first != NIL {
            self.insert_before(self.first, data)
        } else {
            let idx = self.alloc(Node { data, prev: NIL, next: NIL });
            self.first = idx;
            self.last = idx;
            idx
        }
    }

    /// Rewire the list past `handle` (upstream `remove`'s pointer fixups; does not free).
    fn unlink(&mut self, handle: usize) {
        let prev = self.node(handle).prev;
        let next = self.node(handle).next;
        if prev != NIL {
            self.node_mut(prev).next = next;
        } else {
            self.first = next;
        }
        if next != NIL {
            self.node_mut(next).prev = prev;
        } else {
            self.last = prev;
        }
    }

    /// Remove the node at `handle` and return its data (upstream `remove`, plus freeing the slot —
    /// the handle is invalid afterward).
    pub(crate) fn remove(&mut self, handle: usize) -> T {
        self.unlink(handle);
        let node = self.nodes[handle].take().expect("occupied slot");
        self.free.push(handle);
        node.data
    }

    /// Remove and return the last node's data (upstream `pop`).
    pub(crate) fn pop(&mut self) -> Option<T> {
        if self.last == NIL {
            None
        } else {
            Some(self.remove(self.last))
        }
    }

    /// Remove and return the first node's data (upstream `popFirst`).
    pub(crate) fn pop_first(&mut self) -> Option<T> {
        if self.first == NIL {
            None
        } else {
            Some(self.remove(self.first))
        }
    }

    // Traversal (hides the NIL sentinel behind `Option`).
    pub(crate) fn first(&self) -> Option<usize> {
        (self.first != NIL).then_some(self.first)
    }
    pub(crate) fn last(&self) -> Option<usize> {
        (self.last != NIL).then_some(self.last)
    }
    pub(crate) fn next(&self, handle: usize) -> Option<usize> {
        let n = self.node(handle).next;
        (n != NIL).then_some(n)
    }
    pub(crate) fn prev(&self, handle: usize) -> Option<usize> {
        let p = self.node(handle).prev;
        (p != NIL).then_some(p)
    }
    pub(crate) fn get(&self, handle: usize) -> &T {
        &self.node(handle).data
    }
    pub(crate) fn get_mut(&mut self, handle: usize) -> &mut T {
        &mut self.node_mut(handle).data
    }
}
```

## Scope / faithfulness notes

- **Ported (bridged)**: `datastruct.IntrusiveDoublyLinkedList`
  (`DoublyLinkedList`) → `terminal::intrusive_linked_list::DoublyLinkedList`
  (`insert_after`, `insert_before`, `append`, `prepend`, `remove`, `pop`,
  `pop_first`, plus the `first`/`last`/`next`/`prev`/`get`/`get_mut` traversal
  accessors).
- **Faithful**: the list order and every operation's neighbor-rewiring
  (including the `first`/`last` end updates and the empty-list `prepend` path)
  match upstream exactly; forward (`first` → `next`) and backward (`last` →
  `prev`) traversal reproduce the same sequences.
- **Faithful adaptation**: upstream is _intrusive_ (node type `T` carries
  `prev`/`next` raw pointers, the caller owns each node). That does not
  translate to safe Rust, so this port is **arena-owned**: a
  `Vec<Option<Node<T>>>` + a `free: Vec<usize>` list owns the nodes, `usize`
  handles replace `*Node`, and `Option<Node>` + the free list represent freed
  slots (the same idiom as `Lru`). Consequences: insert methods both allocate
  and link (returning a handle); `remove` / `pop` / `pop_first` free the slot
  and return the node's data (upstream leaves the node to the caller). A handle
  to a removed node is invalid afterward — accessing it panics (`expect`), which
  is safer than upstream's use-after-free UB.
- **Deferred**: nothing for this type (the upstream file is small and fully
  covered). The arena-owned model does not support a single node living in two
  lists simultaneously, which upstream's intrusive design technically allows; no
  roastty consumer needs that.
- No C ABI/header/ABI-inventory change (internal Rust). Adds
  `terminal::intrusive_linked_list`.

## Changes

1. `roastty/src/terminal/intrusive_linked_list.rs` (new): `DoublyLinkedList<T>`
   as above.
2. `roastty/src/terminal/mod.rs`: add
   `#[allow(dead_code)] mod intrusive_linked_list;` (alphabetical).
3. Tests (in `intrusive_linked_list.rs`):
   - **the upstream sequence**: build `{1, 2, 3, 4, 5}` via
     `append`/`prepend`/`insert_before`/ `insert_after`, traverse forwards and
     backwards checking the sequences, then `pop_first` / `pop` / `remove(3)`
     leaving `{2, 4}` with `first == 2`, `last == 4`.
   - **empty list**: `pop` / `pop_first` on an empty list return `None`;
     `is_empty` toggles.
   - **single element**: `append` one, `pop_first` returns it, list becomes
     empty.
   - **get_mut**: mutate a node's data through `get_mut`, observe it via `get`.
   - **free-slot reuse**: after removing a node, a subsequent insert reuses the
     freed slot without corrupting the order.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty terminal::intrusive_linked_list
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/terminal/intrusive_linked_list.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `DoublyLinkedList` reproduces upstream's list operations and ordering (insert
  after/before, append, prepend, remove, pop, pop_first) including the
  `first`/`last` end updates and the empty-list path, with forward and backward
  traversal — faithful to `datastruct/intrusive_linked_list.zig`;
- the tests pass (the upstream sequence / empty / single / get_mut / free-slot
  reuse), and the existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the list operations, the neighbor-rewiring, the end
(`first`/`last`) updates, or the traversal order diverge from upstream, an
unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed the design and **approved it with no findings**. It confirmed the
arena-owned design is a faithful safe-Rust adaptation of the upstream intrusive
list: the insert/remove rewiring is correct for the middle, first, last, empty,
and single-node cases; `append` / `prepend` delegation matches upstream; and
`pop` / `pop_first` correctly remove from the ends. Reading neighbor handles
before allocation is sound because free slots are not in the list, so `alloc`
cannot alias an occupied neighbor. The documented divergences are acceptable
(handles replace raw node pointers, removed handles panic instead of becoming
UB, and `remove` returns owned data while freeing the arena slot), the traversal
API is adequate for reproducing upstream's pointer walks, and the test plan
covers the upstream sequence plus the key arena/free-list edge cases.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d564-prompt.md`
- Result: `logs/codex-review/20260604-d564-last-message.md`
