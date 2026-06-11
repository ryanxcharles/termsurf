+++
implementer = "codex"
review_design = "codex-adversarial"
+++

# Experiment 89: Phase F — app notifications config

## Description

Port the pinned upstream `app-notifications` config surface from
`vendor/ghostty/src/config/Config.zig` into `roastty/src/config/mod.rs`.

Upstream defines `app-notifications` after `bell-audio-volume` as a packed bool
struct:

- `clipboard-copy = true`
- `config-reload = true`

Its CLI/config syntax is upstream's packed-struct bool-flag syntax: a standalone
bool sets every flag, and comma-separated `[no-]flag` names override individual
fields while omitted fields keep their defaults. Empty assigned values reset to
the default value, and missing values diagnose as `ValueRequired`.

This experiment is parser/formatter-only. GTK toast delivery, clipboard-copy
notification UI, config-reload notification UI, app C ABI exposure, and any
runtime notification behavior remain later work.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Config::app_notifications: AppNotifications` after `bell_features` and
    before `background` in the current local struct/default region, leaving the
    pre-existing local `bell_audio_path` / `bell_audio_volume` placement
    untouched.
  - Initialize the default to `AppNotifications::default()`.
  - Format `app-notifications` after `bell-features` and before
    `macos-non-native-fullscreen`, using the current local upstream-order slot
    for this config region.
  - Route `Config::set("app-notifications", ...)` through the existing
    `set_packed_field` helper.
  - Add an `AppNotifications` struct with the two upstream flags, `Default`,
    `parse_cli`, and `format_entry`, reusing the local `parse_packed_flags` /
    `EntryFormatter::entry_flags` pattern.
  - Extend default-value, format-order, and aggregate config-set route tests.
  - Add focused tests for:
    - upstream defaults (`clipboard-copy,config-reload` enabled);
    - formatting order and canonical `[no-]flag` output;
    - individual flag enable/disable parsing;
    - standalone bool setting both flags;
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
- `cargo test -p roastty app_notifications`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
- `cargo fmt --check`
- `git diff --check`

Pass criteria:

- `app-notifications` is present in defaults, formatter output, `Config::set`,
  and format-order tests in the same upstream-order region as `bell-features`.
- The packed-flag semantics match upstream's `AppNotifications` defaults and
  `parsePackedStruct` behavior for bool-all, `[no-]flag` lists, empty reset,
  missing values, and invalid names.
- Runtime notification behavior is not claimed or changed by this experiment.

## Design Review

Codex adversarial reviewer `019eb513-4106-7033-8c4e-c916ba975882` returned
**Approved** with no required findings. The reviewer confirmed the README link,
required sections, parser/formatter-only scope, upstream defaults, packed-struct
semantics, local placement plan, and verification checklist.

## Result

**Result:** Pass

Implemented `app-notifications` in `roastty/src/config/mod.rs` as an
`AppNotifications` packed bool struct matching upstream's pinned defaults:

- `clipboard-copy = true`
- `config-reload = true`

`Config` now stores `app_notifications`, initializes it from
`AppNotifications::default()`, formats `app-notifications` immediately after
`bell-features`, and routes `Config::set("app-notifications", ...)` through the
existing packed-field helper.

The parser/formatter surface matches the local packed-flag implementation used
for other upstream packed structs:

- standalone booleans set both flags;
- comma-separated `[no-]flag` values override named flags from the defaults;
- omitted flags keep their defaults;
- raw empty values reset to defaults;
- missing values diagnose as `ValueRequired`;
- unknown flags diagnose as `InvalidValue`;
- formatter output is canonical and includes both flags in upstream field order.

Added coverage in the default audit, formatter-order test, aggregate packed/bool
setter-route test, and a focused `app_notifications` test for defaults,
canonical formatting, both individual named flags, bool-all parsing, empty
reset, missing/invalid diagnostics, and clone/equality.

Verification passed:

- `cargo fmt`
- `cargo test -p roastty app_notifications`
  - 1 targeted test passed
- `cargo test -p roastty config_format_config`
  - 1 targeted test passed
- `cargo test -p roastty`
  - first full run: 4532 unit tests passed and
    `tests::surface_key_respects_clear_on_typing_and_escape_exception` failed;
    rerunning that exact test passed, so the failure was treated as a flaky
    unrelated surface-key test
  - second full run: 4533 unit tests passed
  - after completion-review fix: 4533 unit tests passed
  - ABI harness passed with the existing 10 enum-conversion warnings
  - doc tests passed
- `cargo fmt --check`
- `git diff --check`

No long-lived app or background process was spawned for this experiment.

## Conclusion

`app-notifications` now has the upstream-compatible parser/formatter config
surface. Runtime notification behavior remains later work: GTK toast delivery,
clipboard-copy notification UI, config-reload notification UI, and app C ABI
exposure are not implemented or claimed by this experiment.

## Completion Review

Codex adversarial reviewer `019eb521-772b-7431-8928-9e75c959ba56` initially
returned **Changes Required**. The required finding was real: the focused test
proved `clipboard-copy` named-flag parsing, but did not prove the
`config-reload` named-flag parser branch. The implementation already had the
branch, but the test would not have caught a typo or omission there.

The focused test was fixed to parse `clipboard-copy,no-config-reload` and assert
`clipboard_copy = true`, `config_reload = false`, plus canonical formatter
output.

After the fix, verification passed again:

- `cargo fmt`
- `cargo test -p roastty app_notifications`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
  - 4533 unit tests passed
  - ABI harness passed with the existing 10 enum-conversion warnings
  - doc tests passed
- `cargo fmt --check`
- `git diff --check`
