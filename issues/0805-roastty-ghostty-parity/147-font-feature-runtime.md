# Experiment 147: Font Feature Runtime

## Description

`RUNTIME-007B2B2` still includes OpenType feature/variation effects, metric
adjustment, fallback/shaping visual output, bitmap/color thickening edge cases,
glyph metrics, and broader font pixel parity. A narrow deterministic slice
inside that gap is `font-feature` after config parsing:

- pinned Ghostty copies `font-feature` into renderer `DerivedConfig`;
- renderer init and config change recreate the font shaper with those features;
- default OpenType features are applied first, then parsed user features, so a
  later user feature can override defaults such as `-liga`;
- the shaper cache must not reuse glyph output produced under a different
  feature set.

Roastty already parses and formats `font-feature`, has `shape::Options` and
CoreText feature descriptors, and has tests for feature parsing/merging and
CoreText feature descriptor construction. The active renderer row-shaping path,
however, still calls `face.shape_run(...)`, which applies only default features.
This experiment will wire config-derived `font-feature` into active frame row
shaping and make cached shaped runs feature-aware.

This experiment will split `RUNTIME-007B2B2`:

- `RUNTIME-007B2B2A`: **Oracle complete** for deterministic `font-feature`
  renderer option propagation, default-plus-user feature merging, CoreText
  shaping option application, and feature-aware shaped-run cache separation.
- `RUNTIME-007B2B2B`: **Gap** for remaining font renderer output effects:
  `font-variation*` effects, metric adjustment, fallback/shaping visual output,
  bitmap/color font thickening edge cases, glyph metrics as seen by the
  renderer, broader font pixel parity, and GUI-visible A/B font rendering.

This experiment will not claim font variation parity, metric adjustment parity,
fallback visual parity, glyph metric parity, or full renderer/GUI pixel parity.

## Changes

- `roastty/src/font/run.rs`
  - Add a feature/options-aware row shaping entry point, or equivalent
    parameter, so the row shaper can call `Face::shape_run_options` instead of
    `Face::shape_run` when renderer-provided `shape::Options` are present.
  - Preserve the existing default path for callers that do not provide feature
    options.
  - Add focused tests proving `shape_row` defaults are unchanged and the
    options-aware path passes merged feature options to face shaping.
- `roastty/src/font/shaper_cache.rs`
  - Make cache lookup/insert feature-aware, either by namespacing the cache key
    with a stable feature/options hash or by an equally deterministic cache
    strategy that cannot reuse shaped output across different feature sets.
  - Add tests proving the same text-run hash can store distinct cached glyph
    output for different feature options.
- `roastty/src/renderer/frame_rebuild.rs`
  - Add renderer shaping options to row-format input.
  - Pass those options to cached row shaping when rebuilding rows.
  - Keep `font-shaping-break` row-local application unchanged.
- `roastty/src/renderer/frame_renderer.rs`
  - Add `font_features`/`shape::Options` to `FrameRenderKnobs`, sourced from
    `Config.font_feature.list`.
  - Thread the options into active frame row-format input.
  - Add an active-frame test proving parsed config features reach row-format
    input.
