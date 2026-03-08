# Issue 727: Wezboard second webview positioning

## Goal

Make two browser overlays visible simultaneously in split panes, each positioned
correctly over its own terminal pane.

## Background

Issue 726 proved the second overlay's full pipeline works — SetOverlay →
CreateTab → TabReady → CaContext → CALayerHost all complete successfully. But
both overlays render at the same screen position because `update_ca_layer_frame`
uses global metrics that don't distinguish between panes.

### The positioning formula

Ghostboard's correct formula (Metal.zig:301–302):

```zig
const x: f64 = @as(f64, grid_col) * cw / scale + pl / scale;
const y: f64 = @as(f64, grid_row) * ch / scale + pt / scale;
```

Where `grid_col`/`grid_row` are per-surface grid coordinates, `cw`/`ch` are cell
dimensions, and `pl`/`pt` are padding_left/padding_top. Each Surface owns its
own positioning layer, so coordinates are relative to the Surface's own area.

Wezboard's current formula (conn.rs:986–987, from Issue 726 Exp 7):

```rust
let x = (origin_x as u64 + pane.col * cell_w as u64) as f64 / scale;
let y = (origin_y as u64 + pane.row * cell_h as u64) as f64 / scale;
```

Where `origin_x` = `padding_left` and `origin_y` = `top_bar_height` (set in
resize.rs:84–89), and `col`/`row` come from the TUI's `viewport_rect.x/y`
(ratatui grid coords relative to the terminal pane, e.g., col=0, row=1).

### Why both formulas look equivalent but aren't

Ghostboard: each Surface has its own layer tree. The positioning layer is a
sublayer of the Surface's own view/layer, so `(0, 0)` means "top-left of this
Surface's rendering area." Adding `padding_left` offsets from the Surface edge
to the grid origin.

Wezboard: ONE shared overlay NSView covers the entire window. All panes'
positioning layers are siblings under the same root layer. So `(0, 0)` means
"top-left of the window." The `origin_x`/`origin_y` values from `metrics::get()`
provide the global offset from the window origin to the content area, but they
say nothing about which pane within the content area.

### What the TUI sends

The TUI sends `viewport_rect.x` and `viewport_rect.y` (main.rs:420–421) as `col`
and `row` in SetOverlay. These are ratatui grid coordinates relative to the
terminal pane:

- Single pane: col=0, row=1 (row 0 is the URL bar)
- The TUI has no knowledge of where its pane sits within the terminal window

### The missing piece

The TUI's col/row are pane-relative grid coordinates, but Wezboard needs
window-relative pixel coordinates to position the CALayer. The conversion
requires knowing where each pane sits in the window — information that WezTerm's
mux/layout system has but the TermSurf connection code currently doesn't access.

### Issue 726 Exp 7 failure

Adding `col * cell_w` to `origin_x` doubled the first overlay's margins (pushed
it down and to the right). The second overlay still didn't appear. This happened
because col=0 and row=1 for the first pane, so:

- x = origin_x + 0 \* cell_w = origin_x (correct, unchanged)
- y = origin_y + 1 \* cell_h = origin_y + cell_h (wrong — shifted down by one
  cell, URL bar offset shouldn't be added to the window origin)

The `origin_y` already positions at the content area top. Adding `row * cell_h`
adds the URL bar row offset again, which is wrong because `origin_y` is the
window-level offset (tab bar height), not the pane-level offset.

### Secondary bug: white flash

When a split opens, the first pane resizes. Chromium re-sends CaContext with the
same context ID. `handle_ca_context` swaps the CALayerHost (remove old, add
new), showing a blank frame briefly. This is cosmetic but worth fixing.

## Approach

The core problem is converting pane-relative grid coordinates to window-absolute
pixel coordinates. There are two possible approaches:

### Approach A: Query WezTerm's pane layout

WezTerm knows exactly where each pane sits in the window. The mux/tab system
tracks pane positions. If we can query a pane's pixel origin from the mux, we
can position the overlay precisely.

Pros: Uses WezTerm's own layout system, naturally correct for splits and
resizes. Cons: Requires finding the right API in WezTerm's codebase.

### Approach B: Per-pane metrics

Instead of global metrics, store per-pane pixel origin. When WezTerm computes
pane layout (during resize or split), update each pane's origin in the shared
state. The positioning formula becomes:

```rust
let x = (pane_origin_x + pane.col * cell_w) as f64 / scale;
let y = (pane_origin_y + pane.row * cell_h) as f64 / scale;
```

Where `pane_origin_x`/`pane_origin_y` are the pixel coordinates of this specific
pane's top-left corner within the window.

Pros: Clean separation, doesn't depend on querying WezTerm APIs at render time.
Cons: Requires hooking into WezTerm's layout recalculation.

### Recommended path

Start with Approach A — find where WezTerm computes per-pane pixel positions and
expose that to the TermSurf connection code. This is the more reliable approach
since WezTerm already calculates these positions for its own rendering.

## Experiment 1: Investigate WezTerm's pane position data

### Hypothesis

WezTerm's mux/tab system tracks per-pane positions in pixel coordinates. If we
can access a pane's pixel origin given its pane_id, we can fix the positioning
formula without adding new state.

### Design

1. Search WezTerm's codebase for how it computes pane positions — look for the
   split layout engine, `PositionedPane`, or similar structures that map pane_id
   to pixel coordinates
2. Trace how the rendering code knows where to draw each pane's content
3. Determine if this information is accessible from the TermSurf connection code
   (which runs on the main thread via `promise::spawn::spawn_into_main_thread`)
4. Document findings and propose the specific code change

### Verification

Research only — no code changes. Success = we know exactly which API to call and
what coordinates it returns.
