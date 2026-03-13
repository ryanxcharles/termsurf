# Issue 746: Fix webview overlay positioning

## Goal

The webview overlay must always appear at the correct position and size, even
after tab switches and window resizes. The overlay's pixel coordinates should
come from the same calculation that positions terminal pane content — not from a
separate, duplicated formula.

## Background

When a webview is open in one pane, switching to another tab, resizing the
window, and switching back causes the webview to be wrongly positioned. Terminal
pane content is fine — only the webview overlay is broken.

### How terminal pane content is positioned (correct)

The render loop computes pane positions fresh every frame:

1. `paint_pass()` calls `tab.iter_panes()`, which walks the split tree and
   returns `PositionedPane` structs with `left`, `top`, `width`, `height` in
   cells.
2. `paint_pane()` converts cell positions to pixel positions using
   `padding_left`, `border.left`, `top_bar_height`, `padding_top`, `border.top`,
   and `cell_width`/`cell_height` from `render_metrics`.
3. Edge cases are handled: left-most panes start at `x=0`, top-most panes
   account for the tab bar, internal panes add half-cell offsets for split
   dividers.

This runs every frame, so positions are always correct — including after tab
switches and window resizes.

### How the webview overlay is positioned (broken)

The overlay position is computed in `update_ca_layer_frame()`
(`wezboard-gui/src/termsurf/conn.rs`):

```rust
let (cell_w, cell_h, origin_x, origin_y, border_left, border_top) = metrics::get();
let (pane_left, pane_top) = get_pane_cell_position(&pane.pane_id);
let x_backing = origin_x + border_left + (pane_left + pane.col) * cell_w;
let y_backing = origin_y + border_top + (pane_top + pane.row) * cell_h;
```

This has three bugs:

**Bug 1: `get_pane_cell_position()` only searches the active tab.** It calls
`w.get_active()` and iterates only that tab's panes. When you're on tab B and
resize the window, `reposition_all_overlays()` tries to look up the tab A pane's
position — but can't find it, so it returns `(0, 0)`.

**Bug 2: The formula doesn't match `paint_pane()`.** The terminal renderer has
edge-case handling (left-most panes start at x=0, half-cell offsets for split
dividers). The overlay code has none of this, so even when it finds the right
pane, the position is slightly wrong.

**Bug 3: No reposition on tab switch.** `reposition_all_overlays()` is only
called from `resize()`. Tab switches don't trigger it, so the stale (wrong)
position persists when switching back.

### Root cause

The overlay code duplicates the pane positioning logic instead of using the same
calculation that `paint_pane()` uses. The terminal rendering knows exactly where
each pane goes (via `PositionedPane` + `paint_pane()`), but this information
never reaches the overlay code.

### Why the duplication exists

The overlay code runs on the TermSurf IPC thread and uses CALayer frames (Core
Animation), while `paint_pane()` runs on the render thread and draws GPU quads.
They're in different parts of the code with different APIs. The overlay code
can't call `paint_pane()` directly.

### Proposed solution

Compute the overlay's pixel position during the render pass — where
`PositionedPane` and all padding/border/tab-bar values are already available —
then update the CALayer frame from those coordinates. This eliminates all three
bugs:

- No separate formula (uses the same calculation as terminal content).
- No active-tab-only lookup (the render pass already has the right
  `PositionedPane`).
- Updates every frame (including tab switches).

The render pass could either update the CALayer directly (if on the main thread)
or write the computed pixel rect to a shared location that the TermSurf code
reads.

### References

- `wezboard/wezboard-gui/src/termwindow/render/pane.rs` — `paint_pane()`,
  background rect calculation (lines 111-153)
- `wezboard/wezboard-gui/src/termwindow/render/paint.rs` — `paint_pass()`,
  iterates panes
- `wezboard/wezboard-gui/src/termsurf/conn.rs` — `update_ca_layer_frame()`,
  `reposition_all_overlays()`, `get_pane_cell_position()`
- `wezboard/wezboard-gui/src/termsurf/metrics.rs` — Global atomic metrics
- `wezboard/wezboard-gui/src/termsurf/state.rs` — `Pane` struct with overlay
  state
- `wezboard/wezboard-gui/src/termwindow/resize.rs` — Resize handler, calls
  `metrics::set()` and `reposition_all_overlays()`
- `wezboard/mux/src/tab.rs` — `iter_panes_impl()`, split tree traversal

## Experiments

### Experiment 1: Position overlay from the render pass

#### Description

Move overlay positioning into `paint_pass()`, where `PositionedPane` and all
layout values are already computed. Add a new function
`termsurf::set_overlay_frame()` that takes pixel coordinates and updates the
CALayer directly. Remove the old metrics-based positioning system.

#### Changes

**Add `set_overlay_frame()` to `wezboard-gui/src/termsurf/conn.rs`:**

