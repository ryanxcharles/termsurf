+++
status = "open"
opened = "2026-05-23"
+++

# Issue 786: Grid-Native Split Borders

## Goal

Implement split pane borders that do not overlap terminal content and do not use
pixel-level presentation insets. The active pane should be easy to identify via
a complete border outline, while the terminal grid, PTY size, browser overlays,
mouse mapping, and split dragging remain cell-consistent.

## Background

[Issue 777](../0777-split-border-overlap/README.md) attempted to make split
borders behave like a real margin instead of painting over pane content. That
approach used a presentation-layer pixel inset around pane rendering.

[Issue 785](../0785-split-border-bottom-row/README.md) found that the
presentation-inset model could hide the bottom terminal row. The root cause was
architectural: Wezboard's mux split tree uses terminal cells as its layout
currency, while Issue 777 tried to add pixel border space after the grid had
already been allocated. The rollback in Issue 785 restored the older
grid-consistent behavior, accepting that borders may again sit over pane
content.

The next implementation should work with Wezboard's architecture instead of
fighting it. Borders should be represented in grid space, not as pixel insets
added during rendering.

## Analysis

Wezboard's split layout is cell-based.

In `wezboard/mux/src/tab.rs`, split children are positioned with a shared
one-cell divider:

```rust
fn left_of_second(&self) -> usize {
    match self.direction {
        SplitDirection::Horizontal => self.first.cols as usize + 1,
        SplitDirection::Vertical => 0,
    }
}

fn top_of_second(&self) -> usize {
    match self.direction {
        SplitDirection::Horizontal => 0,
        SplitDirection::Vertical => self.first.rows as usize + 1,
    }
}
```

`iter_splits()` exposes those divider cells as `PositionedSplit`, and
`render/split.rs` already paints and hit-tests them as split UI regions. That
means shared internal dividers already match the mux's native model.

What the current model lacks is an outer border around the visible pane area.
That outer border is important because the active pane needs a complete outline,
not only the internal edges shared with neighboring panes.

Two possible grid-native models were considered:

1. **Full border grid per pane.**

   Each pane owns a top, bottom, left, and right border cell around its content.
   This is conceptually simple, but adjacent panes create double borders unless
   the implementation collapses neighboring borders into a shared divider. Once
   that collapse is added, this approach has effectively reinvented shared
   dividers with more ownership complexity.

2. **Pane outer perimeter plus shared internal dividers.**

   Keep the mux's existing shared one-cell split dividers for internal pane
   boundaries. Add grid-native outer perimeter border cells around the tab or
   visible split area so panes can be outlined all the way around. Pane content
   remains a cell rect that the PTY actually owns.

The second model better matches Wezboard. It extends the split tree's existing
cell-divider model instead of layering per-pane borders over it.

## Proposed Solution

Implement grid-native split borders using:

- existing shared split divider cells between adjacent panes;
- new outer perimeter border cells around the visible split area;
- active-pane-aware coloring for both shared dividers and perimeter edges;
- no pixel inset, no temporary `RenderableDimensions` shrink, and no post-layout
  content clipping.

Conceptually:

```text
tab grid
  perimeter border cells
  pane content cells
  shared divider cells between adjacent panes
```

The PTY should only ever receive the content grid that is actually visible. If a
future design needs border cells to consume additional rows or columns, that
must happen in the mux/layout cell model before PTY dimensions are assigned.
Rendering must not silently hide rows or columns after the fact.

## Constraints

- Do not reintroduce the Issue 777 pixel presentation-inset model.
- Do not shift pane rendering by pixel border widths.
- Do not shrink `RenderableDimensions` inside `paint_pane()`.
- Do not hide terminal rows or columns under border paint.
- Keep split layout, mouse mapping, browser overlays, split hit regions, and PTY
  dimensions in one cell-coordinate system.
- The active pane must have a complete visual outline, including outer edges.
- Shared internal dividers are preferable to duplicated per-pane borders.

## Open Questions

- Should the perimeter border apply to the whole tab grid, each top-level split
  subtree, or each visible pane's exterior edges?
- How should active-pane coloring work for shared dividers between active and
  inactive panes?
- Should `split_border_width` be reinterpreted as a cell-count option, or should
  grid-native borders use a separate configuration option?
- What is the minimum viable implementation that restores a complete active
  outline while keeping PTY dimensions truthful?

For Experiment 1, answer these conservatively:

- apply one outer perimeter around the entire visible split area, not around
  each split subtree or each leaf pane;
- collapse interior edges into the existing shared `PositionedSplit` dividers;
- prefer the active pane's color on shared dividers so the active outline is
  continuous;
- make grid-native borders one cell thick;
- reserve those border cells in mux/layout before PTY dimensions are assigned;
- keep `PositionedPane.left/top/width/height` as the pane content rect and add
  companion border geometry for the reserved border cells;
- do not reinterpret `split_border_width` yet.

