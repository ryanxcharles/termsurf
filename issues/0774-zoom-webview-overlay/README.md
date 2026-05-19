+++
status = "closed"
opened = "2026-04-11"
closed = "2026-05-19"
+++

# Issue 774: Zooming non-browser pane leaves webview overlay visible

## Goal

When zooming a non-browser pane, the browser webview overlay must be hidden so
the zoomed pane is fully visible without obstruction.

## Background

Wezboard supports "zooming" a pane, which temporarily hides all other panes in
the same tab and expands the selected pane to fill the entire tab area. This
works correctly when zooming a browser pane — the webview scales to fill the
space.

However, when zooming a non-browser pane (e.g. a terminal pane) while a browser
pane exists in the same tab, the other panes vanish as expected but the
browser's webview overlay remains visible on top of the zoomed pane. The
CALayerHost overlay is not hidden or repositioned when its owning pane is hidden
by the zoom.

This makes zooming useless for any non-webview pane in a tab that also contains
a browser pane, because the webview covers part or all of the zoomed content.

## Analysis

The zoom operation hides panes at the mux/layout level, but the webview overlay
(rendered via CALayerHost compositing) is managed separately from the pane
layout system. When a pane is hidden by zoom, the corresponding browser overlay
must also be hidden. When zoom is exited, the overlay must be restored.

The fix likely involves:

1. Detecting when a zoom hides a browser pane and sending a message to hide/show
   the overlay.
2. Or updating the overlay position/visibility during the zoom layout
   recalculation so overlays for non-visible panes are hidden.

## Experiments

### Experiment 1: Sync Overlay Visibility on Zoom

#### Description

Fix zoomed non-browser panes by making TermSurf overlay visibility follow the
same zoom-respecting pane set that Wezboard renders.

The mux zoom logic already works: when a tab is zoomed, `tab.iter_panes()`
returns only the zoomed pane, while `tab.iter_panes_ignoring_zoom()` still
returns every pane in the split tree. The terminal renderer uses the
zoom-respecting list, which is why non-zoomed terminal panes disappear
correctly. Browser overlays are separate CALayerHost layers, so a browser pane
that drops out of the render list keeps its previous native layer frame unless
the GUI explicitly hides it.

Wezboard already has a GUI-side hide/show mechanism:
`termsurf::conn::sync_overlay_visibility(active_pane_ids)` updates the TermSurf
pane `visible` flag and calls `setHidden:` on each CA layer. The current bug is
that this sync runs for `WindowInvalidated` and `PaneFocused`, but not for the
`TabResized` notification emitted by zoom/unzoom. This experiment should reuse
that existing path instead of adding a protocol message or moving visibility
policy into the paint loop.

#### Changes

1. **Centralize TermSurf overlay visibility sync.**

   In `wezboard/wezboard-gui/src/termwindow/mod.rs`, add a small `TermWindow`
   helper, for example `sync_termsurf_overlay_visibility`, that:
   - gets the mux;
   - walks every active tab for every mux window, matching the existing
     `WindowInvalidated` / `PaneFocused` behavior;
   - collects pane ids from `tab.iter_panes()`, not
     `tab.iter_panes_ignoring_zoom()` and not `tab.contains_pane`;
   - passes those ids to `crate::termsurf::conn::sync_overlay_visibility`.

   Using `tab.iter_panes()` is the key requirement because it respects zoom.

2. **Use the helper from all relevant notification paths.**

   Replace the duplicated visibility-sync blocks in:
   - `MuxNotification::WindowInvalidated`
   - `MuxNotification::PaneFocused`

   Then call the same helper from:
   - `MuxNotification::TabResized`

   Zoom and unzoom already emit `TabResized`, so this should hide browser
   overlays when their pane is hidden by zoom and restore them when zoom ends.

