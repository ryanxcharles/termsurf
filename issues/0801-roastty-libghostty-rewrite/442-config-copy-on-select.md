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

# Experiment 442: the copy-on-select config enum and its enabled predicate (CopyOnSelect, enabled)

## Description

This experiment ports the `copy-on-select` config enum —
`CopyOnSelect { False, True, Clipboard }` — **and the predicate** the surface
uses to decide whether copy-on-select is active at all. Upstream's `Surface`
short-circuits on `copy_on_select == .false` (early return / `!= .false` guard)
before doing any clipboard work; this experiment captures that as a
`CopyOnSelect::enabled` method. It diversifies the config-type family into the
clipboard subsystem (upstream `Surface.zig`); the clipboard-_target_ selection
(which needs the apprt `Clipboard` type) and the surface call sites stay
deferred.

## Upstream behavior

In `config/Config.zig`, the enum and its `Config` field (an OS-dependent
default):

```zig
@"copy-on-select": CopyOnSelect = switch (builtin.os.tag) {
    .linux => .true,
    .macos => .true,
    else => .false,
},

pub const CopyOnSelect = enum {
    /// Disables copy on select entirely.
    false,
    /// Copy on select is enabled, but goes to the selection clipboard. ... This
    /// is the default.
    true,
    /// Copy on select is enabled and goes to both the system clipboard
    /// and the selection clipboard (for Linux).
    clipboard,
};
```

In `Surface.zig`, the surface gates all copy-on-select work on the enabled
state:

```zig
// If copy on select is false then exit early.
if (self.config.copy_on_select == .false) return;
// ...
switch (self.config.copy_on_select) {
    .false => unreachable, // handled above with an early exit
    .clipboard => /* both standard + selection clipboards */,
    .true => /* selection clipboard */,
}
```

and elsewhere `if (self.config.copy_on_select != .false) { ... }`. A `false`
disables copy-on-select entirely; `true` and `clipboard` both enable it
(differing only in _which_ clipboards receive the selection).

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
/// The `copy-on-select` config (upstream `CopyOnSelect`): whether selecting text
/// copies it, and to which clipboards. The `Config` default is OS-dependent
/// (`True` on macOS / Linux, `False` elsewhere).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CopyOnSelect {
    /// Copy-on-select disabled.
    False,
    /// Enabled; the selection goes to the selection clipboard.
    True,
    /// Enabled; the selection goes to both the system and selection clipboards.
    Clipboard,
}

