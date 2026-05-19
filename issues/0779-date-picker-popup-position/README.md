+++
status = "open"
opened = "2026-04-15"
+++

# Issue 779: Native popups (date picker, select dropdown) render outside webview overlay

## Goal

Native/popup UI elements spawned by the webview — date pickers, `<select>`
dropdowns, and any other OS-level popup Chromium creates — should appear over
the webview where the user clicked, not detached from it in an unrelated screen
region.

## Background

While testing an app with a DaisyUI date input, clicking the date field causes
the picker to pop up in the wrong location. When the webview overlay is
positioned on the right side of the terminal window (e.g., a right split), the
date picker appears on the left — entirely outside the webview's bounds.

The same bug happens with **native `<select>` dropdown boxes**: clicking a
dropdown opens the menu at a detached screen position that doesn't match the
`<select>` element the user clicked. This confirms the problem is not specific
to date pickers — it affects every kind of native popup window Chromium creates.

This is surprising because the webview is composited into the terminal via
CALayerHost (zero-copy GPU texture sharing from Roamium's Chromium process into
Wezboard). Content rendered into that texture is necessarily clipped to the
overlay's rect. The fact that the picker renders outside the overlay strongly
implies it is **not** drawn into the webview's GPU texture at all — it must be a
separate OS-level window (a popup/child window) that Chromium positions using
screen coordinates it computed against its own internal notion of where the
webview lives, which does not match where Wezboard actually composites the
CALayerHost overlay.

In other words: Chromium thinks the webview is at screen coordinates (X, Y), but
Wezboard is actually displaying the layer at (X', Y'). Any popup window Chromium
spawns (date pickers, select dropdowns, autofill, color pickers, context menus
rendered as native windows, etc.) will be placed at the wrong absolute screen
position.

## Analysis

Possible root causes to investigate:

1. **Chromium's view bounds are stale or wrong.** The embedding API
   (`libtermsurf_chromium`) needs to tell Chromium the webview's real on-screen
   rect whenever the overlay moves or resizes, so that popup-positioning code
   inside Chromium uses the correct origin. If we're only updating the
   CALayerHost frame and not informing Chromium's `RenderWidgetHostView` of its
   new screen position, popups will anchor to stale coordinates (often 0,0 or
   the window origin).

2. **Popup windows are separate OS windows.** Chromium typically renders
   `<select>` dropdowns, date pickers, and autofill as platform popup widgets on
   macOS. These are real `NSWindow`s (or `NSPanel`s) positioned in screen
   coordinates. If Chromium's host view reports the wrong screen origin, the
   popup opens in the wrong place.

3. **Coordinate space mismatch.** Wezboard positions the overlay in its own
   window-local coordinates and converts to screen coordinates for CALayerHost.
   Roamium/Chromium may be using a different origin (top-left vs bottom-left, or
   main-screen vs window-local) when computing popup placement.

## Proposed Solutions

- Audit what view/window bounds Roamium reports into Chromium when the GUI sends
  `OverlayReposition` / `OverlayResize` protocol messages. Ensure the
  `RenderWidgetHostView`'s screen rect is updated, not just the compositor layer
  size.
- Add a protocol field (or reuse existing reposition messages) to carry the
  webview's **absolute screen rect**, not just a window-local rect, so Chromium
  can position popups correctly.
- Verify on a right-split pane: open a DaisyUI date input, confirm the picker
  opens aligned to the input field within the overlay.
- Check other popup-style UI while we're here: `<select>` dropdowns, autofill
  suggestions, context menus, color pickers, file chooser anchors.

## Reproduction

### Date picker

1. Build and run Wezboard with a right split hosting a webview.
2. Load a page with a DaisyUI date input (or any `<input type="date">`).
3. Click the date field.
4. Observe: picker appears on the left side of the window, outside the webview
   overlay's bounds.

### Native `<select>` dropdown

1. Build and run Wezboard with a right split hosting a webview.
2. Load a page with a native `<select>` element.
3. Click the dropdown.
4. Observe: the dropdown menu appears at a detached location, not anchored to
   the `<select>` element the user clicked.

## Experiments

### Experiment 1: Add a Native Popup Reproduction Page

#### Description

Create a focused reproduction page in `test-html` for native popup positioning.
Before changing Chromium, Roamium, or the TermSurf protocol, we need a stable
local page that demonstrates the bug using plain browser-native controls.

