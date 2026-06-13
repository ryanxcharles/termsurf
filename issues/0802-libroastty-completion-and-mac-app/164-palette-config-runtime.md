# Experiment 164: Phase F — palette config runtime

## Description

Wire Ghostty's `palette`, `palette-generate`, and `palette-harmonious` config
surface through Roastty's parser, config ABI, and terminal runtime defaults.

Roastty already has a faithful `Palette` parser/formatter helper and terminal
APIs for reading/updating the default 256-color palette, but the top-level
`Config` struct does not expose the three upstream palette fields. As a result,
config files cannot set `palette = N=#rrggbb`, cannot request generated
theme-aware colors for indices 16-255, and spawned/updated surfaces always keep
the hardcoded default palette unless the host calls the lower-level terminal
option directly.

Upstream applies this in `termio/Termio.zig`'s derived config, not in
`Config.finalize()`: explicit `palette` values become the base palette, and
`palette-generate` derives indices 16-255 from the base 16 ANSI colors plus
background/foreground while preserving any user-set indices in the mask. This
experiment ports that behavior at the same runtime boundary.

## Changes

- `roastty/src/config/mod.rs`
  - Add `palette: Palette`, `palette_generate: bool`, and
    `palette_harmonious: bool` to `Config` with upstream defaults.
  - Wire parser reset/diagnostic behavior for `palette`, `palette-generate`, and
    `palette-harmonious`, including `PaletteParseError -> ConfigSetError`.
  - Format the three fields in upstream declaration order after
    `minimum-contrast` and before cursor color fields.
  - Add focused config tests for defaults, parse errors, repeated palette
    assignments, bool reset/default behavior, file/CLI replay stability, and
    formatted output.
- `roastty/src/terminal/color.rs`
  - Port upstream `generate256Color` and its CIELAB interpolation helpers.
  - Add tests matching upstream invariants and byte-level upstream parity:
    base16 preservation, cube endpoint behavior for dark/light themes, mask
    preservation, harmonious light-theme inversion behavior, grayscale ramp
    direction, and fixed upstream-derived RGB golden values for representative
    non-endpoint cube and grayscale indices in dark, light, harmonious-light,
    and masked cases.
- `roastty/src/termio.rs`
  - Extend `TermioSpawnOptions` with the derived palette and apply it to
    `Terminal::init_with_options` output before spawning the PTY child.
  - Add a focused test that spawn options initialize the terminal default
    palette.
- `roastty/src/lib.rs`
  - Add a helper that derives the effective palette from `config.palette`,
    `palette-generate`, `palette-harmonious`, `background`, and `foreground`,
    matching upstream `Termio.DerivedConfig`.
  - Pass the derived palette into `TermioSpawnOptions` when a surface starts.
  - Update `Surface::apply_config` so config reloads update any live terminal's
    default palette and mark the surface dirty.
  - Expose `palette`, `palette-generate`, and `palette-harmonious` via
    `roastty_config_get`; `palette` returns the existing C-compatible
    `RoasttyPalette` layout.
  - Add C ABI tests proving config get/clone/load behavior and app/surface
    runtime palette application for both direct palette values and generated
    palettes.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - After implementation, update the Phase F notes to replace the stale broad
    palette gap with the exact remaining config-field count/gaps.

Out of scope:

- Theme file loading and theme lookup. This experiment consumes the parsed
  `background`, `foreground`, and `palette` values already present on `Config`;
  it does not load new theme files.
- Conditional config reload. Runtime application is limited to direct config
  updates through the existing app/surface update path.
- Font variation/metric config fields, macOS config scalars, and input/keybind
  config fields that remain missing from Phase F.
- Changing OSC 4 / OSC 104 dynamic palette semantics. Runtime OSC changes still
  use the terminal's existing dynamic palette mask and reset behavior.

## Verification

- Format edited Rust:
  - `cargo fmt -p roastty`
- Format issue docs:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/164-palette-config-runtime.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Run focused Rust tests:
  - `cargo test -p roastty palette`
  - `cargo test -p roastty spawn_with_options_initializes_palette_defaults`
  - `cargo test -p roastty config_get_palette`
  - `cargo test -p roastty surface_apply_config_updates_palette`
- Run full Roastty coverage:
  - `cargo test -p roastty`