## Experiments

### Experiment 1: Reserve One-Cell Grid Borders

#### Description

Implement the first true grid-native split border model:

- reserve one-cell outer border space in the mux/layout cell model;
- keep existing one-cell shared internal dividers between adjacent panes;
- assign PTYs only the inner content grid that remains after border/divider
  cells are reserved;
- make the active pane visually outlined on all four sides;
- do not reintroduce pixel insets, temporary render-dimension shrinkage, or
  post-layout clipping.

This experiment intentionally ignores `split_border_width` for the new
grid-native behavior. Borders are one cell thick because Wezboard's split layout
already uses cells as its currency. The existing `split_border_width` config
field remains for compatibility and for the old pre-grid-native rendering path,
but it is not the shape control for this experiment.

PTY dimensions are allowed to change in this experiment. That is the point of
the grid-native model: if border cells consume rows or columns, the PTY must
receive the smaller truthful content size through the normal mux/layout resize
path. Hidden rows are unacceptable; smaller honest PTY dimensions are
acceptable.

#### Non-Negotiable Invariants

- Do not use pixel presentation insets.
- Any PTY row or column changes must come from the normal mux/layout cell
  allocation path, not from paint.
- Do not shrink `RenderableDimensions` inside `paint_pane()`.
- Do not hide terminal rows or columns under border paint.
- Do not break existing shared split divider hit regions or split dragging.
- Browser overlays must remain aligned to pane content.
- Mouse clicks, selection, and terminal mouse forwarding must keep targeting the
  visible terminal cells.
- Single-pane and zoomed-pane behavior remain unchanged: no split outline is
  drawn.

#### Changes

1. **Audit the current split geometry.**

   Confirm the current rollback state:

   ```bash
   rg "pane_render_geometry|PaneRenderGeometry|split_border_width_physical|content_pixel_width|content_pixel_height|content_origin_x|content_origin_y|draw_divider|hit_thickness" \
     wezboard/wezboard-gui
   ```

   Expected: no matches.

   Inspect:
   - `wezboard/mux/src/tab.rs::SplitDirectionAndSize::{left_of_second,top_of_second}`;
   - `wezboard/mux/src/tab.rs::iter_panes()`;
   - `wezboard/mux/src/tab.rs::iter_splits()`;
   - `wezboard/wezboard-gui/src/termwindow/render/split.rs`;
   - `wezboard/wezboard-gui/src/termwindow/render/pane.rs::paint_pane_border`;
   - `wezboard/wezboard-gui/src/termwindow/render/paint.rs`.

   The expected finding is that internal split dividers are already represented
   as shared one-cell grid regions and should be reused.

2. **Reserve one-cell perimeter space in mux/layout.**

   Add a grid-native border reservation before leaf pane PTY dimensions are
   assigned. The layout should produce two concepts:
   - an outer visible pane rect, including border/divider cells;
   - an inner content rect, assigned to the pane's PTY/renderable dimensions.

   Reserve a one-cell perimeter around visible split layouts when more than one
   pane is visible and the pane is not zoomed. The outer perimeter is real grid
   space. It may reduce the content rows/columns available to PTYs, and that
   reduction must be delivered through the normal pane resize path.

   The reservation belongs in `wezboard/mux/src/tab.rs`, at the layout
   computation entry point before leaf pane sizes are propagated to PTYs. The
   expected shape is:
   - if one pane is visible or the pane is zoomed, keep the existing layout;
   - if more than one pane is visible, subtract one cell from each side of the
     visible split area;
   - run the existing split-tree positioning on the resulting inner rect;
   - assign leaf pane PTYs the content rects produced by that existing split
     layout.

   The perimeter wraps the entire visible split area exactly once. Do not add a
   separate perimeter around each subtree or leaf pane. Leaf pane outlines are
   composed from the one outer perimeter plus existing shared internal divider
   cells.

   The existing internal split divider cells should remain shared one-cell grid
   regions. Do not create duplicated per-pane borders between adjacent panes.

   If a pane or split subtree is too small to reserve a border without reducing
   content to zero rows or columns, do not draw that border segment. Preserve at
   least one content row and one content column for every visible pane.

3. **Extend positioned geometry to expose border/content rects.**

   Keep `PositionedPane.left/top/width/height` as the pane content rect. This
   preserves the existing PTY resize path, mouse mapping, selection, terminal
   mouse forwarding, and browser overlay positioning as content-cell operations.

   Add companion geometry for the reserved border cells. This can be a new field
   on `PositionedPane` or a parallel structure returned with positioned panes,
   but it must be produced by mux/layout, not rediscovered from pixels in the
   renderer. The companion geometry should expose:
   - pane outer rect in cells, including adjacent border/divider cells;
   - which edge cells are perimeter border cells;
   - which internal edge cells are existing shared `PositionedSplit` dividers.

   After this experiment, `mouseevent.rs` should not need new border-offset
   arithmetic. If the implementation requires subtracting border cells in mouse
   mapping, that is evidence that `PositionedPane` stopped representing the
   content rect and the design should be rechecked.

