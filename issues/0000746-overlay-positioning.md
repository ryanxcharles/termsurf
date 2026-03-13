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
