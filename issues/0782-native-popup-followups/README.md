+++
status = "open"
opened = "2026-05-21"
+++

# Issue 782: Remaining native popup bugs

## Goal

Fix the native popup bugs that remain after Issue 779 without reopening that
large investigation. Each remaining bug should be isolated, logged, fixed, and
verified one at a time.

## Background

Issue 779 fixed the primary PagePopup y-axis placement bug for date-family
controls. Date, time, date-time, and color controls now appear at the correct y
position in the TermSurf webview overlay.

That work also showed that native widgets are not one unified system in
Chromium:

- date, time, date-time, and color controls use Blink PagePopup widgets;
- `<select>` uses Chromium's AppKit menu path;
- datalist still needs to be isolated because testing was blocked by later popup
  failures.

The remaining failures are separate enough that they should be handled in this
new issue instead of extending Issue 779.

## Remaining Bugs

### PagePopup remains visible after alt-tab

Date, time, date-time, and color popups can remain visible after the user
alt-tabs away from Wezboard. The owning TermSurf window is no longer visible or
active, but the native popup stays on screen.

This is likely a popup lifecycle, owner-window, focus, or deactivation issue. It
should be investigated with logs around window deactivation, PagePopup
visibility, popup widget ownership, and dismissal.

### Select dropdown has the wrong x position

The `<select>` dropdown now has the correct y position, but its x position is
wrong. This path is different from PagePopup:

```text
RenderFrameHostImpl::ShowPopupMenu
PopupMenuHelper::ShowPopupMenu
RenderWidgetHostNSViewBridge::DisplayPopupMenu
WebMenuRunner::runMenuInView
NSPopUpButtonCell
```

Issue 779 confirmed that Chromium logs the select anchor, but AppKit owns the
final menu placement after `NSPopUpButtonCell` takes over. The next select
experiment needs to capture or infer the final menu x position and compare it
against the anchor.

### Native widgets stop opening after select/datalist interactions

After interacting with `<select>` once or twice, native widgets stop opening for
the rest of the session. Later mouse movement still produces cursor updates, so
the browser is not dead, but new popup-open paths stop firing.

This may be an activation, focus, event dispatch, AppKit menu-tracking, or
popup-state cleanup bug. It should be treated independently from positioning
until logs identify where popup requests stop.

### Datalist does not work

Datalist could not be tested reliably because the post-select failure prevents
further native widgets from opening. Once the session-stopping popup failure is
understood, datalist should get its own clean trace and fix path.

## Approach

Do not try to fix every remaining bug in one experiment. Start with one bug,
design the smallest experiment that can identify its cause, record the result,
and only then move to the next bug.

The likely order is:

1. Post-select native-widget shutdown, because it contaminates multi-control
   test runs and can block every later popup experiment.
2. PagePopup alt-tab visibility, because it affects every PagePopup-family
   control that now has correct y placement.
3. Select dropdown x placement.
4. Datalist behavior.

The order may change if new logs show that two symptoms share one root cause.

## Experiments

### Experiment 1: Trace post-select popup shutdown

#### Description

After interacting with a `<select>` dropdown once or twice, later native widgets
stop opening for the rest of the session. Cursor updates still arrive, so the
browser process and basic mouse routing are alive. The missing signal is where a
later click stops:

- before Chromium receives mouse down/up;
- after Chromium receives input but before Blink activates the control;
- after Blink activation but before popup-open IPC;
- inside Chromium because popup/menu state still says a popup is active;
- inside AppKit because menu tracking or window activation did not unwind.

This experiment is logs-only. It must not change popup behavior. The goal is to
capture one clean sequence:

```text
open select -> close select -> click date -> click select again
```

If native widgets stop opening, the logs must identify the first missing
boundary in the second popup-open attempt.

#### Changes

1. **Create the Issue 782 Chromium branch.**

   This experiment modifies Chromium trace code, so it must use a new Chromium
   issue branch:

   ```text
   148.0.7778.97-issue-782
   ```

   Branch it from the tip of `148.0.7778.97-issue-779`, then register the new
   branch in `chromium/README.md`. Do not continue adding Chromium commits to
   the Issue 779 branch.

2. **Use the existing trace gate.**

   All new logs must be gated by the existing trace variable:

   ```text
   TERMSURF_ISSUE_779_TRACE=1
   ```

   Do not add a new behavior flag or a new issue-specific trace flag for this
   experiment.

