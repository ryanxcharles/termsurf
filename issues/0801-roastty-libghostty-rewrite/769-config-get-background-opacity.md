+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 769: Config Get Background Opacity

## Description

Wire the `background-opacity` config key into Roastty's aggregate config and the
public `roastty_config_get` C ABI.

Upstream stores `background-opacity` as an `f64` with a default of `1.0`.
Roastty currently returns that value directly from `roastty_config_get`, and
`config::Config` has only the related `background-image-opacity` field. This
experiment ports the scalar config state and lookup behavior so file and CLI
config layers can affect the ABI result.

This experiment only ports config parsing, formatting, storage, and lookup. It
does not wire compositor/window opacity behavior or add the separate
`background-opacity-cells` runtime behavior.

## Changes

- `roastty/src/config/formatter.rs`
  - Generalize `EntryFormatter::entry_float` so it can format both `f32` and
    `f64` values using the same shortest-decimal output style.
- `roastty/src/config/mod.rs`
  - Add `background_opacity: f64` to `config::Config`.
  - Default it to `1.0`, matching upstream.
  - Include `background-opacity` in `format_config` near `background-blur`,
    preserving the currently implemented upstream declaration order instead of
    grouping it with `background-image-opacity`.
  - Add a small scalar-float setter helper that mirrors upstream
    `parseIntoField`: missing values are `ValueRequired`, empty values reset to
    the field default, and invalid floats are `InvalidValue`.
  - Route `Config::set("background-opacity", ...)` through that helper.
  - Add aggregate tests for defaults, formatting, set routing, empty reset,
    missing values, invalid values, finite out-of-range values that should stay
    unclamped, f64 precision, and clone/partial-eq behavior.
- `roastty/src/lib.rs`
  - Make `roastty_config_get("background-opacity")` read
    `config.parsed.background_opacity` instead of writing the hard-coded
    default.
  - Add C ABI tests proving the key reflects default, file-loaded, CLI-loaded,
    cloned, reset-to-default, and f64-precision values.

## Verification

- `cargo test -p roastty background_opacity -- --nocapture --test-threads=1`
- `cargo test -p roastty config_get_background_opacity -- --nocapture --test-threads=1`
- `cargo test -p roastty config_ -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

The experiment passes if `background-opacity` is stored in aggregate config, can
be set through file and CLI loading, formats in the full config output in
upstream order, rejects missing and invalid values consistently with upstream
scalar floats, resets on an empty value, preserves unclamped finite values,
preserves values that would visibly narrow if stored as `f32`, and is returned
by `roastty_config_get` from parsed state.

## Design Review

Codex reviewed the design and found two blocking plan issues. The first was that
the original verification did not require a precision regression proving
`background-opacity` remains `f64` end-to-end. The second was that the original
formatter placement grouped `background-opacity` before
`background-image-opacity`, which contradicted upstream declaration order and
the local `format_config` contract.

The plan was updated to place `background-opacity` near `background-blur`, add
f64 precision coverage for config parsing/formatting and the C ABI getter, and
add explicit coverage that finite out-of-range float values are parsed and
stored rather than clamped. Codex's non-blocking guidance was to keep
`background-opacity-cells` and compositor opacity behavior out of scope.

## Result

**Result:** Pass

Implemented aggregate config storage for `background-opacity`. `config::Config`
now stores `background_opacity: f64`, defaults it to `1.0`, formats it near
`background-blur` in the currently implemented upstream order, and routes
`Config::set("background-opacity", ...)` through a scalar `f64` parser matching
upstream `parseIntoField` semantics for missing, empty, invalid, and finite
out-of-range values.

`EntryFormatter::entry_float` now accepts both `f32` and `f64` values without
narrowing existing `background-image-opacity` formatting. The
`roastty_config_get("background-opacity")` ABI getter now reads parsed config
state instead of writing a hard-coded default.

Verification passed:

- `cargo test -p roastty background_opacity -- --nocapture --test-threads=1`
- `cargo test -p roastty config_get_background_opacity -- --nocapture --test-threads=1`
- `cargo test -p roastty config_ -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Conclusion

`background-opacity` now reports parsed config state through the C ABI while
remaining a pure config slice. Runtime compositor opacity behavior,
`background-opacity-cells`, and the remaining hard-coded `config_get` scalar
keys remain follow-up work.

## Completion Review

Codex reviewed the completed implementation and found no blocking findings. The
review confirmed that `background_opacity` is stored as `f64`, defaults to
`1.0`, parses with upstream scalar-float semantics, formats through the
aggregate config, and is returned by `roastty_config_get("background-opacity")`
from parsed state.

The review also confirmed the tests cover the scoped regression risks: default,
file, CLI, clone, empty reset, missing and invalid diagnostics, unclamped finite
values, and f64 precision. A non-blocking note pointed out a stale `entry_float`
doc comment after generalizing it for `f32` and `f64`; that comment was updated.