- `issues/0805-roastty-ghostty-parity/font_feature_runtime_parity.py`
  - Add a static guard checking pinned Ghostty's `font-feature` derived config
    copy, shaper recreation, default-plus-user feature markers, Roastty's
    options-aware row-shaping/cache wiring, focused tests, and inventory split.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-007B2B2` into `RUNTIME-007B2B2A` and `RUNTIME-007B2B2B`.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-223 summary. It must remain `Gap`.
- Existing CFG-223/static runtime guards
  - Update current runtime row counts from 54/48/50/4/4 to 55/49/51/4/4.
  - Update references from `RUNTIME-007B2B2` to `RUNTIME-007B2B2B` where they
    mean the remaining font renderer gap.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add the experiment link and update Learnings after the result.

## Verification

Pass criteria:

- Pinned Ghostty evidence shows renderer-derived `font-feature` values are used
  when creating/recreating the shaper.
- Roastty active frame row-format input receives `Config.font_feature.list`.
- Row shaping applies default features plus parsed user features in the same
  order as pinned Ghostty.
- The shaped-run cache cannot reuse glyph output across different feature sets.
- Existing default row-shaping behavior remains unchanged when no user features
  are configured.
- `RUNTIME-007B2B2A` is Oracle complete and cites focused tests plus the new
  static guard.
- `RUNTIME-007B2B2B` remains `Gap` for font variations, metric adjustment,
  fallback/shaping visual output, bitmap/color thickening edge cases, glyph
  metrics, broader font pixel parity, and GUI-visible A/B font rendering.
- `CFG-223` remains `Gap`.

Commands:

```bash
cargo test --manifest-path roastty/Cargo.toml font_feature_runtime
cargo test --manifest-path roastty/Cargo.toml merged_features_defaults_then_user
cargo test --manifest-path roastty/Cargo.toml shape_row_options_default_matches_default_shape
cargo test --manifest-path roastty/Cargo.toml shaper_cache_feature
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/font_feature_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
for f in issues/0805-roastty-ghostty-parity/*_runtime_parity.py; do PYTHONDONTWRITEBYTECODE=1 python3 "$f" >/tmp/$(basename "$f").out || { echo FAIL:$f; cat /tmp/$(basename "$f").out; exit 1; }; done; echo all_runtime_parity_guards=pass
cargo fmt --manifest-path roastty/Cargo.toml
cargo fmt --manifest-path roastty/Cargo.toml --check
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/147-font-feature-runtime.md
git diff --check
```

Fail criteria:

- The active row-shaping path still applies only default features.
- Cache behavior can return shaped glyphs produced for a different feature set.
- The experiment relies only on parser/formatter evidence and does not prove
  renderer row-shaping behavior.
- The experiment promotes font variations, metric adjustment, fallback visual
  output, bitmap/color thickening edge cases, glyph metrics, broad font pixel
  parity, or GUI A/B rendering from the remaining gap.
- CFG-223 is marked complete.

## Design Review

**Reviewer:** Codex adversarial subagent with fresh context.

**Verdict:** Approved.

The reviewer found no findings.

## Result

**Result:** Pass

Roastty now threads `Config.font_feature.list` into active renderer row shaping:
`FrameRenderKnobs::from_config` builds `shape::Options` from the parsed
configuration, frame rebuild inputs carry those options, and row shaping calls
`Face::shape_run_options` for active cached rows. The default row-shaping path
is preserved for callers that do not pass explicit shaping options.

The shaped-run cache is now feature-aware. Default shaping still uses namespace
`0`, while non-empty feature sets derive a deterministic namespace from the
feature strings. Focused tests prove that identical text runs can hold separate
cached glyph output for different feature options.

`RUNTIME-007B2B2` was split as planned:

- `RUNTIME-007B2B2A` is **Oracle complete** for deterministic `font-feature`
  renderer option propagation, default-plus-user feature merging, CoreText
  shaping option application, and feature-aware shaped-run cache separation.
- `RUNTIME-007B2B2B` remains **Gap** for font variations, metric adjustment,
  fallback/shaping visual output, bitmap/color font thickening edge cases, glyph
  metrics, broader font pixel parity, and GUI-visible A/B font rendering.

The regenerated CFG-223 inventory reports:

- `runtime_rows=55`
- `oracle_complete=49`
- `closed=51`
- `incomplete=4`
- `gap=4`
- `cfg223=Gap`

Verification run:

```bash
cargo test --manifest-path roastty/Cargo.toml font_feature_runtime
cargo test --manifest-path roastty/Cargo.toml merged_features_defaults_then_user
cargo test --manifest-path roastty/Cargo.toml shape_row_options_default_matches_default_shape
cargo test --manifest-path roastty/Cargo.toml shaper_cache_feature
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/font_feature_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py
for guard in issues/0805-roastty-ghostty-parity/*_runtime_parity.py; do PYTHONDONTWRITEBYTECODE=1 python3 "$guard" || exit 1; done
cargo fmt --manifest-path roastty/Cargo.toml --check
git diff --check
```

All commands passed.

## Conclusion

The deterministic `font-feature` runtime slice is no longer part of the font
renderer gap. The remaining CFG-223 gap is smaller but still real:
`RUNTIME-007B2B2B`, `RUNTIME-008B2B2`, `RUNTIME-011`, and `RUNTIME-012B2B`
remain open.

## Completion Review

**Reviewer:** Codex adversarial subagent with fresh context.

**Verdict:** Approved.

The reviewer found no findings. It independently verified the focused
`font-feature` and default row-shaping tests, the static font-feature runtime
guard, the residual audit, Rust formatting, whitespace hygiene, additional
feature-merge/cache tests, and CFG-223 counts:

- `runtime_rows=55`
- `oracle_complete=49`
- `closed=51`
- `incomplete=4`
- `gap=4`