4. **Define active-pane border ownership in grid cells.**

   Use visible `PositionedPane` values from `get_panes_to_render()` and visible
   `PositionedSplit` values from `get_splits()`.

   For each visible pane, determine which of its four sides should be drawn:
   - if the side touches a shared split divider, that side is represented by the
     existing divider;
   - if the side touches the outer visible split area/window edge, draw a new
     one-cell perimeter segment;
   - do not draw duplicate borders on both sides of a shared divider.

   The active pane should have a continuous visual outline. When a shared
   divider is adjacent to the active pane, draw that divider using the active
   pane border color.

   Note: a shared divider belongs to both adjacent panes. When the active pane
   colors a shared divider edge, the inactive pane on the other side will also
   appear to have that focused-color edge. That is acceptable. The active pane
   is still the only pane with a fully-focused-colored outline.

5. **Render outer perimeter segments in `render/pane.rs` or a new helper.**

   Add a helper that draws one-cell-thick perimeter segments for visible pane
   exterior edges. The helper should work in cell units:
   - horizontal border segment height = `cell_height`;
   - vertical border segment width = `cell_width`;
   - segment coordinates derive from `PositionedPane.left/top/width/height` and
     the existing padding/tab-bar/OS-border origin calculation;
   - segment color uses `focused_split_border_color` for the active pane and
     `unfocused_split_border_color` otherwise, falling back to `palette.split`.

   This helper must draw only in cells reserved as border cells by the
   mux/layout model. It must not draw over pane content cells.

6. **Update shared divider coloring without changing hit regions.**

   In `render/split.rs`, keep the existing `paint_split()` signature and
   `UIItem` hit region geometry.

   Update only the color choice so a divider adjacent to the active pane is
   drawn with the focused border color. If determining adjacency inside
   `paint_split()` is awkward, pass enough context from `render/paint.rs` to
   choose the color without changing split layout or hit testing.

   Remove the current `split_border_width == 0` gate around `paint_split()`. In
   the grid-native model, shared dividers are structural grid cells and must be
   rendered regardless of the legacy pixel-width option.

7. **Reconcile or replace existing pixel border paths.**

   The current rollback state still has `paint_pane_border()`, which draws
   `split_border_width`-pixel-thick rectangles at pane edges. That path overlaps
   with this experiment's new grid-native perimeter border.

   Pick one explicit resolution:
   - remove `paint_pane_border()` if the new perimeter helper fully replaces it;
     or
   - gate it off when grid-native borders are active; or
   - repurpose it as the one-cell grid-native perimeter renderer.

   Do not leave both the legacy pixel border and the new grid-native perimeter
   active at the same time.

   If repurposing `paint_pane_border()`, its thickness must come from cell
   metrics, not `split_border_width`: vertical segments use `cell_width`, and
   horizontal segments use `cell_height`. It should continue to short-circuit
   when `num_panes <= 1` or the pane is zoomed.

8. **Wire the render order in `render/paint.rs`.**

   Keep the existing order that paints pane content and overlays safely.

   Add perimeter border drawing after pane backgrounds/content are painted and
   before modal/tab/window border layers as appropriate. Shared split dividers
   should continue to be painted through the split path.

   The render order should make the outline visible without obscuring terminal
   content rows/columns. Because this experiment reserves border cells before
   PTY dimensions are assigned, no content should be painted into those cells.

9. **Leave `split_border_width` alone.**

   Do not reinterpret `split_border_width` as a cell count in this experiment.
   Do not remove it. Do not add a new config option yet.

   The result should state explicitly that Experiment 1 implements a one-cell
   grid-native outline independent of `split_border_width`.

#### Verification

1. Build Wezboard:

   ```bash
   scripts/build.sh wezboard
   ```

2. Configure visible colors:

   ```lua
   config.focused_split_border_color = "#7dcfff"
   config.unfocused_split_border_color = "#565f89"
   config.split_border_width = 4
   ```

   `split_border_width` should not control the new grid-native border thickness
   in this experiment.

3. Single-pane and zoomed panes:
   - open a single pane and confirm no split outline is drawn;
   - open a split, zoom one pane, and confirm the zoomed pane does not get a
     split outline;
   - unzoom and confirm outlines return.

4. PTY size truthfulness:
   - record `stty size` in a single pane;
   - open a split and record `stty size` in both panes;
   - confirm any row/column reduction caused by one-cell border reservation is
     reported by `stty size`;
   - confirm the visible terminal grid matches the reported size.

   Opening the first split is expected to shrink pane content by more than the
   new pane's share alone: the whole split area also loses two columns and two
   rows to the outer perimeter, plus one row or column to the internal shared
   divider.

