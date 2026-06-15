# Experiment 150: Font Metric Modifier Runtime

## Description

`RUNTIME-007B2B2B2` still owns remaining font renderer-output gaps. A narrow
deterministic slice inside that row is the runtime use of the parsed `adjust-*`
metric modifier config fields:

- `adjust-cell-width`;
- `adjust-cell-height`;
- `adjust-font-baseline`;
- `adjust-underline-position`;
- `adjust-underline-thickness`;
- `adjust-strikethrough-position`;
- `adjust-strikethrough-thickness`;
- `adjust-overline-position`;
- `adjust-overline-thickness`;
- `adjust-cursor-thickness`;
- `adjust-cursor-height`;
- `adjust-box-thickness`;
- `adjust-icon-height`.

Pinned Ghostty stores these fields in `SharedGridSet.DerivedConfig`, copies them
into `SharedGridSet.Key.metric_modifiers`, hashes the modifier set as part of
the font-grid key, and applies the modifier set during
`Collection.updateMetrics` after calculating metrics from the primary face.

Roastty already has a substantial lower-level port: the config parser/formatter
for all 13 `adjust-*` rows exists, `font::metrics::Modifier`,
`font::metrics::Key`, `font::metrics::ModifierSet`, and `Metrics::apply` exist,
and the lower-level metric modifier tests cover modifier math including
cell-height recentering and icon-height fanout. The remaining deterministic gap
is that `shared_grid_set` does not yet carry the parsed config metric modifiers
into the font-grid key, and `Collection::update_metrics` always stores
unmodified `Metrics::calc(primary_face_metrics)`.

This experiment will split `RUNTIME-007B2B2B2`:

- `RUNTIME-007B2B2B2A`: **Oracle complete** for deterministic `adjust-*` metric
  modifier propagation from config into font-grid key separation and
  `Collection::update_metrics` metric calculation.
- `RUNTIME-007B2B2B2B`: **Gap** for remaining font renderer-output parity:
  fallback/shaping visual output, bitmap/color font thickening edge cases, glyph
  metrics as seen by the renderer beyond modifier math, and broader
  renderer-visible font pixel parity.

This experiment will not claim GUI/pixel proof for modified glyph rendering,
fallback visual parity, color-font thickening parity, or broad font pixel
parity.

## Changes

- `roastty/src/font/collection.rs`
  - Add a `metric_modifiers: font::metrics::ModifierSet` field to `Collection`.
  - Add a small setter or constructor path that lets `shared_grid_set` install a
    modifier set before `update_metrics`.
  - Update `Collection::update_metrics` to apply the stored modifier set after
    `Metrics::calc`, matching pinned Ghostty `Collection.updateMetrics`.
  - Keep existing default behavior unchanged when the modifier set is empty.
  - Add focused tests proving:
    - an empty modifier set still produces the same metrics as
      `Metrics::calc(primary_face_metrics)`;
    - configured modifiers affect cached collection metrics after
      `update_metrics`;
    - cell-height modifiers recenter baseline/decoration positions through the
      existing `Metrics::apply` path rather than bespoke collection logic.
- `roastty/src/font/shared_grid_set.rs`
  - Extend `DerivedConfig` to carry all 13 parsed `adjust-*` config fields.
  - Add `metric_modifiers` to the config-derived `Key`, include it in equality
    and hashing, and build it from the config fields in Ghostty `Metrics.Key`
    order.
  - Pass the key's modifier set into the `Collection` before calling
    `update_metrics`.
  - Add focused tests proving:
    - each canonical `adjust-*` config field maps to the intended
      `font::metrics::Key`;
    - two otherwise-identical configs with different metric modifiers produce
      different font-grid keys/hashes;
    - `build_grid_from_config` returns metrics changed by representative
      configured modifiers.
