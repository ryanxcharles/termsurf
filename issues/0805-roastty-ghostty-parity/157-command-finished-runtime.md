# Experiment 157: Command-Finished Runtime

## Description

`RUNTIME-012B2B2B2` still tracks command-finish notifications. Pinned Ghostty
uses OSC 133 semantic prompt actions to time shell commands:

- `end_input_start_output` writes a `start_command` surface message;
- `end_command` writes a `stop_command` surface message with an exit code;
- `Surface.zig` measures elapsed time between those messages and dispatches
  `.command_finished` with the exit code and nanosecond duration;
- the copied macOS app handles `GHOSTTY_ACTION_COMMAND_FINISHED` by applying
  `notify-on-command-finish`, `notify-on-command-finish-after`, and
  `notify-on-command-finish-action`, then posts a bell and/or user notification.

Roastty currently parses OSC 133 semantic prompt actions and preserves prompt
state, and the copied Swift app already handles
`ROASTTY_ACTION_COMMAND_FINISHED` after expected renames. The missing runtime
slice is the Terminal/Termio/Surface path that turns semantic prompt start/stop
events into a typed command-finished app action.

This experiment is narrower than full command-finish notification GUI parity. It
will prove the deterministic runtime action dispatch and copied Swift handling
source parity, but it will not claim live macOS Notification Center delivery,
actual bell side effects, app-notification toasts, or GUI walkthrough behavior.

## Changes

- Update terminal/termio runtime plumbing:
  - Add a typed terminal command-event queue for semantic prompt command start
    and command stop events.
  - On OSC 133 `EndInputStartOutput`, preserve the existing semantic-output
    screen behavior and queue a command-start event.
  - On OSC 133 `EndCommand`, preserve the existing semantic-output screen
    behavior, parse the exit code using Ghostty's rules (`0` when absent or
    malformed/unparseable, `1` when parsed but outside `u8` range, otherwise the
    valid `u8` value), and queue a command-stop event.
  - Drain command events through `TermioPump` in order with titles, pwd, desktop
    notifications, bells, and child-exit state.
- Update `roastty/src/lib.rs`:
  - Add per-surface command timer state.
  - On command-start, record `Instant::now()`.
  - On command-stop with no timer, do nothing.
  - On command-stop with a timer, clear the timer, compute elapsed nanoseconds,
    and dispatch `ROASTTY_ACTION_COMMAND_FINISHED` with the exit code and
    duration.
  - Add `ROASTTY_ACTION_COMMAND_FINISHED` storage-to-union conversion for
    `roastty_action_command_finished_s`, including `exit_code` and `duration`,
    so copied Swift can read `action.action.command_finished`.
  - Preserve existing child-exited and wait-after-command behavior.
  - Extend test action recording to capture command-finished payloads.
  - Add focused tests proving:
    - terminal OSC 133 command start/stop events are captured without display or
      response side effects;
    - missing and malformed/unparseable stop exit codes map to `0`, parsed
      out-of-range values such as `-1` or `256` map to `1`, and valid `0..255`
      values are preserved;
    - `TermioPump` drains command events;
    - surface dispatch emits `ROASTTY_ACTION_COMMAND_FINISHED` only after a
      start/stop pair;
    - `action_u_from_storage` populates the C union command-finished payload
      with `exit_code` and `duration`;
    - stop without start does not dispatch;
    - repeated start resets the timer like Ghostty's latest `command_timer`;
    - command-finished dispatch does not replace existing child-exited action
      dispatch.
- Add a focused static/runtime guard:
  - `issues/0805-roastty-ghostty-parity/command_finished_runtime_parity.py`
  - Assert pinned Ghostty's stream-handler and surface command-finished source
    markers are present.
  - Assert Roastty has terminal command events, termio pump propagation, surface
    timer/action dispatch, copied Swift `commandFinished` handling, and
    deterministic tests.
- Update `config_runtime_inventory.py` to split `RUNTIME-012B2B2B2` into:
  - an Oracle complete command-finished runtime row owned by this experiment;
  - a remaining notification/link/bell GUI gap row for `app-notifications`, live
    OS banner/sound delivery, actual bell side effects, link hover/cursor UI,
    link previews, and context/menu link flows.
- Regenerate `config-runtime-inventory.md` and `config-matrix.md`.
- Update existing runtime parity guards and `terminal_runtime_residual_audit.py`
  for the new CFG-223 row counts and remaining notification/link/bell gap id.
- Update Issue 805 learnings after the result is known.

## Verification

Pass criteria:

- Focused Rust tests pass:

```bash
cargo test --manifest-path roastty/Cargo.toml command_finished_runtime
cargo test --manifest-path roastty/Cargo.toml terminal_command_event
cargo test --manifest-path roastty/Cargo.toml termio_command_event
```

- The new command-finished parity guard passes:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/command_finished_runtime_parity.py
```

- Adjacent notification guards still pass:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/desktop_notification_rate_limit_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_user_notification_runtime_parity.py
```

- The runtime inventory generator reports one additional Oracle complete row and
  the same total number of unresolved CFG-223 gaps unless implementation
  uncovers a real additional gap. Expected output after this split:
  `runtime_rows=65`, `oracle_complete=59`, `closed=61`, `incomplete=4`, `gap=4`,
  and `cfg223=Gap`.

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
```

- All runtime parity guards and the terminal residual audit still pass:

```bash
for guard in issues/0805-roastty-ghostty-parity/*_runtime_parity.py; do
  PYTHONDONTWRITEBYTECODE=1 python3 "$guard" || exit 1
done
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py
```

- Rust formatting and diff hygiene pass:

```bash
cargo fmt --manifest-path roastty/Cargo.toml --check
git diff --check
```

## Design Review

**Reviewer:** Euclid the 2nd

**Verdict:** Approve after fixes

The first review required correcting Ghostty exit-code semantics for malformed
OSC 133 D values: absent or malformed/unparseable values map to `0`, while
parsed out-of-range values map to `1`. The reviewer also recommended explicitly
including `ROASTTY_ACTION_COMMAND_FINISHED` storage-to-union conversion so the
copied Swift app can read `action.action.command_finished`. The revised design
adds both requirements and was approved on re-review.