```rust
#[cfg(target_os = "macos")]
pub fn set_overlay_frame(pane_id: usize, x: f64, y: f64, w: f64, h: f64) {
    use objc2::msg_send;
    use objc2::runtime::AnyObject;
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};

    let Some(state) = super::state::global() else {
        return;
    };
    let mut st = state.lock().unwrap();
    let id = pane_id.to_string();
    let Some(pane) = st.panes.get_mut(&id) else {
        return;
    };
    if pane.ca_layer_positioning == 0 {
        return;
    }
    unsafe {
        let layer = pane.ca_layer_positioning as *mut AnyObject;
        let frame = CGRect::new(CGPoint::new(x, y), CGSize::new(w, h));
        let _: () = msg_send![layer, setFrame: frame];
    }
}

#[cfg(not(target_os = "macos"))]
pub fn set_overlay_frame(_pane_id: usize, _x: f64, _y: f64, _w: f64, _h: f64) {}
```

**Call it from `paint_pass()` in
`wezboard-gui/src/termwindow/render/paint.rs`:**

After the existing `self.paint_pane()` and `self.paint_pane_border()` calls
(lines 258-260), add an overlay position update for each pane. Compute the
overlay's pixel origin using the same values `paint_pane()` uses:

```rust
for pos in panes {
    // ... existing paint_pane / paint_pane_border calls ...

    // Update webview overlay position using the same coordinates
    // that paint_pane() uses for terminal content.
    {
        let cell_width = self.render_metrics.cell_size.width as f64;
        let cell_height = self.render_metrics.cell_size.height as f64;
        let (padding_left, padding_top) = self.padding_left_top();
        let border = self.get_os_border();
        let tab_bar_height = if self.show_tab_bar && !self.config.tab_bar_at_bottom {
            self.tab_bar_pixel_height().unwrap_or(0.) as f64
        } else {
            0.0
        };
        let top_pixel_y = tab_bar_height + padding_top as f64 + border.top.get() as f64;

        // Pane origin in points — same formula as paint_pane's background_rect
        let pane_x = if pos.left == 0 {
            padding_left as f64 + border.left.get() as f64
        } else {
            padding_left as f64 + border.left.get() as f64
                + (pos.left as f64 * cell_width)
        };
        let pane_y = top_pixel_y + (pos.top as f64 * cell_height);

        // Read overlay cell offset (col, row) from TermSurf state
        let pane_id = pos.pane.pane_id();
        let id_str = pane_id.to_string();
        let overlay_info = {
            if let Some(state) = crate::termsurf::state::global() {
                let st = state.lock().unwrap();
                st.panes.get(&id_str).map(|p| (p.col, p.row, p.pixel_width, p.pixel_height))
            } else {
                None
            }
        };
        if let Some((col, row, pw, ph)) = overlay_info {
            let x = pane_x + (col as f64 * cell_width);
            let y = pane_y + (row as f64 * cell_height);
            let w = pw as f64 / self.dimensions.dpi; // convert backing to points
            let h = ph as f64 / self.dimensions.dpi;
            crate::termsurf::set_overlay_frame(pane_id, x, y, w, h);
        }
    }
}
```

Note: the backing-to-points conversion needs to match what
`update_ca_layer_frame` currently does — dividing by the CALayer's
`contentsScale`. We may need to pass the scale factor or retrieve it from the
window's backing scale. The exact conversion will be verified during
implementation.

**Update `handle_set_overlay()` in `conn.rs`:**

The `handle_set_overlay()` function currently uses `metrics::get()` to convert
overlay cell dimensions to pixel dimensions (lines 440-450). This still needs
cell metrics to compute `pixel_width`/`pixel_height` for the `Resize` message
sent to Chromium. Keep `metrics::set()` for this purpose only — but it no longer
drives positioning.

**Remove old positioning code from `conn.rs`:**

- Delete `update_ca_layer_frame()` (lines 1363-1407).
- Delete `reposition_all_overlays()` (lines 1412-1440).
- Delete `get_pane_cell_position()` (lines 1332-1360).
- Remove `update_ca_layer_frame()` calls from `handle_ca_context()`.

**Remove `reposition_all_overlays()` call from `resize.rs`:**

Delete line 93 (`crate::termsurf::reposition_all_overlays();`). The render pass
now handles repositioning every frame.

**Clean up `state.rs` `Pane` struct:**

Remove fields that are no longer needed:

- `overlay_origin_x` — was cached for logging, no longer computed separately
- `overlay_origin_y` — same
- `overlay_scale` — same

#### Verification

1. Open a webview in a pane. It displays at the correct position.
2. Split the pane. The webview stays correctly positioned in its pane.
3. Switch to a different tab, resize the window, switch back. The webview is at
   the correct position and size.
4. Resize the window while the webview tab is active. The webview tracks the
   pane position correctly.
5. Open a second webview in a split pane. Both overlays are correctly
   positioned.
