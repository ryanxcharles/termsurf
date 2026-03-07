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

### Approach

1. Add three config fields: `focused_split_border_color` (Option<RgbaColor>),
   `unfocused_split_border_color` (Option<RgbaColor>), `split_border_width`
   (Dimension, default 0).
2. Add a `paint_pane_border` method that draws 4 filled rectangles around each
   pane's `background_rect`, choosing color based on `pos.is_active`.
3. Call it from the paint loop after `paint_pane`. When
   `split_border_width > 0`, skip `paint_split` since borders replace dividers.
4. Skip borders when there's only one visible pane (single pane or zoomed).

The border draws as an overlay (layer 2) on top of pane content (layer 0), same
approach as existing split dividers and window borders. Content inset can be
added as a follow-up if the 2px overlap is noticeable.