5. Active pane outline:
   - create a two-pane horizontal split;
   - focus each pane in turn;
   - confirm the active pane has a complete visual outline, including the
     outside window edge and the shared divider edge;
   - repeat with a vertical split.

6. Nested splits:
   - create at least three panes with both horizontal and vertical splits;
   - focus each pane in turn;
   - confirm every active pane can be visually identified by a complete outline;
   - confirm shared internal dividers are not double-thick.

7. Bottom row and edge cells:
   - run `stty size` in split panes;
   - print content on the last visible row and rightmost column;
   - confirm the bottom row and rightmost column remain visible;
   - confirm Codex or Neovim bottom status lines are visible.

8. TUI resize transitions:
   - run a TUI such as Codex, Neovim, or htop in a pane;
   - open a split and confirm the TUI receives the resize and redraws cleanly at
     the smaller truthful content size;
   - close the split and confirm the TUI grows back cleanly;
   - zoom and unzoom a pane and confirm redraws remain clean.

9. Mouse and split dragging:
   - drag shared split dividers;
   - click/select text near pane edges;
   - run a terminal mouse app and confirm mouse forwarding targets visible
     cells;
   - confirm border drawing did not steal terminal-cell clicks.

   Confirm `mouseevent.rs` did not need new border-offset arithmetic for normal
   pane content clicks. `PositionedPane` should still identify content cells.

10. Browser overlays:
    - open a browser pane with `web`;
    - verify the overlay still aligns with the terminal pane;
    - resize splits and verify the overlay follows its pane;
    - open a split next to an existing browser pane and verify the overlay
      resizes to the new content rect;
    - close the split and verify the overlay grows back to the original content
      rect.

11. `split_border_width` compatibility:
    - test with `split_border_width = 0`;
    - test with `split_border_width = 4`;
    - confirm the visible grid-native outline is identical for both values.

    Any visible difference between `0` and `4` indicates leftover legacy pixel
    border behavior and should be fixed before the experiment passes.

#### Pass Criteria

The experiment passes if active split panes have a complete one-cell visual
outline, PTY dimensions truthfully match the visible content grid, no terminal
rows or columns are hidden, shared dividers remain single-cell and draggable,
browser overlays remain aligned, and `scripts/build.sh wezboard` passes.

#### Partial Criteria

The experiment is Partial if the active-pane outline works for simple splits but
one secondary case needs follow-up, such as nested split coloring or an outer
edge segment missing in a complex layout. Partial is not acceptable if terminal
content is hidden or mouse/split dragging regresses.

#### Failure Criteria

The experiment fails if:

- pixel inset geometry is reintroduced;
- `paint_pane()` shrinks `RenderableDimensions`;
- any terminal row or column is hidden;
- PTY dimensions disagree with the visible content grid;
- shared dividers become double-thick;
- split dragging, mouse mapping, selection, terminal mouse forwarding, or
  browser overlay positioning regress;
- `split_border_width` is reinterpreted without an explicit follow-up design.

**Result:** Partial

The experiment successfully moved the border model into real grid layout space:
multi-pane layouts reserve cells before PTY sizing, pane content dimensions are
truthful, and the bottom/right terminal content is no longer hidden by border
paint.

However, the visual rendering is not acceptable. The implementation fills the
entire reserved border cell with the focused/unfocused border color. That is not
the intended design. The reserved cells should create real layout space, but the
cells themselves should visually read as normal background space. The actual
border line should still be drawn as a thin pixel border inside or along the
reserved border space, similar to the old pixel border rendering but no longer
overlapping terminal content.

#### Conclusion

Keep the grid-reserved layout direction. The next experiment should preserve the
truthful PTY/content sizing from this experiment, but change the paint model so
reserved border cells are background-colored spacing and the visible border is a
thin pixel line drawn within that spacing. Full-cell border fills are the wrong
visual treatment.

### Experiment 2: Pixel Lines in Reserved Cells

#### Description

Keep the grid-reserved layout from Experiment 1, but fix the visual rendering.

Experiment 1 proved the correct sizing model: border space must be reserved in
the mux/grid layout before PTY dimensions are assigned. Its mistake was treating
the reserved cells themselves as the visible border by filling entire cells with
focused/unfocused border colors.

The intended model is:

- reserved border cells are real layout space;
- pane content never renders into those cells;
- reserved border cells visually read as normal background spacing;
- the visible border is a thin pixel line drawn inside or along that reserved
  spacing;
- `split_border_width` may control the pixel thickness of that line, but it must
  not control the number of reserved cells.

This restores the visual feel of the old pixel border while keeping the
architecture fixed: the border no longer overlays terminal content because it
now has real grid space to live in.

#### Non-Negotiable Invariants

- Preserve Experiment 1's truthful PTY/content sizing.
- Do not remove the one-cell mux/layout reservation.
- Do not reintroduce paint-time `RenderableDimensions` shrinkage.
- Do not draw full-cell focused/unfocused border fills.
- Do not draw border pixels over terminal content cells.
- Do not change mouse mapping, selection, terminal mouse forwarding, or browser
  overlay content coordinates.