impl CopyOnSelect {
    /// Whether copy-on-select is active at all (upstream's `copy_on_select !=
    /// .false` guard): `False` is off; `True` and `Clipboard` are on.
    pub(crate) fn enabled(self) -> bool {
        !matches!(self, CopyOnSelect::False)
    }
}
```

`enabled` is the `!= .false` guard: `false` for `False`, `true` for `True` and
`Clipboard` — exactly the upstream short-circuit. The `match` is exhaustive (no
wildcard).

## Scope / faithfulness notes

- **Ported (bridged)**: the `CopyOnSelect` config enum (`config/Config.zig`) and
  its enabled predicate (`CopyOnSelect::enabled`, upstream's `Surface`
  `copy_on_select != .false` guard).
- **Faithful**: the enum has the three upstream variants (`false`, `true`,
  `clipboard`); `enabled` returns `false` only for `False`, `true` for `True`
  and `Clipboard` — exactly the upstream disable check.
- **Faithful adaptation**: the OS-dependent `Config` field default (`.true` on
  macOS / Linux, `.false` elsewhere) is documented on the enum but kept off it
  (the other config types keep defaults on the deferred `Config` struct). The
  consumer is modeled as a method (upstream inlines the `!= .false` /
  `== .false` checks in `Surface`).
- **Deferred**: the `Config` struct / parsing (and the OS-switch field default),
  the clipboard-_target_ selection (the `.clipboard → {standard, selection}` /
  `.true → {selection}` switch, which needs the apprt `Clipboard` type), and the
  surface call sites. (Consumed by a later slice; this experiment lands the enum
  and the enabled predicate.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add `pub(crate) enum CopyOnSelect { False, True, Clipboard }` (derive
     `Debug, Clone, Copy, PartialEq, Eq`) and
     `CopyOnSelect::enabled(self) -> bool`
     (`!matches!(self, CopyOnSelect::False)`).
   - broaden the module-level doc: it currently says the config layer holds "the
     leaf config types consumed by the renderer / terminal bridge" —
     `CopyOnSelect` is consumed by the clipboard subsystem (and Experiment 441's
     `MouseShiftCapture` by input), so reword to the neutral "the leaf config
     types consumed by roastty subsystems".
2. Tests (in `config/mod.rs`):
   - `enabled`: `False.enabled() == false`, `True.enabled() == true`,
     `Clipboard.enabled() == true`; the variants distinct and a `Copy`/`Eq`
     round-trip.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty copy_on_select
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `CopyOnSelect` has the three upstream variants and `enabled` returns `false`
  only for `False` (`true` for `True` / `Clipboard`) via an exhaustive `match` —
  faithful to upstream's enum and the `!= .false` guard;
- the tests pass (the predicate; the distinct variants), and the existing tests
  still pass;
- the `Config` struct, the clipboard-target selection, and the surface call
  sites stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a variant is missing/extra, `enabled` treats `True`
or `Clipboard` as disabled (or `False` as enabled), an unrelated item changes,
or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **one
Low finding** (now folded into the Changes), no Required or Recommended
findings. It verified against the vendored upstream: the variants match exactly
(`false`, `true`, `clipboard`, `Config.zig:8619`); the OS-dependent default is
correctly deferred to the future `Config` struct (`Config.zig:2416`);
`enabled()` is the right extraction of the `== .false` / `!= .false` guards used
by `Surface` (`Surface.zig:2337` / `:3840`); and deferring the target selection
is correct — upstream's `.clipboard` vs `.true` clipboard routing is distinct
behavior that needs the clipboard abstraction (`Surface.zig:2346` / `:4010`). It
judged the tests adequate for the slice.

- **Low (fixed)**: the module-level doc says the config layer holds "the leaf
  config types consumed by the renderer / terminal bridge", but `CopyOnSelect`
  (clipboard) and Experiment 441's `MouseShiftCapture` (input) are consumed
  beyond that. Folded into the Changes: the module doc is reworded to the
  neutral "the leaf config types consumed by roastty subsystems".

Review artifacts:

- Prompt: `logs/codex-review/20260604-104218-d442-prompt.md` (design)
- Result: `logs/codex-review/20260604-104218-d442-last-message.md` (design)

## Result

**Result:** Pass

The copy-on-select config enum and its enabled predicate are now live.

- `roastty/src/config/mod.rs`:
  `pub(crate) enum CopyOnSelect { False, True, Clipboard }` (upstream
  `CopyOnSelect`) and `CopyOnSelect::enabled(self) -> bool`
  (`!matches!(self, CopyOnSelect::False)`), the extraction of upstream's
  `Surface` `copy_on_select != .false` guard. The module-level doc was broadened
  to "the leaf config types consumed by roastty subsystems (renderer, font,
  terminal, input, clipboard)".

Test (in `config/mod.rs`): `copy_on_select_enabled_unless_false` —
`False.enabled() == false`, `True.enabled() == true`,
`Clipboard.enabled() == true`; the variants distinct, `Copy`/`Eq`.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2929 passed, 0 failed (+1, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer + config +
  `lib.rs`/header/`abi_harness.c`) clean; `git diff --check` clean.

## Conclusion

The config layer now carries `CopyOnSelect` and its enabled predicate — the
sixth config slice in a row to land its consumer logic alongside the type, and
the first to reach the clipboard subsystem. With renderer, font, terminal-mode,
input, and clipboard consumers now represented, the module doc was broadened to
a neutral "consumed by roastty subsystems". The `Config` struct / parsing, the
clipboard-_target_ selection (the `.clipboard` vs `.true` routing, which needs
the apprt `Clipboard` type), and the surface call sites stay deferred. The
config-type family remains a clean, gated way to advance the rewrite while the
larger coupled subsystems stay deferred.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed `CopyOnSelect { False, True, Clipboard }`
faithfully maps upstream `false`/`true`/`clipboard`; `enabled()` correctly
captures the `!= .false` guard (only `False` disables; `True` and `Clipboard`
enabled); deferring the OS-dependent `Config` default and the clipboard-target
selection is the right scope; and the module-doc Low is resolved by broadening
the consumer list. It judged the test adequate for the slice. No public C
ABI/header impact; nothing needed to change before the result commit.

Review artifacts:

- Prompt: `logs/codex-review/20260604-104434-r442-prompt.md` (result)
- Result: `logs/codex-review/20260604-104434-r442-last-message.md` (result)
