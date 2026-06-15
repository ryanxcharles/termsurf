# Experiment 149: Font Variation Runtime

## Description

`RUNTIME-007B2B2B` still owns several font renderer-output gaps. A narrow
deterministic slice inside that row is the four parsed `font-variation*` config
lists:

- `font-variation`;
- `font-variation-bold`;
- `font-variation-italic`;
- `font-variation-bold-italic`.

Pinned Ghostty stores variation axes on font discovery descriptors. Those
descriptor variations participate in descriptor hashing, survive cloning into
deferred faces, and are reapplied when the CoreText face is loaded because
CoreText collection creation may reset axes.

Roastty already has the lower-level pieces:

- `Config` parses and formats all four `font-variation*` lists;
- `font::discovery::Descriptor` has a `variations` field and includes it in
  descriptor hashing;
- `DeferredFace::load` calls `Face::set_variations`;
- `Face::set_variations` rebuilds the CoreText font from descriptor variation
  copies.

The remaining deterministic gap is that `shared_grid_set` builds config-derived
font descriptors without copying `Config.font_variation*` into the matching
descriptor for each style. This experiment will thread the parsed variation
lists into descriptor construction and prove they affect the config-derived
font-grid key, deferred face loading path, and inventory split.

This experiment will split `RUNTIME-007B2B2B`:

- `RUNTIME-007B2B2B1`: **Oracle complete** for deterministic `font-variation*`
  config propagation into style-specific font discovery descriptors,
  descriptor/key hashing, deferred CoreText face loading, and the existing
  CoreText `set_variations` mechanics.
- `RUNTIME-007B2B2B2`: **Gap** for remaining font renderer-output parity: metric
  adjustment, fallback/shaping visual output, bitmap/color font thickening edge
  cases, glyph metrics as seen by the renderer, and broader renderer-visible
  font pixel parity.

This experiment will not claim visual proof that a variable font changes glyph
pixels, metric-adjustment parity, fallback visual parity, color-font thickening
parity, or broad font pixel parity.

## Changes

- `roastty/src/font/shared_grid_set.rs`
  - Extend `DerivedConfig` to carry all four `font_variation*` config lists.
  - Convert `config::FontVariation` axis ids into `font::discovery::Variation`
    values.
  - Pass the matching variation list into descriptor construction for each
    style:
    - regular descriptors use `font_variation`;
    - bold descriptors use `font_variation_bold`;
    - italic descriptors use `font_variation_italic`;
    - bold-italic descriptors use `font_variation_bold_italic`.
  - Add focused tests proving:
    - each style descriptor receives only its matching variation list;
    - variation differences change the config-derived `Key`/hash;
    - style offsets remain correct with variations present;
    - building a grid with configured variations still yields a usable grid on
      macOS.