3. **Keep the Issue 779 popup trace hooks.**

   Preserve existing logs for:
   - `RenderFrameHostImpl::ShowPopupMenu`;
   - `PopupMenuHelper::ShowPopupMenu`;
   - `RenderWidgetHostNSViewBridge::DisplayPopupMenu`;
   - `WebMenuRunner::runMenuInView`;
   - `DateTimeChooserImpl`;
   - `WebPagePopupImpl::SetWindowRect`;
   - `WebContentsImpl::ShowCreatedWidget`;
   - `RenderWidgetHostViewMac::InitAsPopup`.

4. **Define the popup sequence key.**

   Use one field name consistently:

   ```text
   popup_sequence=...
   ```

   In the browser process, `popup_sequence` is a process-local atomic counter.
   Increment it once per popup-open intent at the top browser entry:
   - `RenderFrameHostImpl::ShowPopupMenu` for `<select>` menus;
   - the browser-side PagePopup entry for date/time/color controls.

   Downstream browser/AppKit logs reuse that same value. Renderer-side logs do
   not need to share the counter; they join by timestamp plus frame/WebContents
   identity fields.

5. **Log select menu lifecycle cleanup.**

   In Chromium's select/AppKit path, add trace lines for:
   - menu open entry;
   - menu selection callback;
   - menu cancel/dismiss callback;
   - `PopupMenuHelper` close/destructor cleanup;
   - renderer/browser notification that the popup menu closed.

   Each line should include enough join fields to follow one select menu:

   ```text
   path=select
   popup_sequence=...
   webcontents=...
   rfh=...
   helper=...
   view/window pointer
   selected_or_cancelled=...
   helper_alive_before/after=...
   ```

   If Chromium already has a process-local popup/menu counter or helper pointer,
   log it. Do not add protocol fields.

6. **Log `popup_menu_helper_` state directly.**

   Specifically log `WebContentsViewMac::popup_menu_helper_.get()` at:
   - the top of `WebContentsViewMac::ShowPopupMenu`, before replacing or
     rejecting any helper;
   - the callback that should clear the helper when the menu closes;
   - `PopupMenuHelper::CloseMenu` and the `PopupMenuHelper` destructor.

   These logs must answer whether an old helper remains alive after the select
   menu closes.

7. **Log the top of the popup-open decision points.**

   Add trace lines before any early return or suppression in:
   - `RenderFrameHostImpl::ShowPopupMenu`;
   - the PagePopup open path used by date/time/color;
   - any known "popup already active" or "suppress popup" guard near those
     paths.

   These logs must answer whether the failed post-select click reaches the
   popup-open functions and, if it does, why the open is rejected.

8. **Log renderer-side popup intent and active state.**

   Add renderer-side trace lines for:
   - PagePopup open intent and close in `WebPagePopupImpl` / `WebViewImpl`;
   - date/time/color open intent before crossing to the browser;
   - select popup intent in the Blink select path;
   - any renderer-side "popup already open" state checked before sending popup
     IPC.

   These logs are required because the failure may happen before the browser
   receives a popup-open request. If all browser-side popup logs are silent, the
   renderer trace must still say whether Blink activated the control and whether
   it suppressed the popup.

9. **Log PagePopup close cleanup.**

   Mirror the close-path logging for PagePopup controls:
   - `WebPagePopupImpl::ClosePopup`;
   - the `WebViewImpl` popup-open/closed state used by PagePopup;
   - browser-side PagePopup widget destruction or close notification if present.

   The trigger appears to be `<select>`, but the symptom affects date/time/color
   too, so both popup families need close-state visibility.

10. **Log mouse click delivery after select closes.**

    Add trace lines for the macOS `RenderWidgetHostViewMac` input path that sees
    mouse down/up events for the main webview after the select menu closes.

    Include:

    ```text
    event type
    location in window/view
    target RenderWidgetHostViewMac pointer
    webcontents pointer if available
    window isKey/isMain/isVisible
    firstResponder class if cheap to log
    ```

    The purpose is not to trace every cursor move. Log clicks only, or keep move
    logs out of the experiment trace, so the result is readable.

11. **Log window activation state around AppKit menu tracking.**

    In the AppKit select menu path, log before opening the menu, after it
    returns, and again at the next attempted popup/click boundary:

    ```text
    window isKey/isMain/isVisible
    app isActive
    firstResponder
    currentEvent type
    ```

    If AppKit leaves the hidden/transparent Chromium shell window in a different
    activation state after the select menu closes, this should make it visible.