3. **Make overlay frame updates unhide visible overlays.**

   In `wezboard/wezboard-gui/src/termsurf/conn.rs`, update `set_overlay_frame`
   so a successful frame update also marks the pane visible and calls
   `setHidden: NO` on `pane.ca_layer_flipped`.

   The target layer matters: `sync_overlay_visibility` hides `ca_layer_flipped`,
   not `ca_layer_positioning` and not `ca_layer_host`. Unhiding only
   `ca_layer_positioning` would leave the parent flipped layer hidden and the
   overlay would remain invisible.

   This makes the frame update path robust if a browser overlay was previously
   hidden by zoom and then becomes visible again. The concrete race to avoid is:
   unzoom clears `self.zoomed`, a paint runs and calls `set_overlay_frame` for
   the now-visible browser pane, but the queued `TabResized` visibility sync has
   not yet been processed. In that case the frame update should restore
   visibility immediately.

   Keep `sync_overlay_visibility` as the authoritative hide/show operation for
   panes that are not currently rendered.

4. **Fix pane-visible checks to respect zoom.**

   In `wezboard/wezboard-gui/src/termwindow/mod.rs`, update `is_pane_visible` so
   it preserves the existing tab-overlay short-circuit, then checks the
   zoom-respecting rendered pane list:
   `tab.iter_panes().iter().any(|p| p.pane.pane_id() == pane_id)`.

   Do not use `tab.contains_pane(pane_id)` for visibility, because it checks
   split-tree membership and therefore treats zoom-hidden panes as visible. This
   is related consistency work: steps 1-3 hide the visible CALayer, while this
   step prevents zoom-hidden panes from triggering output invalidations as if
   they were still visible.

5. **Do not change the TermSurf protocol.**

   This is a GUI compositing bug. The browser engine does not need a new
   visibility message for this experiment. A future optimization may pause
   browser rendering while hidden, but that is not required to fix the visible
   overlay.

#### Verification

1. Build Wezboard:

   ```bash
   scripts/build.sh wezboard
   ```

2. Open a tab with one browser pane and one terminal-only pane.

3. Zoom the browser pane:
   - the browser overlay expands to fill the tab;
   - the terminal pane disappears;
   - unzoom restores both panes and the browser overlay returns to its split
     position.

4. Zoom the terminal-only pane:
   - the browser overlay is hidden immediately;
   - the zoomed terminal pane is unobstructed;
   - scrolling or browser output while zoomed does not make the hidden browser
     overlay reappear;
   - unzoom restores the browser overlay in its original pane.

5. Repeat with focus changes before zooming:
   - focus browser, zoom terminal;
   - focus terminal, zoom terminal;
   - focus browser, zoom browser.

6. While a terminal-only pane is zoomed and the browser overlay is hidden:
   - switch to another tab and back; the browser overlay remains hidden until
     unzoom;
   - resize the window; the browser overlay remains hidden until unzoom.

7. Confirm no protocol changes are present and no Chromium/Roamium changes are
   required.

**Result:** Pass

Implemented the GUI-side overlay visibility sync path. `WindowInvalidated` and
`PaneFocused` now share a single zoom-aware helper, `TabResized` also runs that
helper so zoom/unzoom updates browser overlay visibility, `set_overlay_frame`
marks visible overlays visible again by unhiding `ca_layer_flipped`, and
`is_pane_visible` now preserves tab-overlay handling while using
`tab.iter_panes()` for normal zoom-aware pane visibility.

The debug Wezboard build passes:

```bash
scripts/build.sh wezboard
```

Manual GUI verification passed. Zooming a browser pane expands the browser
overlay, zooming a terminal-only pane hides the browser overlay, and unzooming
restores the browser overlay to its split pane.

#### Conclusion

The implementation follows the existing TermSurf overlay architecture and
requires no protocol, Roamium, or Chromium changes. The zoom-hidden browser
overlay now follows the same visible pane set as the terminal renderer.

## Conclusion

Issue 774 is closed by Experiment 1. Browser overlays now hide when their owning
pane is hidden by zoom, and visible overlays are restored when zoom ends or when
the browser pane itself is zoomed.