- `issues/0805-roastty-ghostty-parity/font_metric_modifier_runtime_parity.py`
  - Add a static guard checking pinned Ghostty's `DerivedConfig` metric fields,
    `Key.metric_modifiers` construction/hash, `Collection.updateMetrics`
    application, Roastty's config fields, key/set wiring, focused tests, and
    inventory split.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-007B2B2B2` into `RUNTIME-007B2B2B2A` and
    `RUNTIME-007B2B2B2B`.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-223 summary. It must remain `Gap`.
- Existing CFG-223/static runtime guards
  - Update current runtime row counts from 57/51/53/4/4 to 58/52/54/4/4.
  - Update references from `RUNTIME-007B2B2B2` to `RUNTIME-007B2B2B2B` where
    they mean the remaining font renderer-output gap.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add the experiment link and update Learnings after the result.

## Verification

Pass criteria:

- Pinned Ghostty evidence shows `SharedGridSet.DerivedConfig` carries all 13
  `adjust-*` config fields, `SharedGridSet.Key` builds and hashes
  `metric_modifiers`, and `Collection.updateMetrics` applies the modifier set to
  calculated primary-face metrics.
- Roastty carries all 13 parsed `adjust-*` fields into a config-derived
  `ModifierSet`.
- Roastty config-derived font-grid keys differ when metric modifiers differ.
- Roastty applies metric modifiers during `Collection::update_metrics`, and
  default empty modifiers preserve existing metrics.
- Representative `build_grid_from_config` metrics reflect configured modifier
  effects, including cell-height recentering through `Metrics::apply`.
- `RUNTIME-007B2B2B2A` is Oracle complete and cites focused tests plus the new
  static guard.
- `RUNTIME-007B2B2B2B` remains `Gap` for fallback/shaping visual output,
  bitmap/color font thickening edge cases, glyph metrics as seen by the renderer
  beyond modifier math, and broader renderer-visible font pixel parity.
- `CFG-223` remains `Gap`.

Commands:

```bash
cargo test --manifest-path roastty/Cargo.toml font_metric_modifier_runtime
cargo test --manifest-path roastty/Cargo.toml metric_modifier_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml metric_modifier_config_formatter_family_oracle
cargo test --manifest-path roastty/Cargo.toml apply_cell_height
cargo test --manifest-path roastty/Cargo.toml apply_icon_height
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/font_metric_modifier_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
for guard in issues/0805-roastty-ghostty-parity/*_runtime_parity.py; do PYTHONDONTWRITEBYTECODE=1 python3 "$guard" || exit 1; done
cargo fmt --manifest-path roastty/Cargo.toml
cargo fmt --manifest-path roastty/Cargo.toml --check
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/150-font-metric-modifier-runtime.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

Fail criteria:

- Any canonical `adjust-*` config field is not represented in the derived
  `ModifierSet`.
- A config field maps to the wrong `font::metrics::Key`.
- The font-grid key/hash ignores modifier differences.
- Collection metrics remain unmodified after configured modifiers are installed.
- Empty modifier behavior changes existing default metrics.
- The experiment promotes fallback visual parity, color-font thickening edge
  cases, renderer glyph pixels, or broad font pixel parity from the remaining
  gap.
- CFG-223 is marked complete.

## Design Review

Adversarial review by fresh-context Codex subagent approved the design with no
required findings. The reviewer verified that the README links Experiment 150 as
`Designed`, the experiment has the required Description/Changes/Verification
sections, the scope is narrow, the plan matches pinned Ghostty's metric modifier
flow, the verification criteria are concrete, and CFG-223 plus broad font pixel
parity remain gaps.

The reviewer raised one optional hygiene finding: the prettier command should
also include regenerated `config-runtime-inventory.md` and `config-matrix.md`.
The plan was updated to include both files in that command.

## Result

**Result:** Pass

Roastty now threads all 13 parsed `adjust-*` metric modifier config fields into
the font-grid runtime path. `DerivedConfig` carries the parsed values, the
config-derived font-grid `Key` builds and hashes a `ModifierSet`, and
`Collection::update_metrics` applies the stored modifiers after calculating
primary-face metrics.

The implementation keeps default behavior unchanged for an empty modifier set.
Focused tests prove all canonical config fields map to the intended
`font::metrics::Key`, modifier differences split font-grid keys, collection
metrics apply modifiers through `Metrics::apply`, cell-height modifiers recenter
baseline and decoration positions through the existing metric logic, and
`build_grid_from_config` returns modified grid metrics.

`RUNTIME-007B2B2B2` was split into:

- `RUNTIME-007B2B2B2A`: **Oracle complete** for deterministic `adjust-*` metric
  modifier propagation into font-grid key separation and collection metric
  calculation.
- `RUNTIME-007B2B2B2B`: **Gap** for fallback/shaping visual output, bitmap/color
  font thickening edge cases, glyph metrics as seen by the renderer beyond
  modifier math, and broader renderer-visible font pixel parity.

Verification passed:

```bash
cargo test --manifest-path roastty/Cargo.toml font_metric_modifier_runtime
cargo test --manifest-path roastty/Cargo.toml metric_modifier_config_parser_family_oracle
cargo test --manifest-path roastty/Cargo.toml metric_modifier_config_formatter_family_oracle
cargo test --manifest-path roastty/Cargo.toml apply_cell_height
cargo test --manifest-path roastty/Cargo.toml apply_icon_height
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/font_metric_modifier_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
for guard in issues/0805-roastty-ghostty-parity/*_runtime_parity.py; do PYTHONDONTWRITEBYTECODE=1 python3 "$guard" || exit 1; done
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py
cargo fmt --manifest-path roastty/Cargo.toml --check
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/150-font-metric-modifier-runtime.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

The regenerated CFG-223 inventory reports:

- `runtime_rows=58`
- `oracle_complete=52`
- `closed=54`
- `incomplete=4`
- `gap=4`
- `cfg223=Gap`

## Conclusion

Metric modifier config is no longer parser-only. It now reaches the same
font-grid key and collection metric calculation stage as pinned Ghostty, so
future font experiments should focus on the remaining `RUNTIME-007B2B2B2B`
visual/glyph-output gaps rather than modifier propagation.

## Completion Review

Adversarial review by fresh-context Codex subagent approved the completed
experiment with no findings. The reviewer independently verified:

- `cargo test --manifest-path roastty/Cargo.toml font_metric_modifier_runtime`;
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/font_metric_modifier_runtime_parity.py`;
- runtime inventory regeneration to `/tmp` with `runtime_rows=58`,
  `oracle_complete=52`, `closed=54`, `incomplete=4`, `gap=4`, `cfg223=Gap`;
- `cargo fmt --manifest-path roastty/Cargo.toml --check`;
- `git diff --check`.

The reviewer confirmed all 13 `adjust-*` fields are carried into `DerivedConfig`
and mapped to expected `font::metrics::Key` entries, `Key` equality/hash
includes metric modifiers in deterministic `MetricKey::ALL` order,
`Collection::update_metrics` applies modifiers after `Metrics::calc`, empty-set
behavior is covered and unchanged, tests are meaningful, the inventory split is
honest, CFG-223 remains `Gap`, and the result commit had not yet been made.
