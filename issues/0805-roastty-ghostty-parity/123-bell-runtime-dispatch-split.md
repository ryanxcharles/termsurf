# Experiment 123: Bell Runtime Dispatch Split

## Description

`RUNTIME-012B` still combines bell delivery with command-finish notifications,
app notifications, hover/cursor UI, link previews, and context/menu link flows.
Pinned Ghostty handles terminal BEL in the live PTY path by having
`termio/stream_handler.zig` write a `.ring_bell` surface message, then
`Surface.zig` throttles repeated bells and asks the app runtime to perform the
ring-bell action.

Roastty already has parser/formatter coverage for `bell-features`,
`bell-audio-path`, and `bell-audio-volume`, plus macOS-side copied action
handling for `ROASTTY_ACTION_RING_BELL`. The missing runtime proof is narrower:
a BEL received from the live PTY-backed terminal must reach the Roastty surface
action dispatch path. The current terminal core can invoke an optional C bell
callback, but `TermioWorker::spawn` rejects terminals with callbacks installed,
so live app surfaces need a non-callback bell event path.

This experiment will split the proven BEL dispatch slice out of `RUNTIME-012B`
without claiming full bell-feature UI/audio parity. Full macOS feature effects
such as system beep, custom bell audio, attention, title/border presentation,
command-finish notifications, app notifications, hover/cursor UI, link previews,
and context/menu link flows remain in the follow-up gap.

## Changes

- `roastty/src/terminal/terminal.rs`
  - Add a callback-independent pending bell counter that increments whenever the
    terminal consumes a BEL.
  - Keep the existing C bell callback behavior intact for embedded terminal API
    callers.
  - Add terminal-core tests proving BEL increments the pending counter with no
    callback and still invokes the existing callback path when configured.
- `roastty/src/termio.rs`
  - Add a bell count to `TermioPump` and drain pending terminal bells after PTY
    output is parsed.
  - Include the bell count in worker event emission so a BEL-only pump reaches
    the surface even when there are no printable bytes or write state changes.
  - Add Termio tests proving a child process that writes BEL produces a pump
    with a nonzero bell count.
- `roastty/src/lib.rs`
  - Convert nonzero pump bell counts into `ROASTTY_ACTION_RING_BELL` surface
    action dispatches, preserving the existing dirty/output handling.
  - Add a surface-side 100ms repeated-bell throttle matching pinned Ghostty's
    `Surface.zig` `.ring_bell` handling before dispatching the app action.
  - Add a surface test proving a synthetic bell pump records the ring-bell
    action, a repeated synthetic bell pump inside the throttle window records no
    duplicate action, and a live PTY-backed test proving BEL output reaches that
    action path through Termio.
- `issues/0805-roastty-ghostty-parity/bell_runtime_dispatch_parity.py`
  - Add a static parity guard that verifies the pinned Ghostty BEL path
    (`stream_handler.zig` `.ring_bell`, `Surface.zig` ring-bell throttle/action)
    and the Roastty path (terminal pending bell count, Termio pump bell count,
    surface `ROASTTY_ACTION_RING_BELL`, Swift ring-bell handler).
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-012B` into:
    - `RUNTIME-012B1`: `Oracle complete` for terminal BEL to live surface
      ring-bell action dispatch.
    - `RUNTIME-012B2`: `Gap` for bell feature UI/audio effects, command-finish
      notifications, app notifications, hover/cursor UI, link previews, and
      context/menu link flows.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the runtime inventory script.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add the experiment link and update Learnings with any durable finding from
    the implementation.

## Verification

Pass criteria:

- `cargo fmt --manifest-path roastty/Cargo.toml --check`
- `cargo test --manifest-path roastty/Cargo.toml bell_runtime`
- `cargo test --manifest-path roastty/Cargo.toml termio_bell`
- `cargo test --manifest-path roastty/Cargo.toml surface_bell`
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/bell_runtime_dispatch_parity.py`
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
- A matrix assertion verifies:
  - `RUNTIME-012B1` is `Oracle complete`;
  - `RUNTIME-012B1` evidence and guard command name the bell runtime tests and
    static parity guard, including repeated-BEL throttle coverage;
  - `RUNTIME-012B2` remains `Gap`;
  - `RUNTIME-012B2` still names bell feature UI/audio effects, command-finish
    notifications, app notifications, hover/cursor UI, link previews, and
    context/menu link flows;
  - `CFG-223` remains `Gap` until all runtime/UI rows are closed.
- `git diff --check`
- No generated `__pycache__` remains under the issue directory.

Fail criteria:

- BEL reaches only the embedded C callback path and not the live PTY-backed app
  surface path.
- The implementation requires installing terminal callbacks on `TermioWorker`
  terminals.
- The inventory claims full bell-feature, notification, preview, or context/menu
  parity without focused runtime or GUI proof.
- The static parity guard cannot find both the pinned Ghostty path and the
  Roastty runtime/action path.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Initial verdict: **Changes required**.

- **Required:** The runtime inventory command listed only `--output`, but
  `config_runtime_inventory.py` also requires `--matrix`. Fixed by adding
  `--matrix issues/0805-roastty-ghostty-parity/config-matrix.md` to the
  verification command.
- **Required:** The design named Ghostty's `.ring_bell` surface path and
  throttle but only planned action dispatch coverage. Fixed by including the
  100ms surface-side repeated-BEL throttle in scope and requiring throttle
  coverage in the surface tests and matrix guard evidence.

Re-review verdict: **Approved**. The reviewer confirmed both required findings
were resolved and reported no new required findings.