`test-html` is the right place because it already hosts manual browser behavior
checks at `http://localhost:9616`, including pages for mouse input, dialogs,
file upload/download, storage, and page zoom. This issue should get the same
kind of small deterministic test page.

The page should make the coordinate bug obvious without implying that the
control's position inside the HTML document is the root cause. The bug is about
the webview overlay's position on the screen: Chromium appears to place native
popup windows using a screen origin that does not match where Wezboard
composites the CALayerHost. The same control should behave differently when the
webview pane is moved to a different split position on the screen.

The page should therefore keep the controls simple and ordinary, and draw clear
visual bounds so the tester can tell whether a popup is attached to the webview
or appears detached elsewhere on the screen.

#### Changes

1. **Add `test-html/public/test-native-popups.html`.**

   The page should include:
   - `<input type="date">`
   - `<input type="time">`
   - `<input type="datetime-local">`
   - `<input type="color">`
   - a native single `<select>`
   - an `<input list="...">` with a `<datalist>` for suggestion-popup behavior

   Use plain HTML, CSS, and minimal JavaScript. No framework dependency is
   needed.

2. **Make the bug visually measurable.**

   The page should:
   - draw an obvious bordered test area representing the visible webview
     content;
   - label each control with the expected behavior: the native popup should
     appear anchored to that control, not far away elsewhere on the screen;
   - show the last focused/clicked control in an on-page log so screenshots make
     clear which element triggered the popup.

3. **Link the page from `test-html/public/index.html`.**

   Add a link under the existing input/browser-behavior section, for example:
   `Native Popups — date picker, select dropdown, color picker, datalist`.

4. **Do not fix the underlying bug in this experiment.**

   This experiment is only for reproduction. It should not modify Wezboard,
   Roamium, Chromium, protobufs, or overlay coordinate code.

#### Verification

1. Start the test server:

   ```bash
   bun test-html/server.ts
   ```

2. Open Wezboard and create a split layout where the browser pane is on the
   right side of the terminal window.

3. Open the reproduction page:

   ```bash
   web http://localhost:9616/test-native-popups.html
   ```

4. Click the native popup controls in the right-side browser pane:
   - date input;
   - select dropdown;
   - color input;
   - datalist input.

5. Confirm the issue is reproducible:
   - the clicked control is visibly inside the webview overlay;
   - the native popup appears detached from that control, outside the webview
     overlay, or at an obviously wrong screen coordinate;
   - the page log identifies which control was clicked.

6. Repeat with the browser pane on the left or as the only pane. The bug may be
   less visible there, but this comparison helps show that the bad offset is
   tied to the webview overlay's screen position, not the control's position
   inside the page.

**Result:** Pass

Added `test-html/public/test-native-popups.html`, a plain HTML reproduction page
for native popup positioning. The page includes native date, time,
datetime-local, color, select, and datalist controls; draws a visible webview
content boundary; and logs the last focused or clicked control so screenshots
can identify which native popup was triggered.

Added the page to `test-html/public/index.html` under Input.

Static serving verification passed using a temporary local server for
`test-html/public`:

```bash
curl -I http://localhost:9617/test-native-popups.html
curl -s http://localhost:9617/
curl -s http://localhost:9617/test-native-popups.html
```

Manual Wezboard reproduction of the native-popup mispositioning remains for the
next experiment.

#### Conclusion

Issue 779 now has a local, deterministic reproduction page. Future experiments
can use `http://localhost:9616/test-native-popups.html` to compare native popup
placement as the webview overlay moves between split positions on screen.

### Experiment 2: Send and Apply Webview Screen Bounds

#### Description

Fix native popup positioning by teaching Chromium the webview's real screen
rect. If the fix is incomplete, leave enough logs in the result to identify
which coordinate system is still wrong.

Experiment 1 proved the bug is tied to the webview overlay's screen position,
not to the HTML control's position inside the page. The current protocol only
sends webview size to Roamium/Chromium. Wezboard separately moves the
CALayerHost to the correct terminal-pane location, but Chromium's native view
still appears to believe it lives somewhere else. Native popup windows use that
Chromium-side screen rect, so they open far away from the webview.

This experiment should make the smallest end-to-end change that can plausibly
fix the issue:

1. compute the overlay's absolute screen rect in Wezboard;
2. send that rect to Roamium with resize/update messages;
3. pass the rect through Roamium's FFI to `libtermsurf_chromium`;
4. update Chromium's WebContents / RenderWidgetHostView host bounds from that
   rect;
