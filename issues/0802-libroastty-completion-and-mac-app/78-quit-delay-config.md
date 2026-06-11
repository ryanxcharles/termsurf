+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 78: Phase F — quit delay config

## Description

Experiment 77 wired `title-report` and `image-storage-limit`. The next unported
upstream config field after the already-ported copy/mouse/action group and
`confirm-close-surface` is:

- `quit-after-last-window-closed-delay`

Upstream declares this as `?Duration = null` in
`vendor/ghostty/src/config/Config.zig`, immediately after
`quit-after-last-window-closed` and before `initial-window`. It controls how
long Ghostty may remain alive after the last surface closes when
`quit-after-last-window-closed` is enabled. Upstream's finalize path logs a
warning for very short durations, but the inspected code does not clamp or
reject them there.

This experiment adds the config parser/formatter surface only. Runtime app
shutdown behavior, CLI `-e` manual-hook side effects, and warning-only finalize
logging are out of scope.

## Changes

- `roastty/src/config/mod.rs`
  - Add `Config::quit_after_last_window_closed_delay: Option<Duration>` with
    upstream default `None`.
  - Route `quit-after-last-window-closed-delay` through `Config::set`, config
    loading diagnostics, clone/equality, and formatting.
  - Parse non-empty values through the existing `Duration::parse_cli` path.
  - Reset empty values to the default `None`.
  - Format `None` as an empty optional entry and `Some(Duration)` with the
    existing duration formatter.
  - Preserve the current local app-lifecycle formatter block by inserting the
    key immediately after `quit-after-last-window-closed`. This experiment does
    not reorder the previously ported `initial-window` field or globally
    reshuffle older config slices.

Out of scope:

- Runtime delayed application shutdown.
- CLI `-e` implied setting behavior.
- Warning/log diagnostics for durations shorter than five seconds.
- `undo-timeout`, the next upstream field after `initial-window`.
- Any broader formatter reordering of already-ported keys.

## Verification

- Run formatting:
  - `cargo fmt`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/78-quit-delay-config.md`
- Run targeted tests:
  - `cargo test -p roastty quit_after_last_window_closed_delay_config`
  - `cargo test -p roastty config_format_config`
- Add concrete test cases proving:
  - the default formats as `quit-after-last-window-closed-delay = `;
  - a valid duration such as `1s 250ms` parses and formats as the normalized
    duration string;
  - `0` remains accepted by the existing duration parser and formats back to an
    empty duration payload when explicitly set;
  - an empty value resets the optional field to `None`;
  - a missing value returns `ValueRequired`;
  - invalid duration values return `InvalidValue`;
  - `Config::load_str` records diagnostics for invalid neighboring delay lines
    while preserving valid parsed values;
  - formatter order includes the delay key immediately after
    `quit-after-last-window-closed`;
  - clone/equality preserves the optional duration.
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run `cargo fmt --check`.
- Run `git diff --check`.
- Run `git status --short` and verify only intended source/docs are present.

**Pass** = the optional quit delay field is represented faithfully on `Config`,
round-trips through config loading/formatting, matches the upstream default and
duration parser behavior, and has targeted and full tests passing.

**Partial** = the parser/formatter field lands, but duration reset, diagnostics,
or formatter-order coverage requires a follow-up.

**Fail** = the key cannot be represented faithfully without first implementing
runtime app shutdown delay behavior.

## Design Review

Codex adversarial reviewer `019eb456-dbc2-7950-be88-3062d1955248` returned
**Approved** with no findings. The reviewer confirmed that the README links
Experiment 78 as `Designed`, the experiment has the required sections, the scope
is narrow enough for the next missing config field, the plan is faithful to
upstream `quit-after-last-window-closed-delay`, the existing local formatter
ordering choice is explicit, and the verification plan includes formatting,
targeted tests, full `cargo test -p roastty`, `cargo fmt --check`,
`git diff --check`, and status inspection.

## Result

**Result:** Pass

Implemented `quit-after-last-window-closed-delay` in `roastty/src/config/mod.rs`
as an optional `Duration` with upstream default `None`. The field now routes
through `Config::set`, config loading diagnostics, clone/equality, and
`format_config`. Non-empty values use the existing `Duration::parse_cli` parser,
empty values reset to `None`, missing values return `ValueRequired`, and invalid
duration strings return `InvalidValue`.

The formatter emits the key in the existing local app-lifecycle block,
immediately after `quit-after-last-window-closed`. `None` and an explicitly set
zero duration both format with an empty payload, matching the shared duration
formatter's `0` representation while preserving the stored
`Some(Duration { duration: 0 })` value internally.

Verification passed:

- `cargo fmt`
- `cargo test -p roastty quit_after_last_window_closed_delay_config`
- `cargo test -p roastty config_format_config`
- `cargo test -p roastty`
  - 4515 unit tests passed
  - ABI harness passed with the existing 10 enum-conversion warnings
  - doc tests passed
- `cargo fmt --check`
- `git diff --check`

## Conclusion

The quit-after-last-window-closed delay config surface now matches upstream's
optional-duration default and parser/formatter behavior for this slice. Runtime
delayed shutdown, CLI `-e` implied settings, and warning-only finalize logging
remain later work. The next upstream config field after `initial-window` is
`undo-timeout`.

## Completion Review

Codex adversarial reviewer `019eb45d-ac47-7782-bb1b-6041050299ef` returned
**Approved** with no findings. The reviewer confirmed that the implementation
matches the approved scope, preserves the documented local formatter ordering
choice, matches upstream default and optional-duration parser/formatter
behavior, records README status and result docs correctly, and has no result
commit after the plan commit yet.

The reviewer independently verified `cargo fmt --check`, `git diff --check`,
`cargo test -p roastty quit_after_last_window_closed_delay_config`,
`cargo test -p roastty config_format_config`, and full `cargo test -p roastty`
with 4515 unit tests passing, the ABI harness passing with the existing 10
enum-conversion warnings, and 0 doc tests.
