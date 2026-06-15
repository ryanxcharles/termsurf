# Experiment 153: Bell Presentation Runtime

## Description

`RUNTIME-012B2B` still groups several notification, bell, hover, preview, and
context-menu behaviors together. This experiment isolates the copied macOS bell
presentation plumbing that sits after the already-proven BEL-to-app action path:

- `BaseTerminalController` aggregate bell-state publishing from all surface
  views in the window;
- close-time bell-state clearing so observers can remove stale bell state;
- `AppDelegate` observation of terminal-window bell state changes;
- dock badge counting and `bell-features = attention` gating;
- copied `bell-features = system`, `audio`, and `attention` handlers for
  `.roasttyBellDidRing`;
- focused-surface title recomputation through `bell-features = title`;
- `SurfaceView` bell-state update and border overlay gating through
  `bell-features = border`.

This is narrower than a full native notification/link/bell GUI walkthrough. It
will not claim that macOS actually plays a configured audio file, bounces the
dock icon, updates the dock tile at runtime under every notification
authorization state, renders border/title pixels, presents native desktop
notifications, implements command-finish notification UI, or handles link
hover/preview/context-menu flows.

## Changes

- Add a focused static parity guard:
  - `issues/0805-roastty-ghostty-parity/bell_presentation_runtime_parity.py`
  - Assert that pinned Ghostty and Roastty copied Swift app sources preserve the
    bell presentation markers after expected Ghostty-to-Roastty renames.
  - Check `BaseTerminalController.swift` for the aggregate
    `surfaceValuesPublisher` bell publisher, `.removeDuplicates()`, main-queue
    delivery, `terminalWindowBellDidChangeNotification`, `hasBell` userInfo,
    close-time clear notification, focused-surface title/bell `combineLatest`,
    `computeTitle`, and `bellFeatures.contains(.title)`.
  - Check `AppDelegate.swift` for terminal-window bell observation,
    `syncDockBadge`, `setDockBadge`, `bellFeatures.contains(.attention)`,
    `.system` gating of `NSSound.beep()`, `.audio` gating of
    `NSSound(contentsOfFile:)`/volume/playback, and `.attention` gating of
    `NSApp.requestUserAttention`.
  - Check `SurfaceView.swift` and `SurfaceView_AppKit.swift` for
    `bellFeatures.contains(.border)`, `BellBorderOverlay`, `.roasttyBellDidRing`
    observation, and surface `bell` state transitions.
  - Reuse the existing `bell_runtime_dispatch_parity.py` guard as prerequisite
    evidence for terminal BEL to `ROASTTY_ACTION_RING_BELL` dispatch.
- Update `config_runtime_inventory.py` to split `RUNTIME-012B2B` into:
  - an Oracle complete copied macOS bell presentation plumbing row owned by this
    experiment;
  - a remaining notification/link/bell GUI gap row for command-finish
    notifications, app-notifications, native desktop notification presentation
    and rate limiting, actual audio/dock/border/title GUI effects, hover/cursor
    UI, link previews, and context/menu link flows.
- Regenerate `config-runtime-inventory.md` and `config-matrix.md`.
- Update existing runtime parity guards and `terminal_runtime_residual_audit.py`
  for the new CFG-223 row counts and remaining notification/link/bell gap id.
- Update Issue 805 learnings with the copied macOS bell presentation finding
  after the result is known.

## Verification

Pass criteria:

- The existing BEL dispatch prerequisite guard passes:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/bell_runtime_dispatch_parity.py
```

- The new static bell presentation parity guard passes:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/bell_presentation_runtime_parity.py
```

- The runtime inventory generator reports one additional Oracle complete row and
  the same total number of unresolved CFG-223 gaps unless this experiment
  discovers a real fixable discrepancy:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
