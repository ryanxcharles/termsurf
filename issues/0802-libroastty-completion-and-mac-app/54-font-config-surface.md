+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 54: Phase F — font config surface

## Description

Phase E finished the Unicode property and terminal print-width path. Phase F now
needs config completeness, but the remaining config surface is too large for one
experiment. The next dependency-first slice is the font config surface that
feeds Ghostty's `font/SharedGridSet.zig` `DerivedConfig`: font-family fields,
font size, synthetic-style policy, and `font-codepoint-map`.

This experiment adds those config fields to Roastty's `Config` parser/formatter
and ports the repeatable font codepoint map shape. It does **not** build the
full SharedGridSet key or live font-collection assembly yet; that should be the
next Phase F experiment after these fields are represented and tested.

## Changes

- `roastty/src/config/mod.rs`
  - Add config fields, defaults, parser routing, formatting, and tests for:
    - `font-family`
    - `font-family-bold`
    - `font-family-italic`
    - `font-family-bold-italic`
    - `font-size`
    - `font-synthetic-style`
    - `font-codepoint-map`
  - Port Ghostty's repeatable string behavior for font-family fields: repeated
    values accumulate, an empty value clears, and CLI font-family values reset
    prior font-family values before applying the first CLI value just as
    upstream's `parseManuallyHook` does for the font-family group.
  - Port `Config.RepeatableCodepointMap.parseCLI` / `formatEntry` using
    `config::unicode_range::UnicodeRangeParser` and
    `font::codepoint_map::CodepointMap`.
  - Add `Config::finalize` behavior for the font-family inheritance rule: when
    `font-family` is non-empty and any styled family list is empty, clone
    `font-family` into `font-family-bold`, `font-family-italic`, and/or
    `font-family-bold-italic`.
  - Keep existing `font-style*` fields as-is; do not wire variation axes, metric
    modifiers, or freetype load flags in this experiment.
- `roastty/src/font/codepoint_map.rs`
  - Expose the small read-only helpers needed for config tests and derived
    config construction, such as length/iteration or equality if the existing
    private storage prevents robust assertions.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Mark Phase E roadmap items complete now that Exp 51-53 have passed.
  - Add this experiment to the index as `Designed`.
  - After implementation, record the durable font-config facts and the next
    SharedGridSet-derived-key step.

## Verification

- Run formatting:
  - `cargo fmt -- roastty/src/config/mod.rs roastty/src/font/codepoint_map.rs`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/54-font-config-surface.md`
- Run targeted tests:
  - `cargo test -p roastty config_font`
  - `cargo test -p roastty config_codepoint_map`
  - `cargo test -p roastty codepoint_map`
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run `git diff --check`.
- Run `git status --short` and verify only intended source/docs are present.

**Pass** = Roastty can parse, format, clone/compare, and finalize the initial
font config surface listed above; `font-codepoint-map` produces a usable
`font::CodepointMap`; targeted and full tests pass; and the next experiment can
build `SharedGridSet`'s derived font key from represented config instead of
hardcoded Menlo defaults.

**Partial** = the font-family/font-size surface lands but `font-codepoint-map`
or finalize inheritance exposes a bounded issue that needs a follow-up; record
the exact missing behavior.

**Fail** = the current config/font abstractions cannot represent these upstream
fields without a larger config architecture change first.

## Design Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, read-only). **Verdict: APPROVED.**

The reviewer found no Required findings. It verified that the README links
Experiment 54 as `Designed`, the experiment has Description / Changes /
Verification plus pass/partial/fail criteria, no source implementation happened
before review, and the scope is a narrow Phase F font-config slice. It also
confirmed the plan is consistent with upstream for CLI overwrite behavior for
font-family, finalize inheritance, macOS `font-size = 13`, packed
`font-synthetic-style`, and repeatable `font-codepoint-map` feeding
`CodepointMap`.

## Result

**Result:** Pass

Roastty now represents and round-trips the initial Phase F font config surface:

- `roastty/src/config/mod.rs`
  - Added `font-family`, `font-family-bold`, `font-family-italic`,
    `font-family-bold-italic`, `font-size`, `font-synthetic-style`, and
    `font-codepoint-map` to `Config`.
  - Wired defaults, parser routing, formatter output, and upstream-order config
    formatting for the new fields.
  - Reused the existing `RepeatableString` implementation for font-family
    fields, including upstream's `overwrite_next` CLI behavior.
  - Added `Config::finalize` inheritance so styled family lists clone the
    regular `font-family` list when they are otherwise unset.
  - Added `RepeatableCodepointMap` using the existing Unicode range parser and
    `font::CodepointMap`.
  - Added tests for font-family accumulation/clearing/CLI overwrite, finalize
    inheritance, synthetic-style flags, font-size parsing, codepoint-map
    parsing/formatting, defaults, and config output order.
- `roastty/src/font/codepoint_map.rs`
  - Added `Debug`, `Clone`, and `PartialEq` derives plus `len`, `is_empty`, and
    insertion-order iteration helpers for config assertions and future derived
    font-key construction.

Verification passed:

- `cargo fmt -- roastty/src/config/mod.rs roastty/src/font/codepoint_map.rs`
- `cargo test -p roastty config_font`
- `cargo test -p roastty config_codepoint_map`
- `cargo test -p roastty codepoint_map`
- `cargo test -p roastty`
  - Unit tests: 4451 passed
  - ABI harness: 1 passed
  - Doctests: 0 tests
- `git diff --check`
- `git status --short`

The ABI harness still emits pre-existing C enum-conversion warnings during
compilation; the harness linked and passed.

## Conclusion

The initial font config surface is durable enough for the next Phase F step. The
next experiment should build `font::SharedGridSet`'s derived key from these
represented config fields instead of hardcoded Menlo-style defaults, while
leaving the remaining font variation/metrics/freetype options for later focused
slices.

## Completion Review

**Reviewer:** Codex-native adversarial subagent Planck
(`multi_agent_v1.spawn_agent`, fresh context, read-only). **Verdict: APPROVED.**

The reviewer returned no findings. It was instructed to inspect the completed
experiment file, README status, working-tree diff, changed source files, and
upstream Ghostty sources; to check scope, upstream fidelity, verification
quality, and the no-result-commit-before-review gate; and to independently
verify claimed commands where feasible.