- Keep shared split divider hit regions and dragging behavior unchanged.
- Single-pane and zoomed-pane behavior remain unchanged: no split outline is
  drawn.

#### Changes

1. **Preserve the Experiment 1 layout model.**

   Do not change the mux/layout reservation unless a bug is discovered while
   implementing the paint fix.

   `PositionedPane.left/top/width/height` must continue to mean the pane content
   rect. `PositionedPaneBorder` or its equivalent companion geometry continues
   to describe reserved border cells.

   The first implementation check is:

   ```bash
   rg "PositionedPaneBorder|grid_border|first_split_layout|split_layout_size" \
     wezboard/mux/src/tab.rs
   ```

   Expected: Experiment 1's grid reservation code remains present.

   Also check the current paint state before editing it:

   ```bash
   rg "paint_pane_border|paint_split" \
     wezboard/wezboard-gui/src/termwindow/render
   ```

   Expected: the current paint code is Experiment 1's full-cell fill behavior
   that this experiment will replace.

2. **Paint reserved cells as background spacing.**

   In `wezboard/wezboard-gui/src/termwindow/render/pane.rs`, update
   `paint_pane_border()` so it does not fill the full reserved cell with border
   color.

   `paint_pane_border()` must continue to return early when `num_panes <= 1` or
   `pos.is_zoomed`.

   Reserved cells should use the window background color, not per-pane
   backgrounds. This creates a consistent gutter regardless of per-pane palette
   or terminal background changes. The existing window-background paint path
   should already cover the reserved cells after Experiment 1; verify that
   first. Only add an explicit fill if the reserved cells appear black,
   unpainted, or otherwise inconsistent with the surrounding window gutter.

   The expected visual result is a normal background-colored gutter around split
   panes, not a one-cell-thick colored block.

3. **Draw a thin pixel border inside the reserved spacing.**

   Still in `paint_pane_border()`, draw the actual visible outline as thin pixel
   rectangles positioned within the reserved border cells:
   - top edge: a horizontal pixel line inside the top reserved cell;
   - bottom edge: a horizontal pixel line inside the bottom reserved cell;
   - left edge: a vertical pixel line inside the left reserved cell;
   - right edge: a vertical pixel line inside the right reserved cell.

   The line color should remain:
   - `focused_split_border_color` for the active pane;
   - `unfocused_split_border_color` for inactive panes;
   - fallback to `palette.split`.

   Use
   `self.config.split_border_width.evaluate_as_pixels(DimensionContext { ... })`
   to convert the configured `Dimension` to physical pixels. This is the same
   conversion the pre-Issue-777 pixel border used.

   `split_border_width = 0` means no visible border line is drawn. The reserved
   gutter cells still exist structurally, but the border line itself is hidden.
   Users should not get a surprise fallback line when they explicitly set zero.

   For `split_border_width > 0`, clamp the line thickness so it cannot recreate
   Experiment 1's full-cell fill:
   - vertical line width `<= max(1, cell_width - 2px)`;
   - horizontal line height `<= max(1, cell_height - 2px)`.

   Clamp silently. The clamp must guarantee at least one pixel of visible gutter
   on each side of the line whenever the cell is large enough to allow it.

4. **Update shared divider rendering to match the new visual model.**

   In `wezboard/wezboard-gui/src/termwindow/render/split.rs`, stop filling the
   whole shared divider cell with focused/unfocused border color.

   Shared divider cells are also reserved layout space. They should visually be
   background-colored spacing with a thin pixel line drawn through them.

   Keep the `UIItem` hit region geometry unchanged. Only the paint rectangle
   should shrink from full-cell fill to thin line.

   Preserve Experiment 1's shared-divider color logic: focused color when the
   divider is adjacent to the active pane, otherwise unfocused color, with
   fallback to `palette.split`. Only the paint geometry changes.

5. **Place lines consistently within reserved cells.**

   Pick one placement rule and apply it consistently:
   - preferred: center each thin line in its reserved cell;
   - acceptable: place each line against the content-facing side of the reserved
     cell.

   Document the chosen rule in the result. Centering is preferred because it
   gives both panes visually equal breathing room around shared dividers.

   The chosen placement rule applies to both perimeter segments in
   `paint_pane_border()` and shared dividers in `paint_split()`.

   If centering, round the centered offset to whole pixels to avoid fuzzy
   anti-aliased lines.

6. **Do not change layout, PTY sizing, or input mapping.**

   This experiment is paint-only unless a direct paint bug requires a tiny
   geometry metadata adjustment. In particular:
   - no changes to `mouseevent.rs` should be necessary;
   - browser overlay coordinates should continue to use pane content positions;
   - `PositionedPane.left/top/width/height` should remain the content rect.

