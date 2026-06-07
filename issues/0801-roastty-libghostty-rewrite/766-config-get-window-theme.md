+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 766: Config Get Window Theme

## Description

Port the simple upstream `window-theme` enum into Roastty's aggregate config and
make `roastty_config_get("window-theme")` return parsed config state.

Upstream `Config.WindowTheme` has five keyword variants: `auto`, `system`,
`light`, `dark`, and `ghostty`, with `auto` as the default. Roastty's
`roastty_config_get("window-theme")` currently always returns the hard-coded
default `auto`, and the aggregate `config::Config` has no `window_theme` field.

This experiment is the next narrow step on the app-facing `config_get` boundary
after Experiment 765. It does not implement theme loading/finalization behavior
where light/dark theme pairs can force `window-theme = system`; it only ports
the direct config field and C ABI lookup for user-provided `window-theme`
values.

## Changes

- `roastty/src/config/mod.rs`
  - Add a `WindowTheme` enum with variants `Auto`, `System`, `Light`, `Dark`,
    and `Ghostty`.
  - Add `keyword`, `from_keyword`, and `format_entry` helpers consistent with
    the existing enum config types.
  - Add `window_theme: WindowTheme` to `config::Config` with default
    `WindowTheme::Auto`.
  - Include `window-theme` in `format_config` output.
  - Route `Config::set("window-theme", ...)` through the enum keyword helper.
  - Add aggregate tests for defaults, formatting, set routing, invalid values,
    and file/CLI parsing.
- `roastty/src/lib.rs`
  - Make `roastty_config_get("window-theme")` read the parsed config field
    instead of returning the static `auto` string.
  - Store stable C string pointers for each `WindowTheme` keyword.
  - Add C ABI tests proving `roastty_config_get` returns default, file-loaded,
    CLI-loaded, and cloned values for all variants.

## Verification

- `cargo test -p roastty window_theme -- --nocapture --test-threads=1`
- `cargo test -p roastty config_get_window_theme -- --nocapture --test-threads=1`
- `cargo test -p roastty config_ -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

The experiment passes if `window-theme` is stored in aggregate config, can be
set through file and CLI loading, formats consistently with other enum config
fields, rejects unknown values as `InvalidValue`, and is returned by
`roastty_config_get` through stable C string pointers.

## Design Review

Codex reviewed the design and approved it with no blocking findings. The review
confirmed that `window-theme` fits the same pattern as other simple keyword enum
fields and that deferring theme-derived finalization behavior is acceptable for
this direct config-state slice.

Non-blocking suggestions from the review: include an invalid CLI/file diagnostic
case for `window-theme = nope`, test all five ABI string variants from at least
one load path plus clone/default, and add a bare `--window-theme` CLI test
expecting `ValueRequired` because this is a plain enum field rather than a
custom parser.

## Result

**Result:** Pass

Implemented `WindowTheme` as a simple keyword enum with all five upstream
variants: `auto`, `system`, `light`, `dark`, and `ghostty`. `config::Config` now
stores `window_theme`, defaults it to `WindowTheme::Auto`, formats it through
`format_config`, and routes `Config::set("window-theme", ...)` through the
existing enum-field helper.

`roastty_config_get("window-theme")` now reads parsed config state and returns
stable nul-terminated C string pointers for every variant.

Verification passed:

- `cargo test -p roastty window_theme -- --nocapture --test-threads=1`
- `cargo test -p roastty config_get_window_theme -- --nocapture --test-threads=1`
- `cargo test -p roastty config_ -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Completion Review

Codex reviewed the completed implementation and found no blocking findings. The
first review confirmed the approved design and suggested optional empty-reset
coverage for `--window-theme=`.

The empty-reset test was added, and Codex re-reviewed the final diff with no
blocking findings or non-blocking suggestions. The final review confirmed that
`window-theme` is stored, defaults to `auto`, routes through `set_enum_field`,
formats correctly, rejects bare/invalid CLI input, supports empty reset back to
default, and returns stable parsed C strings across default/file/CLI/clone
paths.

## Conclusion

`roastty_config_get("window-theme")` now reports direct parsed config state
instead of a hard-coded default. Theme-derived finalization behavior remains a
separate future slice; this experiment completes the app-facing direct config
lookup for the `window-theme` field itself.
