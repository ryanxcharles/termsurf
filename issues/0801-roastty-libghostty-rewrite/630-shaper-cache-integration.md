+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 630: integrate the shaper cache

## Description

Wire the `ShaperCache` from Experiment 629 into the row-shaping path.

Experiment 629 ported upstream `font/shaper/Cache.zig` as a standalone
component, but `renderer/cell.rs` still calls
`shape_row(opts, &mut grid.resolver)` directly. This experiment makes the cache
part of the shared font grid and uses it when rebuilding rows, matching
upstream's renderer behavior: get cached shaped cells by `TextRun.hash`,
otherwise shape the run and cache a copy.

This still does not close the full Issue 801 Shaper checklist line by itself.
After this lands, a follow-up audit should verify the whole
CoreText-shaping/run/cache/feature path and only then check off that line if the
evidence is complete.

## Changes

1. `roastty/src/font/run.rs`
   - Add `shape_row_cached(opts, resolver, cache)`.
   - It drives `RunIterator` exactly like `shape_row`.
   - For each non-special run:
     - try `cache.get(out.run)` first and clone the cached cells into the
       returned `ShapedRun`;
     - on miss, shape with `Face::shape_run`, insert the cells into
       `ShaperCache`, then return the fresh cells.
   - Keep `shape_row` as the uncached convenience path for existing focused
     tests.
   - Add tests proving the cached path returns the same shaped output as the
     uncached path and populates/reuses the cache on repeated rows.
   - Add a sentinel cache-hit test: construct a known `RunOptions`, compute its
     `TextRun.hash`, prepopulate `ShaperCache` with sentinel `shape::Cell`s for
     that hash, and assert `shape_row_cached` returns those sentinel cells
     instead of freshly shaped output. This proves the cached path actually
     reads from the cache rather than always reshaping and replacing the same
     slot.
2. `roastty/src/font/shared_grid.rs`
   - Add `pub shaper_cache: ShaperCache` to `SharedGrid`.
   - Initialize it in `SharedGrid::new`.
3. `roastty/src/renderer/cell.rs`
   - Import and call
     `shape_row_cached(opts, &mut grid.resolver, &mut grid.shaper_cache)` in
     `rebuild_viewport`.
   - Leave `rebuild_row` unchanged; it still consumes owned `ShapedRun`s.
4. `roastty/src/font/shaper_cache.rs`
   - Widen the existing test-only `slot_count` accessor to `pub(crate)` so
     `run.rs` tests can verify cache reuse without adding production API.

## Verification

- `cargo test -p roastty shaper_cache`
- `cargo test -p roastty shape_row`
- `cargo test -p roastty rebuild_viewport`
- `cargo test -p roastty run_hash`
- `cargo test -p roastty`
- `cargo fmt -p roastty -- --check`
- `rg -n "\bghostty_[A-Za-z0-9_]*\b" roastty/src/font/run.rs roastty/src/font/shaper_cache.rs roastty/src/font/shared_grid.rs roastty/src/renderer/cell.rs`
- `git diff --check`

Pass = row shaping uses the cache in the renderer-facing path, cached and
uncached shaping outputs match, repeated identical runs reuse cache storage, and
the existing full Roastty suite stays green.

Fail = cached shaping changes row output, fails to populate/reuse cache entries,
breaks renderer foreground assembly, or requires checking off the Shaper
checklist line before a full audit.

## Design Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED.

Initial review found one required fix: the original test plan did not prove a
real cache hit because same-key replacement could let an implementation always
reshape and still keep a stable slot count. The design now requires a sentinel
cache-hit test that prepopulates `ShaperCache` for a known `TextRun.hash` and
asserts `shape_row_cached` returns those sentinel cells instead of freshly
shaped output.

Follow-up review approved the revised design.

## Result

**Result:** Pass.

The renderer-facing row shaping path now uses `ShaperCache`. `SharedGrid` owns a
cache alongside its resolver and glyph cache, `rebuild_viewport` passes that
cache into the row-shaping driver, and `shape_row_cached` returns cached shaped
cells by `TextRun.hash` before reshaping on misses.

Changes:

- `roastty/src/font/run.rs`: added `shape_row_cached` and the shared
  `shape_row_with` implementation. Cache hits clone cached cells into the owned
  `ShapedRun` output; misses shape through the resolved face and insert a cache
  copy before returning.
- `roastty/src/font/shared_grid.rs`: added `shaper_cache: ShaperCache` and
  initialized it in `SharedGrid::new`.
- `roastty/src/renderer/cell.rs`: changed `rebuild_viewport` to call
  `shape_row_cached(opts, &mut grid.resolver, &mut grid.shaper_cache)`.
- `roastty/src/font/shaper_cache.rs`: widened the test-only `slot_count`
  accessor so `run.rs` can verify cache reuse.
- Tests added in `run.rs`:
  - cached shaping matches uncached output and populates/reuses one cache slot;
  - a prepopulated sentinel cache entry is returned by `shape_row_cached`,
    proving the path actually hits the cache instead of always reshaping.

Verification:

- `cargo test -p roastty shaper_cache` — passed, 7 tests.
- `cargo test -p roastty shape_row` — passed, 3 tests.
- `cargo test -p roastty rebuild_viewport` — passed, 5 tests.
- `cargo test -p roastty run_hash` — passed, 3 tests.
- `cargo test -p roastty` — passed, 3470 unit tests plus 1 ABI harness test.
- `cargo fmt -p roastty -- --check` — clean.
- `rg -n "\bghostty_[A-Za-z0-9_]*\b" roastty/src/font/run.rs roastty/src/font/shaper_cache.rs roastty/src/font/shared_grid.rs roastty/src/renderer/cell.rs`
  — no matches.
- `git diff --check` — clean.

The Issue 801 Shaper checklist line remains unchecked. This experiment wires the
cache, but the line should only be checked after a full audit proves the whole
CoreText shaping/run/cache/feature path is complete.

## Conclusion

The shaped-run cache is now active in the renderer-facing path. The next Shaper
slice should be an audit of the complete line: CoreText shaping, run grouping,
cache integration, feature parsing/application, special-font handling, and
viewport/renderer entry points.

## Completion Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED.

Initial completion review found one required documentation fix:
`rebuild_viewport` still described the old uncached `shape_row` path. The
renderer doc comment and nearby inline comment now describe `shape_row_cached`,
the grid resolver, and the grid shaper-cache borrow. Focused verification after
the fix passed for `cargo fmt -p roastty`,
`cargo test -p roastty rebuild_viewport`, `cargo test -p roastty shape_row`, and
`git diff --check`.

Follow-up review approved the completed output.