#### Verification

1. Build Wezboard:

   ```bash
   scripts/build.sh wezboard
   ```

2. Use visible split-border config:

   ```lua
   config.focused_split_border_color = "#7dcfff"
   config.unfocused_split_border_color = "#565f89"
   config.split_border_width = 4
   ```

3. Two-pane visual check:
   - open a horizontal split;
   - focus each pane;
   - confirm the active pane is outlined by a thin pixel line, not a full-cell
     colored block;
   - repeat with a vertical split.

4. Reserved-cell spacing check:
   - inspect the area between pane content and the border line;
   - confirm reserved cells visually read as normal background spacing;
   - confirm pane text does not touch or sit under the border line.

5. Nested split visual check:
   - create at least three panes with horizontal and vertical nesting;
   - focus each pane;
   - confirm shared dividers are thin lines and not full-cell fills;
   - confirm the active pane remains easy to identify.

6. `split_border_width` behavior:
   - test with `split_border_width = 0`;
   - confirm no visible border line is drawn while reserved gutter cells remain
     in place;
   - test with `split_border_width = 4`;
   - test with a larger value such as `8`;
   - test with an oversized value such as `100`;
   - confirm the value changes only the pixel-line thickness, not the number of
     reserved cells or PTY rows/columns;
   - confirm oversized values are clamped well below full-cell thickness and
     visible gutter spacing remains on both sides of the line.

7. PTY truthfulness regression check:
   - run `stty size` in split panes;
   - print content on the last visible row and rightmost column;
   - confirm the bottom row and rightmost column remain visible.

8. Mouse and split dragging:
   - drag shared split dividers;
   - click/select text near pane edges;
   - run a terminal mouse app and confirm mouse forwarding targets visible
     content cells;
   - confirm the thin border paint does not steal terminal-cell clicks.

9. Browser overlays:
   - open a browser pane with `web`;
   - open a split next to it;
   - confirm the browser overlay remains aligned to the content rect;
   - confirm the thin border appears outside the browser content, not underneath
     it.
   - confirm the browser overlay position is unchanged from Experiment 1; only
     the gutter/border paint should change.

10. Single-pane and zoomed-pane behavior:
    - confirm no split outline is drawn in a single-pane tab;
    - zoom a split pane and confirm the split outline disappears;
    - unzoom and confirm the thin-line outline returns.

#### Pass Criteria

The experiment passes if the grid-reserved layout remains intact, PTY dimensions
truthfully match visible content cells, reserved cells look like background
spacing, visible borders are thin pixel lines, `split_border_width` affects only
line thickness, and `scripts/build.sh wezboard` passes.

#### Partial Criteria

The experiment is Partial if the full-cell fill is removed and the thin-line
model works for simple splits, but one secondary visual issue remains, such as
line centering in nested splits or an imperfect clamp for very large
`split_border_width` values. Partial is not acceptable if terminal content is
hidden again.

#### Failure Criteria

The experiment fails if:

- full reserved cells are still filled with border color;
- terminal rows or columns are hidden;
- PTY dimensions stop matching visible content cells;
- `RenderableDimensions` shrinkage is reintroduced;
- browser overlays overlap or hide borders;
- split dragging, mouse mapping, selection, or terminal mouse forwarding
  regresses;
- `split_border_width` changes reserved-cell count instead of only pixel-line
  thickness.

**Result:** Fail

The implementation removed the full-cell color fill, but the resulting border is
still not a border. The screenshot from manual testing shows three concrete
failures:

- border segments are disconnected and do not form a continuous rectangle around
  the active pane;
- the thin lines intersect at visibly wrong positions, so corners and shared
  divider joins look broken;
- pane content is still visually underneath or too close to the border region,
  so the border does not read as outside the pane content;
- the active highlight does not wrap all the way around the pane.

The user-facing requirement is simpler than the implementation became: a border
should be a solid line around the outside of the pane, like the pre-Issue-777
pixel border, except it must no longer cover terminal content.

#### Conclusion

Experiment 2 failed because it still treated the outline as independent edge
fragments derived from per-pane perimeter cells and shared split cells. That
fragment model is too error-prone: each edge chooses its own local coordinates,
then shared dividers and perimeter segments are expected to meet perfectly. In
practice they do not. The result is disconnected lines, wrong intersections, and
an outline that does not behave like one coherent border rectangle.

The likely root problem is that the paint code is drawing border pieces from
multiple coordinate models:

- `paint_pane_border()` draws top/bottom/left/right perimeter pieces from
  `PositionedPaneBorder`;
- `paint_split()` draws internal shared divider pieces from `PositionedSplit`;
- pane content is rendered from `PositionedPane.left/top/width/height`;
- the implementation assumes those independently-painted pieces will visually
  join into a single active-pane outline.

That assumption is wrong. A proper active border needs one owner and one
rectangle. The next fix should stop trying to synthesize a pane outline from
separate split/divider fragments.

