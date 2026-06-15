# Experiment 141: Desktop Notification Runtime

## Description

Pinned Ghostty routes terminal desktop notification OSCs through three layers:

- terminal OSC parsing produces `show_desktop_notification` for iTerm2 OSC 9
  fallback forms and OSC 777 notifications;
- `termio/stream_handler.zig` converts that terminal event into a surface
  message with fixed-size, nul-terminated title/body buffers;
- `Surface.zig` gates the message with `desktop-notifications` before
  dispatching a desktop notification action to the app.

Roastty already parses OSC 9 and OSC 777 desktop notification commands, and the
copied Swift app already has a `ROASTTY_ACTION_DESKTOP_NOTIFICATION` action
handler. The Rust terminal currently drops `DesktopNotification` OSC actions, so
PTY-backed surfaces cannot emit the action. This experiment will close the
deterministic runtime dispatch slice while leaving native OS notification
presentation, rate limiting, command-finish notifications, link preview UI, and
other GUI behavior in the existing `RUNTIME-012B2` gap.

## Changes

- `roastty/src/terminal/terminal.rs`
  - Store pending desktop notification events when OSC 9/777 notification
    actions are parsed.
  - Add a test-only drain or public internal drain method similar to the bell
    and clipboard event queues.
  - Add terminal tests whose names include `desktop_notification_runtime` for
    OSC 9 and OSC 777 notification capture, including title/body preservation
    where applicable.
- `roastty/src/termio.rs`
  - Carry pending desktop notifications through `TermioPump` or worker events so
    PTY-backed surfaces can consume them.
  - Add a focused Termio test whose name includes `desktop_notification_runtime`
    that writes an OSC notification from a child process and proves the pump
    exposes the notification.
- `roastty/src/lib.rs`
  - Populate the typed `roastty_action_desktop_notification_s` union payload
    from action storage.
  - Prepare Ghostty-equivalent PTY notification C strings: title truncated to 63
    bytes, body truncated to 255 bytes, and both nul-terminated before the app
    callback receives pointers. This mirrors pinned Ghostty's
    `apprt.surface.Message.desktop_notification` buffers.
  - Dispatch `ROASTTY_ACTION_DESKTOP_NOTIFICATION` from live surfaces when
    terminal notifications arrive and `desktop-notifications = true`.
  - Suppress dispatch when `desktop-notifications = false`.
  - Add surface tests whose names include `desktop_notification_runtime` proving
    the parsed config gate, target surface, title/body payload, disabled case,
    and overlong title/body truncation.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Split an oracle-complete desktop-notification runtime dispatch row out of
    `RUNTIME-012B2`.
  - Leave native presentation, notification rate limiting, command-finish
    notifications, app-notifications toasts, bell UI/audio, hover/cursor UI,
    link previews, and context/menu link flows in a reduced `RUNTIME-012B2`
    follow-up gap.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Update the CFG-223 runtime coverage counts after the inventory split.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add a learning that OSC desktop notification parsing was present, but live
    PTY-backed action dispatch requires an explicit
    terminal-to-termio-to-surface event queue.
- `issues/0805-roastty-ghostty-parity/desktop_notification_runtime_parity.py`
  - Add a static guard that checks pinned Ghostty OSC parser, stream-handler,
    `desktop-notifications` gate, and Roastty parser/runtime/action/inventory
    markers.

## Verification

Pass criteria:

- `cargo fmt --manifest-path roastty/Cargo.toml -- --check`
- `cargo test --manifest-path roastty/Cargo.toml desktop_notification_runtime`
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/desktop_notification_runtime_parity.py`
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
- `git diff --check`

The experiment passes only if PTY-backed Roastty surfaces dispatch the same
desktop-notification action payload shape as the copied macOS app expects,
including Ghostty-equivalent 63-byte title and 255-byte body truncation,
`desktop-notifications = false` suppresses that dispatch, every new test is
reachable by the `desktop_notification_runtime` filter, and the inventory
continues to identify the remaining GUI notification/link behaviors as gaps.

## Design Review

Fresh-context adversarial design review initially returned **Changes required**:

- the first design under-specified pinned Ghostty's fixed notification payload
  buffers, so it could have allowed unbounded title/body dispatch instead of
  63-byte title and 255-byte body truncation plus nul termination;
- the first design used the `desktop_notification_runtime` test filter without
  requiring the planned terminal, termio, and surface tests to include that
  substring.

The design was updated to require Ghostty-equivalent truncation semantics,
overlong payload tests, and `desktop_notification_runtime` in every new test
name used by the verification filter.

Re-review returned **Approved**. The reviewer confirmed both prior findings were
resolved and found no new required issues.

## Result

**Result:** Pass

Roastty now carries OSC desktop notifications from the terminal parser through
the PTY-backed runtime path. Terminal OSC 9 and OSC 777 desktop notification
events are retained in a pending queue, `TermioPump` exposes them to surfaces,
and live surfaces dispatch `ROASTTY_ACTION_DESKTOP_NOTIFICATION` when
`desktop-notifications` is enabled. The action payload is prepared as
nul-terminated C strings with pinned Ghostty's 63-byte title and 255-byte body
limits before the app callback receives it.

Verification completed:

- `cargo fmt --manifest-path roastty/Cargo.toml -- --check` — pass.
- `cargo test --manifest-path roastty/Cargo.toml desktop_notification_runtime` —
  pass: 5 tests passed, 0 failed.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/desktop_notification_runtime_parity.py`
  — pass.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  — pass: `runtime_rows=50`, `oracle_complete=43`, `closed=45`, `incomplete=5`,
  `gap=5`, `cfg223=Gap`.
- `git diff --check` — pass.

CFG-223 remains a gap overall, but the completed runtime inventory count moved
from 42 to 43 oracle-complete rows. The remaining notification/link GUI gap is
now `RUNTIME-012B2B`.

## Conclusion

The deterministic OSC desktop notification dispatch slice is now covered.
Roastty already had OSC parser coverage and the copied Swift app action handler;
the missing piece was the live terminal-to-termio-to-surface event queue plus
the Ghostty-sized C-string payload for the app callback. Native OS notification
presentation, rate limiting, command-finish notifications, app notification
toasts, bell UI/audio, link hover/cursor UI, link previews, and context/menu
link flows remain in `RUNTIME-012B2B`.

## Completion Review

Fresh-context adversarial completion review returned **Approved** with no
required findings. The reviewer independently ran the formatter check, focused
Rust test, static parity guard, inventory generation to `/tmp`, and
`git diff --check`, and confirmed the result commit had not yet been made.
