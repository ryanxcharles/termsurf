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

# Experiment 803: Embedded Font Resources

## Description

Port a first resource-backed slice of upstream `font/embedded.zig` into Roastty.
Issue 801's font row still says embedded fonts are missing, while the vendored
Ghostty tree already contains several redistributable font resources under
`vendor/ghostty/src/font/res/` plus their license texts.

Upstream `font/embedded.zig` has two categories:

- generated build dependency blobs such as `jetbrains_mono_variable`,
  `jetbrains_mono_regular`, and `nerd_fonts_symbols_only`;
- resource files that are present in the vendored source tree, such as Noto
  Emoji, Kawkab Mono, Cozette, Monaspace Neon, Terminus, Spleen, and several
  test fonts.

This experiment should port exactly the 15 upstream `@embedFile("res/...")`
symbols named by `font/embedded.zig`:

- `emoji`
- `emoji_text`
- `arabic`
- `test_nerd_font`
- `code_new_roman`
- `inconsolata`
- `geist_mono`
- `jetbrains_mono`
- `julia_mono`
- `cozette`
- `monaspace_neon`
- `terminus_ttf`
- `spleen_bdf`
- `spleen_pcf`
- `spleen_otb`

It should not include unreferenced vendored resources such as `Lilex-VF.ttf` or
extra JetBrains Nerd Font styles, and it should not claim generated default
JetBrains variable/static blobs, Symbols Nerd Font dependency blobs, font
discovery integration, `SharedGridSet` collection construction, or app/frontend
font loading.

## Changes

- `roastty/src/font/embedded.rs`
  - Add an `EmbeddedFont` enum for the resource-backed fonts available in
    `vendor/ghostty/src/font/res/`.
  - Provide metadata for each font: upstream symbol name, file name, license
    family, test-only/default-role classification where applicable, and raw
    bytes through `include_bytes!`.
  - Expose an `ALL` slice for inventory-style tests and future collection
    construction.
  - Add tests that validate the inventory count, unique upstream names, exact
    expected upstream symbol/file coverage, non-empty bytes, format signatures
    (`OTTO`, `ttcf`, `\0\1\0\0`, `STARTFONT`, or PCF magic), and license text
    availability.
- `roastty/src/font/mod.rs`
  - Export the new internal `embedded` module.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the `opentype` / `embedded` checklist row to
    record the resource-backed embedded font inventory as partial while keeping
    generated default-font blobs and integration open.

## Verification

- Inspect:
  - `vendor/ghostty/src/font/embedded.zig`
  - `vendor/ghostty/src/font/res/README.md`
  - `vendor/ghostty/src/font/res/OFL.txt`
  - `vendor/ghostty/src/font/res/MIT.txt`
  - `vendor/ghostty/src/font/res/BSD-2-Clause.txt`
- Run:
  - `cargo fmt -p roastty`
  - `cargo test -p roastty embedded -- --nocapture --test-threads=1`
  - `cargo test -p roastty opentype -- --nocapture --test-threads=1`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/803-embedded-font-resources.md`
- Run:
  - `git diff --check`

The experiment passes if Roastty has a tested embedded-font resource inventory
for the 15 upstream resource-backed embedded font symbols and the checklist row
remains partial. It is Partial if only metadata lands without byte access. It
fails if the implementation would need generated default-font blobs before any
resource-backed inventory is useful.

## Design Review

Codex reviewed the design and found one blocking scope ambiguity: the original
plan said both "available resource-file inventory" and "vendored font files",
while requiring upstream symbol names. Those are different sets because
`vendor/ghostty/src/font/res/` contains files not referenced by upstream
`font/embedded.zig`, such as `Lilex-VF.ttf` and extra JetBrains Nerd Font
styles. The design now scopes the experiment to the exact 15 upstream
`@embedFile("res/...")` entries and requires tests for exact symbol/file
coverage. Codex re-reviewed the corrected design and approved it with no
blocking findings because the inventory boundary is now explicit, verification
covers the relevant risks, and the plan avoids generated default-font, Symbols
Nerd Font, font-grid, discovery, and frontend-loading claims.

## Result

**Result:** Pass

Roastty now has `font::embedded`, a resource-backed inventory for the exact 15
upstream `@embedFile("res/...")` entries:

- `emoji`
- `emoji_text`
- `arabic`
- `test_nerd_font`
- `code_new_roman`
- `inconsolata`
- `geist_mono`
- `jetbrains_mono`
- `julia_mono`
- `cozette`
- `monaspace_neon`
- `terminus_ttf`
- `spleen_bdf`
- `spleen_pcf`
- `spleen_otb`

Each inventory entry exposes its upstream symbol name, file name, license
family, role, and raw bytes. The implementation intentionally excludes
unreferenced vendored resources, generated JetBrains default-font blobs, the
Symbols Nerd Font blob, and all font-grid/discovery/frontend integration.

Verification:

- Inspected:
  - `vendor/ghostty/src/font/embedded.zig`
  - `vendor/ghostty/src/font/res/README.md`
  - `vendor/ghostty/src/font/res/OFL.txt`
  - `vendor/ghostty/src/font/res/MIT.txt`
  - `vendor/ghostty/src/font/res/BSD-2-Clause.txt`
- `cargo fmt -p roastty` — passed
- `cargo test -p roastty embedded -- --nocapture --test-threads=1` — 5 passed
- `cargo test -p roastty opentype -- --nocapture --test-threads=1` — 32 passed
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/803-embedded-font-resources.md`
  — passed
- `git diff --check` — passed

## Conclusion

The font row can now move from "embedded fonts missing" to a partial embedded
resource foundation. The remaining embedded-font work is generated default-font
blob replacement, Symbols Nerd Font blob replacement, and wiring these resources
into font discovery, collection construction, `SharedGridSet`, and frontend/app
font loading.

## Completion Review

Codex reviewed the staged result and found no blocking findings. The review
approved the inventory because it matches the exact 15 upstream
`@embedFile("res/...")` entries, includes `JetBrainsMonoNerdFont-Regular.ttf`
only for `test_nerd_font`, and excludes `Lilex-VF.ttf`, extra JetBrains Nerd
Font styles, generated JetBrains blobs, and `nerd_fonts_symbols_only`. The
review also approved the tests and docs because they cover exact symbol/file
order and count, uniqueness, non-empty bytes, signatures, license text
availability, and role comments while keeping the checklist partial and avoiding
font-grid/discovery/frontend integration claims.
