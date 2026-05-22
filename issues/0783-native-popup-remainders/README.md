+++
status = "open"
opened = "2026-05-22"
+++

# Issue 783: Remaining native popup bugs after Shell fix

## Goal

Fix the native popup bugs that remain after Issue 782, one focused bug at a
time, without reopening the completed Shell-window mouse transparency work.

## Background

Issue 779 fixed the primary y-axis placement bug for Blink PagePopup controls.
Date, time, date-time, and color controls now appear at the correct y position
inside the TermSurf webview overlay.

Issue 782 then fixed the session-wide native-widget shutdown that happened after
interacting with `<select>`. The root cause was an invisible Chromium Shell
window that overlapped Wezboard while still accepting AppKit mouse events. The
fix made TermSurf-managed Shell windows consistently mouse-transparent with
`ignoresMouseEvents=YES`.

That leaves several smaller but still user-visible popup bugs. They are
different enough that each should be isolated with its own experiment before any
fix is attempted.

## Remaining Bugs

### PagePopup remains visible after alt-tab

Date, time, date-time, and color popups can remain visible after the user
alt-tabs away from Wezboard. The owning TermSurf window is no longer active or
visible to the user, but the popup remains on screen.

This likely belongs to popup lifecycle, owner-window, app deactivation, or
window-ordering behavior. It should be investigated with logs around
`NSApplication` activation changes, Shell/Popup window visibility, PagePopup
close paths, and popup widget ownership.

### Select dropdown has the wrong x position

The `<select>` dropdown has the correct y position, but its x position is still
wrong. This path does not use Blink PagePopup. It goes through Chromium's AppKit
menu path:

```text
RenderFrameHostImpl::ShowPopupMenu
PopupMenuHelper::ShowPopupMenu
RenderWidgetHostNSViewBridge::DisplayPopupMenu
WebMenuRunner::runMenuInView
NSPopUpButtonCell
```

Chromium can log the select anchor before `NSPopUpButtonCell` takes over, but
AppKit owns the final menu placement. The next select experiment needs to
capture or infer the final x position and compare it against the anchor.

### Datalist does not work

Datalist could not be tested cleanly while the post-select shutdown was present.
Now that Issue 782 fixed the shutdown, datalist should get a fresh isolated run.
Its popup path may be different from both Blink PagePopup controls and AppKit
select menus.

### RenderWidgetPopupWindow cleanup is suspicious

Issue 782 traces repeatedly showed visible `RenderWidgetPopupWindow` entries at
level `101`, with `ignoresMouseEvents=false`, after popup interactions. These
windows did not cause the post-select shutdown once the main Shell window became
mouse-transparent, so they should not be treated as the next root cause by
default.

They should be revisited only if they explain one of the remaining symptoms,
especially PagePopup visibility after app deactivation.

## Approach

Handle one bug at a time. Do not bundle PagePopup deactivation, select x
placement, and datalist into a single fix.

The recommended order is:

1. PagePopup remains visible after alt-tab, because it affects every
   PagePopup-family control and may also explain the lingering
   `RenderWidgetPopupWindow` observations.
2. Select dropdown x position, because the y-axis and post-select shutdown are
   already fixed, leaving x placement as a clean AppKit-menu positioning bug.
3. Datalist behavior, because it needs a clean independent path trace now that
   native widgets no longer shut down after select.

If a trace proves that two remaining symptoms share one root cause, adjust the
order in the experiment result before designing the next experiment.

## Constraints

- Do not change the Issue 782 Shell-window mouse transparency fix unless a new
  trace proves it is wrong.
- Do not add runtime experiment flags.
- Keep using the existing trace gate for temporary diagnostic logs:
  `TERMSURF_ISSUE_779_TRACE=1`.
- If Chromium code changes are needed, create a new Issue 783 Chromium branch
  before editing Chromium, then register that branch in `chromium/README.md`.
- Design and implement one experiment at a time.

## Experiments

### Experiment 1: Clean trace surface for PagePopup alt-tab

#### Description

The next bug is PagePopup visibility after app/window deactivation. Date, time,
date-time, and color controls use Blink PagePopup widgets. After opening one of
these popups, alt-tabbing away from Wezboard can leave the native popup visible
on screen even though the owning TermSurf app is no longer active.

Before adding new logs, remove the broad diagnostics from Issue 782 that were
specific to the solved post-select shutdown. Those logs answered their question
and now make the next trace hard to read. This experiment should leave only a
small PagePopup lifecycle trace focused on activation, visibility, popup-window
ownership, and close paths.

