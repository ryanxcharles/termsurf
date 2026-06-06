+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 634: Font Backend And Library

## Description

Make Roastty's macOS-only font backend and library boundary explicit.

The broad Issue 801 font checklist line still names `library` and `backend` as
missing alongside the now-ported `Collection`, `CodepointResolver`,
`CodepointMap`, `DeferredFace`, and `discovery`. Upstream Ghostty has a richer
compile-time backend enum and a `Library` type that is FreeType-backed on
FreeType builds and a no-op on CoreText builds. Roastty currently targets the
macOS/CoreText path only, so the faithful boundary is small: expose the active
backend as CoreText and provide the no-op library type that callers such as a
future `SharedGridSet` can own.

This experiment should not add FreeType, Fontconfig, HarfBuzz, Windows, or WASM
backend support.

## Upstream behavior

`vendor/ghostty/src/font/backend.zig` defines backend variants and capability
predicates such as `hasCoretext`, `hasFreetype`, `hasFontconfig`, and
`hasHarfbuzz`. `vendor/ghostty/src/font/library.zig` maps CoreText-family
backends to `NoopLibrary`, because CoreText does not require process-wide font
library state.

Roastty's macOS slice can represent this as:

- `Backend::CoreText`;
- `Backend::active() == Backend::CoreText`;
- capability predicates matching upstream's CoreText backend;
- a zero-sized `Library` with infallible `new`/`init` and no-op `deinit`;
- module exports from `font/mod.rs`.

## Changes

1. Add `roastty/src/font/backend.rs`:
   - `Backend` enum with `CoreText`;
   - `Backend::active`, `has_coretext`, `has_freetype`, `has_fontconfig`,
     `has_harfbuzz`, and `has_wasm_canvas`;
   - tests that the active backend is CoreText and the capability predicates
     match upstream's CoreText row.
2. Add `roastty/src/font/library.rs`:
   - zero-sized `Library`;
   - infallible `new` and `init` constructors;
   - no-op `deinit`;
   - tests proving the type is zero-sized, clone/copy friendly, and infallible.
3. Export both modules from `roastty/src/font/mod.rs` and update the stale
   module comment so it no longer says faces/shaping are future work.
4. Update the Issue 801 font checklist line to stop listing `library` and
   `backend` as missing, while keeping the broader line unchecked until the
   remaining pieces on it are audited or closed.

## Verification

- `cargo test -p roastty font::backend`
- `cargo test -p roastty font::library`
- `cargo test -p roastty font::tests`
- `cargo test -p roastty`
- `cargo fmt -p roastty -- --check`
- `rg -n "library.*backend.*missing|rasterization, faces, and shaping land in later experiments|font_backend|ghostty_" roastty/src/font issues/0801-roastty-libghostty-rewrite/README.md`
- `git diff --check`

Pass = Roastty has explicit macOS CoreText backend/library modules with
upstream-matching CoreText capability predicates, the no-op library boundary is
available to future grid-set work, and the issue checklist no longer claims
`library`/`backend` are missing.

Fail = the modules imply unsupported cross-platform behavior, change existing
font runtime behavior, or the checklist overclaims completion of
`Collection`/resolver/discovery/`SharedGridSet` work.

## Design Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED.

The reviewer found no overclaiming, missing tests, or incorrect upstream mapping
in the design.

## Result

**Result:** Pass

Roastty now has an explicit macOS/CoreText font backend boundary in
`roastty/src/font/backend.rs`. The active backend is `Backend::CoreText`, and
the capability predicates match upstream's CoreText backend row: CoreText is
available, while FreeType, Fontconfig, HarfBuzz, and browser Canvas are not.

Roastty also has the CoreText no-op font library boundary in
`roastty/src/font/library.rs`. `Library` is zero-sized, infallible to create,
copyable, and has a no-op `deinit`, matching upstream's `NoopLibrary` mapping
for CoreText-family backends.

`font/mod.rs` exports both modules and its module comment now reflects the
current ported font surface instead of saying faces/shaping are future work. The
Issue 801 checklist no longer lists `library`/`backend` as missing, but the
broader `Collection`/resolver/discovery line stays unchecked because final
parity audit remains separate work.

Verification:

- `cargo fmt -p roastty`
- `cargo test -p roastty font::backend` — 2 tests passed
- `cargo test -p roastty font::library` — 2 tests passed
- `cargo test -p roastty font::tests` — 2 tests passed
- `cargo test -p roastty` — 3486 unit tests passed, plus the ABI harness
- `cargo fmt -p roastty -- --check`
- `rg -n "library.*backend.*missing|rasterization, faces, and shaping land in later experiments|font_backend" roastty/src/font issues/0801-roastty-libghostty-rewrite/README.md`
  — no matches
- `rg -n "ghostty_" roastty/src/font` — no matches
- `git diff --check`

The planned `ghostty_` grep was split from the README stale-wording grep because
the issue README intentionally contains policy text forbidding `ghostty_*`
compatibility names.

## Conclusion

The `library` and `backend` pieces of the broad font checklist are now concrete
Roastty modules rather than implicit assumptions. The next font-adjacent work
can focus on final parity audits for collection/resolver/discovery or on the
missing `SharedGridSet` cache layer.

## Completion Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED.

The reviewer found no correctness issues, upstream-mapping errors, checklist
overclaiming, or verification gaps in the staged result.
