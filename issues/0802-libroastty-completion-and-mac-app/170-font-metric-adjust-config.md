# Experiment 170: Phase F — font metric adjust config

## Description

Remove the upstream `adjust-*` font metric fields from the remaining Phase F
public-config tail.

Upstream defines 13 optional `MetricModifier` fields immediately after
`alpha-blending` and before `grapheme-width-method`:

- `adjust-cell-width`
- `adjust-cell-height`
- `adjust-font-baseline`
- `adjust-underline-position`
- `adjust-underline-thickness`
- `adjust-strikethrough-position`
- `adjust-strikethrough-thickness`
- `adjust-overline-position`
- `adjust-overline-thickness`
- `adjust-cursor-thickness`
- `adjust-cursor-height`
- `adjust-box-thickness`
- `adjust-icon-height`

Each value is either an absolute integer delta such as `-2` or a percentage
delta such as `20%`. Upstream stores percentages as multipliers (`20%` becomes
`1.2`, `-15%` becomes `0.85`, and `-100%` or lower clamps to `0.0`) and formats
them back as the delta percentage.

This experiment wires parser/formatter/storage behavior only. Applying the
modifiers to live font metrics remains font runtime work.

## Changes

- `roastty/src/config/mod.rs`
  - Import/reuse `crate::font::metrics::Modifier` as the config value type for
    upstream `MetricModifier`.
  - Add the 13 optional `adjust-*` fields to `Config`, defaulting to `None`.
  - Add config parse/format glue for `Modifier` if the existing font metrics
    type does not already expose config-friendly methods:
    - missing values report `ValueRequired`;
    - empty set values reset the optional field to `None`;
    - integer values parse as absolute `i32` deltas;
    - trailing `%` values parse as percent deltas with upstream clamping at
      `<= -100%`;
    - invalid values report `InvalidValue`;
    - formatting emits absolute integers or delta percentages using shortest
      decimal formatting.
  - Format the 13 fields in upstream declaration order after `alpha-blending`
    and before `grapheme-width-method`.
  - Route `Config::set` for all 13 keys through optional-child `MetricModifier`
    semantics.
  - Update default/order tests and add a focused `font_metric_adjust_config_*`
    parse/format/reset/load/diagnostic/clone test that touches every key.

- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Mark Experiment 170 as `Designed`.
  - After result, update the Phase F remaining-public-options count from 16 to 3
    and change the remaining-tail wording to `freetype-load-flags`, `input`, and
    `keybind` if this passes.

## Verification

Before implementation:

- Codex-native adversarial design review approves this experiment.
- Commit the reviewed plan separately from the result.

After implementation:

- `cargo test -p roastty font_metric_adjust_config`
- `cargo test -p roastty config_format_config_emits_fields_in_upstream_order`
- `cargo test -p roastty`
- `cargo fmt --check -p roastty`
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/170-font-metric-adjust-config.md issues/0802-libroastty-completion-and-mac-app/README.md`
- `git diff --check`

**Pass** = all 13 `adjust-*` keys parse, format, reset, load, clone, and report
diagnostics with upstream optional `MetricModifier` default/order semantics, and
the full roastty suite passes.

**Partial** = the direct parser/formatter fields land, but ordering, reset
behavior, diagnostics, or full-suite verification remains incomplete.

**Fail** = the fields cannot be added without conflicting with existing font
metrics storage, formatter ordering, or optional config semantics.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Wegener`, fresh context.

**Verdict:** Approved with no findings.

The reviewer verified that the README links Experiment 170 as `Designed`, the
experiment has the required sections, the scope is bounded to the upstream
`adjust-*` metric config group, the plan is faithful to upstream
`MetricModifier` and optional `parseIntoField` semantics, and the verification
includes focused tests, the full roastty suite, Rust formatting, markdown
prettier, and `git diff --check`.

## Result

**Result:** Pass

Roastty now stores, parses, and formats all 13 upstream `adjust-*` font metric
modifier config fields:

- `adjust-cell-width`
- `adjust-cell-height`
- `adjust-font-baseline`
- `adjust-underline-position`
- `adjust-underline-thickness`
- `adjust-strikethrough-position`
- `adjust-strikethrough-thickness`
- `adjust-overline-position`
- `adjust-overline-thickness`
- `adjust-cursor-thickness`
- `adjust-cursor-height`
- `adjust-box-thickness`
- `adjust-icon-height`

The fields default to `None`, format as void lines when unset, and appear in
upstream declaration order after `alpha-blending` and before
`grapheme-width-method`. Non-empty values reuse the existing
`font::metrics::Modifier` storage: integer values become absolute `i32` deltas,
trailing-percent values become percent multipliers, and `-100%` or lower clamps
to `0.0`. Missing values report `ValueRequired`; invalid values report
`InvalidValue`; set-but-empty values reset the optional field to `None`.

This is parser/formatter/storage only. Applying the configured modifiers to live
font metrics remains font runtime work.

The Phase F public-config tail is now three keys: `freetype-load-flags`,
`input`, and `keybind`.

Verification:

- `cargo test -p roastty font_metric_adjust_config` passed 1 filtered unit test
  plus the ABI harness filter.
- `cargo test -p roastty config_format_config_emits_fields_in_upstream_order`
  passed 1 filtered unit test plus the ABI harness filter.
- `cargo test -p roastty` passed 4,866 Rust unit tests, 0 failed, 4 ignored; the
  C ABI harness passed with the existing enum-conversion warnings; doc tests
  passed with 0 tests.
- `cargo fmt --check -p roastty` passed.
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/170-font-metric-adjust-config.md issues/0802-libroastty-completion-and-mac-app/README.md`
  passed.
- `git diff --check` passed.

## Conclusion

The `adjust-*` font metric group is no longer a public config field gap. The
next Phase F config slice should wire `freetype-load-flags`; after that, only
`input` and `keybind` remain in the public config tail tracked here.

## Completion Review

**Reviewer:** Codex-native adversarial review subagent `Sagan`, fresh context.

**Verdict:** Approved after one required verification rerun.

The first completion review found that the reviewer’s independent
`cargo test -p roastty` run hit a failure in
`tests::surface_foreground_pid_reports_worker_foreground_pid_after_start`, while
a focused rerun of that test passed. The implementation did not change. I reran
the full `cargo test -p roastty`; the previously failing test passed, the full
unit suite passed with 4,866 tests, the ABI harness passed with the existing
enum-conversion warnings, and doc tests passed with 0 tests. Re-review approved
the result with no remaining findings.
