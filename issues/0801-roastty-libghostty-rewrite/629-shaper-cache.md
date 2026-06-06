+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 629: the shaper cache

## Description

Port upstream `font/shaper/Cache.zig`: the shaped-run cache keyed by
`TextRun.hash`.

Issue 801's font checklist still says:

```markdown
- [ ] Shaper (CoreText shaping, run, cache, feature) — missing
```

The earlier shaper experiments already implemented the CoreText shaping core,
glyph offsets, non-LTR sort, cluster mapping, clustered input, default and user
features, the special-font fast path, run grouping, `TextRun.hash`, and
row/viewport shaping. Experiment 361 explicitly left the shaped-run cache for a
later experiment while preserving the run hash needed to key it.

This experiment fills that cache gap only. It does not wire the cache into the
renderer's rebuild loop yet and does not close the whole Shaper checklist line.
Renderer integration can be a later, separately reviewed slice once the cache
type exists.

## Upstream behavior

`vendor/ghostty/src/font/shaper/Cache.zig` defines:

- a cache table keyed by `TextRun.hash`;
- `get(run) -> ?[]const shape.Cell`;
- `put(run, cells)`, which duplicates the shaped cells before caching them;
- fixed-bucket LRU behavior supplied by upstream `datastruct/CacheTable`;
- capacity of 256 buckets with 8 entries per bucket.

Roastty already ported `datastruct/cache_table.zig` as
`roastty/src/terminal/cache_table.rs` in Experiment 555. It currently stores
`Copy` values, so the Rust cache should store indices into an owned
`Vec<Option<Vec<shape::Cell>>>` rather than storing `Vec<shape::Cell>` directly
inside the table.

## Changes

1. `roastty/src/terminal/mod.rs`: expose the existing `cache_table` module as
   `pub(crate)` so the font subsystem can use the generic fixed-bucket cache.
2. `roastty/src/font/mod.rs`: add a `shaper_cache` module.
3. `roastty/src/font/shaper_cache.rs`: add `ShaperCache`:
   - `new()` initializes a `CacheTable<u64, usize, IdentityContext, 256, 8>`.
   - `get(&mut self, run: TextRun) -> Option<&[shape::Cell]>` looks up
     `run.hash`, bumps the cache-table entry to most-recent, and returns the
     stored shaped cells.
   - `put(&mut self, run: TextRun, cells: &[shape::Cell])` duplicates `cells`.
     If the run already exists, replace the existing owned copy instead of
     appending a duplicate key. This is an intentional Roastty deviation from
     the upstream `CacheTable.put` duplicate-key behavior, but it is
     caller-observable equivalent for intended use: the renderer only calls
     `put` after a `get` miss, and same-key duplicate entries should never be
     semantically meaningful. Replacement also prevents stale duplicate slots if
     a test or future caller refreshes a run. If a bucket eviction occurs, free
     the evicted slot for reuse via a free-list.
   - `clear()` removes table entries and stored cell vectors.
4. Tests in `shaper_cache.rs`:
   - miss then hit returns the cached cells;
   - cached cells are an owned copy, not a borrow of the caller's input;
   - putting the same run replaces the existing value;
   - same-bucket inserts evict the least-recently-used run;
   - a `get` hit bumps a run to most-recent before the next eviction;
   - repeated same-bucket evictions reuse freed slots, proved by an internal
     test-only slot-count accessor staying bounded after more inserts than one
     bucket can hold;
   - `clear` removes all cached entries.

## Verification

- `cargo test -p roastty shaper_cache`
- `cargo test -p roastty cache_table`
- `cargo test -p roastty run_hash`
- `cargo test -p roastty shape_row`
- `cargo test -p roastty`
- `cargo fmt -p roastty -- --check`
- `rg -n "\bghostty_[A-Za-z0-9_]*\b" roastty/src/font/shaper_cache.rs roastty/src/font/run.rs roastty/src/terminal/cache_table.rs`
- `git diff --check`

Pass = the upstream-shaped-run cache exists as a reusable Roastty font
component, duplicates shaped cells on insert, has fixed-bucket LRU behavior, and
preserves all existing shaping/run/cache-table tests without wiring new renderer
behavior.

Fail = the cache aliases caller-owned shaped-cell storage, fails to evict
least-recently-used same-bucket entries, disrupts the existing run/shaping path,
or requires renderer integration to be correct.

## Design Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED.

Initial review found two required fixes. First, the plan specified same-key
replacement even though upstream `CacheTable.put` allows duplicate keys; the
plan now documents replacement as an intentional Roastty deviation that is
caller-observable equivalent for the intended renderer miss-then-put use and
prevents stale duplicate slots for tests or future refresh callers. Second, the
plan said evicted slots were freed for reuse without proving bounded storage;
the test plan now requires an internal slot-count assertion showing repeated
same-bucket evictions reuse freed slots.

Follow-up review approved the revised plan.

## Result

**Result:** Pass.

The shaped-run cache now exists as a reusable Roastty font component. It is
keyed by `TextRun.hash`, stores cache-owned copies of `shape::Cell` slices, uses
the already-ported fixed-bucket `CacheTable` for upstream-style LRU behavior,
and reuses evicted slots so repeated same-bucket churn does not grow storage
without bound.

Changes:

- `roastty/src/terminal/mod.rs`: made the existing `cache_table` module
  `pub(crate)` so the font subsystem can reuse it.
- `roastty/src/font/mod.rs`: added the `shaper_cache` module.
- `roastty/src/font/shaper_cache.rs`: added `ShaperCache`, the identity hash
  context, owned slot storage, free-list reuse, `new`, `get`, `put`, `clear`,
  and seven unit tests covering miss/hit behavior, copy ownership, same-key
  replacement, LRU eviction, MRU bumping, bounded slot reuse, and clear.

Verification:

- `cargo test -p roastty shaper_cache` — passed, 7 tests.
- `cargo test -p roastty cache_table` — passed, 4 tests.
- `cargo test -p roastty run_hash` — passed, 3 tests.
- `cargo test -p roastty shape_row` — passed, 1 test.
- `cargo test -p roastty` — passed, 3468 unit tests plus 1 ABI harness test.
- `cargo fmt -p roastty -- --check` — clean.
- `rg -n "\bghostty_[A-Za-z0-9_]*\b" roastty/src/font/shaper_cache.rs roastty/src/font/run.rs roastty/src/terminal/cache_table.rs`
  — no matches.
- `git diff --check` — clean.

This experiment does not wire the cache into `shape_row` or the renderer rebuild
loop. The Issue 801 Shaper checklist item remains unchecked until that
integration is implemented or separately audited.

## Conclusion

The missing Shaper cache component is now ported. The remaining Shaper work is
integration: route row shaping through the cache at the renderer or row-driver
boundary, then audit the full CoreText shaping/run/cache/feature line as one
coherent path.

## Completion Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED.

The reviewer checked the staged diff for cache ownership, same-key replacement,
fixed-bucket LRU semantics through `CacheTable<u64, usize>`, free-list slot
reuse, test coverage, and whether the documentation avoids overclaiming Shaper
completion. No required fixes were found.
