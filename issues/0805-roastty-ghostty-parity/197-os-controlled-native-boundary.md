# Experiment 197: OS-controlled native boundary closure

## Description

Experiment 196 showed that a direct live `NSSound` audio playback guard is not
safe for unattended automation in this macOS VM because the rebuilt Roastty
debug app can trigger a microphone TCC prompt. The only remaining CFG-223 gap is
`RUNTIME-012B2B2B2B2B3C`, which currently asks for deterministic proof of
OS-controlled notification delivery, audible bell output, and OS-visible Dock
attention state beyond the already-proven app request paths.

This experiment will close that residual at the correct parity boundary. The
app-controlled behavior is whether Roastty reaches the same copied macOS APIs as
pinned Ghostty with equivalent config, lifecycle, and dispatch state. The final
OS presentation after those requests is controlled by macOS authorization,
desktop settings, focus state, audio devices, and TCC prompts, so it must not be
a required unattended regression guard.

## Changes

- Add a focused static/runtime residual guard, tentatively
  `issues/0805-roastty-ghostty-parity/os_controlled_native_boundary_parity.py`.
  The guard will prove:
  - copied source parity for the remaining app-controlled native paths:
    - `vendor/ghostty/macos/Sources/App/macOS/AppDelegate.swift` versus
      `roastty/macos/Sources/App/macOS/AppDelegate.swift`, normalized for
      Ghostty/Roastty renames, existing `appendUITestTrace` hooks,
      `let requestID = NSApp.requestUserAttention(.informationalRequest)`, and
      the current async UserNotifications compatibility normalization already
      used by `macos_user_notification_runtime_parity.py`;
    - `vendor/ghostty/macos/Sources/Ghostty/Ghostty.App.swift` versus
      `roastty/macos/Sources/Roastty/Roastty.App.swift`, normalized for renames
      and existing UI-test URL hooks;
    - the notification lifecycle slice in
      `vendor/ghostty/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift`
      versus
      `roastty/macos/Sources/Roastty/Surface View/SurfaceView_AppKit.swift`,
      including request construction, sound/category/userInfo, delivered
      notification cleanup, and click response routing;
    - `vendor/ghostty/macos/Sources/Ghostty/GhosttyPackage.swift` versus
      `roastty/macos/Sources/Roastty/RoasttyPackage.swift` for notification
      category/action identifiers.
  - required app-controlled notification oracles:
    - `macos_user_notification_runtime_parity.py` passes for copied lifecycle
      source parity and explicit markers: `UNMutableNotificationContent()`,
      `content.sound = UNNotificationSound.default`, `UNNotificationRequest`,
      `UNUserNotificationCenter.current().add(request)`,
      `removeDeliveredNotifications(withIdentifiers:)`, `requireFocus` userInfo,
      and notification response routing;
    - `macos_live_user_notification_delivery.py` either proves delivered
      notification content through `getDeliveredNotifications` when the VM is
      authorized, or records `authorization_status = 1`, `alert_setting = 2`,
      `sound_setting = 2`, `userNotification settings status=1 alert=2 sound=2`,
      and `userNotification uiTestAction=blocked status=1` as the current OS
      authorization boundary. If the authorized branch does not pass on an
      authorized VM, this experiment must not close the row.
  - required bell/audio oracles:
    - `bell_presentation_runtime_parity.py` passes for copied bell source parity
      and exact request-path anchors: `NSSound.beep()`,
      `NSSound(contentsOfFile: configPath.path, byReference: false)`,
      `sound.volume = roastty.config.bellAudioVolume`, and `sound.play()`;
    - existing live trace evidence from
      `macos_notification_link_bell_trace_runtime.py` proves
      `ringBell target=surface`,
      `appBell system=false audio=true attention=false`, the configured
      `bell-audio-path`, and volume request;
    - Experiment 196 evidence is recorded as the reason physical audio-device
      playback may not be required in unattended guards: the rebuilt debug app
      can trigger the declared microphone TCC prompt
      `NSMicrophoneUsageDescription`.
  - required Dock attention/badge oracles:
    - `macos_live_bell_attention_dock_state.py` proves inactive-app
      `NSApp.requestUserAttention(.informationalRequest)` dispatch by requiring
      `appBell active=false` and `appBell attentionRequest=0`;
    - the same guard records Dock badge authorization state
      `authorizationStatus=1 badgeSetting=2`;
    - source parity anchors cover `terminalWindowHasBell`, `syncDockBadge`, and
      `setDockBadge`.
  - final stale-gap assertions:
    - `RUNTIME-012B2B2B2B2B3C` must no longer contain `Gap`,
      `Still need deterministic proof`,
      `actual OS notification delivery/banner/sound`, `audible bell output`, or
      `OS-visible dock-attention bounce/state beyond AppKit request dispatch`;
    - the row evidence must explicitly state that notification banner/sound,
      physical speaker output, and Dock bounce/state are closed only at the
      copied macOS API request and authorization-state boundary, not as
      deterministic OS-presentation pixel/audio claims.
