+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 767: Config Get Window Save State

## Description

Port the simple upstream `window-save-state` enum into Roastty's aggregate
config and make `roastty_config_get("window-save-state")` return parsed config
state.

Upstream `Config.WindowSaveState` has three keyword variants: `default`,
`never`, and `always`, with `default` as the field default. Roastty currently
returns the hard-coded default `default` from `roastty_config_get`, and
`config::Config` has no `window_save_state` field.

This experiment is another narrow app-facing `config_get` slice. It does not
wire macOS window restoration behavior; it only ports the direct config field
and C ABI lookup for user-provided `window-save-state` values.

## Changes

- `roastty/src/config/mod.rs`
  - Add a `WindowSaveState` enum with variants `Default`, `Never`, and `Always`.
  - Add `keyword`, `from_keyword`, and `format_entry` helpers consistent with
    the existing enum config types.
  - Add `window_save_state: WindowSaveState` to `config::Config` with default
    `WindowSaveState::Default`.
  - Include `window-save-state` in `format_config` output.
  - Route `Config::set("window-save-state", ...)` through the enum keyword
    helper.
  - Add aggregate tests for defaults, formatting, set routing, invalid values,
    and file/CLI parsing.
- `roastty/src/lib.rs`
  - Make `roastty_config_get("window-save-state")` read the parsed config field
    instead of returning the static `default` string.
  - Store stable C string pointers for each `WindowSaveState` keyword.
  - Add C ABI tests proving `roastty_config_get` returns default, file-loaded,
    CLI-loaded, cloned, and reset-to-default values.

## Verification

- `cargo test -p roastty window_save_state -- --nocapture --test-threads=1`
- `cargo test -p roastty config_get_window_save_state -- --nocapture --test-threads=1`
- `cargo test -p roastty config_ -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

The experiment passes if `window-save-state` is stored in aggregate config, can
be set through file and CLI loading, formats consistently with other enum config
fields, rejects missing and unknown values with the expected diagnostics, and is
returned by `roastty_config_get` through stable C string pointers.

## Design Review

Codex reviewed the design and approved it with no blocking findings. The review
confirmed that `window-save-state` is a simple keyword enum with an existing
string-shaped ABI getter, making it an appropriate narrow `config_get` slice.
Deferring macOS window restoration behavior is correct because this experiment
only exposes direct config state.

Non-blocking suggestions from the review: test all three ABI variants
(`default`, `never`, and `always`), include bare CLI `--window-save-state` as
`ValueRequired`, include unknown values as `InvalidValue`, and include empty
reset coverage.

## Result

**Result:** Pass

Implemented `WindowSaveState` as a simple keyword enum with all three upstream
variants: `default`, `never`, and `always`. `config::Config` now stores
`window_save_state`, defaults it to `WindowSaveState::Default`, formats it
through `format_config`, and routes `Config::set("window-save-state", ...)`
through the existing enum-field helper.

`roastty_config_get("window-save-state")` now reads parsed config state and
returns stable nul-terminated C string pointers for every variant.

Verification passed:

- `cargo test -p roastty window_save_state -- --nocapture --test-threads=1`
- `cargo test -p roastty config_get_window_save_state -- --nocapture --test-threads=1`
- `cargo test -p roastty config_ -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Completion Review

Codex reviewed the completed implementation and found no blocking findings or
non-blocking suggestions. The review confirmed that `WindowSaveState` is stored
in aggregate `Config`, defaults to `default`, routes through `set_enum_field`,
formats correctly, produces `ValueRequired` / `InvalidValue` diagnostics for
bare / unknown CLI input, and returns stable parsed C strings through
`roastty_config_get("window-save-state")`.

## Conclusion

`roastty_config_get("window-save-state")` now reports direct parsed config state
instead of a hard-coded default. Window restoration behavior remains a separate
macOS app/runtime slice; this experiment completes the direct config lookup for
the field itself.
