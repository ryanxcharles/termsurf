+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 632: CoreText DeferredFace

## Description

Port the macOS/CoreText `DeferredFace` building block.

The Issue 801 font checklist still groups `Collection` / `CodepointResolver` /
`CodepointMap` / `DeferredFace` / `discovery` / `library` / `backend` as
missing. Current Roastty source already has eager `Collection`,
`CodepointResolver`, `CodepointMap`, CoreText discovery, and a macOS-only
backend boundary, but `Collection` explicitly says the deferred-face arm is
deferred and there is no first-class `DeferredFace` type. Discovery currently
has a private `deferred_face(desc) -> Face` helper that eagerly creates a
`Face`.

This experiment should introduce a standalone CoreText-backed `DeferredFace`
type and migrate discovery's private helper to use it. It should not yet add
deferred entries to `Collection`; that is a follow-up integration experiment.

## Upstream behavior

`vendor/ghostty/src/font/DeferredFace.zig` stores backend-specific lightweight
metadata. For CoreText, it stores a CoreText font descriptor/font handle plus
variations, can answer codepoint/presentation support without fully loading the
font for rendering, and can load into a real `Face` when the collection needs
one.

Roastty's macOS-only version can be narrower:

- own a `CTFontDescriptor` with the discovery character-set attribute removed;
- own the requested `Variation` list;
- lazily construct short-lived `Face`s for `has_codepoint` and `load`;
- apply variations on load.

## Changes

1. Add `roastty/src/font/deferred_face.rs`:
   - `DeferredFace::from_descriptor(desc, variations)` copies the descriptor
     with `kCTFontCharacterSetAttribute = kCFNull`, matching upstream's
     `DiscoverIterator.next` behavior.
   - `DeferredFace::load()` creates a `Face` from the descriptor at size 12 and
     applies the stored variations.
   - `DeferredFace::has_codepoint(cp, presentation)` checks glyph coverage and
     color/text presentation through a loaded probe face.
2. Add `pub(crate) mod deferred_face;` in `roastty/src/font/mod.rs`.
3. Update `roastty/src/font/discovery.rs`:
   - replace the private eager `deferred_face(desc) -> Face` helper with
     `DeferredFace::from_descriptor(...).load()`;
   - keep `discover_faces` and `discover_fallback_faces` returning `Face`s for
     now, so resolver behavior does not change in this slice.
4. Tests:
   - `DeferredFace::load` can load a discovered Menlo descriptor and render `M`;

- `DeferredFace::has_codepoint` reports true for a supported text glyph and
  false for a permanent noncharacter;
- presentation filtering is explicit: a Menlo descriptor reports `M` as text but
  not emoji, and an Apple Color Emoji descriptor reports an emoji codepoint as
  emoji but not text;
- variation application through `DeferredFace::load` preserves a renderable
  face;
- discovery's existing face/fallback tests still pass.

## Verification

- `cargo test -p roastty deferred_face`
- `cargo test -p roastty discover_faces`
- `cargo test -p roastty discover_fallback`
- `cargo test -p roastty codepoint_override`
- `cargo test -p roastty discovery_fallback`
- `cargo test -p roastty`
- `cargo fmt -p roastty -- --check`
- `rg -n "DeferredFace.*missing|deferred-face arm is deferred|Deferred-face loading.*later|deferred_face\\(" roastty/src/font`
- `rg -n "\bghostty_[A-Za-z0-9_]*\b" roastty/src/font/deferred_face.rs roastty/src/font/discovery.rs roastty/src/font/collection.rs roastty/src/font/codepoint_resolver.rs`
- `git diff --check`

Pass = Roastty has a first-class CoreText `DeferredFace` type, discovery uses it
as the bridge from descriptor to loaded face, existing resolver/discovery
behavior stays green, and the remaining Collection deferred-entry integration is
clearly left for the next experiment.

Fail = `DeferredFace` cannot load usable faces, cannot answer codepoint support,
breaks existing discovery/resolver behavior, or overclaims that Collection
deferred entries are complete.

## Design Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED.

Initial review found one required fix: the plan said `has_codepoint` would check
text/emoji presentation, but the tests only covered supported and missing
glyphs. The design now requires explicit presentation-filter tests: Menlo `M`
must match text and not emoji, and an Apple Color Emoji codepoint must match
emoji and not text.

Follow-up review approved the revised design.

## Result

**Result:** Pass

Roastty now has a first-class macOS/CoreText `DeferredFace` type in
`roastty/src/font/deferred_face.rs`. It owns a character-set-filter-free
`CTFontDescriptor`, preserves requested variations, can load into a renderable
`Face`, and can answer glyph support with explicit text/emoji presentation
filtering.

Discovery now uses `DeferredFace::from_descriptor(...).load()` as the bridge
from ranked CoreText descriptors to eager `Face`s. That removes the private
`deferred_face(desc) -> Face` helper from `discovery.rs` without changing the
current discovery, fallback, or resolver surface. `Collection` remains
eager-entry-only for now; deferred collection entries are intentionally left for
the next integration experiment.

Verification:

- `cargo fmt -p roastty`
- `cargo test -p roastty deferred_face` — 5 tests passed
- `cargo test -p roastty discover_faces` — 4 tests passed
- `cargo test -p roastty discover_fallback` — 2 tests passed
- `cargo test -p roastty codepoint_override` — 3 tests passed
- `cargo test -p roastty discovery_fallback` — 3 tests passed
- `cargo test -p roastty` — 3475 unit tests passed, plus the ABI harness
- `cargo fmt -p roastty -- --check`
- `rg -n "DeferredFace.*missing|deferred-face arm is deferred|Deferred-face loading.*later|deferred_face\\(" roastty/src/font`
  — no matches after removing stale collection wording
- `rg -n "\bghostty_[A-Za-z0-9_]*\b" roastty/src/font/deferred_face.rs roastty/src/font/discovery.rs roastty/src/font/collection.rs roastty/src/font/codepoint_resolver.rs`
  — no matches
- `git diff --check`

## Conclusion

This proves the CoreText deferred-face primitive can live independently in
Roastty and can replace discovery's private eager helper while keeping existing
behavior green. The next experiment should wire deferred faces into
`Collection`/resolver storage, so descriptor-backed fallback entries can stay
lazy until they are actually needed.

## Completion Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED.

The reviewer found no correctness bugs, stale documentation, or overclaiming in
the staged result.