- Update `config_runtime_inventory.py`:
  - change `RUNTIME-012B2B2B2B2B3C` from `Gap` to `Oracle complete` if the guard
    proves all app-controlled request boundaries are accounted for;
  - rewrite the row behavior and evidence as a final OS-controlled native
    presentation boundary audit instead of an unresolved demand for physical OS
    presentation;
  - include the acceptance rationale in the inventory row itself: pinned Ghostty
    and Roastty use the same copied AppKit/UserNotifications/NSSound request
    paths, all app-controlled branches are covered by source parity plus live
    request-boundary traces, and the remaining OS presentation depends on
    macOS/TCC state outside either app's deterministic control;
  - keep `divergences.md` unchanged unless implementation discovers an actual
    Ghostty/Roastty behavior difference. This is intended as a parity-boundary
    closure, not an intentional divergence.
- Update `notification_link_bell_gui_residual_parity.py` and
  `bell_presentation_runtime_parity.py` so they assert the closed residual row,
  the current runtime counts, and the absence of stale gap wording.
- Regenerate `config-runtime-inventory.md` and `config-matrix.md`.
- Update Issue 805 `README.md` Learnings and Experiments index after the result
  is known.

## Verification

Pass criteria:

- The new guard proves the final residual row has no app-controlled behavior
  left unaccounted for and that OS/TCC presentation details are classified only
  after copied API request paths and authorization-state boundaries are already
  proven by prior live guards.
- Notification delivery is closed only if the guard proves one of two explicit
  outcomes:
  - on an authorized VM, `macos_live_user_notification_delivery.py` reaches the
    `getDeliveredNotifications` branch and proves the deterministic notification
    ID/title/body/category/surface userInfo; or
  - on the current denied VM, the inventory row explicitly accepts notification
    banner/sound delivery as an OS authorization boundary after the copied
    request construction, `UNUserNotificationCenter.add(request)`,
    delivered-notification cleanup, foreground-presentation delegate, and
    response-routing source parity all pass.
- Audio is closed only if the guard proves copied `NSSound` request source
  parity, live configured audio-path/volume request dispatch, and Experiment
  196's microphone TCC prompt boundary. It must not claim physical audible
  speaker output.
- Dock attention is closed only if the guard proves copied
  `NSApp.requestUserAttention(.informationalRequest)` source parity, live
  inactive-app request dispatch, and Dock badge authorization-state capture. It
  must not claim deterministic Dock animation pixels.
- `RUNTIME-012B2B2B2B2B3C` becomes `Oracle complete`.
- CFG-223 becomes `Pass` with zero gap rows.
- Runtime counts are exact after regeneration. Expected direction: runtime rows
  remain 98, Oracle-complete rows increase from 94 to 95, closed rows increase
  from 97 to 98, incomplete rows drop from 1 to 0, gap rows drop from 1 to 0,
  and CFG-223 changes from `Gap` to `Pass`.
- No new live guard may depend on accepting a TCC prompt or on a human-visible
  notification/audio/Dock animation. Existing live guards may be invoked only
  where they are already deterministic in this VM.
- The result must clearly state that parity is being closed at the
  app-controlled macOS API request boundary, not by claiming deterministic
  control over macOS notification banners, physical speaker output, or Dock
  animation pixels.

Commands:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_user_notification_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_live_user_notification_delivery.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/os_controlled_native_boundary_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/notification_link_bell_gui_residual_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/bell_presentation_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/197-os-controlled-native-boundary.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

The result must state the exact runtime row count, Oracle-complete count, closed
count, incomplete count, gap count, CFG-223 status, and remaining gap IDs.

## Design Review

Fresh-context Codex adversarial reviewer `Heisenberg the 3rd` reviewed the
initial design and returned **Changes Required**.

