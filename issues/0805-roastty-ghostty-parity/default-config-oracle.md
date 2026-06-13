# Default Config Oracle

Experiment 8 records the first reusable A/B oracle for pinned Ghostty default
config formatting versus Roastty `Config::default().format_config(...)`.

## Reference

- Ghostty commit: `2c62d182cec246764ff725096a70b9ef44996f7f`
- Ghostty executable:
  `vendor/ghostty/zig-out/Ghostty.app/Contents/MacOS/ghostty`
- Ghostty fixture: `roastty/testdata/issue805-ghostty-default-config.txt`
- Roastty implementation: `roastty/src/config/mod.rs`

## Regeneration

Capture the pinned Ghostty default config:

```bash
vendor/ghostty/zig-out/Ghostty.app/Contents/MacOS/ghostty \
  +show-config --default --no-pager \
  > logs/issue805-exp8-ghostty-default-config.txt
cp logs/issue805-exp8-ghostty-default-config.txt \
  roastty/testdata/issue805-ghostty-default-config.txt
```

Capture Roastty's current default formatter output while running the oracle
test:

```bash
ROASTTY_DEFAULT_CONFIG_OUT=/Users/astrohacker/dev/termsurf/logs/issue805-exp8-roastty-default-config.txt \
  cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle -- --nocapture
```

Run the durable guard without regenerating the output log:

```bash
cargo test --manifest-path roastty/Cargo.toml config_default_format_oracle
```

## Normalization

The test normalizes only the application rename:

- `Ghostty` and `Roastty` become `{App}`.
- `ghostty` and `roastty` become `{app}`.

The test does not normalize semantic values. It compares all non-`keybind` and
non-`command-palette-entry` lines exactly and in order.

## Current Result

Current counts from `logs/issue805-exp8-default-config-diff-summary.txt`:

- Ghostty raw lines: 635
- Roastty raw lines: 628
- Comparable lines excluding `keybind` and `command-palette-entry`: 454 on each
  side
- Comparable exact match after app-name normalization: true
- Ghostty `keybind` lines: 93
- Roastty `keybind` lines: 86
- `keybind` multiset mismatches: 135
- Ghostty `command-palette-entry` lines: 88
- Roastty `command-palette-entry` lines: 88
- `command-palette-entry` multiset mismatches: 2
- Total missing normalized lines: 72
- Total extra normalized lines: 65
- Missing key counts: `keybind` 7
- Extra key counts: none

## Gaps

The remaining default-format diffs are tracked as gaps, not intentional
divergences:

- Default keybinding formatting and default keybinding contents differ.
- One command-palette default entry has different escaped text formatting,
  producing one missing and one extra multiset element.

This oracle proves default values, formatter output, and formatter order only
for the comparable default-format surface. It does not prove parser behavior,
diagnostics, config-file precedence, CLI/env precedence, config reload, or
runtime effects.
