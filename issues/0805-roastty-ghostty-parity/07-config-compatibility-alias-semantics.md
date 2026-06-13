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