This experiment is logs-only except for deleting obsolete trace lines. Do not
change popup behavior. Do not add any runtime experiment flag. Continue using
`TERMSURF_ISSUE_779_TRACE=1` as the single trace gate.

#### Changes

1. **Create the Issue 783 Chromium branch before editing Chromium.**

   Chromium changes for this issue must happen on a new branch:

   ```text
   148.0.7778.97-issue-783
   ```

   Branch it from the latest useful Issue 782 Chromium branch tip, then add the
   branch to `chromium/README.md`. Do not continue committing Issue 783 work to
   `148.0.7778.97-issue-782`.

2. **Remove obsolete Issue 782 shutdown logs.**

   Remove or disable trace lines that were only useful for the solved
   post-select shutdown:
   - Wezboard `NSApplication sendEvent:` and `NSWindow sendEvent:` swizzle logs;
   - Wezboard `appkit_view`, `window_event`, `mouse_event_impl`,
     `before_try_forward_mouse`, and `mouse_forward_boundary` click/move logs;
   - Roamium mouse forwarding boundary logs;
   - Chromium input-router, `RouteOrProcessMouseEvent`, `ProcessMouseEvent`, and
     `WebFrameWidgetImpl::HandleInputEvent` logs;
   - Blink `EventHandler`, `MouseEventManager`, and form-control default-handler
     logs added only to diagnose post-select click loss;
   - select-specific helper lifecycle logs that are unrelated to PagePopup
     deactivation.

   Keep the actual Issue 782 fix: TermSurf Shell windows must still set and
   reassert `ignoresMouseEvents=YES`.

3. **Keep only low-noise existing logs that help this bug.**

   Preserve or simplify logs for:
   - Shell window state when it helps correlate owner window visibility;
   - PagePopup open and close paths;
   - `RenderWidgetPopupWindow` creation, visibility, ordering, and destruction;
   - app/window activation notifications.

   The target trace should be readable without cursor-move or input-router
   floods.

4. **Add Wezboard app/window deactivation logs.**

   In Wezboard's macOS app/window layer, log only activation and visibility
   transitions relevant to alt-tab:
   - `NSApplicationDidResignActiveNotification`;
   - `NSApplicationDidBecomeActiveNotification`;
   - `NSWindowDidResignKeyNotification`;
   - `NSWindowDidBecomeKeyNotification`;
   - `NSWindowDidMiniaturizeNotification`, if applicable;
   - `NSWindowDidDeminiaturizeNotification`, if applicable;
   - any existing Wezboard hide/show or focus events that are already wired.

   Include:

   ```text
   app_is_active
   key_window / main_window pointer and class
   Wezboard window frame, is_key, is_main, is_visible
   first_responder class
   active pane id if cheap
   ```

   Use a concise summary line:

   ```text
   [issue-779-trace] pagepopup_alt_tab boundary=wezboard_activation event=...
   ```

5. **Add Chromium app/window deactivation logs.**

   In the Chromium/Roamium process, log app and Shell-window activation changes
   around alt-tab:
   - `NSApplicationDidResignActiveNotification`;
   - `NSApplicationDidBecomeActiveNotification`;
   - Shell `NSWindowDidResignKeyNotification`;
   - Shell `NSWindowDidBecomeKeyNotification`;
   - Shell hide/show/order changes if there is already a delegate or
     notification hook.

   Include:

   ```text
   app_is_active
   shell window pointer, frame, alpha, is_visible, is_key, is_main
   shell ignoresMouseEvents
   ordered Chromium windows top 5 with class, frame, level, visible, key/main
   ```

   Use:

   ```text
   [issue-779-trace] pagepopup_alt_tab boundary=chromium_activation event=...
   ```

6. **Add focused PagePopup lifecycle logs.**

   Keep or add logs at the PagePopup open/close boundaries:
   - `DateTimeChooserImpl::Open` / constructor before opening;
   - `DateTimeChooserImpl::CancelPopup`;
   - `DateTimeChooserImpl::EndChooser`;
   - `DateTimeChooserImpl::DidClosePopup`;
   - `WebViewImpl::OpenPagePopup`;
   - `WebViewImpl::CancelPagePopup`;
   - `WebViewImpl::ClosePagePopup`;
   - `WebViewImpl::CleanupPagePopup`;
   - `WebPagePopupImpl::ClosePopup`;
   - `WebContentsImpl::ShowCreatedWidget` for the popup widget.

   Include:

   ```text
   control type if known: date|time|datetime-local|color|unknown
   WebContents / frame / popup object pointer
   popup window or widget host view pointer
   popup rect and anchor rect
   popup visible/open state before and after close
   reason if close/cancel is triggered
   ```

   Use:

   ```text
   [issue-779-trace] pagepopup_alt_tab boundary=pagepopup_lifecycle event=...
   ```