5. log both the Wezboard rect and the Chromium rect so a failed attempt reveals
   the remaining mismatch.

#### Changes

1. **Extend the TermSurf resize message with screen bounds.**

   In `proto/termsurf.proto`, add optional-compatible fields to `Resize`:
   - `double screen_x`
   - `double screen_y`
   - `double screen_width`
   - `double screen_height`
   - `double screen_scale`

   These should represent the webview overlay rect in Chromium-style screen DIP:
   top-left origin, device-independent points, not terminal cells and not
   backing pixels. Keep the existing `pixel_width` and `pixel_height` fields
   unchanged for content size.

   Regenerate Rust protobuf bindings using the repo's existing protobuf build
   path.

2. **Compute the overlay screen rect in Wezboard.**

   In `wezboard/wezboard-gui/src/termsurf/conn.rs`, where `set_overlay_frame`
   already receives the CA layer frame in backing pixels, compute the matching
   screen-space rect for the webview:
   - convert the backing-pixel overlay frame into the overlay NSView's logical
     coordinate system using the same scale logic already used by
     `set_overlay_frame`;
   - convert that rect from the overlay NSView/window coordinate system to
     screen coordinates using Cocoa APIs, not hand-rolled origin math;
   - convert the Cocoa/AppKit screen rect into the protocol's Chromium-style
     top-left screen DIP convention before sending;
   - record the resulting `screen_x`, `screen_y`, `screen_width`,
     `screen_height`, and `screen_scale` on the TermSurf pane state.

   Add targeted debug logs showing:
   - pane id;
   - backing frame;
   - logical/view frame;
   - Cocoa/AppKit screen rect;
   - protocol screen rect;
   - scale/dpi used.

3. **Send screen bounds to Roamium on resize/update.**

   In `wezboard/wezboard-gui/src/termsurf/conn.rs`, when sending `Resize` to the
   browser process, include the latest screen bounds stored for that pane. This
   applies to:
   - existing-pane `SetOverlay` resize path;
   - the frame update path after `set_overlay_frame` changes the overlay
     position.

   Do not send a `Resize` on every paint. Store the last screen-bounds message
   sent for each pane and only send when content size or screen bounds changed
   beyond a small tolerance, for example 0.5 DIP for position/size and exact
   change for pixel dimensions.

   If the tab id is not known yet, store the bounds and send them as soon as the
   tab id becomes available.

4. **Pass bounds through Roamium FFI.**

   In `roamium/src/dispatch.rs` and `roamium/src/ffi.rs`, extend the resize path
   so `Resize` calls a new or expanded FFI function with both size and screen
   bounds, for example:
   `ts_set_view_bounds(handle, pixel_width, pixel_height, screen_x, screen_y, screen_width, screen_height, screen_scale)`.

   Keep the old size-only behavior available as a fallback if bounds are zero.

5. **Apply bounds inside Chromium.**

   In the Chromium fork, create a new issue branch for Issue 779 following the
   Chromium branch policy.

   Add the `libtermsurf_chromium` API needed by Roamium. In the implementation,
   update the WebContents view / RenderWidgetHostView host so Chromium's screen
   rect matches the rect sent by Wezboard.

   The first candidate implementation should update the per-tab WebContents /
   RenderWidgetHostView native view bounds, not move the shared host `NSWindow`.
   Moving the whole host window can break multiple tabs/webviews in the same
   Roamium process. If the native view frame is relative to its host `NSWindow`,
   normalize the incoming absolute screen rect by subtracting the host window's
   screen origin before calling `setFrame:` or the equivalent Chromium view
   bounds API.

   If updating the per-tab native view does not affect popup placement, keep the
   logs and record that result. The next candidate would be a more direct
   RenderWidgetHostViewMac screen-coordinate override, such as the path used by
   popup positioning / `GetBoundsInRootWindow` / screen-info conversion.

   Add Chromium-side logs showing, for each resize/bounds update:
   - incoming screen rect;
   - host window screen rect/origin;
   - normalized native-view frame;
   - WebContents native view bounds;
   - RenderWidgetHostView bounds;
   - any available window/screen bounds returned by Chromium after the update.

6. **Keep diagnostics if the fix fails.**

   If native popups still open in the wrong place, do not remove the logs before
   recording the result. The result should include enough log excerpts to answer
   which of these is true:
   - Wezboard computed the wrong screen rect;
   - Wezboard computed the right rect but sent the wrong values;
   - Roamium received the right values but passed wrong values through FFI;
   - Chromium received the right values but updated the wrong view/window;
   - Chromium reports the right view bounds but native popups use another
     coordinate path.

