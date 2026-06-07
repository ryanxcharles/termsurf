+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 765: Config Get Window Decoration

## Description

Wire the already-ported `window-decoration` config type into the aggregate
Roastty config and the public `roastty_config_get` C ABI.

`WindowDecoration::parse_cli` and `WindowDecoration::format_entry` were ported
in earlier experiments, but `config::Config` still does not store a
`window_decoration` field and `roastty_config_get("window-decoration")` always
returns the hard-coded default `auto`. That means default-file, CLI, and
recursive config loading can parse many fields, but this app-facing config
lookup still cannot reflect a user-provided `window-decoration` value.

This experiment keeps the slice narrow. It does not attempt to complete every
`roastty_config_get` key, port `window-theme` / `window-save-state`, or wire
macOS titlebar behavior. It moves one app-facing key from hard-coded default to
real parsed config state.

## Changes

- `roastty/src/config/mod.rs`
  - Add `window_decoration: WindowDecoration` to `config::Config`.
  - Set the upstream default `WindowDecoration::Auto`.
  - Include `window-decoration` in `format_config` output.
  - Route `Config::set("window-decoration", ...)` through
    `WindowDecoration::parse_cli`, preserving the existing boolean and keyword
    parsing behavior.
  - Add aggregate config tests for default, set, format, and file/CLI parsing.
- `roastty/src/lib.rs`
  - Make `roastty_config_get("window-decoration")` read the parsed config field
    instead of returning the static `auto` string.
  - Store stable C string pointers for each `WindowDecoration` keyword.
  - Add C ABI tests proving `roastty_config_get` returns values loaded from a
    file and from CLI args, survives clone/free patterns, and still returns the
    default for a fresh config.

## Verification

- `cargo test -p roastty window_decoration -- --nocapture --test-threads=1`
- `cargo test -p roastty config_get_window_decoration -- --nocapture --test-threads=1`
- `cargo test -p roastty config_ -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

The experiment passes if `window-decoration` is stored in the aggregate config,
can be set through file and CLI config loading, is formatted consistently with
the existing formatter, and is returned by `roastty_config_get` through stable C
string pointers.

## Design Review

Codex reviewed the design and approved it with no blocking findings. The review
confirmed that `window-decoration` is an appropriate narrow slice because its
parser and formatter are already ported and the existing C ABI shape for
`roastty_config_get("window-decoration")` is a stable `*const c_char`.

Non-blocking suggestions from the review: test all returned ABI variants
(`auto`, `client`, `server`, and `none`), include boolean CLI forms (`true` →
`auto`, `false` → `none`), add an invalid-value diagnostic test for the new
aggregate route, and optionally include a recursive/default-file smoke test.

## Result

**Result:** Pass

Implemented aggregate config storage and C ABI lookup for `window-decoration`.
`config::Config` now stores the field, defaults it to `WindowDecoration::Auto`,
formats it through `format_config`, and routes
`Config::set("window-decoration", ...)` through the existing
`WindowDecoration::parse_cli` parser. Invalid values now surface as
`ConfigSetError::InvalidValue` through the aggregate config path.

`roastty_config_get("window-decoration")` now reads the parsed config field and
returns stable nul-terminated C string pointers for `auto`, `client`, `server`,
and `none`.

Verification passed:

- `cargo test -p roastty window_decoration -- --nocapture --test-threads=1`
- `cargo test -p roastty config_get_window_decoration -- --nocapture --test-threads=1`
- `cargo test -p roastty config_ -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Completion Review

Codex reviewed the completed implementation and found no blocking findings. The
first review confirmed that the implementation satisfied the approved slice and
suggested extra non-blocking tests for bare CLI form and explicit non-default
format output.

Those tests were added, and Codex re-reviewed the final diff with no blocking
findings. The final review confirmed coverage for default, all file variants,
CLI bare/boolean/keyword forms, clone/free behavior, invalid CLI diagnostics,
and explicit formatted output.

## Conclusion

`roastty_config_get("window-decoration")` is no longer a hard-coded default. One
more app-facing config key now flows through the same parsed config path used by
file and CLI loading, moving the `config_get` boundary incrementally toward real
config state.