12. **Add one concise summary line for each attempted popup.**

    Emit a summary line at each attempted popup boundary:

    ```text
    native_popup_attempt
      attempt=N
      control=select|date|unknown
      boundary=input|blink|browser-popup-open|appkit-open|cleanup
      outcome=entered|opened|closed|cancelled|suppressed|missing
      reason=...
    ```

    The `boundary` field is a fixed enum for this experiment:
    - `input`
    - `blink`
    - `browser-popup-open`
    - `appkit-open`
    - `cleanup`

    Do not invent new boundary values without updating this experiment text.

    The summary does not need to be perfect automation. It only needs to make
    the trace easy to scan and compare with the detailed lines.

#### Verification

0. Confirm the Chromium branch:

   ```bash
   cd /Users/ryan/dev/termsurf/chromium/src
   git branch --show-current
   ```

   The branch must be:

   ```text
   148.0.7778.97-issue-782
   ```

1. Build through the project scripts:

   ```bash
   cd /Users/ryan/dev/termsurf
   scripts/build.sh chromium
   scripts/build.sh roamium
   scripts/build.sh webtui --release
   scripts/build.sh wezboard
   ```

2. Start the test page server:

   ```bash
   cd /Users/ryan/dev/termsurf
   bun test-html/server.ts
   ```

3. Run a trace-off baseline.

   Start Wezboard once without `TERMSURF_ISSUE_779_TRACE`, confirm the test page
   is usable, and confirm no `[issue-779-trace]` lines are emitted. Stop that
   run before starting the traced run.

4. Start Wezboard with deterministic logs:

   ```bash
   cd /Users/ryan/dev/termsurf
   mkdir -p logs/issue-782-exp1-state/termsurf

   TERMSURF_ISSUE_779_TRACE=1 \
   XDG_STATE_HOME="$PWD/logs/issue-782-exp1-state" \
   RUST_LOG=info \
   ./wezboard/target/debug/wezboard-gui \
   2>&1 | tee logs/issue-782-exp1-wezboard.log
   ```

5. Launch the TUI:

   ```bash
   /Users/ryan/dev/termsurf/webtui/target/release/web \
     --browser /Users/ryan/dev/termsurf/chromium/src/out/Default/roamium \
     http://localhost:9616/test-native-popups.html
   ```

6. Run one controlled interaction sequence:
   - click the date control and confirm it opens;
   - close it;
   - click the `<select>` dropdown and choose or dismiss one item;
   - click the date control again;
   - click the `<select>` dropdown again;
   - if widgets stop opening, stop the test immediately and preserve the logs.

7. Extract the trace:

   ```bash
   rg -a "\[issue-779-trace\]|native_popup_attempt|popup_sequence|popup_menu_helper_|ShowPopupMenu|PopupMenuHelper|DisplayPopupMenu|WebMenuRunner|DateTimeChooserImpl|WebPagePopupImpl|WebViewImpl|ShowCreatedWidget|InitAsPopup|mouse.*down|mouse.*up|firstResponder|isKey|isMain|app isActive|menu.*close|menu.*cancel|menu.*dismiss|popup.*active|popup.*suppress" \
     logs/issue-782-exp1-wezboard.log \
     logs/issue-782-exp1-state/termsurf/webtui-trace.log \
     logs/issue-782-exp1-state/termsurf/roamium-trace.log \
     logs/issue-782-exp1-state/termsurf/chromium-server.log \
     > logs/issue-782-exp1-trace.log
   ```

8. Pass criteria:
   - the first date click shows the normal PagePopup open chain;
   - the select click shows the full select menu open and cleanup chain;
   - after select closes, the next failed click shows exactly where the chain
     stops: no mouse click delivered, no Blink activation, popup-open
     suppressed, AppKit/menu state stuck, or another concrete boundary;
   - if browser-side popup-open logs are silent, renderer-side logs explain
     whether Blink received the activation and whether it suppressed the popup;
   - select cleanup logs show whether `popup_menu_helper_` was cleared;
   - logs are quiet enough to read without cursor-move floods.

9. Partial criteria:
   - the failure reproduces and the trace narrows the cause to a subsystem, but
     another experiment is needed to identify the exact function or state flag;
   - the failure does not reproduce, but the trace proves repeated date/select
     interactions can work in a clean run;
   - if mouse down/up stops reaching Chromium after `<select>` closes, run a
     follow-up experiment with Wezboard's mouse-forwarding path logged to
     distinguish AppKit input absorption from missing GUI forwarding.

