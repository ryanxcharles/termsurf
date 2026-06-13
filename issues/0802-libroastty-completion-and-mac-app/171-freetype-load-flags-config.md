# Experiment 171: Phase F — FreeType load flags config

## Description

Remove `freetype-load-flags` from the remaining Phase F public-config tail.

Upstream defines `freetype-load-flags` immediately after `grapheme-width-method`
and before `theme` as a packed boolean struct with five flags:

- `hinting`, default `true`
- `force-autohint`, default `false`
- `monochrome`, default `false`
- `autohint`, default `true`
- `light`, default `true`

The CLI/config syntax is upstream's packed-struct behavior: a standalone boolean
sets every flag, and comma-separated `[no-]flag` tokens override individual
flags while omitted flags keep their defaults.

This experiment wires config parser/formatter/storage behavior only. Applying
these flags during live FreeType glyph loading remains font-backend runtime
work, and macOS continues to use CoreText.

## Changes

- `roastty/src/config/mod.rs`
  - Add a `FreetypeLoadFlags` packed-flag config type with upstream defaults and
    keywords.
  - Add `Config::freetype_load_flags`, defaulting to upstream's `.{}` defaults.
  - Format `freetype-load-flags` after `grapheme-width-method` and before
    `theme`.
  - Route `Config::set("freetype-load-flags", ...)` through the existing
    `set_packed_field` helper:
    - missing values report `ValueRequired`;
    - empty set values reset to the default packed flags;
    - `true` and `false` set all five flags;
    - comma-separated `[no-]hinting`, `[no-]force-autohint`, `[no-]monochrome`,
      `[no-]autohint`, and `[no-]light` tokens update only named flags;
    - unknown tokens report `InvalidValue`.
  - Update default/order tests.
  - Add a focused `freetype_load_flags_config_*` test covering defaults,
    formatting, bool-all parsing, partial token parsing, whitespace around comma
    tokens, reset, load diagnostics, CLI set, clone, and invalid values.

- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link Experiment 171 as `Designed`.
  - After result, update the Phase F remaining-public-options count from 3 to 2
    and leave only `input` and `keybind` in that public-config tail if this
    passes.
  - After result, add an operating note that `freetype-load-flags` is
    parser/formatter-only until a FreeType font backend consumes it.

## Verification

Before implementation:

- Codex-native adversarial design review approves this experiment.
- Commit the reviewed plan separately from the result.

After implementation:

- `cargo test -p roastty freetype_load_flags_config`
- `cargo test -p roastty config_format_config_emits_fields_in_upstream_order`
- `cargo test -p roastty`
- `cargo fmt -p roastty`
- `cargo fmt --check -p roastty`
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/171-freetype-load-flags-config.md issues/0802-libroastty-completion-and-mac-app/README.md`
- `git diff --check`

**Pass** = `freetype-load-flags` parses, formats, resets, loads, clones, reports
diagnostics, and appears in upstream order with packed-struct defaults, and the
full roastty suite passes.

**Partial** = the direct parser/formatter field lands, but ordering, reset
behavior, diagnostics, or full-suite verification remains incomplete.

**Fail** = the field cannot be added without conflicting with existing config
formatting, packed-flag helpers, or font config storage.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Averroes`, fresh
context.

**Verdict:** Approved after one required verification fix.

The first review found that the verification plan listed
`cargo fmt --check -p roastty` but not the required actual
`cargo fmt -p roastty` run for planned Rust edits. I added
`cargo fmt -p roastty` before the check-only formatter command. Re-review
confirmed the finding was resolved and found no new required issues.
