+++
status = "open"
opened = "2026-03-26"
+++

# Issue 767: Webview overlay offset by title bar height

## Goal

Fix the webview overlay positioning so it aligns with terminal pane content when
the macOS title bar is visible (default
`window_decorations = "TITLE | RESIZE"`).

## Background

With `window_decorations = "RESIZE"`, the overlay positions correctly. With the
default decorations (title bar visible), the webview is exactly one title bar
height too high — it ignores the title bar offset that macOS applies to terminal
content.

This is the sixth overlay positioning bug. Prior issues fixed coordinate
formulas, duplicate code paths, and timing:

| Issue | Problem                          | Fix                                                |
| ----- | -------------------------------- | -------------------------------------------------- |
| 626   | ~10px X/Y offset                 | Added intermediate flipped layer + Y-flip formula  |
| 627   | No resize update                 | Restored resize pipeline                           |
| 746   | Duplicated formula, tab switches | Moved positioning into `paint_pass()`              |
| 747   | Multi-screen split clobbering    | Made `update_ca_layer_frame()` first-creation only |
| 749   | Initial flash at wrong position  | Deferred CALayerHost creation to render pass       |

After Issue 749, the coordinate **values** are correct — there is exactly one
code path, and it reuses `paint_pane()`'s return value. The bug is not in the
values but in the **coordinate space** they are applied to.

## Analysis

### The overlay is added to the wrong parent view

In `conn.rs` `get_or_create_overlay()`, the overlay NSView is added as a
**sibling** of the contentView inside `NSThemeFrame`:

```rust
let ns_view = fe.ns_view_for_mux_window(mux_window_id)?;  // = contentView
let superview = msg_send![ns_view, superview];              // = NSThemeFrame
let frame = msg_send![ns_view, frame];
let overlay = msg_send![overlay, initWithFrame: frame];
msg_send![superview, addSubview: overlay];                  // sibling of contentView
```

### Why it breaks with a title bar

The difference between `RESIZE` and `TITLE | RESIZE` is the window style mask
(`window.rs` `decoration_to_mask()`):

- **`RESIZE`** includes `NSWindowStyleMask::FullSizeContentView`. The
  contentView fills the entire window. The overlay (same frame, same parent) is
  at the same position. Coordinates match.

- **`TITLE | RESIZE`** does **not** include `FullSizeContentView`. macOS
  positions the contentView below the title bar via NSThemeFrame's internal
  layout. The overlay is also added to NSThemeFrame with the contentView's
  frame, but NSThemeFrame is a private Apple class — it gives the contentView
  special positioning treatment that arbitrary subviews do not receive.

### Why terminal panes are unaffected

Terminal panes render via Metal/wgpu into the contentView's own layer. The
contentView (`WindowView`) is flipped (`isFlipped` returns `YES`), and macOS
positions it correctly below the title bar. Rendering coordinates are relative
to the contentView's coordinate space, which is always correct.

The overlay is a separate NSView in NSThemeFrame. Even though it reads the same
`(pane_pixel_x, pane_pixel_y)` from `paint_pane()`, those coordinates are
applied to a view whose position within the window does not match the
contentView's position when a title bar is present.

### Why previous fixes didn't catch this

Every prior fix focused on coordinate **values** — the formula, the code path,
the timing. The values are correct. The assumption that the overlay and
contentView share the same coordinate origin was never challenged because with
`RESIZE` (`FullSizeContentView`), the assumption is true.

## Proposed Solution

Move the overlay from being a sibling of contentView in NSThemeFrame to being a
**subview of contentView** itself.

In `get_or_create_overlay()`, change:

```rust
// Before:
let superview = msg_send![ns_view, superview];   // NSThemeFrame
let frame = msg_send![ns_view, frame];           // in NSThemeFrame coords
msg_send![superview, addSubview: overlay];

// After:
let bounds = msg_send![ns_view, bounds];          // in contentView coords
let overlay = msg_send![overlay, initWithFrame: bounds];
msg_send![ns_view, addSubview: overlay];          // child of contentView
```

This eliminates the NSThemeFrame dependency entirely:

1. The overlay's coordinate space is the contentView's coordinate space — the
   same space where terminal rendering happens.
2. NSView subviews render on top of their parent's layer, so the overlay still
   appears above terminal content.
3. `hitTest:` returning nil still passes input through to the contentView.
4. Title bar, fullscreen, notch — all handled automatically because macOS
   already positions the contentView correctly.
5. No need to match the contentView's exact frame in a private Apple view.