10. Fail criteria:
    - the logs still only show cursor movement after the failure;
    - the trace cannot distinguish input delivery, Blink activation, Chromium
      popup suppression, and AppKit menu cleanup;
    - the experiment changes popup behavior instead of only adding logs.

**Result:** Partial

The failure reproduced. After opening the `<select>` menu once and clicking away
to dismiss it, subsequent native widgets stopped opening for the duration of the
session.

The trace was useful. Before the select interaction, the date/PagePopup path
opened and cleaned up normally:

- `DateTimeChooserImpl::ctor.before_open`
- `WebViewImpl::OpenPagePopup`
- `WebContentsImpl::ShowCreatedWidget`
- `RenderWidgetHostViewMac::InitAsPopup`
- `WebViewImpl::CleanupPagePopup.after page_popup_after=0`

The select path also opened and cleaned up normally:

- `MenuListSelectType::ShowPopup`
- `ExternalPopupMenu::ShowInternal`
- `RenderFrameHostImpl::ShowPopupMenu`
- `WebContentsViewMac::ShowPopupMenu`
- `PopupMenuHelper::PopupMenuClosed`
- `WebContentsViewMac::OnMenuClosed.after popup_menu_helper_after_reset=0`
- `SetNativePopupIsVisible native_popup_is_visible_after=0`

This rules out the simplest browser-side cleanup theory: `popup_menu_helper_`
does not remain alive after the select menu closes. Blink also clears the
select's native popup visible state.

After the select menu closes, mouse clicks still reach Chromium:

```text
RouteOrProcessMouseEvent event_type=MouseDown ... has_webcontents=1 should_route=1
RouteOrProcessMouseEvent event_type=MouseUp ... has_webcontents=1 should_route=1
```

But those post-select clicks no longer produce any native-control activation
logs:

- no `DateTimeChooserImpl::ctor.before_open`
- no `MenuListSelectType::ShowPopup`
- no `ExternalPopupMenu::ShowInternal`
- no `RenderFrameHostImpl::ShowPopupMenu`
- no `WebContentsImpl::ShowCreatedWidget`

The chain therefore stops after Chromium receives the mouse event, but before
Blink's form-control popup activation paths run. The bug is not missing input
delivery, not a stuck `popup_menu_helper_`, and not a browser-side popup-open
suppression. It is somewhere in the renderer-side hit-test/focus/activation path
after an AppKit select-menu dismissal.

One trace hygiene issue surfaced: the `display_popup_menu_sent` log in
`PopupMenuHelper::ShowPopupMenu` runs after `DisplayPopupMenu`, even though the
helper may already have been deleted reentrantly by the menu-close callback.
That log must be removed or moved before committing the trace patch. It does not
change the diagnostic conclusion.

#### Conclusion

Experiment 1 narrows the post-select shutdown bug to the renderer activation
boundary. The next experiment should trace the delivered mouse event through
Blink hit-testing, focus handling, and form-control activation to find why a
click that reaches Chromium no longer invokes `DateTimeChooserImpl`,
`MenuListSelectType::ShowPopup`, or related native popup entry points after the
select menu is dismissed.

### Experiment 2: Trace Blink activation after select dismissal

Experiment 1 proved that clicks still reach Chromium after the `<select>` menu
is dismissed, but Blink no longer reaches the native popup entry points. This
experiment traces the same click inside Blink: event entry, hit testing,
dispatch/default action, focus state, user activation, and form-control
activation gates.

The leading hypothesis is a modal-loop input-state imbalance: the AppKit
`NSMenu` used for `<select>` may consume the mouse-up or dismissal event while
Blink still believes the original select-opening press is active. If Blink's
`EventHandler` remains stuck with `mouse_pressed_ = true` and
`mouse_down_node_ = <select>`, later clicks can reach Chromium but fail to start
a fresh click sequence or synthesize a normal `click` event for form controls.

This is still a logs-only experiment. Do not change popup behavior. Use the
existing `TERMSURF_ISSUE_779_TRACE=1` trace gate and the existing
`148.0.7778.97-issue-782` Chromium branch.

#### Changes

1. **Clean up the Experiment 1 unsafe trace line first.**

   In `content/browser/renderer_host/popup_menu_helper_mac.mm`, remove or move
   the `display_popup_menu_sent` log that currently runs after
   `DisplayPopupMenu`. The helper can be destroyed reentrantly by the callback,
   so no log may read `this` after `DisplayPopupMenu` returns. If this line is
   still useful, log the intended values before the call instead.