7. **Add popup window state logs.**

   For `RenderWidgetPopupWindow` / popup `NSWindow` instances associated with
   PagePopup, log:
   - creation/init as popup;
   - `orderFront` / visibility changes if reachable;
   - `windowDidResignKey`, `windowDidBecomeKey`, and close notifications;
   - destruction/deallocation if a safe hook exists;
   - current frame, level, visible, key/main, parent/child relationship, and
     `ignoresMouseEvents`.

   Use:

   ```text
   [issue-779-trace] pagepopup_alt_tab boundary=popup_window event=...
   ```

8. **Do not add broad input tracing.**

   This experiment should not log every mouse event, input router step, Blink
   event handler, or select menu lifecycle. The only user interaction that
   matters is opening a PagePopup and then deactivating the app/window.

#### Verification

1. Confirm the Chromium branch:

   ```bash
   cd /Users/ryan/dev/termsurf/chromium/src
   git branch --show-current
   ```

   It must be:

   ```text
   148.0.7778.97-issue-783
   ```

2. Build through project scripts:

   ```bash
   cd /Users/ryan/dev/termsurf
   scripts/build.sh chromium
   scripts/build.sh roamium
   scripts/build.sh wezboard
   scripts/build.sh webtui
   ```

3. Start the native popup test page:

   ```bash
   cd /Users/ryan/dev/termsurf
   bun test-html/server.ts
   ```

4. Start Wezboard with fresh Experiment 1 logs:

   ```bash
   cd /Users/ryan/dev/termsurf
   mkdir -p logs/issue-783-exp1-state/termsurf

   TERMSURF_ISSUE_779_TRACE=1 \
   XDG_STATE_HOME="$PWD/logs/issue-783-exp1-state" \
   RUST_LOG=info \
   ./wezboard/target/debug/wezboard-gui \
   2>&1 | tee logs/issue-783-exp1-wezboard.log
   ```

5. Launch the TUI:

   ```bash
   /Users/ryan/dev/termsurf/webtui/target/debug/web \
     --browser /Users/ryan/dev/termsurf/chromium/src/out/Default/roamium \
     http://localhost:9616/test-native-popups.html
   ```

6. Run the minimum sequence:
   - click the date control and confirm the popup opens;
   - alt-tab to another app while the popup is still open;
   - observe whether the popup remains visible;
   - alt-tab back to Wezboard;
   - stop the run.

   If date does not reproduce the bug, repeat once with time, then once with
   color. Do not test select or datalist in this experiment.

7. Extract the focused trace:

   ```bash
   rg -a "\[issue-779-trace\]|pagepopup_alt_tab|DateTimeChooserImpl|WebViewImpl::.*PagePopup|WebPagePopupImpl|ShowCreatedWidget|RenderWidgetPopupWindow|NSApplicationDid|NSWindowDid|chromium_activation|wezboard_activation|popup_window|pagepopup_lifecycle" \
     logs/issue-783-exp1-wezboard.log \
     logs/issue-783-exp1-state/termsurf/webtui-trace.log \
     logs/issue-783-exp1-state/termsurf/roamium-trace.log \
     logs/issue-783-exp1-state/termsurf/chromium-server.log \
     > logs/issue-783-exp1-trace.log
   ```

8. Pass criteria:
   - the trace is quiet enough to read without input-router or cursor floods;
   - the popup open path names the PagePopup object and popup window/widget;
   - alt-tab produces Wezboard and Chromium activation/deactivation logs;
   - the trace shows whether PagePopup close/cancel cleanup fires on
     deactivation;
   - the trace shows whether the popup `NSWindow` remains visible after
     deactivation;
   - the result identifies the next fix boundary: Wezboard deactivation message,
     Chromium app activation hook, WebView/PagePopup cleanup, or popup window
     ownership.

9. Partial criteria:
   - obsolete logs are mostly removed, but one noisy subsystem remains;
   - the alt-tab bug reproduces and window state is visible, but one missing
     pointer prevents joining popup object to popup window;
   - the bug does not reproduce for date but does reproduce for time or color.

10. Fail criteria:
    - the trace is still dominated by Issue 782 shutdown logs;
    - the experiment changes popup behavior;
    - no app/window activation boundary is logged during alt-tab.
