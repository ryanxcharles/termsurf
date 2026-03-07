# Issue 723: Add focused/unfocused split pane borders to Wezboard

## Goal

Add configurable colored borders around split panes in Wezboard that
differentiate active vs inactive panes, matching the Ghostboard feature set.

## Background

Ghostboard (Ghostty fork) implemented this feature across Issues 667-669 and
672. The final solution uses three config keys — `focused-split-border-color`,
`unfocused-split-border-color`, and `split-border-width` — rendered as SwiftUI
overlay rectangles with content inset by the border width.

Wezboard (WezTerm fork) does not have this feature. WezTerm only draws a thin
1px split divider line using the palette's `split` color via `filled_rectangle`
on layer 2. There is no per-pane border, no focused/unfocused distinction, and
no configurable width.

The user's desired config:

```
focused_split_border_color = "#7dcfff"
unfocused_split_border_color = "#565f89"
split_border_width = 2
```

## Analysis

### Existing rendering infrastructure

Wezboard already has the exact rendering primitives needed:

- **`filled_rectangle`** — Draws colored quads on a specified layer. Used by
  window borders (`borders.rs`), split dividers (`split.rs`), and scrollbar
  thumbs (`pane.rs`).
- **Layer 2** — The overlay layer used by split dividers, drawn on top of pane
  content (layer 0).
- **`background_rect`** — Computed per-pane in `paint_pane` (lines 110-152 of
  `pane.rs`). Gives the full pixel region for each pane, accounting for padding,
  tab bar, OS borders, and edge detection.
- **Window border pattern** — `borders.rs` draws exactly 4 filled rectangles
  (top/bottom/left/right) using `filled_rectangle`. This is the pattern to
  follow.

### Config pattern

WezTerm's config uses `RgbaColor` for colors and `Dimension` with
`#[dynamic(try_from = "crate::units::PixelUnit")]` for pixel widths. The
`WindowFrameConfig` in `color.rs` shows the exact pattern for border width/color
fields.

### Key files

| File                                                     | Role                                                                 |
| -------------------------------------------------------- | -------------------------------------------------------------------- |
| `wezboard/config/src/config.rs`                          | Config struct — add 3 new fields near `inactive_pane_hsb` (line 621) |
| `wezboard/config/src/color.rs`                           | `WindowFrameConfig` pattern, `default_zero_pixel` helper             |
| `wezboard/wezboard-gui/src/termwindow/render/pane.rs`    | Pane rendering, `background_rect` computation, `is_active` flag      |
| `wezboard/wezboard-gui/src/termwindow/render/paint.rs`   | Paint loop — iterates panes (249-258) then splits (260-266)          |
| `wezboard/wezboard-gui/src/termwindow/render/split.rs`   | Current split divider rendering (thin `underline_height` line)       |
| `wezboard/wezboard-gui/src/termwindow/render/borders.rs` | Window border rendering — pattern to follow (4 rectangles)           |

### Content inset

Without insetting, the border (layer 2) paints over the outermost pixels of
terminal text (layer 0). Ghostboard solved this in Issue 672 by reducing the
content area by the border width on each interior edge.

In WezTerm, pane content positioning is controlled by two values in
`paint_pane`:

- **`left_pixel_x`** (line 340) — horizontal start of text rendering, computed
  from `padding_left + border.left + (pos.left * cell_width)`.
- **`top_pixel_y`** (line 78) — vertical start, computed from
  `top_bar_height + padding_top + border.top`.

The `background_rect` (lines 110-152) has edge detection logic: it checks
`pos.left == 0`, `pos.top == 0`, and whether `pos.left + pos.width` reaches the
terminal's column count to decide whether to extend to the window edge.

The inset should only apply on **interior edges** — edges where panes meet other
panes. Window-edge panes don't need inset on the side touching the window frame,
since the window's own padding already provides separation. The edge detection
logic in `background_rect` already identifies which edges are interior vs
window-edge.

### Approach

1. Add three config fields: `focused_split_border_color` (Option<RgbaColor>),
   `unfocused_split_border_color` (Option<RgbaColor>), `split_border_width`
   (Dimension, default 0).
2. Add a `paint_pane_border` method that draws 4 filled rectangles around each
   pane's `background_rect`, choosing color based on `pos.is_active`.
3. Inset pane content by adjusting `left_pixel_x`, `top_pixel_y`, and
   `background_rect` in `paint_pane` — shift text rendering inward by
   `border_width` on interior edges so the border doesn't cover terminal text.
4. Call `paint_pane_border` from the paint loop after `paint_pane`. When
   `split_border_width > 0`, skip `paint_split` since borders replace dividers.
5. Skip borders when there's only one visible pane (single pane or zoomed).