```

- All runtime parity guards still pass:

```bash
for guard in issues/0805-roastty-ghostty-parity/*_runtime_parity.py; do
  PYTHONDONTWRITEBYTECODE=1 python3 "$guard" || exit 1
done
```

- The terminal residual audit still passes:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py
```

- Markdown and diff hygiene pass:

```bash
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/README.md \
  issues/0805-roastty-ghostty-parity/153-bell-presentation-runtime.md \
  issues/0805-roastty-ghostty-parity/config-runtime-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

Adversarial subagent `019eca0a-8396-7d73-a528-95d8d2de536c` reviewed the design
with fresh context.

Initial verdict: `CHANGES REQUIRED`.

Required finding:

- The planned static guard claimed copied macOS bell presentation plumbing but
  did not explicitly require separate proof for `bell-features = system`,
  `audio`, and `title` gates. A guard could have passed while missing the
  `NSSound.beep()` system gate, configured-audio gate, or focused-surface
  title/bell `combineLatest` plus `bellFeatures.contains(.title)` path.

Fix:

- Tightened the design so the guard must prove the `system`, `audio`,
  `attention`, `title`, and `border` bell feature gates separately, including
  title/bell `combineLatest`, `computeTitle`, `.system` beep gating, `.audio`
  sound/volume/playback gating, and `.attention` request gating.

Re-review verdict: `APPROVED`.

The reviewer confirmed the prior finding was resolved and found no new required
issues.

## Result

**Result:** Pass

Implemented the static copied macOS bell presentation parity guard and split the
notification/link/bell runtime inventory:

- `RUNTIME-012B2B1`: **Oracle complete** for copied macOS bell presentation
  plumbing.
- `RUNTIME-012B2B2`: **Gap** for remaining notification/link/bell GUI effects:
  command-finish notifications, app-notifications, native desktop notification
  presentation and rate limiting, actual audio/dock/border/title GUI effects,
  hover/cursor UI, link previews, and context/menu link flows.

The new guard proves that Roastty preserves pinned Ghostty's copied macOS bell
presentation plumbing after expected product renames. It checks aggregate window
bell-state publishing, close-time bell clearing, terminal-window bell
observation, dock badge counting, surface bell-state updates, border overlay
gating, focused-surface title/bell recomputation, and separate
`bell-features = system`, `audio`, `attention`, `title`, and `border` gates.

Verification passed:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/bell_runtime_dispatch_parity.py
```

Output:

```text
bell_runtime_dispatch_parity=pass
```

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/bell_presentation_runtime_parity.py
```

Output:

```text
bell_presentation_runtime_parity=pass
```

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
```

Output:

```text
runtime_rows=61
oracle_complete=55
closed=57
audit_covered=0
incomplete=4
gap=4
cfg223=Gap
```

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/desktop_notification_runtime_parity.py
```

Output:

```text
desktop_notification_runtime_parity=pass
```

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/terminal_runtime_residual_audit.py
```

Output:

```text
terminal_runtime_residual_audit=pass
```

## Conclusion

Roastty preserves the copied macOS bell presentation plumbing for this bounded
source-level slice. This closes the deterministic app-host bell wiring after the
live BEL dispatch path without claiming actual macOS sound, dock, border,
titlebar, native notification, or link GUI side effects.

CFG-223 remains open with four unresolved runtime gaps: remaining font renderer
output effects, remaining renderer-visible visual effects, remaining macOS app
workflow/UI effects, and remaining notification/link/bell GUI effects.

## Completion Review

Adversarial subagent `019eca11-86ca-7e21-accd-c754a0d81226` reviewed the
completed experiment with fresh context and returned `VERDICT: APPROVED`.

Findings: none.

The reviewer independently verified the BEL dispatch prerequisite guard, the new
bell presentation parity guard, regenerated runtime inventory to `/tmp`, the
full runtime parity guard loop, the terminal runtime residual audit, and
`git diff --check`. The reviewer also confirmed that no result commit existed
after plan commit `3dfda1aae`, that changes are confined to
`issues/0805-roastty-ghostty-parity`, that the README marks Experiment 153 as
`Pass`, and that the inventory split keeps `RUNTIME-012B2B1` source-level copied
bell plumbing separate from `RUNTIME-012B2B2` remaining GUI/OS/audio/dock/
border/title/notification/link gaps.
