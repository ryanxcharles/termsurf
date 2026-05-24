+++
status = "open"
opened = "2026-05-23"
+++

# Issue 787: Split border has extra outer margin

## Goal

When multiple panes are open, the visible split-pane border should be the outer
visual boundary of the pane area. There should not be an additional grid-cell
looking margin outside the border.

## Background

Issue 786 introduced grid-native split borders. The important architectural
change was correct: when split borders are active, Wezboard reserves grid cells
for border/gutter space instead of drawing borders over terminal content. This
keeps PTY dimensions truthful and prevents pane content from being hidden.

However, the current visual result has too much space outside the border. In the
observed screenshot, the split border is visibly inset from the outside of the
available pane area. It does not read as a border around the pane layout; it
reads as a border drawn inside an extra margin.

## Analysis

There are two independent spacing systems stacking together:

1. Wezboard's normal `window_padding` lives outside the terminal grid. The
   default padding is one cell on the left/right and half a cell on the
   top/bottom.
2. Issue 786 reserves a one-cell grid perimeter around split layouts when
   multiple panes are visible.

The split-border renderer then centers the visible pixel border line inside the
reserved perimeter cell. The resulting outside edge is effectively:

```text
window edge
→ window padding
→ part of the reserved border cell
→ visible border line
→ rest of the reserved border cell
→ pane content
```

This explains the screenshot: even if the border line is centered within the
reserved cell, the reserved cell begins after normal window padding. The two
systems together create an unintended outer margin.

Relevant code:

- `wezboard/config/src/config.rs` defines `window_padding`; defaults are one
  cell left/right and half-cell top/bottom.
- `wezboard/mux/src/tab.rs::grid_border_inset_for_size()` returns a one-cell
  inset for split layouts.
- `wezboard/mux/src/tab.rs::iter_panes_impl()` shifts pane content inward by
  that inset and records `PositionedPaneBorder` metadata.
- `wezboard/wezboard-gui/src/termwindow/render/pane.rs::paint_pane_border()`
  centers the visible line inside the reserved border cell.

The core bug is not content clipping. The core bug is visual ownership of the
outer area: when split borders are active, the outer split border should own the
outside boundary instead of being pushed inward by normal window padding plus
its own reserved gutter.

## Proposed Solution

Design the fix around the relationship between window padding and split-border
perimeter reservation.

Possible approaches:

1. **Make split-border perimeter consume window padding at the outside edge.**
   When a grid-native split border is active, reduce effective window padding on
   sides where the split perimeter already provides spacing. This keeps the
   border close to the outer pane area while preserving normal padding for
   single-pane and zoomed-pane modes.
2. **Draw the outer border line at the outside edge of the reserved perimeter
   cell.** This removes the half-cell outside gap from the border cell, but it
   does not address the normal window padding that still exists outside the
   reserved cell.
3. **Special-case split layouts in render-coordinate calculation.** Keep the mux
   layout unchanged, but compute split-border outer pixel coordinates from the
   terminal content area's outside edge rather than from padded pane
   coordinates.

The preferred direction is likely approach 1: when split borders are active,
normal window padding and the split-border perimeter should not both create
outer spacing. Single-pane and zoomed-pane behavior should remain unchanged.

The fix should preserve Issue 786's core invariant: pane content must remain
inside the content grid, and the visible border must never cover terminal cells.

## Experiments

### Experiment 1: Let Split Borders Replace Outer Padding

#### Description

When grid-native split borders are active, the one-cell split-border perimeter
already provides outer spacing around the split layout. We should not also apply
normal `window_padding` outside that perimeter. This experiment makes the
effective render/input padding conditional: in multi-pane non-zoomed layouts
with grid-native split borders active, subtract the split-border perimeter from
the effective window padding used to position terminal content, pane borders,
split hitboxes, browser overlays, and mouse coordinates.

The goal is not to remove `window_padding` globally. Single-pane and zoomed-pane
layouts should keep existing padding behavior. The change only applies when the
split layout has a reserved one-cell perimeter.

This is a presentation/input-coordinate fix. It should not change the mux split
tree's content sizing model from Issue 786: pane content dimensions and PTY
sizes must remain truthful and must still exclude reserved border cells.

#### Changes

1. Add a small helper on the GUI/render side that reports whether the current
   visible tab layout has an active grid-native split-border perimeter. It
   should return true only when:
   - more than one pane is visible;
   - no pane is zoomed;
   - visible `PositionedPane` values include `border: Some(...)`.
2. Add a helper for effective content padding used by split layouts. It should
   start from `padding_left_top()` and subtract one cell of effective padding on
   axes where the split-border perimeter already supplies that outside space:
   - left/right visual positioning should not get both `window_padding.left` and
     the one-cell border perimeter;
   - top/bottom visual positioning should not get both `window_padding.top` and
     the one-cell border perimeter.