2. **Trace renderer mouse event entry.**

   In Blink's widget input entry path, add click-only trace lines for
   `MouseDown`, `MouseUp`, and `MouseMove` only if movement is explicitly needed
   for hit-test state. Prefer no move logs. Candidate hooks:
   - `third_party/blink/renderer/core/frame/web_frame_widget_impl.cc`
   - `third_party/blink/renderer/platform/widget/widget_base.cc`

   Log:
   - event type;
   - event position in widget/root-frame coordinates;
   - `WebFrameWidgetImpl*`;
   - local root frame pointer;
   - focused frame pointer;
   - `Page` focus/active state if available;
   - currently focused element summary.

3. **Trace Blink mouse press/release state and hit testing.**

   In the renderer event handling path, make `EventHandler` the primary trace
   target. Candidate hook:
   - `third_party/blink/renderer/core/input/event_handler.cc`

   Add logs at `LocalFrame::EventHandler::HandleMousePressEvent` entry and after
   it updates mouse state. Log:
   - event type;
   - `mouse_pressed_` before and after;
   - `mouse_down_node_` before and after;
   - hit-test result for the new press;
   - whether this is a fresh press or a press while Blink already thinks a press
     is active.

   Add logs at `LocalFrame::EventHandler::HandleMouseReleaseEvent` entry and
   around the click-synthesis decision. Log:
   - `mouse_pressed_` at release entry;
   - `mouse_down_node_`;
   - release hit-test target;
   - whether the release target matches the press target;
   - whether Blink synthesizes or dispatches a `click`;
   - the exact reason if click synthesis is skipped.

   The smoking-gun condition is a post-select fresh user click entering
   `HandleMousePressEvent` with `mouse_pressed_ = true` and `mouse_down_node_`
   still pointing at the dismissed `<select>`.

4. **Trace Blink hit testing for the click target.**

   Around the same mouse down/up hit-test paths, add a summary line for the
   target. Log:
   - event type;
   - hit node pointer;
   - hit element pointer;
   - element tag name;
   - element id;
   - input type for `HTMLInputElement`;
   - whether the element is disabled/read-only;
   - layout object pointer/type;
   - document/frame pointer;
   - local/root coordinates;
   - whether the target is inside the expected native popup test page.

5. **Trace DOM event dispatch and default action.**

   Add logs at the boundary where Blink dispatches mouse/pointer/click events
   and decides whether to run a default action. Candidate hooks:
   - `third_party/blink/renderer/core/input/event_handler.cc`
   - `third_party/blink/renderer/core/events/event_dispatcher.cc`
   - `third_party/blink/renderer/core/dom/events/event_target.cc`

   Restrict these logs to events targeting `HTMLInputElement`,
   `HTMLSelectElement`, and `HTMLOptionElement`. Log for `pointerdown`,
   `mousedown`, `mouseup`, and `click`:
   - target element summary;
   - whether default was prevented;
   - whether the event is trusted;
   - whether the event has user gesture/user activation;
   - whether the event dispatch completed normally;
   - whether a default action is queued or skipped.

   If `HandleMousePressEvent` and `HandleMouseReleaseEvent` run but no `click`
   event reaches the form control after select dismissal, the bug is in mouse to
   click synthesis. If `click` reaches the form control but no popup opens, the
   bug is in the form-control default handler.

6. **Trace form-control activation gates and default handlers.**

   Add entry and suppression logs immediately before the native popup entry
   points for:
   - date/time/datetime/color inputs before `DateTimeChooserImpl` or equivalent
     picker creation;
   - `<select>` before `MenuListSelectType::ShowPopup`;
   - datalist suggestion opening if the hook is easy to locate.

   Also log entry into:
   - `HTMLInputElement::DefaultEventHandler`;
   - `HTMLSelectElement::DefaultEventHandler`;
   - any nearby helper that turns `click`/DOM activation into a picker open.

   Log:
   - event type;
   - element pointer/tag/id/type;
   - disabled/read-only state;
   - focused/active state;
   - user activation state;
   - popup/menu already-open state;
   - exact reason for any early return.

7. **Trace focus changes around the post-select click.**

   Add logs in the renderer focus path for focus changes caused by mouse
   activation. Candidate hooks:
   - `third_party/blink/renderer/core/page/focus_controller.cc`
   - `third_party/blink/renderer/core/dom/document.cc`
   - `third_party/blink/renderer/core/html/forms/html_form_control_element.cc`

   Log:
   - old focused element summary;
   - new focused element summary;
   - frame/document;
   - whether focus was refused or redirected;
   - whether the page/frame is active.

