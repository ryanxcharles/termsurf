# Experiment 165: Font Fallback Render Runtime

## Description

`RUNTIME-007B2B2B2B` still groups the remaining font renderer output effects:
fallback/shaping visual output, bitmap/color font thickening edge cases, glyph
metrics as seen by the renderer beyond modifier math, and broader
renderer-visible font pixel parity.

A narrow deterministic slice inside that gap is font fallback resolution and
fallback render data before full visual comparison:

- pinned Ghostty's run iterator first tries the cell/grapheme font, then
  substitutes `U+FFFD`, then substitutes a space;
- pinned Ghostty's codepoint resolver searches fallback faces for regular style,
  marks discovered faces as fallback, and uses the default fallback size
  adjustment;
- pinned Ghostty's shared grid resolves a codepoint to a glyph id and renders
  that glyph through `renderGlyph`;
- Roastty's run iterator ports the same `U+FFFD`/space fallback substitution,
  CoreText fallback discovery rejects LastResort-only results, shared-grid
  fallback discovery caches without duplicate faces, and `render_codepoint`
  renders a present glyph by first resolving the glyph id.

This experiment will split the current font gap into:

- `RUNTIME-007B2B2B2B1`: **Oracle complete** for deterministic font fallback
  resolution, fallback substitution, fallback discovery caching, LastResort
  rejection, and present-glyph render-codepoint data.
- `RUNTIME-007B2B2B2B2`: **Gap** for remaining font renderer output effects:
  broad fallback/shaping visual output, bitmap/color font thickening edge cases,
  glyph metrics as seen by the renderer beyond modifier math, and broader
  renderer-visible font pixel parity.

This experiment will not claim screenshot-level fallback font rendering, exact
font-family choice parity across hosts, broad shaping visual parity,
bitmap/color font thickening parity, glyph metric pixel parity, or full font
pixel parity.

## Changes

- `roastty/src/font/run.rs`
  - Add or strengthen focused tests proving the run iterator substitutes
    `U+FFFD` for an unrenderable codepoint when the replacement glyph is
    available, and that kitty unicode placeholders still substitute a space.
  - Preserve the existing run grouping, cache, and shaping behavior.
- `roastty/src/font/face/coretext.rs`
  - Use existing tests proving CoreText fallback discovery finds CJK and emoji
    fonts for Menlo-missing codepoints and rejects LastResort-only
    noncharacters.
- `roastty/src/font/shared_grid.rs`
  - Use existing tests proving discovered fallback faces are cached without
    duplicates, deferred discovery preloads the fallback face before caching,
    present codepoints render via resolved glyph id, and missing codepoints
    return `None`.
- `issues/0805-roastty-ghostty-parity/font_fallback_runtime_parity.py`
  - Add a static guard checking pinned Ghostty's run fallback chain, resolver
    fallback discovery/addDeferred markers, shared-grid render-codepoint path,
    Roastty's matching fallback substitution/discovery/render tests, the
    inventory split, and CFG-223 counts.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-007B2B2B2B` into the complete fallback runtime row and the
    reduced remaining font renderer-output gap.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-223 summary. It must remain `Gap`.
- Existing CFG-223/runtime guards
  - Update expected counts from 71 runtime rows, 64 Oracle-complete rows, and 67
    closed rows to 72 runtime rows, 65 Oracle-complete rows, and 68 closed rows.
    Incomplete and gap counts remain 4.
  - Update references from `RUNTIME-007B2B2B2B` to `RUNTIME-007B2B2B2B2` where
    they mean the remaining font renderer-output gap.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add the experiment link and update Learnings after the result.

## Verification

Pass criteria:

- Pinned Ghostty evidence shows the run fallback chain tries cell/grapheme
  resolution, then `U+FFFD`, then space, and substitutes only the fallback
  codepoint into the shaping run.
- Pinned Ghostty evidence shows resolver fallback discovery is regular-style
  scoped, marks discovered faces as fallback, and uses the default fallback size
  adjustment.
- Roastty has non-vacuous tests for:
  - unrenderable-cell fallback substitution to `U+FFFD`;
  - kitty placeholder substitution to space;
  - CoreText fallback discovery for Menlo-missing CJK and emoji codepoints;
  - LastResort rejection for a noncharacter;
  - shared-grid fallback discovery caching without duplicate faces;
  - deferred fallback face preload before caching;
  - present-codepoint render data via resolved glyph id;
  - missing-codepoint render result `None`.
- `RUNTIME-007B2B2B2B1` is `Oracle complete` and cites the focused tests plus
  `font_fallback_runtime_parity.py`.
- `RUNTIME-007B2B2B2B2` remains `Gap` for broad fallback/shaping visual output,
  bitmap/color font thickening edge cases, glyph metrics beyond modifier math,
  and broader renderer-visible font pixel parity.
- CFG-223 remains `Gap`.

Commands:

```bash
cargo test --manifest-path roastty/Cargo.toml next_missing_codepoint_substitutes_replacement
cargo test --manifest-path roastty/Cargo.toml next_placeholder_is_space
cargo test --manifest-path roastty/Cargo.toml font_for_codepoint
cargo test --manifest-path roastty/Cargo.toml get_index_discovery_fallback
cargo test --manifest-path roastty/Cargo.toml get_index_deferred_discovery_preloads_face_before_caching
cargo test --manifest-path roastty/Cargo.toml render_codepoint
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/font_fallback_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
for f in issues/0805-roastty-ghostty-parity/*_runtime_parity.py issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py issues/0805-roastty-ghostty-parity/link_hover_preview_dispatch_parity.py issues/0805-roastty-ghostty-parity/link_hover_modifier_refresh_parity.py issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py; do PYTHONDONTWRITEBYTECODE=1 python3 "$f" >/tmp/$(basename "$f").out || { echo FAIL:$f; cat /tmp/$(basename "$f").out; exit 1; }; done; echo parity_guards=pass
cargo fmt --manifest-path roastty/Cargo.toml
cargo fmt --manifest-path roastty/Cargo.toml --check
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/165-font-fallback-render-runtime.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

Fail criteria:

- The experiment claims full fallback visual parity, screenshot parity, exact
  host font-family parity, broad shaping visual parity, glyph metric parity, or
  broad font pixel parity from fallback resolver/render-data tests.
- The run fallback chain can skip `U+FFFD` or substitute an entire missing
  grapheme instead of a single fallback codepoint.
- Fallback discovery can add duplicate faces for repeated codepoint lookups.
- LastResort-only fallback results are accepted as real fallback faces.
- `RUNTIME-007B2B2B2B2` omits any remaining broad font visual/pixel gaps.
- CFG-223 is marked complete.

## Design Review

**Reviewer:** Codex adversarial subagent with fresh context.

**Verdict:** Approved.

The reviewer found no required, optional, or nit findings. It confirmed the
README links Experiment 165 as `Designed`, the design has the required sections,
the scope keeps broad visual/font pixel parity in the remaining gap, and the
proposed checks align with pinned Ghostty fallback paths.