3. Clamp the effective padding subtraction to zero. If a user configures less
   than one cell of window padding, do not produce negative padding or shift
   content outside the window chrome.
4. Use the effective split-layout padding consistently in:
   - `paint_pane()` / `paint_pane_border()` coordinates;
   - `paint_split()` UIItem hitbox coordinates;
   - browser overlay positioning code paths that derive from pane coordinates;
   - mouse event coordinate conversion for pane selection and split dragging.
5. Keep plain `padding_left_top()` behavior for single-pane and zoomed-pane
   layouts.
6. Do not change `wezboard/mux/src/tab.rs` content sizing or Issue 786's
   reserved-cell model unless the render/input helper proves insufficient. The
   mux should still reserve the one-cell perimeter; this experiment changes
   where that perimeter is placed relative to normal window padding.

#### Non-Negotiable Invariants

- Pane content must never be painted underneath the border.
- PTY size, pane content grid, browser overlay size, and visible content area
  must continue to agree.
- Single-pane and zoomed-pane window padding must remain unchanged.
- Split resize hitboxes must stay aligned with the visible border.
- Browser overlays must stay aligned with their pane content.
- Mouse clicks must still land in the expected terminal/browser cell.
- No Chromium, Roamium, webtui, or protocol changes.

#### Verification

1. Build and run debug Wezboard.
2. With default `window_padding`, open a horizontal split and verify the outer
   split border no longer has an extra one-cell-looking margin outside it.
3. Open a vertical split and verify the same for the top/bottom outer border.
4. Compare against the screenshot that opened this issue. The visible border
   should read as the outer boundary of the split layout, not as an inset line
   inside an extra margin.
5. Verify single-pane mode still has normal `window_padding`.
6. Verify zooming a pane removes the split-border perimeter and restores normal
   padding behavior; unzooming restores the multi-pane border placement.
7. Drag a split border with the mouse. The resize cursor and draggable area
   should align with the visible border, including after this padding change.
8. Click terminal cells near the top-left and bottom-right of each pane. Input
   should still land in the expected cell.
9. Open a browser pane in a split. Verify the overlay aligns with pane content
   and does not overlap the border.
10. Test a custom config with `window_padding = 0`. The effective padding helper
    must not shift the border outside the window or produce negative
    coordinates.
11. Test a custom config with larger-than-default `window_padding`. The
    split-border perimeter should consume one cell worth of outer padding, while
    any remaining configured padding should still be visible.

#### Pass Criteria

With multiple panes open, the visible border appears as the outer visual
boundary of the split layout instead of being inset behind an additional margin.
All coordinate-dependent behaviors still align: split dragging, mouse clicks,
terminal cell mapping, and browser overlays.

#### Partial Criteria

The extra outer margin is fixed visually, but one coordinate-dependent behavior
needs follow-up, such as browser overlay alignment or split-drag hitboxes.
Record the failing behavior and design the next experiment around that specific
coordinate path.

#### Failure Criteria

- The extra outer margin remains visible with default `window_padding`.
- Pane content is painted under the border.
- Single-pane or zoomed-pane padding changes unexpectedly.
- Browser overlays or mouse input become offset from pane content.
- The experiment changes PTY sizing or mux split allocation instead of only
  resolving render/input placement relative to padding.

**Result:** Pass

Experiment 1 fixed the visible extra outer margin. The implementation kept the
Issue 786 mux/PTY sizing model intact and changed only border painting:
`paint_pane_border()` now expands outer perimeter edges into the existing window
padding area, while internal divider-facing edges remain centered in their
reserved grid cells.

Debug Wezboard built successfully, and manual testing confirmed the border now
reads as the outer visual boundary of the multi-pane layout instead of an inset
line behind an extra margin.

#### Conclusion

The issue was primarily visual placement, not PTY sizing or pane layout. The
successful fix was to let the outer border line occupy the padding-side edge of
the reserved border geometry, avoiding the stacked appearance of normal
`window_padding` plus a centered split-border gutter.

The issue remains open for any follow-up polish or additional verification.

### Experiment 2: Use Pixel Gutters For Outer Borders

#### Description

Experiment 1 proved that the visible border should own the outside edge of the
multi-pane area, but it also exposed the deeper mismatch: Issue 786 reserves a
full terminal cell on every outer side for a border that is only a few pixels
thick. That is real non-overlap space, so it prevents the original
border-over-content bug, but it is visually excessive. The problem is most
obvious vertically because terminal cells are much taller than they are wide.

The correct model is:

```text
window/tab chrome
pixel-sized outer border gutter
pane content grid
```

not:

```text
window/tab chrome
full terminal-cell outer border gutter
pane content grid
```

This experiment should replace the full-cell **outer perimeter** reservation
with a pixel-sized outer gutter. Pane content must still never render under the
border. The PTY/grid dimensions must remain truthful by deriving the content
grid from the pixel area left after the outer gutter is subtracted.