- `issues/0805-roastty-ghostty-parity/font_variation_runtime_parity.py`
  - Add a static guard checking pinned Ghostty descriptor variation fields, hash
    participation, clone/deferred propagation, CoreText `setVariations`, Roastty
    config fields, descriptor construction, tests, and inventory split.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-007B2B2B` into `RUNTIME-007B2B2B1` and `RUNTIME-007B2B2B2`.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-223 summary. It must remain `Gap`.
- Existing CFG-223/static runtime guards
  - Update current runtime row counts from 56/50/52/4/4 to 57/51/53/4/4.
  - Update references from `RUNTIME-007B2B2B` to `RUNTIME-007B2B2B2` where they
    mean the remaining font renderer-output gap.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add the experiment link and update Learnings after the result.

## Verification

Pass criteria:

- Pinned Ghostty evidence shows font discovery descriptors carry variation axes,
  descriptor hashing includes variations, cloning preserves variations, deferred
  faces retain variations, and CoreText face load reapplies variations.
- Roastty `DerivedConfig` carries all four parsed `font-variation*` lists into
  `shared_grid_set`.
- Roastty regular, bold, italic, and bold-italic descriptors receive the
  matching style-specific variation list and no variation list from another
  style.
- Roastty config-derived font-grid keys differ when variation values differ.
- Existing no-variation behavior remains unchanged.
- Roastty can build a usable config-derived font grid with configured
  variations.
- `RUNTIME-007B2B2B1` is Oracle complete and cites focused tests plus the new
  static guard.
- `RUNTIME-007B2B2B2` remains `Gap` for metric adjustment, fallback/shaping
  visual output, bitmap/color font thickening edge cases, glyph metrics as seen
  by the renderer, and broader font pixel parity.
- `CFG-223` remains `Gap`.

Commands:

```bash
cargo test --manifest-path roastty/Cargo.toml font_variation_runtime
cargo test --manifest-path roastty/Cargo.toml font_variation_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml font_variation_config_formatter_family_oracle
cargo test --manifest-path roastty/Cargo.toml deferred_face_load_applies_variations
cargo test --manifest-path roastty/Cargo.toml set_variations_runs_on_face
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/font_variation_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
for guard in issues/0805-roastty-ghostty-parity/*_runtime_parity.py; do PYTHONDONTWRITEBYTECODE=1 python3 "$guard" || exit 1; done
cargo fmt --manifest-path roastty/Cargo.toml
cargo fmt --manifest-path roastty/Cargo.toml --check
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/149-font-variation-runtime.md
git diff --check
```

Fail criteria:

- Any style-specific `font-variation*` config list is not represented in its
  matching font discovery descriptor.
- A style receives another style's variation list.
- The config-derived font-grid key/hash ignores variation differences.
- Configured variations bypass deferred face loading or CoreText
  `set_variations`.
- Existing no-variation behavior changes.
- The experiment promotes metric adjustment, fallback visual parity, color-font
  thickening edge cases, glyph metrics, or broad font pixel parity from the
  remaining gap.
- CFG-223 is marked complete.

## Design Review

Adversarial review by fresh-context Codex subagent approved the design with no
required findings. The reviewer verified that the README links Experiment 149 as
`Designed`, the experiment has the required Description/Changes/Verification
sections, the scope is a narrow `RUNTIME-007B2B2B` font-variation slice, the
style-specific variation mapping matches pinned Ghostty `SharedGridSet.zig`, the
verification includes focused tests/static guard/inventory regeneration/hygiene
checks, and the plan keeps CFG-223 plus broader visual/pixel font parity as
gaps.

## Result

**Result:** Pass

Roastty now threads all four parsed `font-variation*` lists into style-specific
config-derived font discovery descriptors:

- regular descriptors receive `font-variation`;
- bold descriptors receive `font-variation-bold`;
- italic descriptors receive `font-variation-italic`;
- bold-italic descriptors receive `font-variation-bold-italic`.

The config-derived font-grid key now changes when variation values change, style
offsets remain stable with variations present, no-variation configs keep
descriptor variation lists empty, and a configured-variation grid still resolves
ASCII on macOS. The implementation also carries pinned Ghostty's styled
variation retry: if a non-regular descriptor with variations cannot discover a
styled face, Roastty retries discovery with bold/italic search bits cleared.

`RUNTIME-007B2B2B` was split into:

- `RUNTIME-007B2B2B1`: **Oracle complete** for deterministic `font-variation*`
  config propagation into style-specific descriptors, font-grid key separation,
  deferred face loading, and CoreText variation application mechanics.
- `RUNTIME-007B2B2B2`: **Gap** for metric adjustment, fallback/shaping visual
  output, bitmap/color font thickening edge cases, glyph metrics as seen by the
  renderer, and broader renderer-visible font pixel parity.

Verification passed:

```bash
cargo test --manifest-path roastty/Cargo.toml font_variation_runtime
cargo test --manifest-path roastty/Cargo.toml font_variation_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml font_variation_config_formatter_family_oracle
cargo test --manifest-path roastty/Cargo.toml deferred_face_load_applies_variations
cargo test --manifest-path roastty/Cargo.toml set_variations_runs_on_face
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/font_variation_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
for guard in issues/0805-roastty-ghostty-parity/*_runtime_parity.py; do PYTHONDONTWRITEBYTECODE=1 python3 "$guard" || exit 1; done
cargo fmt --manifest-path roastty/Cargo.toml --check
git diff --check
```

The regenerated CFG-223 inventory reports:

- `runtime_rows=57`
- `oracle_complete=51`
- `closed=53`
- `incomplete=4`
- `gap=4`
- `cfg223=Gap`

## Conclusion

Font variation config is no longer only parsed and stored. It now reaches the
font-grid descriptor/runtime path in the same style-specific shape as pinned
Ghostty, participates in font-grid key separation, and remains wired through
deferred CoreText face loading. Future font experiments should focus on the
remaining `RUNTIME-007B2B2B2` visual/metric gaps rather than descriptor
propagation.

## Completion Review

Adversarial review by fresh-context Codex subagent approved the completed
experiment with no required, optional, or nit findings. The reviewer
independently verified:

- `cargo test --manifest-path roastty/Cargo.toml font_variation_runtime`;
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/font_variation_runtime_parity.py`;
- `cargo fmt --manifest-path roastty/Cargo.toml --check`;
- `git diff --check`;
- regenerated runtime inventory counts: `runtime_rows=57`, `oracle_complete=51`,
  `closed=53`, `incomplete=4`, `gap=4`, `cfg223=Gap`.

The reviewer noted one non-blocking command caveat: the requested `/tmp` matrix
regeneration path needed a seeded matrix file because
`config_runtime_inventory.py` updates an existing matrix instead of creating one
from scratch. After seeding from the current matrix, regeneration succeeded and
the only expected matrix difference was the embedded `/tmp` evidence path.
