# Issue 602: Pink Texture Overlay

## Goal

Render a pink GPU quad at the grid coordinates specified by `web`, entirely in
Zig. When the user runs `web https://example.com` in a Ghost pane, a pink
rectangle appears at the viewport coordinates. Resize updates the rectangle.
Disconnect clears it.

## Background

Issue 601 proved XPC works from Zig — Ghost can receive `set_overlay` messages
from `web` with grid coordinates, URL, and profile. But it doesn't do anything
with them yet. This issue makes the overlay visible.

In ts5, the pink texture was Issue 505. The overlay pipeline, Surface methods,
and C API were all built in that series (Issues 505–512). But ts5 built
everything in a mix of Swift and Zig. Ghost starts fresh from upstream Ghostty
and builds it all in Zig.

### What Ghost has (from upstream Ghostty)

Ghost inherited upstream Ghostty's renderer, which has no overlay support:

**Shader pipelines** (`ghost/src/renderer/metal/shaders.zig`):

- `bg_color` — full-screen background
- `cell_bg` — cell backgrounds
- `cell_text` — terminal text
- `image` — Kitty image protocol
- `bg_image` — background image

No `pink_overlay` or `overlay` pipeline.

**Render loop** (`ghost/src/renderer/generic.zig`, `drawFrame()`):

1. Background (bg_color or bg_image)
2. Kitty images below backgrounds
3. Cell backgrounds
4. Kitty images below text
5. Cell text
6. Kitty images above text
7. Debug overlay (hyperlink highlights, semantic prompts — not content)
8. Post-processing (custom shaders)

No overlay render step for external content.

**Surface** (`ghost/src/Surface.zig`):

- No pane ID or UUID field
- No overlay state (coordinates, IOSurface)
- No `setOverlay()` / `clearOverlay()` methods
- Identified only by memory address

**Surface management** (`ghost/src/App.zig`):

- `surfaces: ArrayListUnmanaged` — flat list
- Lookup by pointer comparison only (no ID-based lookup)
- `draw_mutex` exists on the renderer for thread-safe state updates

**C API** (`ghost/src/apprt/embedded.zig`):

- No overlay-related exports
- No `ghostty_surface_set_overlay` or similar

**Debug overlay** (`ghost/src/renderer/Overlay.zig`):

- CPU-rendered debug visualization (hyperlink highlights, semantic prompts)
- Renders via z2d to a pixel buffer, displayed as an image layer
- Not suitable for GPU-composited content overlays

### What ts5 built (for reference, not to copy verbatim)

ts5 added these TermSurf-specific pieces across Issues 505–512:

**Metal shaders** (`ts5/src/renderer/shaders/shaders.metal`):

- `pink_overlay_vertex` / `pink_overlay_fragment` — solid hot pink quad
- `overlay_vertex` / `overlay_fragment` — IOSurface texture quad

The pink vertex shader converts grid coordinates to pixel coordinates:

```metal
float2 origin = float2(params.grid_col, params.grid_row) * uniforms.cell_size;
float2 size = float2(params.grid_width, params.grid_height) * uniforms.cell_size;
```

The projection matrix already includes padding, so the shader doesn't add it.

**Pipeline definition** (`ts5/src/renderer/metal/shaders.zig`):

```zig
.{ "pink_overlay", .{
    .vertex_fn = "pink_overlay_vertex",
    .fragment_fn = "pink_overlay_fragment",
    .blending_enabled = false,
} },
```

**Params struct** (`ts5/src/renderer/metal/shaders.zig`):

```zig
pub const PinkOverlay = extern struct {
    grid_col: f32 = 0,
    grid_row: f32 = 0,
    grid_width: f32 = 0,
    grid_height: f32 = 0,
    pixel_width: f32 = 0,
    pixel_height: f32 = 0,
};
```

**Renderer state** (`ts5/src/renderer/generic.zig`):

```zig
pink_overlay: shaderpkg.PinkOverlay = .{},
```

**Surface methods** (`ts5/src/Surface.zig`):

- `setOverlay(col, row, width, height)` — sets grid coordinates under
  `draw_mutex`, queues render
- `clearOverlay()` — zeros coordinates, releases IOSurface, queues render

**C API exports** (`ts5/src/apprt/embedded.zig`):

- `ghostty_surface_set_overlay(surface, col, row, width, height)`
- `ghostty_surface_clear_overlay(surface)`

**Pane ID propagation**: Each surface sets `TERMSURF_PANE_ID` as a UUID in the
shell environment, inherited by child processes including `web`.

### What we need to build

1. **Pane ID on Surface** — UUID field, set during creation, propagated as
   `TERMSURF_PANE_ID` env var to child processes
2. **Surface lookup by pane ID** — find a Surface from a UUID string
3. **Pink overlay shader** — vertex + fragment in `shaders.metal`
4. **Pipeline definition** — add `pink_overlay` to `shaders.zig`
5. **Overlay params struct** — grid coordinates in `shaders.zig`
6. **Overlay state on renderer** — params field in `generic.zig`
7. **Render step in drawFrame()** — draw the pink quad after text/images
8. **Surface methods** — `setOverlay()` / `clearOverlay()` with `draw_mutex`
9. **Wire XPC to Surface** — `handleSetOverlay` looks up surface, calls
   `setOverlay()`; disconnect calls `clearOverlay()`

### Key technical details from ts5

**Grid-to-pixel conversion**: The projection matrix includes padding. The vertex
shader multiplies grid coordinates by `uniforms.cell_size` to get pixel
position. No padding adjustment needed in the shader.

**Thread safety**: XPC callbacks arrive on a background queue. `setOverlay()`
locks `draw_mutex` before writing coordinates. `drawFrame()` holds `draw_mutex`
during rendering. This serializes access.

**Resize**: Cell size is determined by font metrics and doesn't change on
terminal resize. Grid dimensions and padding change. The `web` TUI sends a new
`set_overlay` message with updated coordinates on resize. The overlay position
stays correct because it's derived from cell size (stable) and grid position
(updated by `web`).

## Ideas for experiments

1. **Pane ID and surface lookup** — Add UUID to Surface, propagate as env var,
   implement lookup by pane ID. Proves the XPC handler can find the right
   surface.

2. **Pink overlay rendering** — Add the shader, pipeline, renderer state, and
   render step. Wire `handleSetOverlay` to call `setOverlay()` on the looked-up
   surface. Pink rectangle appears at the correct grid coordinates.

3. **Resize and cleanup** — Verify resize updates the rectangle dimensions and
   disconnect clears it.
