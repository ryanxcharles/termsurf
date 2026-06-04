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

# Experiment 454: the confirm-close-surface config enum and its confirm decision (ConfirmCloseSurface, needs_confirm)

## Description

This experiment ports the `confirm-close-surface` config enum —
`ConfirmCloseSurface { False, True, Always }` — **and the config's contribution
to the close-confirmation decision**. Upstream's `Surface.needsConfirmQuit`
switches on the config: `always` always confirms, `false` never confirms, and
`true` confirms only when the terminal is **not** at a shell prompt (i.e. a
command is running). This experiment captures that as a
`ConfirmCloseSurface::needs_confirm(at_prompt)` method (parallel to Experiment
439's `should_animate` and 449's `should_notify`). The surrounding surface state
(read-only / child-exited early returns, the live `cursorIsAtPrompt`
computation) stays deferred.

## Upstream behavior

In `config/Config.zig`, the enum and its `Config` field (default `.true`):

```zig
@"confirm-close-surface": ConfirmCloseSurface = .true,

pub const ConfirmCloseSurface = enum(c_int) {
    false,
    true,
    always,
};
```

In `Surface.zig`, `needsConfirmQuit` switches on the config (after the read-only
and child-exited early returns):

```zig
return switch (self.config.confirm_close_surface) {
    .always => true,
    .false => false,
    .true => !self.io.terminal.cursorIsAtPrompt(),
};
```

`always` always confirms; `false` never confirms; `true` confirms only when the
cursor is **not** at a shell prompt (a command appears to be running, so closing
would interrupt it).

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
/// The `confirm-close-surface` config (upstream `ConfirmCloseSurface`): whether
/// closing a surface asks for confirmation. The `Config` default is `True`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConfirmCloseSurface {
    /// Never confirm.
    False,
    /// Confirm only when a command appears to be running.
    True,
    /// Always confirm.
    Always,
}

impl ConfirmCloseSurface {
    /// Whether closing needs confirmation, given whether the terminal is at a
    /// shell prompt (the config's part of upstream `Surface.needsConfirmQuit`):
    /// `Always` always confirms, `False` never confirms, `True` confirms only when
    /// **not** `at_prompt`.
    pub(crate) fn needs_confirm(self, at_prompt: bool) -> bool {
        match self {
            ConfirmCloseSurface::Always => true,
            ConfirmCloseSurface::False => false,
            ConfirmCloseSurface::True => !at_prompt,
        }
    }
}
```

`needs_confirm` returns the config's confirm decision: `Always → true`,
`False → false`, `True → !at_prompt` — exactly the upstream `switch`. The
`match` is exhaustive (no wildcard).

## Scope / faithfulness notes

- **Ported (bridged)**: the `ConfirmCloseSurface` config enum
  (`config/Config.zig`) and its confirm decision
  (`ConfirmCloseSurface::needs_confirm`, the config's part of upstream's
  `Surface.needsConfirmQuit` switch).
- **Faithful**: the enum has the three upstream variants (`false`, `true`,
  `always`); `needs_confirm` returns `true` for `Always`, `false` for `False`,
  and `!at_prompt` for `True` — exactly the upstream switch.
- **Faithful adaptation**: upstream declares the enum `enum(c_int)` for
  `ghostty.h` extern compatibility; in roastty this config is internal
  (`pub(crate)`, not yet crossing roastty's C ABI), so a plain Rust enum is the
  faithful internal mapping (a `#[repr(C)]` would be added if/when roastty
  exposes it across its C boundary). The `cursorIsAtPrompt()` result is the
  `at_prompt` parameter (upstream reads it from the live terminal under a lock);
  the method returns the positive "needs confirm" decision.
- **Deferred**: the `Config` struct / parsing (and the `.true` field default),
  the surrounding `needsConfirmQuit` logic (the read-only and child-exited early
  returns, the renderer-state lock), and the live `cursorIsAtPrompt`
  computation. (Consumed by a later slice; this experiment lands the enum and
  the config decision.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add `pub(crate) enum ConfirmCloseSurface { False, True, Always }` (derive
     `Debug, Clone, Copy, PartialEq, Eq`) and
     `ConfirmCloseSurface::needs_confirm(self, at_prompt: bool) -> bool`
     (exhaustive `match`).
2. Tests (in `config/mod.rs`):
   - `needs_confirm`: the full truth table over the three variants ×
     `at_prompt ∈ {true, false}` — `Always` → `true`/`true`, `False` →
     `false`/`false`, `True` → `false` (at prompt) / `true` (not at prompt);
     plus the variants distinct and a `Copy`/`Eq` round-trip.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty confirm_close_surface
cargo test -p roastty needs_confirm
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `ConfirmCloseSurface` has the three upstream variants and `needs_confirm`
  returns `true` for `Always`, `false` for `False`, `!at_prompt` for `True` via
  an exhaustive `match` — faithful to upstream's enum and the `needsConfirmQuit`
  switch;
- the tests pass (the full truth table; the distinct variants), and the existing
  tests still pass;
- the `Config` struct, the surrounding `needsConfirmQuit` logic, and the live
  `cursorIsAtPrompt` stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a variant is missing/extra, `needs_confirm` maps a
case the wrong way (e.g. `True` confirming when at the prompt), a wildcard
`match` arm hides a future variant, an unrelated item changes, or any public C
API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It verified against the vendored upstream: the variants match
exactly (`false`, `true`, `always`, `Config.zig:5235`); the default `.true` is
correctly documented as deferred to the future `Config` field
(`Config.zig:2499`); `needs_confirm()` is an exact extraction of the config
switch (`Surface.zig:947`, `Always → true`, `False → false`,
`True → !at_prompt`); parameterizing on `at_prompt` is the right boundary (the
terminal lock/read and the read-only / child-exited early returns belong in the
eventual `needsConfirmQuit` call-site port, `Surface.zig:939`); a plain internal
enum is appropriate (`repr(C)` can wait until this crosses roastty's C ABI); and
the full 3×2 truth-table test is adequate.

Review artifacts:

- Prompt: `logs/codex-review/20260604-113316-d454-prompt.md` (design)
- Result: `logs/codex-review/20260604-113316-d454-last-message.md` (design)