#### Verification

1. Build all affected components:

   ```bash
   scripts/build.sh wezboard
   scripts/build.sh roamium
   scripts/build.sh chromium
   ```

2. Start `test-html`:

   ```bash
   bun test-html/server.ts
   ```

3. Open the reproduction page in a browser pane:

   ```bash
   web http://localhost:9616/test-native-popups.html
   ```

4. Put the browser pane in at least three screen positions:
   - only pane;
   - left split;
   - right split or top-right split.

5. In each position, click:
   - date input;
   - select dropdown;
   - datalist input.

   Also click the color input and record what happens, but do not treat color
   picker anchoring as required for this experiment. On macOS Chromium may use
   the global `NSColorPanel`, which is not necessarily anchored to the webview.

6. Pass criteria:
   - each native popup opens anchored to the clicked control;
   - moving the webview pane to another split position does not detach the
     popup;
   - Wezboard logs and Chromium logs agree on the webview screen rect within 1
     DIP.
   - continuous resize or pane movement for several seconds produces a small
     finite number of screen-bounds resize sends, not one send per paint frame.

7. Fail criteria:
   - any native popup still opens far outside the webview;
   - logs do not clearly show where the coordinate mismatch occurs.

8. If the result is Fail or Partial, include the relevant Wezboard, Roamium, and
   Chromium log excerpts in the result before designing the next experiment.

**Result:** Fail

Implemented the screen-bounds path through the stack:

- `Resize` now carries absolute webview bounds in Chromium-style screen DIPs
  (`screen_x`, `screen_y`, `screen_width`, `screen_height`, `screen_scale`).
- Wezboard computes the overlay's Cocoa screen rect from the CALayerHost view,
  converts it to top-left DIP coordinates, stores it on the pane, and sends
  throttled resize messages only when size or screen bounds change.
- Roamium forwards bounded resize messages through the new `ts_set_view_bounds`
  FFI call, falling back to `ts_set_view_size` when bounds are unavailable.
- Chromium branch `148.0.7778.97-issue-779` adds `ts_set_view_bounds` and
  applies the incoming screen rect to the per-tab `RenderWidgetHostView` while
  preserving the existing content-size and compositor resize path.

Build verification passed:

```bash
scripts/build.sh wezboard
scripts/build.sh roamium
scripts/build.sh chromium
```

Manual verification failed. Running the latest local Wezboard, Roamium,
Chromium, and `web` TUI builds produced the exact same visible behavior as
before: native inputs still open completely outside the Wezboard window, far
outside the webview bounds.

#### Conclusion

This disproves the first fix candidate. Passing the absolute webview rect
through `Resize` and applying it to `RenderWidgetHostView::SetBounds` does not
affect the coordinate path used by macOS native controls. The next experiment
must move deeper into Chromium's macOS view/root-window coordinate plumbing,
likely `RenderWidgetHostViewMac` popup positioning, root-window bounds, or
screen-info conversion.

### Experiment 3: Trace Native Popup Coordinates

#### Description

Experiment 2 failed because the webview screen rect reached Chromium but native
popups still opened completely outside the Wezboard window. That means the
implementation updated a value that rendering or generic bounds code may see,
but the macOS native popup path does not use it.

This experiment is research-first. Do not attempt another fix yet. Instead,
trace the exact Chromium macOS coordinate path used by native controls, compare
Roamium against a known-good content shell window, and add enough logs to
identify the correct injection point for the next fix.

The working hypothesis is that content shell works because its `NSWindow`,
`NSView`, `WebContentsView`, and `RenderWidgetHostViewMac` live in a real
visible AppKit hierarchy. Roamium renders via CALayerHost in Wezboard, but
Chromium's native view hierarchy still lives in Roamium's hidden host window.
Native popups likely ask that hidden hierarchy for screen coordinates.

#### Changes

1. **Map the native popup call paths.**

   In `chromium/src/`, inspect the macOS code paths for:
   - `<select>` popup menus;
   - datalist / autofill suggestions;
   - date and time pickers;
   - root-window and screen-coordinate conversion helpers used by those
     controls.

   Start with local source searches around:
   - `RenderWidgetHostViewMac`;
   - `ShowPopupMenu`;
   - `PopupMenuHelper`;
   - `AutofillPopup`;
   - `DateTimeChooser`;
   - `GetBoundsInRootWindow`;
   - `GetViewBounds`;
   - `convertRect`;
   - `convertRectToScreen`.

   Record the findings in this experiment before implementing a fix. The result
   should name the functions that compute the final popup anchor rect.

