# Experiment 7: Config Compatibility Alias Semantics

## Description

Experiment 6 proved that the pinned Ghostty config inventory contains eight
compatibility map entries. Some are true legacy key aliases, and some are
legacy-value shims on canonical keys:

- `background-blur-radius`
- `adw-toolbar-style`
- `gtk-tabs-location`
- `cursor-invert-fg-bg`
- `selection-invert-fg-bg`
- `bold-is-bright`
- `gtk-single-instance`
- `macos-dock-drop-behavior`

Those compatibility rows remain `Gap` because inventory only proves that the
entries exist upstream, not that Roastty accepts them or applies Ghostty's
compatibility semantics. This experiment should prove and, where needed, fix the
Roastty parser behavior for all eight entries as a small config-only slice.

The scope is alias parsing and immediate config-state effects only. Runtime GUI
effects, app behavior, and full config option semantics remain later
experiments.

## Changes

- `roastty/src/config/mod.rs`
  - Add or verify parser support for each of the eight compatibility entries.
  - Preserve Ghostty semantics from `vendor/ghostty/src/config/Config.zig`:
    - `background-blur-radius` maps to `background-blur`.
    - `adw-toolbar-style` maps to `gtk-toolbar-style`.
    - `gtk-tabs-location = hidden` sets `window-show-tab-bar = never`.
    - `cursor-invert-fg-bg` truthy/default sets `cursor-color = cell-foreground`
      and `cursor-text = cell-background`.
    - `selection-invert-fg-bg` truthy/default sets
      `selection-foreground = cell-background` and
      `selection-background = cell-foreground`.
    - `bold-is-bright` truthy/default sets `bold-color = bright`.
    - `gtk-single-instance = desktop` maps to `gtk-single-instance = detect`.
    - `macos-dock-drop-behavior = window` maps to
      `macos-dock-drop-behavior = new-window`.
  - Add focused unit tests that cover accepted values, ignored/false values
    where Ghostty ignores them, and invalid values where Ghostty rejects them.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Update the eight compatibility rows from `Gap` to `Pass` only if the unit
    tests prove the compatibility semantics.
  - Leave the 203 canonical option rows as `Gap` unless this experiment proves a
    specific canonical behavior as part of an alias mapping.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning if the experiment discovers a reusable alias-compatibility
    pattern or a Ghostty semantic wrinkle that future config experiments should
    know.

## Verification

Pass/fail criteria:

- Every compatibility map entry from `config-inventory.md` has a focused Roastty
  unit test that proves its parser effect against the pinned Ghostty behavior.
- The tests cover both direct `Config::set`-style parsing and `load_str` config
  file parsing when those paths can differ.
- The tests prove false/ignored values for the boolean compatibility shims do
  not accidentally set the replacement config values.
- The updated matrix marks exactly the eight compatibility rows `Pass` and
  leaves unproven canonical rows as `Gap`.
- Rust formatting, markdown formatting, focused tests, and diff hygiene pass.

Suggested commands:

```bash
cargo fmt --manifest-path roastty/Cargo.toml --check
cargo test --manifest-path roastty/Cargo.toml config_compatibility_alias
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/07-config-compatibility-alias-semantics.md \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

If Roastty already implements some aliases, this experiment should still add
explicit tests and matrix evidence rather than relying on source inspection.

## Design Review

Fresh-context adversarial design review approved the design with no required
findings.

## Result

**Result:** Pass

Roastty now implements and tests all eight Ghostty compatibility map entries
from the pinned config source.

Changes made:

- `roastty/src/config/mod.rs`
  - Added true renamed-key support for `background-blur-radius` and
    `adw-toolbar-style`.
  - Added removed boolean compatibility shims for `cursor-invert-fg-bg`,
    `selection-invert-fg-bg`, and `bold-is-bright`.
  - Reused the already-present canonical compatibility behavior for
    `gtk-tabs-location = hidden`, `gtk-single-instance = desktop`, and
    `macos-dock-drop-behavior = window`.
  - Added `config_compatibility_alias_semantics`, a focused unit test covering
    direct `Config::set` parsing, `load_str` config-file parsing, false/no-op
    boolean values, and invalid diagnostics.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Updated exactly the eight compatibility rows to `Pass`.
  - Left all 203 canonical config option rows as `Gap`.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Added a learning that Ghostty compatibility entries mix renamed keys, legacy
    values, and removed boolean shims.

Verification results:

- `logs/issue805-exp7-config-compatibility-alias-test.log` records
  `config::tests::config_compatibility_alias_semantics ... ok` with 1 passed, 0
  failed.
- `logs/issue805-exp7-config-matrix-counts.log` records:
  - `total_cfg_rows=212`
  - `alias_rows=8`
  - `alias_pass_rows=8`
  - `canonical_rows=203`
  - `canonical_pass_rows=0`
  - `total_pass_rows=9`
- `cargo fmt --manifest-path roastty/Cargo.toml --check` passed and was saved to
  `logs/issue805-exp7-cargo-fmt-check.log`.
- `prettier --write --prose-wrap always --print-width 80` passed for the edited
  markdown files.
- `git diff --check` passed.

## Conclusion

The eight Ghostty compatibility map entries are no longer semantic gaps at the
parser/config-state level. Canonical config behavior remains mostly unproven and
should continue in narrow semantic groups such as defaults/formatting,
diagnostics, file precedence, or runtime effects.

## Completion Review

Fresh-context adversarial completion review approved the result with no required
findings. The reviewer confirmed that the result commit had not yet been made,
the README and experiment statuses match `Pass`, exactly the eight compatibility
rows are marked `Pass`, canonical config rows remain `Gap`, the focused test
covers direct `Config::set`, `load_str`, false/no-op boolean cases, and invalid
diagnostics, and the saved verification logs match the claimed results.
