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

# Experiment 610: datastruct ArrayListCollection

## Description

Another self-contained, dependency-free port taken while the search subsystem's
last piece (the outer libxev `Thread`) stays blocked: upstream
`datastruct/array_list_collection.zig`. `ArrayListCollection(T)` is a fixed-size
collection of growable lists with a bulk `reset` that clears every list while
retaining its capacity. Upstream uses it in `renderer/cell.zig` (a yet-unported
subsystem), so landing it now keeps the datastruct layer ahead of its consumers.

It joins roastty's other `datastruct/*` ports, which live in `terminal/` (e.g.
`blocking_queue`, `circ_buf`, `segmented_pool`).

## Upstream behavior (`array_list_collection.zig`)

```zig
pub fn ArrayListCollection(comptime T: type) type {
    return struct {
        const ArrayListT = std.ArrayListUnmanaged(T);
        lists: []ArrayListT,

        pub fn init(alloc, list_count, initial_capacity) !Self {
            const self = .{ .lists = try alloc.alloc(ArrayListT, list_count) };
            for (self.lists) |*list| list.* = try .initCapacity(alloc, initial_capacity);
            return self;
        }

        pub fn deinit(self, alloc) void {
            for (self.lists) |*list| list.deinit(alloc);
            alloc.free(self.lists);
        }

        /// Clear all lists in the collection, retaining capacity.
        pub fn reset(self) void {
            for (self.lists) |*list| list.clearRetainingCapacity();
        }
    };
}
```

## Rust mapping (`roastty/src/terminal/array_list_collection.rs`, new file)

`std.ArrayListUnmanaged(T)` → `Vec<T>`; the outer `[]ArrayListT` →
`Vec<Vec<T>>`. Allocation is infallible (`Allocator.Error` drops) and `Drop`
replaces `deinit`, so there is no explicit teardown. `Vec::clear` retains
capacity (matching `clearRetainingCapacity`). Upstream exposes the `lists` field
directly; roastty exposes it through accessors.

```rust
//! A fixed-size collection of growable lists with a bulk capacity-retaining `reset` (port of
//! upstream `datastruct/array_list_collection`).

/// A collection of `list_count` growable lists, supporting a bulk `reset` that clears every list
/// while retaining its allocated capacity (upstream `ArrayListCollection`).
pub(crate) struct ArrayListCollection<T> {
    lists: Vec<Vec<T>>,
}

impl<T> ArrayListCollection<T> {
    /// Create a collection of `list_count` empty lists, each pre-allocated to `initial_capacity`
    /// (upstream `init`).
    pub(crate) fn new(list_count: usize, initial_capacity: usize) -> Self {
        let mut lists = Vec::with_capacity(list_count);
        for _ in 0..list_count {
            lists.push(Vec::with_capacity(initial_capacity));
        }
        Self { lists }
    }

    /// Clear every list, retaining its capacity (upstream `reset`).
    pub(crate) fn reset(&mut self) {
        for list in &mut self.lists {
            list.clear();
        }
    }

    /// The number of lists in the collection.
    pub(crate) fn len(&self) -> usize {
        self.lists.len()
    }

    /// The lists, immutably (upstream's public `lists` field).
    pub(crate) fn lists(&self) -> &[Vec<T>] {
        &self.lists
    }

    /// The lists, mutably (so callers can append to a chosen list).
    pub(crate) fn lists_mut(&mut self) -> &mut [Vec<T>] {
        &mut self.lists
    }
}
```

Registered in `terminal/mod.rs` as
`#[allow(dead_code)] mod array_list_collection;` (alphabetically, between `ansi`
and `bitmap_allocator`).

### Notes / deviations

- `Drop` replaces `deinit`; allocation is infallible so `init`'s error union is
  gone (`new` returns `Self`).
- `Vec::clear` retains capacity (upstream `clearRetainingCapacity`), so `reset`
  is a faithful bulk clear.
- Upstream's public `lists` field becomes `lists()` / `lists_mut()` / `len()`
  accessors (roastty avoids public fields on its datastruct ports).

## Verification

