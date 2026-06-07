+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 771: Config Get Notify Finish After

## Description

Wire the `notify-on-command-finish-after` duration config key into Roastty's
aggregate config and the public `roastty_config_get` C ABI.

Upstream stores `notify-on-command-finish-after` as `Config.Duration` with a
default of five seconds. Its C ABI conversion returns milliseconds via
`Duration.cval()` / `asMilliseconds()`. Roastty currently returns the default
`5000` milliseconds directly from `roastty_config_get`, and `config::Config` has
no aggregate field for this key.

This experiment only ports config parsing, formatting, storage, millisecond ABI
conversion, and lookup. It does not wire runtime notification timers or command
completion notification behavior.

## Changes

- `roastty/src/config/mod.rs`
  - Add `notify_on_command_finish_after: Duration` to `config::Config`.
  - Default it to five seconds, matching upstream `5 * std.time.ns_per_s`.
  - Include `notify-on-command-finish-after` in `format_config` before
    `notify-on-command-finish`, preserving the currently implemented upstream
    order among available fields.
  - Update the full key-order test to assert `background-opacity`,
    `bell-audio-volume`, `notify-on-command-finish-after`, then
    `notify-on-command-finish`.
  - Route `Config::set("notify-on-command-finish-after", ...)` through the
    existing `Duration::parse_cli` path using `set_value_field`.
  - Add a `Duration` millisecond conversion matching upstream
    `asMilliseconds()`: truncate sub-millisecond values and saturate at
    `c_uint::MAX` / `u32::MAX`, then write that value through the existing
    `usize` ABI slot.
  - Add aggregate tests for defaults, formatting, set routing, empty reset,
    missing values, invalid values, repeated-unit parsing such as `1s 250ms`,
    sub-millisecond formatting, millisecond truncation, `u32::MAX as usize`
    saturation, and clone/partial-eq behavior.
- `roastty/src/lib.rs`
  - Make `roastty_config_get("notify-on-command-finish-after")` read
    `config.parsed.notify_on_command_finish_after` and write milliseconds as
    `usize`.
  - Add C ABI tests proving the key reflects default, file-loaded, CLI-loaded,
    cloned, reset-to-default, truncation, saturation, and diagnostic values.
  - Add C ABI diagnostic tests proving bare CLI values report `ValueRequired`,
    invalid CLI values report `InvalidValue`, and the getter remains at the
    `5000` millisecond default after those failed parses.

## Verification

- `cargo test -p roastty notify_on_command_finish_after -- --nocapture --test-threads=1`
- `cargo test -p roastty config_get_notify_finish_after -- --nocapture --test-threads=1`
- `cargo test -p roastty config_ -- --nocapture --test-threads=1`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

The experiment passes if `notify-on-command-finish-after` is stored in aggregate
config, can be set through file and CLI loading, formats in full config output
in upstream order among implemented fields, uses upstream duration
parse/reset/error semantics, converts to ABI milliseconds with truncation and
saturation, and is returned by `roastty_config_get` from parsed state.

Concrete expectations: the default formats as `5s` and reads as `5000`
milliseconds; `1s 250ms` sums to `1250` milliseconds; a sub-millisecond value
such as `999us` formats below `1ms` but reads as `0`; an empty
`--notify-on-command-finish-after=` resets to `5s` / `5000`; and saturated
values read as `u32::MAX as usize`.

## Design Review

Codex reviewed the design and found no blocking findings. The review confirmed
that the experiment is a narrow aggregate `Duration` storage and parsed
`config_get` slice, and that runtime notification timers should stay out of
scope.

The plan was amended to pin the formatter key-order expectation, public C ABI
diagnostics for bare and invalid CLI values, and the exact ABI conversion:
truncate nanoseconds to milliseconds, saturate at `u32::MAX`, then write through
the existing `usize` ABI slot.