8. **Do not implement a synthetic mouse-up yet.**

   A likely future fix, if the stuck-press hypothesis is confirmed, is to
   balance Blink state when the AppKit select menu closes, potentially by
   sending or simulating the missing mouse-up at `PopupMenuHelper` /
   `WebMenuRunner` close time. Do not implement that in this experiment. This
   experiment should only log enough to prove whether that fix is needed.

   It is fine to grep Chromium for existing platform precedent such as
   `SyntheticMouseEvent`, `mouse up after menu`, `popup_was_hidden_`, or
   menu-close balancing logic, but do not change behavior yet.

9. **Use explicit comparison labels.**

   Every new summary line should include a comparison phase label when possible:

   ```text
   phase=pre_select_success
   phase=select_open_close
   phase=post_select_failure
   ```

   If the code cannot know the phase directly, include enough join fields to
   infer it from time order:
   - monotonic timestamp if available;
   - event type;
   - target element id/type;
   - widget/frame/document pointer.

10. **Keep logs readable.**

    Do not log cursor movement floods. Do not log every DOM event on the page.
    Restrict output to mouse/pointer/click/focus/default-action events and only
    when `TERMSURF_ISSUE_779_TRACE=1` is set.

#### Verification

1. Build through the project scripts:

   ```bash
   cd /Users/ryan/dev/termsurf
   scripts/build.sh chromium
   scripts/build.sh roamium
   scripts/build.sh webtui
   scripts/build.sh wezboard
   ```

2. Start the test page server:

   ```bash
   cd /Users/ryan/dev/termsurf
   bun test-html/server.ts
   ```

3. Start Wezboard with fresh logs:

   ```bash
   cd /Users/ryan/dev/termsurf
   mkdir -p logs/issue-782-exp2-state/termsurf

   TERMSURF_ISSUE_779_TRACE=1 \
   XDG_STATE_HOME="$PWD/logs/issue-782-exp2-state" \
   RUST_LOG=info \
   ./wezboard/target/debug/wezboard-gui \
   2>&1 | tee logs/issue-782-exp2-wezboard.log
   ```

4. Launch the TUI:

   ```bash
   /Users/ryan/dev/termsurf/webtui/target/debug/web \
     --browser /Users/ryan/dev/termsurf/chromium/src/out/Default/roamium \
     http://localhost:9616/test-native-popups.html
   ```

5. Run the minimum comparison sequence:
   - click the date control and confirm it opens;
   - close it;
   - click the `<select>` dropdown and dismiss it by clicking outside;
   - click the same date control again;
   - stop immediately after the failed post-select date click.

6. Extract the trace:

   ```bash
   rg -a "\[issue-779-trace\]|native_popup_attempt|blink_mouse|HandleMousePressEvent|HandleMouseReleaseEvent|mouse_pressed|mouse_down_node|hit_test|dispatch|default_action|DefaultEventHandler|focus|activation|DateTimeChooserImpl|MenuListSelectType|ExternalPopupMenu|ShowPopup|OpenPagePopup|ShowCreatedWidget|InitAsPopup|RouteOrProcessMouseEvent" \
     logs/issue-782-exp2-wezboard.log \
     logs/issue-782-exp2-state/termsurf/webtui-trace.log \
     logs/issue-782-exp2-state/termsurf/roamium-trace.log \
     logs/issue-782-exp2-state/termsurf/chromium-server.log \
     > logs/issue-782-exp2-trace.log
   ```

7. Pass criteria:
   - the trace includes one successful pre-select date click and one failed
     post-select date click;
   - both clicks have renderer mouse-event entry logs;
   - both clicks have hit-test result logs;
   - the trace identifies the first divergence between success and failure:
     wrong hit-test target, missing click dispatch, default action prevented,
     focus/active-state refusal, user activation missing, or form-control
     activation suppression;
   - specifically, the trace proves or disproves the stuck-press hypothesis by
     showing `mouse_pressed_` and `mouse_down_node_` before and after the
     select-dismissal boundary;
   - log volume remains small enough to compare the two clicks by hand.

8. Partial criteria:
   - the trace narrows the failure to one renderer subsystem but not one exact
     branch;
   - the failure does not reproduce, but the trace proves successful repeated
     date/select/date activation with all expected renderer boundaries.

9. Fail criteria:
   - the trace again only proves that Chromium receives mouse events;
   - the trace cannot compare hit test, dispatch, focus, default action, and
     form-control activation between the successful and failed clicks;
   - the experiment changes popup behavior instead of only adding logs.
