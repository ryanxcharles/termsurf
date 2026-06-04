+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 449: the notify-on-command-finish config enum and its notify decision (NotifyOnCommandFinish, should_notify)

## Description

This experiment ports the `notify-on-command-finish` config enum —
`NotifyOnCommandFinish { Never, Unfocused, Always }` — **and the per-config
notify decision** the surface applies when a command finishes. Upstream's apprt
short-circuits its notification path on the config and the focused state; this
experiment captures the config's contribution as a
`NotifyOnCommandFinish::should_notify(focused)` method (parallel to Experiment
439's `CustomShaderAnimation::should_animate`). The apprt notification path
itself (the manual-override flag, the duration threshold, the bell / notify
actions) stays deferred.

## Upstream behavior

In `config/Config.zig`, the enum and its `Config` field (default `.never`):

```zig
@"notify-on-command-finish": NotifyOnCommandFinish = .never,

pub const NotifyOnCommandFinish = enum {
    never,
    unfocused,
    always,
};
```

In the apprt surface (`apprt/gtk/class/surface.zig`), the notify path skips
early based on the config and the focused state (`return true` is the skip /
no-notify early return):

```zig
if (!notify_next_command_finish) {
    if (cfg.@"notify-on-command-finish" == .never) return true;
    if (cfg.@"notify-on-command-finish" == .unfocused and self.getFocused()) return true;
}
// ... else falls through to notify (subject to the duration threshold and the action)
```

So, ignoring the manual override and the duration threshold, the config decides
whether to notify on a finished command: `never` never notifies; `unfocused`
notifies only when the window is **not** focused; `always` always notifies.

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
/// The `notify-on-command-finish` config (upstream `NotifyOnCommandFinish`): when
/// to notify on a finished command. The `Config` default is `Never`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NotifyOnCommandFinish {
    /// Never notify.
    Never,
    /// Notify only when the window is unfocused.
    Unfocused,
    /// Always notify.
    Always,
}

impl NotifyOnCommandFinish {
    /// Whether to notify on a finished command, given the window's focused state
    /// (the config's contribution to upstream's apprt notify path): `Never` never
    /// notifies, `Unfocused` notifies only when **not** `focused`, `Always` always
    /// notifies.
    pub(crate) fn should_notify(self, focused: bool) -> bool {
        match self {
            NotifyOnCommandFinish::Never => false,
            NotifyOnCommandFinish::Unfocused => !focused,
            NotifyOnCommandFinish::Always => true,
        }
    }
}
```

`should_notify` returns whether to notify: `Never → false`,
`Unfocused → !focused`, `Always → true` — the inverse of the upstream
`return true` (skip) early returns. The `match` is exhaustive (no wildcard).

## Scope / faithfulness notes

- **Ported (bridged)**: the `NotifyOnCommandFinish` config enum
  (`config/Config.zig`) and its notify decision
  (`NotifyOnCommandFinish::should_notify`, the config's contribution to
  upstream's apprt notify path).
- **Faithful**: the enum has the three upstream variants (`never`, `unfocused`,
  `always`); `should_notify` returns `false` for `Never`, `!focused` for
  `Unfocused`, `true` for `Always` — exactly the config's part of the upstream
  skip logic (`never` skips; `unfocused` skips when focused; `always` never
  skips).
- **Faithful adaptation**: upstream phrases it as an early "skip notification"
  `return true`; the method returns the positive "should notify" decision (its
  inverse), with the focused state as a parameter (upstream reads
  `self.getFocused()`).
- **Deferred**: the `Config` struct / parsing (and the `.never` field default),
  and the apprt notification path itself — the `notify_next_command_finish`
  manual override, the `notify-on-command-finish-after` duration threshold, and
  the `notify-on-command-finish-action` bell / notify actions. (Consumed by a
  later slice; this experiment lands the enum and the per-config decision.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add `pub(crate) enum NotifyOnCommandFinish { Never, Unfocused, Always }`
     (derive `Debug, Clone, Copy, PartialEq, Eq`) and
     `NotifyOnCommandFinish::should_notify(self, focused: bool) -> bool`
     (exhaustive `match`).
2. Tests (in `config/mod.rs`):
   - `should_notify`: the full truth table over the three variants ×
     `focused ∈ {true, false}` — `Never` → `false`/`false`, `Unfocused` →
     `false`/`true`, `Always` → `true`/`true`; plus the variants distinct and a
     `Copy`/`Eq` round-trip.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty notify_on_command_finish
cargo test -p roastty should_notify
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `NotifyOnCommandFinish` has the three upstream variants and `should_notify`
  returns `false` for `Never`, `!focused` for `Unfocused`, `true` for `Always`
  via an exhaustive `match` — faithful to upstream's enum and the apprt skip
  logic;
- the tests pass (the full truth table; the distinct variants), and the existing
  tests still pass;
- the `Config` struct and the apprt notification path stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a variant is missing/extra, `should_notify` maps a
case the wrong way (e.g. `Unfocused` notifying when focused), a wildcard `match`
arm hides a future variant, an unrelated item changes, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It verified against the vendored upstream: the variants match
exactly (`never`, `unfocused`, `always`, `Config.zig:10214`); the default
`.never` is correctly documented as a deferred Config-field default
(`Config.zig:1218`); `should_notify()` correctly inverts the upstream
early-return skip logic (`surface.zig:1156` — `.never` skips, `.unfocused` skips
only when focused, `.always` does not skip on config alone); and deferring the
`notify_next_command_finish` manual override, the duration threshold, and the
action execution is the right boundary (this method is the baseline config
decision, with the manual override handled by the future call site). It judged
the full truth-table test adequate.

Review artifacts:

- Prompt: `logs/codex-review/20260604-111331-d449-prompt.md` (design)
- Result: `logs/codex-review/20260604-111331-d449-last-message.md` (design)
