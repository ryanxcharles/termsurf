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

# Experiment 555: the fixed-bucket LRU cache (CacheTable)

## Description

This experiment ports upstream `datastruct/cache_table.zig` — `CacheTable`, an
associative cache with **fixed-size buckets and within-bucket LRU eviction**. It
is a hash table where each bucket holds at most `BUCKET_SIZE` entries; inserting
into a full bucket evicts the least-recently-used entry, and a lookup bumps the
found entry to the most-recent position. It's a cheap, allocation-free cache for
recomputable values (the kind of structure roastty uses for glyph / shaping
caches). roastty homes its data structures under `terminal::` (like
`offset_hash_map`, `ref_counted_set`), so this lands at `terminal::cache_table`.

## Upstream behavior

`datastruct/cache_table.zig` —
`CacheTable(K, V, Context, bucket_count, bucket_size)`:

- Storage: `buckets[bucket_count][bucket_size]` of `KV{key, value}` +
  `lengths[bucket_count]` (used slots per bucket). `bucket_count` must be a
  power of two.
- `Context` provides `hash(K) u64` and `eql(K, K) bool` (and an **optional**
  `evicted(K, V)` hook).
- `put(key, value) ?KV`: `idx = hash(key) % bucket_count`. If the bucket has
  room (`length < bucket_size`), append and return `null`. Otherwise `rotateIn`
  — append the new entry at the end, shifting the bucket left by one, and return
  (and `evicted`-hook) the entry that fell off the front (index 0, the LRU).
- `get(key) ?V`: scan the bucket from the **end** (most recent) toward the
  start; on a match at `i`, `rotateOnce(bucket[i..len])` (rotate that sub-slice
  left once, moving the found entry to the end = most-recent) and return its
  value; else `null`.
- `clear()`: (call the `evicted` hook for every entry, then) reset all lengths
  to `0`.

The upstream test (identity hash, `2×2` table): fill with keys `0..=3` (each
`put` returns `null`); a 5th `put(4, …)` evicts the oldest (`{0,0}`); `get(0)`
is then `null`.

## Rust mapping (`roastty/src/terminal/cache_table.rs`)

A const-generic struct over `BUCKET_COUNT` / `BUCKET_SIZE`, parameterized by a
`CacheContext` trait (`hash` + `eql`). The optional `evicted` hook is dropped in
favor of `put` **returning** the evicted entry (upstream already returns it as
`?KV` — the idiomatic Rust equivalent of the callback). Keys and values are
`Copy` (upstream stores them by value in fixed arrays):

```rust
//! A fixed-bucket cache with within-bucket LRU eviction (port of upstream
//! `datastruct/cache_table`).

/// Provides hashing and equality for a `CacheTable` key (upstream's `Context`).
pub(crate) trait CacheContext<K> {
    fn hash(&self, key: &K) -> u64;
    fn eql(&self, a: &K, b: &K) -> bool;
}

/// An associative cache with `BUCKET_COUNT` fixed-size buckets of `BUCKET_SIZE` entries each;
/// a full bucket evicts its least-recently-used entry on insert (upstream `CacheTable`).
pub(crate) struct CacheTable<
    K: Copy,
    V: Copy,
    C: CacheContext<K>,
    const BUCKET_COUNT: usize,
    const BUCKET_SIZE: usize,
> {
    buckets: [[Option<(K, V)>; BUCKET_SIZE]; BUCKET_COUNT],
    lengths: [u8; BUCKET_COUNT],
    context: C,
}

impl<K: Copy, V: Copy, C: CacheContext<K>, const BUCKET_COUNT: usize, const BUCKET_SIZE: usize>
    CacheTable<K, V, C, BUCKET_COUNT, BUCKET_SIZE>
{
    pub(crate) fn new(context: C) -> Self {
        assert!(BUCKET_COUNT.is_power_of_two(), "bucket_count must be a power of two");
        assert!(BUCKET_SIZE >= 1 && BUCKET_SIZE <= u8::MAX as usize, "invalid bucket_size");
        Self {
            buckets: [[None; BUCKET_SIZE]; BUCKET_COUNT],
            lengths: [0; BUCKET_COUNT],
            context,
        }
    }

    /// Insert `(key, value)`. If a full bucket forced an eviction, the removed entry is
    /// returned (upstream's `?KV` / `evicted` hook).
    pub(crate) fn put(&mut self, key: K, value: V) -> Option<(K, V)> {
        let idx = (self.context.hash(&key) % BUCKET_COUNT as u64) as usize;
        let len = self.lengths[idx] as usize;

        if len < BUCKET_SIZE {
            self.buckets[idx][len] = Some((key, value));
            self.lengths[idx] += 1;
            return None;
        }

        // Full bucket: evict the front (LRU), shift left, append the new entry at the end.
        let evicted = self.buckets[idx][0].take();
        for i in 1..BUCKET_SIZE {
            self.buckets[idx][i - 1] = self.buckets[idx][i].take();
        }
        self.buckets[idx][BUCKET_SIZE - 1] = Some((key, value));
        evicted
    }

    /// Look up `key`, returning its value (and bumping it to most-recently-used) or `None`.
    pub(crate) fn get(&mut self, key: K) -> Option<V> {
        let idx = (self.context.hash(&key) % BUCKET_COUNT as u64) as usize;
        let len = self.lengths[idx] as usize;

        // Scan from the most-recent (end) toward the start.
        let mut i = len;
        while i > 0 {
            i -= 1;
            let (k, v) = self.buckets[idx][i].expect("slots below the length are populated");
            if self.context.eql(&key, &k) {
                // Bump the found entry to the end (most-recent): rotate [i..len] left once.
                for j in i + 1..len {
                    self.buckets[idx][j - 1] = self.buckets[idx][j].take();
                }
                self.buckets[idx][len - 1] = Some((k, v));
                return Some(v);
            }
        }
        None
    }

    /// Remove all entries.
    pub(crate) fn clear(&mut self) {
        for idx in 0..BUCKET_COUNT {
            for slot in 0..self.lengths[idx] as usize {
                self.buckets[idx][slot] = None;
            }
            self.lengths[idx] = 0;
        }
    }
}
```

