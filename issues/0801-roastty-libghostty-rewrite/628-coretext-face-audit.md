+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 628: CoreText Face audit

## Description

Verify and close the stale Issue 801 checklist item for CoreText `Face`
rasterization and face-metric extraction.

The README still says:

```markdown
- [ ] CoreText `Face` (rasterization + face-metric extraction) — missing
```

Current Roastty source and prior experiments already contain the relevant
implementation: CoreText `CTFont` wrapping, OpenType table access, scalar
metrics, glyph measurement, `Face::get_metrics`, glyph rasterization, and atlas
`render_glyph`. This experiment should not change behavior unless the
verification finds a real gap. It should make one doc-only source edit: update
the stale module comment in `roastty/src/font/face/coretext.rs` that still says
metric assembly and glyph rasterization land in later experiments. If the gates
pass, update the checklist line to:

```markdown
- [x] CoreText `Face` (rasterization + face-metric extraction)
```

This does not close the adjacent `Shaper` checklist item even though
`coretext.rs` also contains shaping methods. That is a separate line and should
get its own audit or implementation experiment.

## Current implementation surface

- `roastty/src/font/face/coretext.rs` — defines `Face`, wraps CoreText `CTFont`,
  copies font tables, exposes scalar metrics and glyph measurement, implements
  `get_metrics`, rasterizes glyphs into coverage bitmaps, and renders glyphs
  into `Atlas` entries. Its module comment is stale and should be updated to
  describe the current implemented surface.
- `roastty/src/font/opentype/` — contains the `head`, `hhea`, `os2`, `post`, and
  `sfnt` parsers used by `Face::get_metrics`.
- `roastty/src/font/atlas.rs` and `roastty/src/font/glyph.rs` — provide the
  atlas and returned glyph value used by `render_glyph`.
- Prior issue docs already mark the CoreText Face build-up as passing:
  Experiments 250-255 cover table copy, scalar metrics, glyph measurement,
  metrics assembly, rasterization, and atlas render-glyph.

## Verification

- `cargo test -p roastty face::coretext::tests::face_copies_and_parses_head` —
  proves `CTFont` creation and table-copy/parsing work.
- `cargo test -p roastty face::coretext::tests::glyph_measurement` — proves
  CoreText glyph lookup/advance/bounds measurement works.
- `cargo test -p roastty face::coretext::tests::get_metrics` — proves
  `Face::get_metrics` extracts sane face metrics and feeds `Metrics::calc`.
- `cargo test -p roastty face::coretext::tests::rasterize_glyph_has_ink` —
  proves glyph rasterization produces ink for a live glyph.
- `cargo test -p roastty face::coretext::tests::rasterize_space_is_empty_or_none`
  — proves an outline-less space glyph returns no ink.
- `cargo test -p roastty face::coretext::tests::render_glyph` — proves
  CoreText-rendered glyphs can be written into the atlas and represented as
  `Glyph` values.
- `cargo test -p roastty` — full Roastty test suite stays green.
- forbidden compatibility-name grep on `roastty/src/font/face/coretext.rs`,
  `roastty/src/font/opentype`, `roastty/src/font/atlas.rs`, and
  `roastty/src/font/glyph.rs` — clean for `ghostty_*` symbols.
- `git diff --check` — clean.

Pass = the current source and tests prove CoreText `Face` rasterization and
metric extraction are implemented for Issue 801, allowing that checklist item to
be checked without new code.

## Design Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED.

Initial review found two Required issues. First, the `rasterize_glyph` test
filter selected only `rasterize_glyph_has_ink` and missed
`rasterize_space_is_empty_or_none`, so the verification overclaimed coverage.
Second, `roastty/src/font/face/coretext.rs` still had a stale module comment
saying metric assembly and glyph rasterization would land in later experiments.

The design now includes a doc-only source edit for the stale module comment and
separate focused gates for both rasterization tests. Follow-up review confirmed
the filters select the intended tests and that the gates are broad enough for
the CoreText `Face` checklist item while leaving Shaper separate.

## Result

**Result:** Pass.

The audit found the CoreText `Face` checklist line was stale. The implementation
already covers `CTFont` face construction, OpenType table copying and parsing,
glyph measurement, face-metric extraction, glyph rasterization, and atlas-backed
glyph rendering. The only source edit was to update the stale
`roastty/src/font/face/coretext.rs` module comment so it describes the current
implemented surface instead of saying metrics and rasterization would land
later.

Focused gates:

- `cargo test -p roastty face::coretext::tests::face_copies_and_parses_head` —
  passed, 1 test.
- `cargo test -p roastty face::coretext::tests::glyph_measurement` — passed, 1
  test.
- `cargo test -p roastty face::coretext::tests::get_metrics` — passed, 2 tests.
- `cargo test -p roastty face::coretext::tests::rasterize_glyph_has_ink` —
  passed, 1 test.
- `cargo test -p roastty face::coretext::tests::rasterize_space_is_empty_or_none`
  — passed, 1 test.
- `cargo test -p roastty face::coretext::tests::render_glyph` — passed, 5 tests.

Broad gates:

- `cargo test -p roastty` — passed, 3461 unit tests plus 1 ABI harness test.
- `cargo fmt -p roastty -- --check` — clean.
- `rg -n "\bghostty_[A-Za-z0-9_]*\b" roastty/src/font/face/coretext.rs roastty/src/font/opentype roastty/src/font/atlas.rs roastty/src/font/glyph.rs`
  — no matches.
- `git diff --check` — clean.

The README checklist now marks CoreText `Face` as implemented. The adjacent
`Shaper` checklist item remains unchecked because it is separate Issue 801
scope.

## Conclusion

CoreText `Face` did not need new behavior for Issue 801. Prior implementation
and the current focused tests prove the required rasterization and face-metric
extraction path, so the checklist can move forward while keeping shaping for a
separate audit.

## Completion Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED.

The reviewer checked that the diff is valid to close only the CoreText `Face`
checklist item, that the documentation does not overclaim Shaper coverage, and
that the result-review gate is ready to record before the result commit. No
required fixes were found.
