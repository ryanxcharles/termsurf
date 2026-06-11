+++
implementer = "codex"
review_design = "codex-adversarial"
+++

# Experiment 88: Phase F — bell features config

## Description

Port the pinned upstream `bell-features` config surface from
`vendor/ghostty/src/config/Config.zig` into `roastty/src/config/mod.rs`.

Upstream defines `bell-features` immediately after `custom-shader-animation` as
a packed bool struct:

- `system = false`
- `audio = false`
- `attention = true`
- `title = true`
- `border = false`

Its CLI/config syntax is upstream's packed-struct bool-flag syntax: a standalone
bool sets every flag, and comma-separated `[no-]flag` names override individual
fields while omitted fields keep their defaults. Empty assigned values reset to
the default value, and missing values diagnose as `ValueRequired`.

This experiment is parser/formatter-only. Runtime bell delivery, system alert
callbacks, app attention behavior, title markers, alerted borders, and custom
audio playback remain later work.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Config::bell_features: BellFeatures` in upstream order after
    `custom_shader_animation`; in the current local struct/default region this
    means before `background`, leaving the pre-existing local `scroll_to_bottom`
    placement untouched.
  - Initialize the default to `BellFeatures::default()`.
  - Format `bell-features` after `custom-shader-animation` and before
    `macos-non-native-fullscreen`, matching the local format order slot for this
    region.
  - Route `Config::set("bell-features", ...)` through the existing
    `set_packed_field` helper.
  - Add a `BellFeatures` struct with the five upstream flags, `Default`,
    `parse_cli`, and `format_entry`, reusing the local `parse_packed_flags` /
    `EntryFormatter::entry_flags` pattern already used for
    `NotifyOnCommandFinishAction`, `FontSyntheticStyle`, `ScrollToBottom`, and
    other packed fields.
  - Extend default-value, format-order, and config-set route tests.
  - Add focused tests for:
    - upstream defaults (`attention,title` enabled; `system,audio,border`
      disabled);
    - formatting order and canonical `[no-]flag` output;
    - individual flag enable/disable parsing;
    - standalone bool setting all five flags;
    - empty value resetting to defaults;
    - missing value returning `ValueRequired`;
    - unknown flags returning `InvalidValue`;
    - clone/equality preserving values.

- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed` in the experiment index.
  - After implementation, add an operating note describing the parser-only
    status and runtime work left open.

## Verification

Before implementation:

- Codex-native adversarial design review approves the experiment.
- Plan commit exists before source edits begin.

After implementation:

- `cargo fmt`
- `cargo test -p roastty bell_features`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
- `cargo fmt --check`
- `git diff --check`

Pass criteria:

- `bell-features` is present in defaults, formatter output, `Config::set`, and
  format-order tests in the same upstream-order region as
  `custom-shader-animation`.
- The packed-flag semantics match upstream's `BellFeatures` defaults and
  `parsePackedStruct` behavior for bool-all, `[no-]flag` lists, empty reset,
  missing values, and invalid names.
- Runtime bell behavior is not claimed or changed by this experiment.

## Design Review

Codex adversarial reviewer `019eb500-503e-7ce3-ad15-1599d3a2c23e` initially
returned **Changes Required**. The required finding was real: the plan said to
add `bell_features` after `custom_shader_animation` and before
`scroll_to_bottom`, but the current local struct/default region already places
`scroll_to_bottom` before `custom_shader_animation`, so that instruction was
contradictory.

The design was fixed to add `bell_features` after `custom_shader_animation` and
before `background`, explicitly leaving the existing local `scroll_to_bottom`
placement untouched. The reviewer re-reviewed the fix and returned **Approved**,
confirming the corrected placement matches the current local field/default
region and formatter slot.