2. **Add Chromium-side popup coordinate logs.**

   On the Issue 779 Chromium branch, add temporary diagnostic logs with a
   consistent prefix, for example `[termsurf-popup-trace]`, at the coordinate
   functions identified in step 1.

   For each popup event, log:
   - popup/control type when available (`select`, autofill/datalist, date/time);
   - incoming anchor rect from Blink or renderer code;
   - `RenderWidgetHostViewMac` bounds;
   - `GetBoundsInRootWindow` result;
   - `GetViewBounds` or equivalent view bounds result;
   - native `NSView` frame and bounds;
   - native `NSWindow` frame;
   - `NSView convertRect:toView:nil` result;
   - `NSWindow convertRectToScreen:` result;
   - final popup screen rect passed to AppKit or Chromium popup UI.

   The logs should be detailed but temporary. They are the experiment output,
   not the final product.

3. **Keep the existing TermSurf bounds logs.**

   Preserve the Experiment 2 logs in Wezboard and Chromium that show:
   - Wezboard's computed webview screen rect;
   - the `Resize` payload sent to Roamium;
   - Chromium's received `ts_set_view_bounds` values.

   The important comparison is between TermSurf's known intended webview rect
   and the rect used by the native popup path.

4. **Compare Roamium with content shell.**

   Build and run a known-good Chromium/content shell target if available from
   the local checkout. Open the same reproduction page:

   ```bash
   http://localhost:9616/test-native-popups.html
   ```

   In content shell, click the same native controls and collect the same
   `[termsurf-popup-trace]` logs. Content shell does not need to run TermSurf;
   it is the baseline for how Chromium behaves when the AppKit view hierarchy
   matches the visible window.

   If content shell requires a different target or runner, record the exact
   command used in the result.

5. **Run the Roamium reproduction with logs enabled.**

   Run the local debug builds, open the reproduction page in Wezboard, and put
   the browser pane in a position where the bug is obvious, such as the
   top-right split.

   Click at least:
   - select dropdown;
   - datalist input;
   - date input.

   Color input can be clicked and recorded, but it remains a known exception if
   Chromium delegates it to the global `NSColorPanel`.

6. **Analyze the logs before proposing a fix.**

   The result must answer these questions:
   - Does Wezboard compute the correct visible webview screen rect?
   - Does Chromium receive that same rect through `ts_set_view_bounds`?
   - Which native popup coordinate function ignores or loses that rect?
   - Which native view/window rect is still wrong?
   - How does the same function differ in content shell?
   - What is the next fix location: `RenderWidgetHostViewMac`,
     `WebContentsViewMac`, root-window bounds, screen-info conversion, host
     `NSWindow`/`NSView` placement, or popup-specific code?

#### Verification

1. Build the affected targets:

   ```bash
   scripts/build.sh chromium
   scripts/build.sh roamium
   scripts/build.sh wezboard
   ```

   Build content shell or the nearest available known-good Chromium shell target
   if it is not already present.

2. Start the reproduction server:

   ```bash
   bun test-html/server.ts
   ```

3. Collect logs from content shell:
   - open `http://localhost:9616/test-native-popups.html`;
   - click select, datalist, and date controls;
   - save the relevant `[termsurf-popup-trace]` log excerpts.

4. Collect logs from Roamium/Wezboard:
   - run local Wezboard;
   - run local `web` with `--browser` pointing at
     `chromium/src/out/Default/roamium`;
   - open the reproduction page;
   - move the browser pane to the top-right or another visibly offset split;
   - click select, datalist, and date controls;
   - save the relevant Wezboard, Roamium, and Chromium log excerpts.

5. Pass criteria:
   - the experiment identifies the exact Chromium function or object that
     computes the wrong popup screen rect in Roamium;
   - logs include both the correct TermSurf/Wezboard webview rect and the wrong
     native popup rect;
   - content shell logs show the corresponding known-good coordinate path;
   - the conclusion names one concrete next fix location.

6. Fail criteria:
   - logs are too broad to determine which coordinate conversion is wrong;
   - content shell cannot be used and no equivalent known-good baseline is
     recorded;
   - the result proposes another fix without first identifying the coordinate
     source used by native popups.
