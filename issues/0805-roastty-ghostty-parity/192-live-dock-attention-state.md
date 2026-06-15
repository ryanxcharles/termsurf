# Experiment 192: Live dock attention state

## Description

Experiment 191 split real OS link cursor pixels out of the remaining
`RUNTIME-012B2B2B2B2B3C` gap. The residual row still includes actual OS
notification delivery/banner/sound, audible bell output, measurable
dock-attention state, Quick Look/native link preview display, and external
Launch Services handler delivery.

This experiment targets only the dock-attention slice of bell behavior. Pinned
Ghostty and Roastty both call
`NSApp.requestUserAttention(.informationalRequest)` when `bell-features`
contains `attention`, and both update the Dock badge from aggregate terminal
bell state when macOS notification badge authorization allows it. Existing
guards prove the source plumbing and some live bell dispatch, but they do not
prove a measurable live attention request in the running app.

The expected outcome is a new Oracle-complete runtime row for live dock
attention request state, or a documented failure explaining which macOS/VM
boundary prevents deterministic measurement.

## Changes

- Add a focused live guard, tentatively
  `issues/0805-roastty-ghostty-parity/macos_live_bell_attention_dock_state.py`.
  - Launch the built debug Roastty app with an isolated config/defaults suite.
  - Use `bell-features = no-system,no-audio,attention,no-title,no-border` so the
    test isolates dock/user-attention behavior and does not claim audible sound,
    title prefix, or border pixels.
  - Start a terminal command that writes a ready marker, sleeps briefly, emits
    BEL, then sleeps long enough for the guard to observe state.
  - After the terminal is ready and before BEL fires, make another application
    frontmost and record that Roastty is no longer the frontmost app. This keeps
    the attention request meaningful instead of asking the active app to request
    attention for itself.
  - Require live trace evidence for the BEL path: `ringBell target=surface`,
    `appBell system=false audio=false attention=true`, and
    `surfaceBell state=true`.
  - Add a trace-only line around
    `NSApp.requestUserAttention(.informationalRequest)` that records the
    returned attention request ID while preserving the production call. The
    guard must require a nonzero request ID and must verify the trace line is
    emitted only when the `attention` flag is enabled.
  - Treat Dock badge label updates as opportunistic OS-authorized evidence, not
    a hard pass criterion. The current VM has previously reported notification
    authorization denied, and `syncDockBadge()` intentionally avoids setting the
    badge in denied/provisional/ephemeral states. If authorization allows badge
    updates, record and require `dockBadge bellCount=1 label=1`; otherwise
    record the authorization state and leave badge display out of the claim.
  - Check for new Roastty crash reports.
- Add or update minimal source parity checks so the guard verifies pinned
  Ghostty still calls `NSApp.requestUserAttention(.informationalRequest)` and
  still computes the Dock badge from aggregate bell state.
- Update `config_runtime_inventory.py` according to the outcome:
  - If the guard passes, split a new Oracle-complete row from
    `RUNTIME-012B2B2B2B2B3C` for live dock attention request state.
  - Keep `RUNTIME-012B2B2B2B2B3C` as a `Gap` for actual OS notification
    delivery/banner/sound, audible bell output, Quick Look/native link preview
    display, and external Launch Services handler delivery.
  - Do not claim audible bell output or notification banner/sound delivery.
- Update residual guards and stale CFG-223 counts if a new runtime row is split.
- Regenerate `config-runtime-inventory.md` and `config-matrix.md`.
- Update Issue 805 `README.md` Learnings and Experiments index after the result
  is known.

## Verification

Pass criteria:

- The guard proves exact debug-app launch, isolated config/defaults, terminal
  marker evidence, background/frontmost transition before BEL, and no new
  Roastty crash report.
- The guard proves Roastty is not frontmost before the child process emits BEL.
- The guard proves the live attention-enabled bell path with trace evidence for
  `ringBell target=surface`, `appBell system=false audio=false attention=true`,
  and `surfaceBell state=true`.
- The guard proves `NSApp.requestUserAttention(.informationalRequest)` returns a
  nonzero request ID for the attention-enabled run and that an otherwise
  identical attention-disabled control run does not emit the attention request
  line.
- If macOS notification badge authorization allows badge updates, the guard also
  records and requires `dockBadge bellCount=1 label=1`. If authorization does
  not allow badge updates, the guard records that state and does not claim Dock
  badge display.
- The experiment result does not claim actual OS notification delivery,
  notification banner/sound, audible bell output, Quick Look/native preview
  display, or external URL delivery.
- Inventory counts and remaining gap IDs are updated exactly and asserted by
  guards.

Commands:

```bash
(cd roastty && macos/build.nu --action build)
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_live_bell_attention_dock_state.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/notification_link_bell_gui_residual_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/192-live-dock-attention-state.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

The result must state the exact runtime row count, Oracle-complete count, closed
count, incomplete count, gap count, CFG-223 status, and remaining gap IDs.

## Design Review

Fresh-context Codex adversarial reviewer `Descartes the 3rd` reviewed the
current design against the Issue 805 workflow, the remaining CFG-223 residual
gap, and the pinned Ghostty/Roastty macOS bell attention and Dock badge source.

Initial note resolved before final verdict: local source inspection showed Dock
badge label updates are authorization-dependent because `syncDockBadge()`
intentionally skips badge setting in denied/provisional/ephemeral notification
states. The design was narrowed so a nonzero
`NSApp.requestUserAttention(.informationalRequest)` request ID is the required
oracle, while Dock badge label evidence is opportunistic when macOS allows it.

Final verdict: **Approved**.
