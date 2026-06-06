+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 636: SharedGrid Codepoint Cache

## Description

Port the missing codepoint lookup side of Ghostty's `font/SharedGrid.zig` into
Roastty's `SharedGrid`.

Experiment 635 finished collection point-size loading, which makes deferred
faces safe to load through the grid. Roastty's current `SharedGrid` already owns
the atlases, resolver, grid metrics, shaper cache, and glyph render cache, but
it does not yet mirror upstream's `codepoints` cache, `getIndex`,
`hasCodepoint`, or `cellSize`. `render_codepoint` also bypasses the grid cache
and calls the resolver directly.

This experiment should add the upstream-style codepoint cache and route
codepoint rendering through it. It should stay single-threaded for now: Roastty
does not yet have Ghostty's `RwLock` sharing model or `SharedGridSet` reference
manager.

## Upstream behavior

`vendor/ghostty/src/font/SharedGrid.zig` stores a
`CodepointKey { style, codepoint, presentation } -> ?Collection.Index` map.
`getIndex` caches both positive and negative resolution results, and when a
positive result is a real face it preloads the face so later glyph lookup and
rendering do not need to discover/load again. `hasCodepoint` delegates to the
resolver collection using `None` presentation as "any presentation". `cellSize`
returns the current grid metrics as a renderer `CellSize`.

Roastty's narrow equivalent can be:

- add a `codepoints: HashMap<CodepointKey, Option<Index>>` field to
  `SharedGrid`;
- add
  `SharedGrid::get_index(cp, style, presentation) -> Result<Option<Index>, EntryError>`;
- cache positive and negative results;
- on a cached positive real-font result, ensure the face has already been loaded
  through `resolver.collection_mut().get_face(index)`;
- never leave a new positive cache entry behind if real-face preloading fails;
- skip preloading for special sprite indexes;
- add `SharedGrid::has_codepoint(index, cp, presentation) -> bool`;
- add `SharedGrid::cell_size() -> crate::renderer::size::CellSize`;
- change `render_codepoint` to call `SharedGrid::get_index` instead of
  `resolver.get_index`.

## Changes

1. Update `roastty/src/font/shared_grid.rs`:
   - add a private `CodepointKey` matching upstream's style/codepoint/optional
     presentation tuple;
   - add the `codepoints` cache field to `SharedGrid`;
   - initialize the cache in `SharedGrid::new`;
   - implement `cell_size`;
   - implement `get_index` with positive/negative caching, real-face preloading,
     and an `EntryError` result for preload failures;
   - cache a resolved value only after a positive real-face preload succeeds, or
     immediately for `None`/sprite results, which gives the same no-poison
     guarantee as upstream's `errdefer` rollback;
   - implement `has_codepoint`;
   - update `render_codepoint` to resolve via `self.get_index`.
2. Tests:
   - ASCII codepoints resolve to the regular face and `has_codepoint` accepts
     `None` presentation as any presentation;
   - a missing codepoint is cached as `None`;
   - a second lookup reuses the cache rather than adding duplicate discovery
     fallback entries;
   - a deferred discovery fallback resolved through `SharedGrid::get_index`
     preloads the face and caches the resulting index;
   - the no-poison preload-error contract is documented in the result: current
     CoreText `DeferredFace::load` is infallible and resolver-produced indexes
     are valid, so the implementation should be structured to insert positive
     real-face cache entries only after `get_face` succeeds rather than relying
     on an induced failing deferred face;
   - sprite codepoints cache as the special sprite index without trying to
     preload a real face;
   - `cell_size` reports the grid metrics width and height.

## Verification

- `cargo test -p roastty shared_grid`
- `cargo test -p roastty discovery_fallback`
- `cargo test -p roastty collection_deferred`
- `cargo test -p roastty font::tests`
- `cargo test -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

Pass = `SharedGrid` owns the upstream-style codepoint cache, lookup preloads
deferred real faces exactly once, positive and negative cache entries are
reused, preload errors cannot poison new cache entries, sprite indexes avoid
real-face preload, existing glyph rendering behavior stays green, and the code
remains scoped to the current single-threaded Roastty grid.

Fail = lookups bypass the cache, negative results are not cached, deferred faces
remain unloaded after `SharedGrid::get_index`, preload errors can leave a stale
positive cache entry, sprite indexes try to load through the collection,
`render_codepoint` diverges from the cached lookup path, or the experiment grows
into `SharedGridSet`/locking/config construction.

## Design Review

**Reviewer:** Codex (gpt-5.5) · session `019e9a84-c7f1-7302-9e25-466e569d5326`

**Verdict:** APPROVED after revision.

Initial review found three required fixes: `SharedGrid::get_index` must return a
`Result` because preloading can fail; the design must guarantee a preload error
does not poison the cache; and the verification plan must cover or explicitly
explain the preload-error contract. The plan now returns
`Result<Option<Index>, EntryError>`, caches positive real-face results only
after successful preload, and records why current CoreText deferred loading
cannot induce a normal preload failure in tests. Follow-up review approved the
revised design.