Promising next approaches:

1. **Single active-pane outline painter.** Keep the grid-reserved layout, but
   draw one continuous pixel rectangle from the active pane's outer border rect.
   The painter owns all four sides and all four corners. Shared dividers remain
   drag hit regions, but the active outline is not assembled from
   `paint_split()` fragments.
2. **Outer-rect metadata for each pane.** Extend the Experiment 1 geometry so
   every visible pane exposes one authoritative outer rect that includes its
   reserved border/divider cells. Use that rect to draw the active border as a
   continuous rectangle, then optionally draw inactive divider/background
   accents separately.
3. **Return closer to pre-Issue-777 border math.** Reuse the old pixel-border
   rectangle calculation, but feed it the grid-reserved outer rect instead of
   the content rect. This preserves the old visual semantics while solving the
   original overlap problem by moving the rectangle outside content.

The next experiment should be framed around a single continuous active-pane
border rectangle. It should not try to make `paint_pane_border()` perimeter
segments and `paint_split()` shared divider segments line up by convention.

### Experiment 3: Restore One-Rectangle Border Paint

#### Description

Fix the visual model by returning to the pre-Issue-777 border concept: draw one
continuous pixel rectangle around the active pane.

Experiments 1 and 2 overcomplicated the paint layer. Experiment 1 correctly
reserved grid space so the PTY/content size is truthful. Experiment 2 failed
because it tried to create one border from multiple independently painted edge
fragments. The next attempt should combine the two good ideas:

- keep Experiment 1's grid-reserved layout so borders do not cover terminal
  content;
- restore the old single-rectangle pixel border painting model so the border
  looks like a normal border.

The border should look like it did before Issue 777: one solid, connected pixel
outline around the pane. The difference is that its rectangle is now based on
the grid-reserved outer rect rather than the pane content rect.

#### Non-Negotiable Invariants

- Preserve Experiment 1's mux/layout reservation and truthful PTY sizing.
- Do not shrink `RenderableDimensions` in `paint_pane()`.
- Do not paint over terminal content cells.
- Do not synthesize the active border from `paint_split()` fragments.
- Do not change mouse mapping, selection, terminal mouse forwarding, or browser
  overlay content coordinates.
- Keep split divider `UIItem` hit regions and dragging behavior unchanged.
- Single-pane and zoomed-pane behavior remain unchanged: no split outline is
  drawn.

#### Changes

1. **Preserve the layout model.**

   Do not change `wezboard/mux/src/tab.rs` unless inspection finds a direct bug
   in the outer-rect metadata. `PositionedPane.left/top/width/height` must
   remain the pane content rect. `PositionedPaneBorder` remains the source for
   the reserved outer geometry.

   Pre-check:

   ```bash
   rg "PositionedPaneBorder|grid_border|first_split_layout|split_layout_size" \
     wezboard/mux/src/tab.rs
   ```

   Expected: Experiment 1's grid reservation is still present.

2. **Stop painting shared dividers.**

   In `wezboard/wezboard-gui/src/termwindow/render/split.rs`, make
   `paint_split()` draw nothing. It should keep only the existing `UIItem` hit
   region used for split dragging.

   There is no neutral divider/background aid in this experiment. The active
   pane is identified only by the one-rectangle border drawn by
   `paint_pane_border()`. If inactive dividers later need a subtle visual aid,
   that belongs in a follow-up experiment.

3. **Draw one continuous rectangle from the pane outer rect.**

   In `wezboard/wezboard-gui/src/termwindow/render/pane.rs`, rewrite
   `paint_pane_border()` so it computes one outer pixel rectangle and draws the
   border from that rectangle.

   `paint_pane_border()` continues to return early when `num_panes <= 1`,
   `pos.is_zoomed`, or `pos.is_active == false`. Experiment 3 draws only the
   active pane border.

   Use the pane's `PositionedPaneBorder` metadata to compute:
   - `outer_x`;
   - `outer_y`;
   - `outer_width`;
   - `outer_height`.

   Concrete derivation:
   - if `PositionedPaneBorder` exposes `outer_left`, `outer_top`, `outer_width`,
     and `outer_height`, use those fields directly;
   - otherwise derive the outer rect from `PositionedPane.left/top/width/height`
     plus one cell on each side whose border flag is set.

   Convert the outer rect from cell coordinates to pixels using the same
   padding, tab-bar, and OS-chrome offsets used by pane rendering:
   `padding_left + os_border.left.get()` for x and
   `top_bar_height + padding_top + os_border.top.get()` for y. The border must
   not draw into OS chrome.

   Then draw a single coherent border around that rectangle, conceptually the
   same as the old pre-Issue-777 pixel border:

   ```text
   top:    (outer_x, outer_y, outer_width, thickness)
   bottom: (outer_x, outer_y + outer_height - thickness,
            outer_width, thickness)
   left:   (outer_x, outer_y, thickness, outer_height)
   right:  (outer_x + outer_width - thickness, outer_y,
            thickness, outer_height)
   ```

   The four edges overlap at the corners. Corners are implicit in this geometry
   and must never be computed separately. This matches the pre-Issue-777
   `paint_pane_border()` pattern.

   Do not draw each edge from separate local content-facing calculations. The
   rectangle is the source of truth.