- `cargo build -p roastty` — no warnings.
- `cargo test -p roastty` — no regressions; new tests:
  - `new_creates_empty_lists_with_capacity` — `new(3, 8)` yields 3 empty lists,
    each with `capacity() >= 8`.
  - `reset_clears_all_lists_retaining_capacity` — push into each list, `reset`,
    then every list is empty but `capacity() >= 8` is retained.
  - `lists_mut_allows_independent_appends` — appending to one list does not
    affect the others; `lists()` reflects the contents.
- `cargo fmt -p roastty -- --check` — clean.
- no-ghostty grep on the new file / `terminal/mod.rs` — clean.
- `git diff --check` — clean.

Pass = the collection constructs `list_count` capacity-`initial_capacity` lists,
supports independent per-list appends, and `reset` empties every list while
retaining capacity.

## Design Review

Codex reviewed the design and **APPROVED** it with **no Required findings**,
confirming the mapping is faithful: `Vec<Vec<T>>` matches the fixed outer
collection of growable lists; `Vec::with_capacity(initial_capacity)` matches
per-list `initCapacity`; `Vec::clear()` retains allocation like
`clearRetainingCapacity`; `Drop` is the right replacement for explicit `deinit`;
the `lists()` / `lists_mut()` / `len()` accessors preserve the upstream
public-field behavior without exposing the field; and generic
`ArrayListCollection<T>` is the natural equivalent of the comptime-generic type.

- **Optional (deferred)**: register as `pub(crate) mod` if a non-`terminal`
  consumer (the renderer) later needs it. Kept as a private
  `#[allow(dead_code)] mod`, consistent with roastty's nearby datastruct ports
  (`blocking_queue`, `message_data`, …); the visibility can widen in the same
  slice that ports the renderer consumer.

Review artifacts:

- Prompt: `logs/codex-review/20260605-d610-prompt.md`
- Result: `logs/codex-review/20260605-d610-last-message.md`

## Result

**Result:** Pass

Implemented `roastty/src/terminal/array_list_collection.rs` (registered
`#[allow(dead_code)] mod array_list_collection;` in `terminal/mod.rs`), porting
upstream `ArrayListCollection(T)` as a generic `ArrayListCollection<T>` over
`Vec<Vec<T>>`: `new(list_count, initial_capacity)` (each inner `Vec` pre-sized),
`reset` (`Vec::clear` each, retaining capacity), and `len` / `lists` /
`lists_mut` accessors. `Drop` replaces `deinit`; allocation is infallible so
`new` returns `Self` directly.

Three tests cover construction (N empty lists at capacity), the bulk
capacity-retaining `reset` (now asserting each list's exact pre-reset capacity
is unchanged — the adopted Optional), and independent per-list appends. Gates:
`cargo fmt --check` clean, `cargo build -p roastty` no warnings,
`cargo test -p roastty` **3358 passed / 0 failed** (3355 → 3358, +3), no-ghostty
grep clean, `git diff --check` clean.

## Completion Review

Codex reviewed the completed experiment and **APPROVED** it with **no Required
findings**, confirming the port is faithful: `Vec<Vec<T>>` maps the fixed outer
slice of growable lists; `new` creates exactly `list_count` pre-allocated empty
lists; `reset` uses `Vec::clear` (matching `clearRetainingCapacity`); `Drop`
covers cleanup; the accessor surface matches the upstream public `lists` field;
and the private module registration is consistent with nearby datastruct ports.

- **Optional (adopted)**: the retention test now records each list's exact
  capacity before `reset` and asserts it is unchanged after.

Review artifacts:

- Prompt: `logs/codex-review/20260605-r610-prompt.md`
- Result: `logs/codex-review/20260605-r610-last-message.md`

## Conclusion

`datastruct/array_list_collection` is fully ported — a clean, dependency-free
slice keeping roastty's datastruct layer ahead of its (yet-unported) renderer
consumer. The dependency boundaries that gate the larger remaining work are
unchanged: the outer search `Thread` (libxev), regex/oniguruma, and the URI
parser (`std.Uri`). roastty's other self-contained, unblocked modules —
remaining `os/` (`locale`, `resourcesdir`, …) and any further `datastruct/`
utilities — remain the natural next slices for Issue 801, which stays open and
broad.