`put` and `get` reproduce upstream's `rotateIn` / `rotateOnce` exactly (shift
the bucket left by one, appending at the end). The fixed
`[[Option<(K, V)>; BUCKET_SIZE]; BUCKET_COUNT]` is the allocation-free
equivalent of Zig's `[bucket_count][bucket_size]KV` + `lengths`; slots below
`lengths[idx]` are always `Some`. `% BUCKET_COUNT` matches upstream's modulo (it
uses `%`, not a power-of-two mask, even though the count is asserted to be a
power of two).

## Scope / faithfulness notes

- **Ported (bridged)**: `datastruct.CacheTable` →
  `terminal::cache_table::CacheTable` with `put` / `get` / `clear`; the
  `Context` (`hash` / `eql`) → a `CacheContext` trait.
- **Faithful**: `idx = hash % bucket_count`; append-when-room; full-bucket LRU
  eviction via rotate-in (return the evicted entry); `get`'s most-recent-first
  scan + rotate-to-end LRU bump; `clear` resetting all buckets.
- **Faithful adaptation**: the comptime `CacheTable(K,V,Context,count,size)` → a
  const-generic `CacheTable<K, V, C, BUCKET_COUNT, BUCKET_SIZE>`; the fixed Zig
  arrays → fixed Rust arrays of `Option<(K, V)>` (K, V `Copy`, as upstream
  stores by value); upstream's `rotateIn` / `rotateOnce` → in-place left-shift
  loops; the optional `evicted` **callback** → `put` **returning** the evicted
  entry (upstream returns it too); the power-of-two / size asserts preserved.
- **Deferred / dropped**: the `evicted`-hook-on-`clear` notification (the
  callback mechanism is replaced by `put`'s return; `clear` drops `Copy` entries
  without per-item notification — documented); upstream's `AutoContext`
  convenience (callers provide a `CacheContext`).
- No C ABI/header/ABI-inventory change (internal Rust). New
  `terminal::cache_table` module.

## Changes

1. `roastty/src/terminal/cache_table.rs` (new): `CacheContext`, `CacheTable`
   (`new` / `put` / `get` / `clear`).
2. `roastty/src/terminal/mod.rs`: add `#[allow(dead_code)] mod cache_table;`.
3. Tests (in `cache_table.rs`, with an identity-hash `CacheContext` over `u32`
   and a `2×2` table):
   - **upstream test**: `put(0..=3, 0)` each return `None`; `put(4, 0)` returns
     `Some((0, 0))` (the evicted LRU); `get(0)` is then `None`.
   - **lookup hit / miss**: `get` of a present key returns its value; `get` of
     an absent key is `None`.
   - **LRU bump**: with bucket `[0, 2]`, `get(0)` bumps `0` to most-recent
     (`[2, 0]`); a subsequent `put` into that bucket evicts `2` (now the LRU),
     not `0` (`get(2)` ⇒ `None`, `get(0)` ⇒ `Some`).
   - **clear**: after `clear()`, all previously-present keys `get` ⇒ `None`.
4. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty cache_table
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/terminal/cache_table.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `CacheTable` buckets by `hash % bucket_count`, appends when there's room,
  evicts the LRU (returning it) when a bucket is full, bumps a found entry to
  most-recent on `get`, and `clear`s all buckets — faithful to
  `datastruct/cache_table.zig`;
- the tests pass (the upstream test + hit/miss + LRU bump + clear), and the
  existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the bucketing, the LRU eviction / bump, or the
`clear` behavior diverges from upstream, an unrelated item changes, or any
public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. Codex confirmed the mechanics match upstream — `put` appends while
there's room, full buckets evict index `0` and shift left, and `get` scans
most-recent to least-recent then rotates the hit to the end before returning the
copied value; the `Option<(K, V)>` + `lengths` invariant is a sound Rust
stand-in for Zig's uninitialized fixed arrays; and `% BUCKET_COUNT` preserves
upstream's modulo behavior. Codex agreed the `Copy` bounds and the dropped
clear-time eviction hook are acceptable for this slice (Zig's table
stores/rotates by value, real cache use handles cleanup outside `CacheTable`,
and `put` still returns the evicted entry), and that the tests cover the
upstream case plus the LRU-bump and clear behaviors.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d555-prompt.md` (design)
- Result: `logs/codex-review/20260604-d555-last-message.md` (design)
