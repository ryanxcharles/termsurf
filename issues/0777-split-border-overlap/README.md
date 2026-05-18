+++
status = "open"
opened = "2026-04-11"
+++

# Issue 777: Split border overlaps pane content and blocks mouse resize

## Goal

The `split_border_width` border must not overlap pane content or block
mouse-driven pane resizing.

## Background

Wezboard has a config option `split_border_width = 4` that draws a border around
each terminal pane. This border has two problems:

### 1. Border overlaps pane content

The border is drawn on top of the pane's content area rather than outside it.
With `split_border_width = 4`, the outermost 4 pixels of terminal content are
hidden behind the border. The pane needs padding or margin equal to the border
width so content is inset and fully visible.

### 2. Border covers the mouse resize handle

WezTerm uses a thin invisible hit region between panes for mouse-driven resizing
(click and drag to resize splits). The border is drawn on top of this region,
visually covering it and — more critically — intercepting or blocking mouse
events. With the border enabled, it is impossible to resize panes with the
mouse.

## Analysis

This issue is a regression from the behavior that worked in the archived Ghostty
fork. Ghostboard's final border behavior is documented in Issue 672: the border
was an overlay, but the rendered surface was reduced by the border width and
offset inward. That made the border behave like CSS `box-sizing: border-box`:
the pane owns a full outer rectangle, the border occupies the edge of that
rectangle, and terminal content renders inside the remaining inner rectangle.

The intended behavior is:

1. A single tab with one pane has no split border.
2. Creating a split makes borders appear on both panes.
3. The focused pane uses `focused_split_border_color`.
4. Unfocused panes use `unfocused_split_border_color`.
5. The border occupies real pane space. Content is inset by exactly
   `split_border_width` logical pixels on all four sides.
6. The split divider remains visible and mouse-draggable, even when pane borders
   replace the old thin divider drawing.

### What Wezboard currently does

Wezboard has the first half of this behavior:

- `paint.rs` iterates visible panes, calls `paint_pane`, then calls
  `paint_pane_border`.
- `paint_pane_border` draws four layer-2 rectangles around each pane when there
  is more than one pane and `split_border_width > 0`.
- `paint_pane_border` selects the focused/unfocused color from
  `focused_split_border_color` and `unfocused_split_border_color`, falling back
  to the palette split color.
- `paint.rs` skips the old `paint_split` divider when `split_border_width > 0`,
  so the old thin divider is not drawn on top of the new pane borders.

That is why the visual state is partly correct: no border for a single pane,
border appears after splitting, and focused/unfocused colors can work.

### Why the content inset is wrong

The border is currently just paint. It does not participate in pane layout or
line rendering.

In `paint_pane`, the `num_panes` parameter is named `_num_panes` and is not
used. The line-rendering origin is still computed as:

```rust
let left_pixel_x = padding_left
    + border.left.get() as f32
    + (pos.left as f32 * self.render_metrics.cell_size.width as f32);
```

The line-rendering top position is likewise based on the pane's unmodified
`top_pixel_y`, and `render_screen_line` still receives:

```rust
pixel_width: self.dims.cols as f32
    * self.term_window.render_metrics.cell_size.width as f32,
```

None of these values account for `split_border_width`. As a result, terminal
text and background fills still begin at the original pane edge and span the
original pane width. The border is then drawn later on layer 2, directly over
the outer pixels of that content.

There is also a unit bug: the border width is currently treated as physical
framebuffer pixels. Users configure border widths in UI terms, so
`split_border_width = 4` should mean 4 logical pixels, not 4 physical pixels. On
a 2x Retina display, that should draw and reserve 8 physical pixels. On a 1x
display, it should draw and reserve 4 physical pixels. The conversion from
logical pixels to physical pixels must happen once, using the window's current
scale/DPI, before painting, content inset, and hit-region geometry are computed.

The historical failed attempts in Issue 723 explain why this must be a per-pane
content inset, not a global resize:

- Reducing only `pixel_width` does not move the left/top origin.
- Shifting only selected "interior" edges misses the edges where the border is
  still drawn.
- Subtracting border pixels from the global window size in `resize.rs` changes
  the terminal cell count but does not allocate padding inside each pane.

The successful Ghostboard-style model is simpler: every bordered pane gets
uniform inner padding on all four sides, equal to the physical-pixel value
computed from `split_border_width` logical pixels for the current display.

### Why mouse resize is wrong

Mouse resizing depends on UI hit regions, not just pixels on screen.

`paint_split` does two things:

1. It draws the old split divider line.
2. It pushes a `UIItemType::Split(split.clone())` into `self.ui_items`.

`mouseevent.rs` later uses that UI item to set the resize cursor, start split
dragging, and call `tab.resize_split_by(...)`.

When `split_border_width > 0`, `paint.rs` skips `paint_split`. That removes the
old visual divider, which is correct, but it also removes the only code path
that registers the split resize hit region. `paint_pane_border` draws border
rectangles but does not register a `UIItemType::Split`, so the mouse has no
split target to hover, click, or drag.

### Fix direction

The fix should restore the Ghostboard model in Wezboard:

1. **Compute a per-pane border inset.** In `paint_pane`, evaluate
   `split_border_width` into physical pixels only when `num_panes > 1` and the
   pane is not zoomed. Otherwise the inset is zero. The configured value should
   be interpreted as logical pixels; convert it to physical pixels using the
   current window scale/DPI before using it for geometry.
2. **Apply that inset to content on all four sides.** Add the inset to
   `left_pixel_x`, add it to the top coordinate used by `LineRender`, and reduce
   the width passed to `render_screen_line` by `2 * inset`. This makes text and
   line backgrounds render inside the border.
3. **Keep the pane background consistent.** Inset the pane background fill by
   the same amount, or explicitly decide that background may extend under the
   border while text and line fills do not. The Ghostboard behavior was cleaner:
   content, progress overlays, and surface rendering all lived inside the
   border.
4. **Register split resize hit regions even when borders replace dividers.**
   Either keep a non-drawing `paint_split` path that only pushes
   `UIItemType::Split`, or move split hit-region registration into a separate
   helper called regardless of whether the old divider is drawn. The hit region
   should use the same logical-to-physical border width conversion as painting,
   cover the visible border/divider area, and remain large enough to drag
   comfortably with the mouse.
5. **Do not solve this in `resize.rs`.** The terminal cell count may stay the
   same; the issue is pixel placement inside each pane. Global resize math does
   not create per-pane padding.

The implementation should prove both requirements together: border padding must
move content inward by exactly `split_border_width` logical pixels, converted to
the correct physical pixel count for the current display, and the split
divider/hit region must remain visible and clickable for mouse resizing.
