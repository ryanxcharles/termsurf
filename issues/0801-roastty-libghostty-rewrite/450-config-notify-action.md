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

# Experiment 450: the notify-on-command-finish-action config type (NotifyOnCommandFinishAction)

## Description

This experiment ports the `notify-on-command-finish-action` config type —
`NotifyOnCommandFinishAction`, a two-flag struct (`bell`, `notify`) selecting
what happens when a command-finish notification fires. It is the companion to
Experiment 449's `NotifyOnCommandFinish` (which decides _whether_ to notify);
this type decides _what_ the notification does. Its intrinsic field defaults
(`bell = true`, `notify = false`) are meaningful (the `Config` field sets them),
so the `Default` impl is hand-written, like `ScrollToBottom` (Experiment 448).
The apprt consumer reads the two flags directly (`if (action.bell) …`,
`if (action.notify) …`); the action execution (ringing the bell, sending the
notification) stays deferred.

## Upstream behavior

In `config/Config.zig`, the type and its `Config` field:

```zig
@"notify-on-command-finish-action": NotifyOnCommandFinishAction = .{
    .bell = true,
    .notify = false,
},

pub const NotifyOnCommandFinishAction = packed struct {
    bell: bool = true,
    notify: bool = false,
};
```

In the apprt surface (`apprt/gtk/class/surface.zig`), the action's flags are
read to drive the notification:

```zig
const action = cfg.@"notify-on-command-finish-action";
if (action.bell) self.setBellRinging(true);
if (action.notify) notify: { ... send a desktop notification ... }
```

`NotifyOnCommandFinishAction` has two independent flags: `bell` (ring the bell
on a finished command, default `true`) and `notify` (send a desktop
notification, default `false`). The `Config` field default sets `bell = true`,
`notify = false` (matching the struct's own field defaults).

## Rust mapping (`roastty/src/config/mod.rs`)

```rust
/// The `notify-on-command-finish-action` config (upstream
/// `NotifyOnCommandFinishAction`): what a command-finish notification does.
/// `bell` (default `true`) rings the bell; `notify` (default `false`) sends a
/// desktop notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NotifyOnCommandFinishAction {
    /// Ring the bell on a finished command.
    pub bell: bool,
    /// Send a desktop notification on a finished command.
    pub notify: bool,
}

impl Default for NotifyOnCommandFinishAction {
    /// Upstream's field defaults `bell = true`, `notify = false`.
    fn default() -> Self {
        Self {
            bell: true,
            notify: false,
        }
    }
}
```

The hand-written `Default` matches upstream's field defaults (`bell = true`,
`notify = false`); a derived `Default` would make `bell` `false`. The two flags
are independent `bool`s. The apprt reads them directly (no method).

## Scope / faithfulness notes

- **Ported (bridged)**: the `NotifyOnCommandFinishAction` config type
  (`config/Config.zig`), with its intrinsic field defaults.
- **Faithful**: the struct has the two upstream flags (`bell`, `notify`); the
  `Default` is `bell = true`, `notify = false` (upstream's field defaults, which
  the `Config` field also sets explicitly).
- **Faithful adaptation**: upstream is a `packed struct` (bit-packed storage);
  in Rust it is a plain value struct (no ABI involved — internal config), so a
  derived layout is fine. The `Default` is hand-written because Rust's derived
  `Default` for `bool` is `false`, not upstream's `bell = true`. No method is
  extracted — the apprt consumer is plain `.bell` / `.notify` field access.
- **Deferred**: the string parsing, the `formatEntry`, the `Config` struct that
  holds the key, and the apprt action execution (ringing the bell, sending the
  desktop notification) that reads the two flags. (Consumed by a later slice;
  this experiment lands the value type and its defaults.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/config/mod.rs`:
   - add
     `pub(crate) struct NotifyOnCommandFinishAction { pub bell: bool, pub notify: bool }`
     (derive `Debug, Clone, Copy, PartialEq, Eq`) and a hand-written
     `impl Default` (`bell: true`, `notify: false`).
2. Tests (in `config/mod.rs`):
   - `NotifyOnCommandFinishAction::default()` has `bell == true`,
     `notify == false`; a `{ bell: false, notify: true }` value differs from the
     default and round-trips `Copy`/`Eq`; the two flags are independent (a value
     differing only in `notify` is `!=`).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty notify_on_command_finish_action
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `NotifyOnCommandFinishAction` has the two upstream flags and the `Default` is
  `bell = true`, `notify = false` — faithful to upstream's type and field
  defaults;
- the tests pass (the default; the independent flags; `Copy`/`Eq`), and the
  existing tests still pass;
- the parsing, the `Config` struct, and the apprt action execution stay
  deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if a flag is missing/extra, the `Default` is wrong
(e.g. `bell = false`), an unrelated item changes, or any public C API/ABI
changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It verified against the vendored upstream:
`NotifyOnCommandFinishAction { bell, notify }` matches the two-flag packed
struct (`Config.zig:10221`); the hand-written default is exact (`bell = true`,
`notify = false`, matching both the field defaults and the `Config` field
literal, `Config.zig:1232`); not extracting a helper method is the right call
(the consumer reads the two fields directly and performs imperative side
effects, `surface.zig:1163`); and deferring parsing, formatting, `Config`
wiring, and action execution is appropriately scoped. It judged the tests (the
non-derived default, value semantics, flag independence) adequate.

Review artifacts:

- Prompt: `logs/codex-review/20260604-111741-d450-prompt.md` (design)
- Result: `logs/codex-review/20260604-111741-d450-last-message.md` (design)
