# Experiment 105: Phase F — quit-delay finalize warning

## Description

Port the next small upstream `Config.finalize()` behavior after Experiment 104:
warning when `quit-after-last-window-closed-delay` is set below five seconds.

Upstream does not reject, clamp, or otherwise mutate the configured delay. It
only logs a warning because very short delays can make Ghostty quit before the
first window is shown:

```zig
if (self.@"quit-after-last-window-closed-delay") |duration| {
    if (duration.duration < 5 * std.time.ns_per_s) {
        log.warn(
            "quit-after-last-window-closed-delay is set to a very short value ({f}), which might cause problems",
            .{duration},
        );
    }
}
```

Roastty currently has the `quit-after-last-window-closed-delay` parser/formatter
surface from Experiment 78, but no config-module logging facade. This experiment
should add a core finalize warning record to `ConfigFinalizeReport` so the
behavior is deterministic and testable without turning an upstream warning into
a config parse diagnostic or runtime shutdown behavior.

This is a config-finalize slice only. It must not implement delayed app
shutdown, CLI `-e` side effects, link matcher mutation, key-remap finalization,
app ABI diagnostic/log plumbing, or a general logging framework.

## Changes

- `roastty/src/config/mod.rs`
  - Add a small warning representation for config finalization, either as a
    typed enum or a private/public-to-crate struct in `ConfigFinalizeReport`.
  - Extend `ConfigFinalizeReport` to carry zero or more warnings while
    preserving the existing theme report behavior.
  - During scalar finalization, if
    `quit_after_last_window_closed_delay == Some(duration)` and
    `duration.duration < 5 * NS_PER_S`, append the warning to the finalize
    report.
  - Do not mutate `quit_after_last_window_closed_delay`.
  - Do not warn for `None` or for durations greater than or equal to five
    seconds.
  - Keep the warning ordering faithful to upstream by recording it after
    link-url/default-link handling would occur and before the auto-update /
    faint-opacity / key-remap tail. Because Roastty does not yet own upstream's
    repeatable link matcher list, this experiment should not implement or fake
    the link-url mutation.
  - Add focused tests proving:
    - `None` emits no warning;
    - a delay just below five seconds emits exactly the quit-delay warning and
      preserves the configured duration;
    - exactly five seconds emits no warning;
    - a longer duration emits no warning;
    - existing scalar finalization still runs and any existing theme report
      shape remains intact.

## Verification

Pass criteria:

1. `cargo test -p roastty config_quit_delay_finalize_warning`
2. `cargo test -p roastty quit_after_last_window_closed_delay_config`
3. `cargo test -p roastty config_finalize_scalar_tail`
4. `cargo test -p roastty`
5. `cargo fmt --check`
6. `git diff --check`
7. `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/105-quit-delay-finalize-warning.md issues/0802-libroastty-completion-and-mac-app/README.md`

The full `cargo test -p roastty` run must pass. The existing ABI harness may
print its known enum-conversion warnings, but no new failures are acceptable.

## Design Review

Codex-native adversarial review ran in fresh context with subagent
`019eb64f-0965-72c1-b58c-d3cb6dcffeec`.

Initial verdict: **CHANGES REQUIRED**

Required finding:

- Verification omitted an explicit markdown formatting check for the edited
  issue docs.

Fix:

- Added a `prettier --check --prose-wrap always --print-width 80` verification
  step for this experiment file and the issue README.

Re-review verdict: **APPROVED**

Remaining findings: None.
