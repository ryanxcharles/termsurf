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

## Result

**Result:** Partial

The initial approved oracle was too strong for this VM/macOS build. The first
implementation attempted to require a nonzero return value from
`NSApp.requestUserAttention(.informationalRequest)`. The live trace instead
showed the app was inactive when the attention branch ran, but AppKit returned
`0`:

```text
appBell system=false audio=false attention=true
appBell attention=true
appBell active=false
appBell attentionRequest=0
```

The guard was adjusted to record that behavior honestly instead of treating it
as proof of an OS-visible Dock bounce. The passing run proves:

- the debug app launched with isolated config/defaults;
- Roastty was backgrounded before BEL (`front_pid_before_bel = 444`, not the
  Roastty PID);
- the attention-enabled run emitted `ringBell target=surface`,
  `appBell system=false audio=false attention=true`, `appBell active=false`,
  `appBell attentionRequest=0`, and `surfaceBell state=true`;
- the attention-disabled control emitted
  `appBell system=false audio=false attention=false` with no attention request
  or active-state trace;
- both runs recorded `dockBadge authorizationStatus=1 badgeSetting=2`, so badge
  labels are intentionally unavailable in this VM authorization state;
- no new Roastty crash report was observed.

The inventory now splits `RUNTIME-012B2B2B2B2B3C7` as Oracle complete for live
inactive-app Dock attention request dispatch and badge authorization capture.
`RUNTIME-012B2B2B2B2B3C` remains a `Gap` for actual OS notification
delivery/banner/sound after authorization is available, audible bell output,
OS-visible Dock attention bounce/state beyond AppKit request dispatch, Quick
Look/native link preview display beyond the copied SwiftUI URLHoverBanner, and
external Launch Services handler delivery.

The regenerated CFG-223 counts are:

- runtime rows: 96
- Oracle complete: 92
- closed: 95
- audit covered: 0
- incomplete: 1
- runtime gaps: 1
- CFG-223 status: `Gap`

Verification logs:

- `logs/issue805-exp192-build-1.log` for the build before the first guard run
- `logs/issue805-exp192-dock-attention-1.log` for the failed standalone
  notification-settings probe
- `logs/issue805-exp192-build-2.log` for the rebuild with app-owned badge
  authorization trace
- `logs/issue805-exp192-dock-attention-2.log` for the failed nonzero request-ID
  oracle
- `logs/issue805-exp192-build-3.log` for the rebuild with active-state trace
- `logs/issue805-exp192-dock-attention-3.log` for the second failed nonzero
  request-ID oracle, proving `active=false` and `attentionRequest=0`
- `logs/issue805-exp192-dock-attention-4.log` for the passing adjusted guard
- `logs/issue805-exp192-config-runtime-inventory-1.log`
- `logs/issue805-exp192-residual-guard-1.log`
- `logs/issue805-exp192-py-compile-1.log`
- `logs/issue805-exp192-prettier-check-5.log`
- `logs/issue805-exp192-diff-check-3.log`

## Conclusion

Experiment 192 did not prove OS-visible Dock bounce/state. It did prove the live
background AppKit attention request dispatch path and the VM's badge
authorization branch, and it showed that a nonzero `requestUserAttention` return
value is not a reliable oracle here. The next dock-related attempt needs a
different OS-visible measurement, such as a reliable Dock UI/screenshot oracle,
or should move to another remaining residual slice.

## Completion Review

Fresh-context Codex adversarial reviewer `Russell the 3rd` reviewed the
completed experiment, implementation diff, inventory split, residual guard, and
verification logs.

Final verdict: **Approved**.

Optional finding accepted for future improvement: the passing live guard log
contains only `macos_live_bell_attention_dock_state=pass`, while the detailed
JSON evidence is written to
`/tmp/termsurf-issue805-exp192-dock-attention-latest.json`. The reviewer agreed
the script assertions prove the behavior, so this is not blocking.

Nit fixed: the result log list now points at the latest post-update
`prettier-check-5` and `diff-check-3` logs.
