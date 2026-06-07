+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "default"
reasoning = "medium"

[review.result]
agent = "codex"
model = "default"
reasoning = "medium"
+++

# Experiment 800: SharedGridSet Foundation

## Description

Port the ownership and refcount foundation of upstream `font/SharedGridSet.zig`
into Roastty without pulling in the full upstream font configuration model yet.

Upstream `SharedGridSet` is the renderer-facing cache for expensive `SharedGrid`
instances: it keys grids by the derived font configuration, protects the map
with a mutex, increments the refcount when another surface uses the same
configuration, and removes/deinitializes a grid when the last reference is
released. Roastty already has the `SharedGrid` render and codepoint-cache path,
but there is no set-level owner, so the checklist still correctly reports
`SharedGridSet` ownership/refcount/locking as missing.

This experiment should add the reusable cache mechanics only. The key should be
generic or otherwise small enough that future config-derived keys can replace it
without pretending the full Ghostty `DerivedConfig` port is complete.

## Changes

- `roastty/src/font/shared_grid_set.rs`
  - Add a `SharedGridSet` that stores keyed `SharedGrid` instances behind a
    mutex-protected map.
  - Return shared handles to cached grids and increment an explicit set-owned
    refcount on repeated refs for the same key.
  - Add an explicit deref/release path that decrements the refcount and removes
    the grid when the count reaches zero.
  - Keep the constructor path caller-supplied so this foundation does not claim
    to build upstream `DerivedConfig`, font discovery, collections, metrics, or
    font library ownership yet.
  - Add focused unit tests for same-key reuse, distinct-key allocation, missing
    deref behavior, and mutable grid access through the shared handle.
- `roastty/src/font/mod.rs`
  - Export the new module alongside `shared_grid`.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the `SharedGrid` / `SharedGridSet` checklist
    row to mark the new set-level refcount/locking foundation as partial while
    preserving the remaining upstream config-derived key and renderer ownership
    gaps.

## Verification

- `cargo fmt -p roastty`
- `cargo test -p roastty shared_grid_set -- --nocapture --test-threads=1`
- `cargo test -p roastty shared_grid -- --nocapture --test-threads=1`
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/800-shared-grid-set-foundation.md`
- `git diff --check`

The experiment passes if the new set-level owner reuses a grid for matching
keys, maintains explicit refcounts, removes a grid after the final deref, and
keeps existing `SharedGrid` behavior intact. It is Partial if the set foundation
works but needs a small follow-up for handle ergonomics or checklist wording. It
fails if the new module cannot provide safe shared access without changing
`SharedGrid` behavior.

## Design Review

Codex reviewed the design and initially found one blocking process issue: the
experiment file was missing Issue 801's required provenance frontmatter. After
adding the `[implementer]`, `[review.design]`, and `[review.result]`
frontmatter, Codex re-reviewed the corrected design and approved it with no
blocking findings. The review approved the scope because the experiment is
limited to `SharedGridSet` ownership/refcount/locking mechanics, avoids claiming
full upstream `DerivedConfig` or renderer ownership parity, preserves checklist
partiality, and includes focused new tests plus existing `shared_grid`
regression coverage.