Required finding accepted: the initial pass criteria could have closed the final
residual by reclassifying unproven behavior without a concrete oracle for each
remaining slice. The design now requires explicit notification, audio, and Dock
oracles, including authorized and denied-VM notification outcomes, copied
`NSSound` request source parity plus live audio-path/volume request evidence,
and copied/live Dock attention request evidence without claiming deterministic
OS presentation.

Required finding accepted: the initial new-guard description was too broad. The
design now names exact source files, allowed normalizations, required trace and
source anchors, stale-gap wording that must disappear, and inventory acceptance
wording that must be present before `RUNTIME-012B2B2B2B2B3C` can close.

Optional finding accepted: the design now explicitly runs
`macos_user_notification_runtime_parity.py`.

Fresh-context Codex re-reviewer `Locke the 3rd` approved the revised design with
no required findings.

## Result

**Result:** Pass

Experiment 197 closed the final CFG-223 runtime/UI gap at the app-controlled
macOS API request and authorization-state boundary. It did not claim
deterministic control over macOS notification banners, notification sounds,
physical speaker output, microphone TCC prompts, or Dock animation pixels after
Roastty has made the copied OS request.

Implementation changes:

- Added `os_controlled_native_boundary_parity.py`, a focused closure guard for
  the final `RUNTIME-012B2B2B2B2B3C` row.
- Updated `config_runtime_inventory.py` so `RUNTIME-012B2B2B2B2B3C` is
  `Oracle complete` with explicit evidence for:
  - copied UserNotifications source parity and denied-VM authorization-state
    boundary;
  - copied `NSSound` request source parity, live configured audio-path/volume
    request evidence, and Experiment 196's microphone TCC prompt boundary;
  - copied `NSApp.requestUserAttention(.informationalRequest)` source parity,
    live inactive-app request dispatch, and Dock badge authorization-state
    capture.
- Regenerated `config-runtime-inventory.md` and `config-matrix.md`; CFG-223 is
  now `Pass`.
- Updated `macos_user_notification_runtime_parity.py`,
  `bell_presentation_runtime_parity.py`, and
  `notification_link_bell_gui_residual_parity.py` so they assert the closed
  residual row, exact counts, and absence of stale gap wording.

The live user-notification guard ran again in this VM and recorded the expected
authorization boundary:

- `authorization_status = 1`
- `alert_setting = 2`
- `sound_setting = 2`
- trace `userNotification settings status=1 alert=2 sound=2`
- trace `userNotification uiTestAction=blocked status=1`
- no new Roastty crash reports

Final CFG-223 counts:

- runtime rows: 98
- Oracle-complete runtime rows: 95
- closed rows: 98
- incomplete rows: 0
- gap rows: 0
- CFG-223 status: `Pass`
- remaining gap IDs: none

Commands run:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_user_notification_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/bell_presentation_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/notification_link_bell_gui_residual_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/macos_live_user_notification_delivery.py > logs/issue805-exp197-user-notification.log 2>&1
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/os_controlled_native_boundary_parity.py
python3 -m py_compile issues/0805-roastty-ghostty-parity/*.py
rm -rf issues/0805-roastty-ghostty-parity/__pycache__
prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/197-os-controlled-native-boundary.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md
git diff --check
```

## Completion Review

Fresh-context Codex adversarial reviewer `Helmholtz the 3rd` reviewed the
completed experiment result and returned **Approved** with no findings.

The reviewer independently verified that
`os_controlled_native_boundary_parity.py`,
`notification_link_bell_gui_residual_parity.py`,
`bell_presentation_runtime_parity.py`, and
`macos_user_notification_runtime_parity.py` pass; markdown formatting passes;
and `git diff --check` passes. The reviewer also confirmed the result commit had
not yet been made, the latest relevant commit was the Exp197 plan commit, the
runtime inventory counts are 98 runtime rows, 95 Oracle-complete rows, 98 closed
rows, 0 incomplete rows, and 0 gaps, and the result does not claim deterministic
OS notification banner/sound, physical speaker output, microphone TCC prompt
automation, or Dock animation pixels.

## Conclusion

The final runtime/UI residual is closed. CFG-223 now passes with every runtime
row either Oracle-complete, not applicable, or an accepted intentional
divergence. The issue no longer has an unresolved runtime/UI gap for
notification, audio, or Dock presentation: all app-controlled behavior is
covered by copied source parity and focused request-boundary/live-state guards,
while final OS presentation remains explicitly outside deterministic app
control.