Internal split dividers are different. They are part of the cell-native split
architecture, provide mouse resize hit targets, and should remain grid-native
one-cell divider cells for this experiment.

#### Changes

1. Revert or supersede the Experiment 1 paint-only expansion if it conflicts
   with the new geometry. The result of Experiment 2 should not depend on
   drawing a border over a full reserved outer cell.
2. Split the border model into two concepts:
   - **outer perimeter gutter:** pixel-sized space around the full visible split
     layout;
   - **internal split divider cells:** existing grid-native one-cell separators
     between panes.
3. Compute the outer perimeter gutter size from pixel values, not terminal
   cells:
   - start with `split_border_width`;
   - clamp to at least 1 pixel when borders are enabled;
   - add a small fixed pixel breathing room only if needed for visual polish;
   - do not convert this gutter into a full grid row or column.
4. In the resize/layout path, derive the terminal content grid from the pixel
   area that remains after subtracting:
   - normal window chrome/padding;
   - the pixel outer perimeter gutter on all four sides;
   - internal one-cell split dividers.
5. Update `wezboard/mux/src/tab.rs` so outer split layout reservation no longer
   subtracts two full rows and two full columns solely for the outer perimeter.
   The mux should still allocate internal split divider cells and should still
   produce truthful pane content dimensions.
6. Extend the positioned geometry exposed to the renderer so each pane can
   distinguish:
   - content rect in grid cells;
   - internal divider adjacency;
   - outer perimeter pixel rect for pane edges that touch the outside of the
     split layout.
7. Paint outer border lines in the pixel gutter outside pane content. Paint
   internal divider-facing borders using the existing grid-native divider
   geometry.
8. Update browser overlay positioning and mouse mapping only as required to use
   the new content rect. Do not add offsets by guesswork; every coordinate path
   must share the same content-rect source of truth.
9. Preserve single-pane and zoomed-pane behavior. No outer split-border gutter
   should be applied when there is no visible split layout.
10. Run `cargo fmt` after Rust edits and accept the formatter output.

#### Non-Negotiable Invariants

- Borders must never overlap terminal text, browser overlays, or pane content.
- PTY rows/cols must match the visible content grid.
- Outer top/bottom borders must not consume a full terminal row.
- Outer left/right borders must not consume a full terminal column.
- Internal split dividers remain grid-native and draggable.
- Split-drag hitboxes must remain aligned with visible internal dividers.
- Browser overlays must align with pane content after every split, resize, zoom,
  and unzoom transition.
- Single-pane and zoomed-pane layouts retain their normal padding behavior.
- No Chromium, Roamium, webtui, or protocol changes.

#### Verification

1. Build and run debug Wezboard.
2. With default `window_padding`, open a horizontal split. Verify the outer top
   and bottom borders no longer have a full-row-looking gap between the border
   and pane content.
3. Open a vertical split. Verify the outer left and right borders no longer have
   a full-column-looking gap between the border and pane content.
4. Verify the border still does not cover the first or last visible text cell in
   any pane.
5. Run `stty size` in each pane and visually confirm the reported row/column
   count matches the visible content grid. There must be no hidden rows or
   hidden columns.
6. Resize the window. The outer pixel gutter should remain pixel-sized; it must
   not snap back to a full grid row or column.
7. Drag internal split dividers. The resize cursor and hitbox should remain
   aligned with the visible divider.
8. Open a browser pane in a split. Verify the browser overlay aligns with pane
   content and does not overlap the border.
9. Toggle zoom on a pane. Zoomed mode should remove split borders and restore
   normal single-pane padding; unzooming should restore the pixel-gutter outer
   border and internal dividers.
10. Test `window_padding = 0`. The outer pixel gutter should still provide real
    non-overlap border space without shifting content outside the window.
11. Test larger-than-default `window_padding`. Additional configured padding
    should remain outside the pixel border gutter without becoming a second
    full-cell border margin.

#### Pass Criteria

The multi-pane border appears as a normal thin border around the pane area:
there is real non-overlap space for the border, but no full terminal-cell gutter
on any outer side. PTY size, visible content grid, mouse hitboxes, split resize,
and browser overlays all agree.

#### Partial Criteria

The outer full-cell gutter is removed visually, but one coordinate-dependent
path still needs follow-up, such as browser overlay placement, split-drag
hitboxes, or a resize transition. Record the failing path and design the next
experiment around that exact geometry source.

#### Failure Criteria

- The border overlaps terminal text or browser content.
- The outer top/bottom border still consumes a full terminal row.
- The outer left/right border still consumes a full terminal column.
- PTY size disagrees with the visible content grid.
- Internal split dragging regresses.
- Single-pane or zoomed-pane padding changes unexpectedly.
