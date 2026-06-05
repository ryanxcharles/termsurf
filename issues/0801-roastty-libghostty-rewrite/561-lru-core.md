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

# Experiment 561: the LRU cache core (Lru)

## Description

This experiment ports the **core** of upstream `datastruct/lru.zig` — `HashMap`,
a least-recently-used cache with capacity-bounded eviction. It's a hash map
keyed by `K` whose entries are kept in LRU order; a cache miss at capacity
evicts the least-recently-used entry. roastty uses this kind of cache for font
glyph / shaping results. This experiment ports the core (`init`, `get`,
`get_or_put_with` with eviction); `resize` (shrink-and-return-evicted) is
deferred. roastty homes its data structures under `terminal::`, so this lands at
`terminal::lru`.

## Upstream behavior

`datastruct/lru.zig` — `HashMap(K, V, Context, max_load)` (the LRU map): a
`std.HashMapUnmanaged(K, *Entry)` plus a `std.DoublyLinkedList` of
`Entry { data: KV, node }`, with `capacity`. Each entry's `node` is an intrusive
list node; `@fieldParentPtr` recovers the `Entry` from a `*node`.

- `init(capacity)`: empty map + empty queue.
- `get(key) ?V`: a plain map lookup returning the value. It does **not** change
  recency.
- `getOrPut(key) GetOrPutResult`:
  - If the key exists: move its node to the end of the queue
    (most-recently-used) and return
    `{ found_existing: true, value_ptr: &existing_value, evicted: null }`.
  - Else: it inserts the key into the map. `evict = map.count() > capacity`
    (i.e. the buffer was already at capacity). If **not** evicting, allocate a
    new `Entry`. If evicting, **reuse** the least-recently-used entry
    (`queue.popFirst`): remove its old key from the map and recycle its slot.
    Either way, store the entry in the map, append its node to the end
    (most-recently-used), set its key, and return
    `{ found_existing: false, value_ptr: &slot_value, evicted: <the recycled entry's KV, or null> }`.
    `value_ptr` points at space the caller writes.
- `GetOrPutResult { value_ptr: *V, found_existing: bool, evicted: ?KV }`.

The upstream `getOrPut` test (capacity 2): insert `1`, `2`;
`getOrPut(1)`/`getOrPut(2)` are hits that bump recency; `getOrPut(3)` evicts the
LRU (`1`); after bumping `2`, `getOrPut(4)` evicts `3`. The `get` test: `get(1)`
returns the value, `get(2)` is `null`.

## Rust mapping (`roastty/src/terminal/lru.rs`)

Rust can't do `@fieldParentPtr` intrusive lists or an uninitialized `value_ptr`
safely, so the faithful-behavior port uses an **arena** (a `Vec<Node>` indexed
by `usize`, with a `NIL` sentinel) for the doubly-linked LRU order, a
`HashMap<K, usize>` for lookup, and a **closure** `get_or_put_with` (the value
is computed on a miss — the safe equivalent of writing the uninitialized
`value_ptr`):