- Run checks:
  - `cargo fmt --check -p roastty`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/164-palette-config-runtime.md issues/0802-libroastty-completion-and-mac-app/README.md`
  - `git diff --check`

**Pass** = Roastty parses/formats/exposes the three upstream palette config
fields, generated palette output matches upstream invariants plus fixed
upstream-derived golden RGB samples, new terminals use the derived config
palette, live config updates refresh terminal defaults, and full Roastty tests
pass.

**Partial** = parser/ABI parity lands, but generated palette derivation or live
surface application needs a follow-up experiment.

**Fail** = the palette fields cannot be wired without conflicting with existing
terminal dynamic-color behavior or the C ABI layout.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Chandrasekhar`, fresh
context.

**Verdict:** Approved after one required verification fix.

**Findings:**

- Required: the initial design only required generated-palette invariant tests,
  which would not prove byte-faithful parity with upstream CIELAB interpolation.

**Fix:** Added explicit verification scope for fixed upstream-derived RGB golden
values covering representative non-endpoint cube and grayscale indices across
dark, light, harmonious-light, and masked cases.

The reviewer re-reviewed the fix and approved the design with no remaining
findings.

## Result

**Result:** Pass

Roastty now exposes Ghostty's `palette`, `palette-generate`, and
`palette-harmonious` config fields through the Rust config surface, C config
ABI, and terminal runtime defaults.

The config parser stores the upstream `Palette` value and user-set mask on
`Config`, formats all 256 `palette = N=#rrggbb` entries in upstream order, and
supports empty-value resets plus diagnostics for missing values, invalid colors,
overflowing indices, and invalid boolean flags. File/CLI replay preserves
palette assignments and generator flags.

The terminal color module now ports upstream `generate256Color`: CIELAB
interpolation derives the 216-color cube and 24-step grayscale ramp from the
base palette plus background/foreground, skips explicit user-set indices, and
keeps the light-theme `palette-harmonious` behavior. Tests pin representative
non-endpoint RGB samples for dark, light, harmonious-light, and masked cases.

Runtime wiring now derives the effective config palette in the same boundary as
upstream `Termio.DerivedConfig`. New `Termio` instances receive the derived
palette through `TermioSpawnOptions`, `roastty_config_get("palette")` returns
the raw configured C-compatible palette like upstream's C config getter, and
`roastty_surface_update_config` refreshes the default palette for live workers
while preserving the terminal's dynamic palette override semantics.

Verification:

- `cargo test -p roastty palette` passed 55 filtered unit tests plus the ABI
  harness filter.
- `cargo test -p roastty spawn_with_options_initializes_palette_defaults` passed
  1 filtered unit test plus the ABI harness filter.
- `cargo test -p roastty config_get_palette` passed 1 filtered unit test plus
  the ABI harness filter.
- `cargo test -p roastty surface_apply_config_updates_palette` passed 1 filtered
  unit test plus the ABI harness filter.
- `cargo test -p roastty surface_foreground_pid_reports_worker_foreground_pid_after_start`
  passed when rerun alone after one unrelated full-suite foreground-PID mismatch
  in an earlier full run.
- Final `cargo test -p roastty` passed 4,860 Rust unit tests, 0 failed, 4
  ignored; the C ABI harness passed with the existing enum-conversion warnings;
  doc tests passed with 0 tests.
- `cargo fmt --check -p roastty` passed.
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/164-palette-config-runtime.md issues/0802-libroastty-completion-and-mac-app/README.md`
  passed.
- `git diff --check` passed.

## Completion Review

**Reviewers:** Codex-native adversarial review subagents `Kierkegaard` and
`Ampere`, fresh context.

`Kierkegaard` approved the completed result with no findings after independently
verifying the result commit had not been made, `git diff --check`,
`cargo fmt --check -p roastty`, focused palette/runtime/config tests, and
Prettier.

`Ampere` found one required upstream-fidelity issue:
`roastty_config_get("palette")` returned the generated runtime palette, while
upstream's C config getter returns the raw configured palette and keeps
generation inside `Termio.DerivedConfig`.

**Fix:** `roastty_config_get("palette")` now returns
`config.parsed.palette.value`, and the C ABI test proves generated-palette flags
do not affect the raw returned palette while clone/load behavior still preserves
the raw palette and boolean flags. The result text now documents the raw C
getter semantics separately from runtime derivation.

`Ampere` re-reviewed the fix and approved it with no remaining required
findings.

## Conclusion

The palette config group is no longer a parser-only helper or a host-only
terminal option. It now behaves as a first-class upstream config field group:
config files can define explicit 256-color entries, generated palettes are
derived and tested at byte-level sample points for runtime use, the C config
getter preserves upstream raw-config semantics, and both new and live terminals
receive the effective defaults. The remaining Phase F config surface excludes
palette and is now 28 public upstream keys.