4. **Use old border visual semantics.**

   The visible active border should use `focused_split_border_color`, falling
   back to `palette.split`. `unfocused_split_border_color` is unused in
   Experiment 3 because inactive panes do not draw borders.

   `split_border_width` controls pixel thickness, as it did before Issue 777.
   Negative values are treated as `0`. Clamp positive thickness so it cannot
   exceed half of the reserved cell dimension:
   - max vertical thickness: `floor(cell_width / 2)`;
   - max horizontal thickness: `floor(cell_height / 2)`.

   `split_border_width = 0` means no visible border line.

   Place the border centered in the reserved outer gutter around the content
   rect. For thickness `t` and reserved cell dimension `d`, use
   `floor((d - t) / 2)` as the gutter offset so the line snaps to whole pixels.
   Do not fall back to outer-edge placement unless centering produces a visible
   corner-join artifact; if that fallback is used, document why in the result.

5. **Draw active border only.**

   Draw only the focused/active pane's border. Inactive panes have no visible
   border line in Experiment 3.

   Rationale: active-only border drawing avoids shared-boundary overpaint
   between active and inactive pane rectangles. If inactive pane outlines are
   needed, they should be designed in a later experiment after the active border
   is correct.

6. **Do not change content positioning.**

   Browser overlays, terminal rendering, selection, and mouse forwarding must
   continue to use pane content coordinates. The only visual change should be
   the border paint.

#### Verification

1. Build Wezboard:

   ```bash
   scripts/build.sh wezboard
   ```

2. Two-pane border check:
   - open a horizontal split;
   - focus the left pane, then the right pane;
   - confirm the active pane has one continuous connected border rectangle;
   - confirm corners connect cleanly;
   - repeat with a vertical split.

3. Screenshot regression check:
   - reproduce the layout from the failed Experiment 2 screenshot
     (`screenshot.png`): two side-by-side panes with a visible active-pane
     border failure;
   - confirm there are no broken line intersections;
   - confirm the content is not underneath the border;
   - confirm the active highlight wraps all the way around.

4. Nested split check:
   - create at least three panes with horizontal and vertical nesting;
   - focus each pane;
   - confirm each active pane gets exactly one connected rectangle;
   - confirm the active border is not assembled from visible split fragments.

5. `split_border_width` behavior:
   - test `split_border_width = 0` and confirm no border line is drawn;
   - test `split_border_width = 4` and confirm a normal thin border appears;
   - test an oversized value such as `100` and confirm it clamps without
     covering content or filling the entire reserved cell.

6. PTY/content truthfulness:
   - run `stty size` in split panes;
   - print text on the last visible row and rightmost column;
   - confirm all content remains visible.

7. Mouse and split dragging:
   - drag shared split dividers;
   - click/select text near pane edges;
   - run a terminal mouse app and confirm mouse forwarding targets content cells
     correctly.

8. Browser overlays:
   - open a browser pane with `web`;
   - split next to it;
   - confirm the overlay remains aligned to the content rect;
   - confirm the border is outside the browser content and does not sit under or
     over it.
   - confirm no part of the browser overlay hides the border, and no part of the
     border overlaps the overlay.

9. Single-pane and zoomed-pane behavior:
   - confirm no split outline is drawn in a single-pane tab;
   - zoom a split pane and confirm the outline disappears;
   - unzoom and confirm the active border returns.

#### Pass Criteria

The experiment passes if the active pane border is exactly one connected pixel
rectangle around the outside of the pane, drawn from one
`outer_x/outer_y/outer_width/outer_height` source of truth. There must be no
additional decorative rectangles, inner highlights, double borders, or visible
split-fragment contributions. Content is not underneath the border, PTY
dimensions remain truthful, split dragging/mouse/browser overlays still work,
and `scripts/build.sh wezboard` passes.

#### Partial Criteria

The experiment is Partial if the exactly-one-rectangle active border works for
simple two-pane splits but one secondary case remains imperfect, such as nested
split placement or a browser-overlay visual edge case. Partial is not acceptable
if the active border is disconnected, content is under the border, or inactive
border drawing is introduced.

#### Failure Criteria

The experiment fails if:

- the active pane border is still assembled from disconnected fragments;
- border corners do not connect cleanly;
- terminal content appears underneath the border;
- `paint_split()` remains visually responsible for the active-pane border;
- inactive pane borders are introduced in this experiment;
- PTY dimensions stop matching visible content cells;
- browser overlay, mouse mapping, selection, or split dragging regresses.