```rust
//! A least-recently-used cache (port of the core of upstream `datastruct/lru` `HashMap`).

use std::collections::HashMap;
use std::hash::Hash;

const NIL: usize = usize::MAX;

struct Node<K, V> {
    key: K,
    value: V,
    prev: usize, // NIL = none
    next: usize, // NIL = none
}

/// The result of `get_or_put_with` (upstream `GetOrPutResult`).
pub(crate) struct GetOrPut<'a, K, V> {
    /// The entry's value (the existing one on a hit, the freshly-inserted one on a miss).
    pub(crate) value: &'a mut V,
    /// Whether the key already existed.
    pub(crate) found_existing: bool,
    /// The entry evicted to make room, if any.
    pub(crate) evicted: Option<(K, V)>,
}

/// A capacity-bounded LRU cache (upstream `datastruct.lru.HashMap`). `head` is the
/// least-recently-used end, `tail` the most-recently-used.
pub(crate) struct Lru<K: Copy + Eq + Hash, V> {
    nodes: Vec<Node<K, V>>,
    map: HashMap<K, usize>,
    head: usize,
    tail: usize,
    capacity: usize,
}

impl<K: Copy + Eq + Hash, V> Lru<K, V> {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            nodes: Vec::new(),
            map: HashMap::new(),
            head: NIL,
            tail: NIL,
            capacity,
        }
    }

    /// Get a value without changing recency (upstream `get`).
    pub(crate) fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key).map(|&idx| &self.nodes[idx].value)
    }

    /// Get the entry for `key`, or insert it (computing the value with `make_value` only on a
    /// miss), evicting the least-recently-used entry if at capacity (upstream `getOrPut`).
    pub(crate) fn get_or_put_with<F: FnOnce() -> V>(
        &mut self,
        key: K,
        make_value: F,
    ) -> GetOrPut<'_, K, V> {
        if let Some(&idx) = self.map.get(&key) {
            self.bump(idx); // move to most-recently-used
            return GetOrPut {
                value: &mut self.nodes[idx].value,
                found_existing: true,
                evicted: None,
            };
        }

        // Compute the value up front (before mutating the cache) so a panic in `make_value`
        // can't leave the structure half-evicted; upstream similarly only writes the value
        // after `getOrPut` returns.
        let value = make_value();
        let at_capacity = self.map.len() >= self.capacity;
        let (idx, evicted) = if at_capacity {
            // Reuse the least-recently-used node's slot.
            let lru = self.head;
            self.unlink(lru);
            let old_key = self.nodes[lru].key;
            self.map.remove(&old_key);
            let old = std::mem::replace(
                &mut self.nodes[lru],
                Node { key, value, prev: NIL, next: NIL },
            );
            // Return the genuinely-evicted entry `(old.key, old.value)`. NOTE: upstream's
            // `evicted.key` is actually the *new* key (it assigns `entry.data.key = key` before
            // forming `evicted`), so only `evicted.value` is meaningful upstream — and that's all
            // its tests/consumers use. roastty intentionally corrects the key to the evicted one;
            // the value (the meaningful field) is identical.
            (lru, Some((old.key, old.value)))
        } else {
            // Allocate a new node.
            let idx = self.nodes.len();
            self.nodes.push(Node { key, value, prev: NIL, next: NIL });
            (idx, None)
        };

        self.link_tail(idx);
        self.map.insert(key, idx);
        GetOrPut {
            value: &mut self.nodes[idx].value,
            found_existing: false,
            evicted,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.map.len()
    }

    pub(crate) fn capacity(&self) -> usize {
        self.capacity
    }

    /// Unlink a node from the LRU list (helper).
    fn unlink(&mut self, idx: usize) {
        let prev = self.nodes[idx].prev;
        let next = self.nodes[idx].next;
        if prev != NIL {
            self.nodes[prev].next = next;
        } else {
            self.head = next;
        }
        if next != NIL {
            self.nodes[next].prev = prev;
        } else {
            self.tail = prev;
        }
        self.nodes[idx].prev = NIL;
        self.nodes[idx].next = NIL;
    }

    /// Append a node at the most-recently-used end (helper).
    fn link_tail(&mut self, idx: usize) {
        self.nodes[idx].prev = self.tail;
        self.nodes[idx].next = NIL;
        if self.tail != NIL {
            self.nodes[self.tail].next = idx;
        } else {
            self.head = idx;
        }
        self.tail = idx;
    }

    /// Move a node to the most-recently-used end (helper).
    fn bump(&mut self, idx: usize) {
        self.unlink(idx);
        self.link_tail(idx);
    }
}
```

The `get_or_put` reuse-on-evict (no allocation when full) is preserved: on a
miss at capacity, the `head` (LRU) node's slot is recycled via `mem::replace` —
the old `(key, value)` is moved out as `evicted`, the new entry's value comes
from `make_value`, and the slot is re-linked at the tail. Below capacity, a new
node is pushed. Because `get_or_put` only ever recycles (never frees) a slot,
the arena needs no free list (the `resize`-shrink case, which does free, is
deferred). `make_value` is called **only on a miss** — the safe equivalent of
upstream's "write `value_ptr` only when `!found_existing`" — and its value is
computed **before** the cache is mutated, so a panic in `make_value` cannot
leave the structure half-evicted.

