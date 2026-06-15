# Experiment 155: macOS User Notification Runtime

## Description

`RUNTIME-012B2B2` still groups command-finish notifications, app-notifications,
native desktop notification presentation and rate limiting, actual bell side
effects, link hover/cursor UI, link previews, and context/menu link flows.

This experiment isolates the copied macOS user-notification presentation and
lifecycle slice that sits after the already-proven OSC desktop notification
runtime dispatch:

- app delegate notification category registration, action identifier, delegate
  installation, foreground presentation callback, notification response
  callback, and app-termination cleanup;
- `Roastty.App` desktop-notification action dispatch to the target surface,
  authorization request, authorized-settings gate, `showUserNotification` call,
  foreground `shouldPresentNotification` focus/window logic, and notification
  click/dismiss routing;
- `SurfaceView_AppKit` notification identifier tracking, notification content
  construction, surface UUID / `requireFocus` userInfo, default sound/category,
  delivery through `UNUserNotificationCenter`, cleanup when the surface is
  removed, cleanup when the surface gains focus, delayed cleanup for focused
  surfaces, and click-to-focus behavior.

This is narrower than full notification parity. It will not claim Ghostty core's
desktop-notification rate limiting, command-finish notification generation,
`app-notifications` toasts, actual OS banner/sound delivery in a running VM, or
any link hover/preview/context-menu behavior.

## Changes

- Add a focused static parity guard:
  - `issues/0805-roastty-ghostty-parity/macos_user_notification_runtime_parity.py`
  - Assert that pinned Ghostty and Roastty `AppDelegate.swift`,
    `Ghostty.App.swift` / `Roastty.App.swift`, and `GhosttyPackage.swift` /
    `RoasttyPackage.swift` match after expected Ghostty-to-Roastty renames.
  - Extract and normalized-compare the notification-relevant
    `SurfaceView_AppKit.swift` blocks after expected Ghostty-to-Roastty renames,
    rather than only checking marker presence. The guard must compare the full
    lifecycle blocks covering `notificationIdentifiers`, deinit notification
    cleanup, focus-driven notification cleanup, `showUserNotification`, and
    `handleUserNotification`. This must fail on meaningful drift such as moving
    identifier tracking before/after successful scheduling, changing
    `requireFocus` userInfo, changing sound/category/request fields, dropping
    the delayed focused cleanup condition, or changing click-to-focus routing.
  - Assert pinned Ghostty's `Surface.zig` notification rate limiter remains
    outside this source-level Swift slice so the inventory does not overclaim
    rate-limiting parity.
- Update `config_runtime_inventory.py` to split `RUNTIME-012B2B2` into:
  - an Oracle complete copied macOS user-notification presentation/lifecycle row
    owned by this experiment;
  - a remaining notification/link/bell GUI gap row for command-finish
    notifications, `app-notifications`, native notification rate limiting,
    actual OS banner/sound delivery, actual bell side effects, link hover/cursor
    UI, link previews, and context/menu link flows.
- Regenerate `config-runtime-inventory.md` and `config-matrix.md`.
- Update existing runtime parity guards and `terminal_runtime_residual_audit.py`
  for the new CFG-223 row counts and remaining notification/link/bell gap id.
- Update Issue 805 learnings with the macOS user-notification finding after the
  result is known.

## Verification

Pass criteria:

- The new static macOS user-notification parity guard passes:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_user_notification_runtime_parity.py
```

- The existing deterministic OSC desktop notification runtime guard still
  passes:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/desktop_notification_runtime_parity.py
```

- The runtime inventory generator reports one additional Oracle complete row and
  the same total number of unresolved CFG-223 gaps unless this experiment
  discovers a real fixable discrepancy. Expected output after this split:
  `runtime_rows=63`, `oracle_complete=57`, `closed=59`, `incomplete=4`, `gap=4`,
  and `cfg223=Gap`.

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
  issues/0805-roastty-ghostty-parity/155-macos-user-notification-runtime.md \
  issues/0805-roastty-ghostty-parity/config-runtime-inventory.md \
  issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Design Review

**Reviewer:** Mendel the 2nd

**Verdict:** Approve

The reviewer found no blocking issues after the design was tightened to require
normalized comparison of the notification-relevant `SurfaceView_AppKit.swift`
lifecycle blocks, rather than marker-only checks. Residual risk: this experiment
is intentionally static source parity and does not prove live macOS notification
authorization, banner/sound delivery, Notification Center persistence, or async
scheduling behavior in a running app.

## Result

**Result:** Pass

The copied macOS user-notification presentation and lifecycle slice is now split
out as `RUNTIME-012B2B2A` with a dedicated static guard. The guard proves:

- `AppDelegate.swift`, `Roastty.App.swift`, and `RoasttyPackage.swift` match
  pinned Ghostty after expected Ghostty-to-Roastty renames;
- the notification-relevant `SurfaceView_AppKit.swift` lifecycle blocks match
  pinned Ghostty after expected renames, including identifier tracking,
  surface-removal cleanup, focus cleanup, notification content/request fields,
  `requireFocus` userInfo, delivery callback tracking, delayed focused cleanup,
  and click-to-focus routing;
- pinned Ghostty's core desktop-notification rate limiter is present in
  `Surface.zig` and remains outside this Swift source-level slice.

The remaining notification/link/bell GUI gap moved to `RUNTIME-012B2B2B` and
continues to track command-finish notifications, app-notifications, native
desktop notification rate limiting, live OS banner/sound delivery, actual bell
side effects, link hover/cursor UI, link previews, and context/menu link flows.

Verification passed:

```text
macos_user_notification_runtime_parity=pass
desktop_notification_runtime_parity=pass
bell_presentation_runtime_parity=pass
terminal_runtime_residual_audit=pass
```

The runtime inventory generator reported:

```text
runtime_rows=63
oracle_complete=57
closed=59
audit_covered=0
incomplete=4
gap=4
cfg223=Gap
```

All `*_runtime_parity.py` guards passed.

## Conclusion

The source-level macOS user-notification lifecycle copied from pinned Ghostty is
closed for parity. This does not close live notification behavior: the next
notification experiment should target one of the remaining runtime/GUI slices in
`RUNTIME-012B2B2B`, especially Ghostty core rate limiting or actual macOS
banner/sound delivery.

## Completion Review

**Reviewer:** Parfit the 2nd

**Verdict:** Approve

The reviewer found no blocking issues. They confirmed the experiment result and
README status are consistent, the new guard normalized-compares the
notification-relevant `SurfaceView_AppKit.swift` lifecycle blocks, the
`RUNTIME-012B2B2A` / `RUNTIME-012B2B2B` split is honest, CFG-223 counts are
consistent, and the verification commands pass. Residual risk remains limited to
the explicitly tracked live-notification, command/app notification, bell
side-effect, and link UI gaps in `RUNTIME-012B2B2B`.