**`evicted` key (intentional correction of an upstream quirk)**: upstream forms
`evicted = entry.data` _after_ assigning the new key (`entry.data.key = key`),
so its `evicted.key` is the **new** key, not the evicted one — only
`evicted.value` (the old value) is meaningful, and that's all upstream's
tests/consumers use. roastty returns the genuinely-evicted
`(old.key, old.value)`: the value matches upstream exactly, and the key is
corrected (so it isn't a footgun). This is the one deliberate divergence,
documented here and at the call site.

## Scope / faithfulness notes

- **Ported (bridged)**: `datastruct.lru.HashMap` core — `init` → `new`; `get`;
  `getOrPut` → `get_or_put_with`; `GetOrPutResult` → `GetOrPut`. Plus `len` /
  `capacity`.
- **Faithful**: `get` returns the value without changing recency; `get_or_put`
  bumps a hit to MRU; on a miss it evicts the LRU (reusing its slot) only when
  at capacity, appends the new entry at MRU, and reports the evicted **value**
  (the meaningful field; see the `evicted`-key correction below).
- **Intentional divergence (documented)**: `evicted` returns the
  genuinely-evicted `(old.key, old.value)`, whereas upstream's `evicted.key` is
  erroneously the _new_ key (an ordering quirk — it assigns the new key before
  forming `evicted`). The value — the only field upstream's tests/consumers use
  — is identical.
- **Faithful adaptation**: the intrusive `DoublyLinkedList` +
  `HashMap<K, *Entry>` (raw pointers, `@fieldParentPtr`) → an **arena**
  `Vec<Node>` with `usize` indices (`NIL` sentinel) for the list +
  `HashMap<K, usize>`; the uninitialized `value_ptr` the caller writes → a
  `make_value` **closure** computed only on a miss (the value returned as a
  `&mut V` for further mutation); `Allocator.Error` → infallible (Rust
  `Vec`/`HashMap` allocation aborts); `K: Copy + Eq + Hash` (cache keys are
  small/copyable; upstream stores the key by value in each entry).
- **Deferred**: `resize` (grow is trivial, but shrink frees the `delta` LRU
  entries and returns them — it needs the arena's free-slot handling, a
  follow-up slice); the `Context`-based `getOrPutContext` / `getContext`
  (roastty uses `AutoHashMap`, i.e. the derived `Hash`/`Eq` — Rust's `Hash + Eq`
  bound is that).
- No C ABI/header/ABI-inventory change (internal Rust). New `terminal::lru`
  module.

## Changes

1. `roastty/src/terminal/lru.rs` (new): `Lru`, `GetOrPut`, the `Node` arena +
   list helpers.
2. `roastty/src/terminal/mod.rs`: add `#[allow(dead_code)] mod lru;`.
3. Tests (in `lru.rs`), porting the upstream `getOrPut` / `get` tests over
   `Lru<u32, u8>`:
   - **get_or_put sequence (capacity 2)**: `get_or_put_with(1, ||1)` and
     `(2, ||2)` are misses (`!found_existing`, no evicted);
     `get_or_put_with(1, …)` / `(2, …)` are hits (`found_existing`) that bump
     recency; `get_or_put_with(3, ||3)` evicts the LRU, with
     `evicted == Some((1, 1))`; after a hit on `2`, `get_or_put_with(4, ||4)`
     evicts with `evicted` value `3`.
   - **make_value not called on a hit**: a hit's `make_value` closure (which
     would `panic!`) is never invoked.
   - **get**: after inserting `1`, `get(&1) == Some(&1)` and `get(&2) == None`;
     `get` does not change recency (a `get` then a capacity-forcing miss still
     evicts the true LRU).
   - **len / capacity**: track the entry count and the fixed capacity.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty terminal::lru
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/terminal/lru.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `Lru` keeps entries in LRU order, `get` reads without changing recency, and
  `get_or_put_with` hits-bump / misses-insert-evicting-the-LRU-when-full
  (computing the value only on a miss and reporting the evicted entry) —
  faithful to `datastruct/lru.zig`'s `HashMap` core;
- the tests pass (the upstream get_or_put / get sequences +
  make_value-only-on-miss + len), and the existing tests still pass;
- `resize` and the `Context` variants stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the recency / eviction order, the miss-only value
computation, or the evicted reporting diverges from upstream, an unrelated item
changes, or any public C API/ABI changes.

## Design Review

Codex's first design review raised **one Required** finding (and an Optional),
both addressed; the corrected design was **re-reviewed and approved with no
findings**.

- **`evicted.key` divergence (Required, addressed)**: upstream's `evicted.key`
  is erroneously the _new_ key (it assigns `entry.data.key = key` before forming
  `evicted`), so only `evicted.value` is meaningful and used. The design now
  **documents** that roastty intentionally returns the genuinely-evicted
  `(old.key, old.value)` — the value matches upstream exactly, the key is
  corrected — as a deliberate fix (in prose, scope notes, and at the call site),
  rather than an accidental divergence.
- **(Optional, adopted)**: `make_value()` is computed **before** any list/map
  mutation on the miss path, so a panic in it cannot leave the cache
  half-evicted (resolving the unwind-safety concern).

On re-review Codex confirmed the corrected design is faithful for recency,
capacity, slot reuse, and miss-only value construction, the documented
`evicted`-key correction is acceptable, and the updated cap-2 test expectation
is coherent.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d561-prompt.md` (design),
  `logs/codex-review/20260604-d561b-prompt.md` (design re-review)
- Result: `logs/codex-review/20260604-d561-last-message.md` (design),
  `logs/codex-review/20260604-d561b-last-message.md` (design re-review)
