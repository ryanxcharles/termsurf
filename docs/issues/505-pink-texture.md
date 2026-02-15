# Issue 505: Pink Texture Overlay

## Background

Issue 504 built the `web` TUI chrome — a ratatui-based terminal application that
draws a URL bar, viewport border, and status bar inside a Ghostty pane. The
viewport is the region where the browser content will eventually render. The
`web` TUI knows the exact pixel coordinates and size of its viewport (it prints
them inside the viewport itself).

This issue is the next step: **render a solid pink GPU texture where the browser
viewport is supposed to be.** No browser, no Chromium, no IPC — just a pink
rectangle rendered by Ghostty's Metal pipeline at the correct position inside
the terminal pane. Pink because it's unmistakably visible.

This is the foundational experiment for browser pane rendering. If we can
overlay a texture at arbitrary pixel coordinates inside a Ghostty pane, we can
overlay anything — including Chromium's IOSurface output.

## Prior Art

### What Previous Generations Taught Us

#### ts1 (Ghostty + WKWebView)

ts1 used WKWebView as a native macOS NSView overlaid on the terminal pane. No
GPU texture compositing was needed — WKWebView handled its own rendering. The
overlay was positioned using NSView frame coordinates. This approach only worked
on macOS and is not applicable to ts5's in-process Chromium strategy.

#### ts3 (WezTerm + out-of-process CEF via XPC)

ts3 used wgpu to composite CEF-rendered IOSurfaces into WezTerm's terminal
panes. Key lessons:

- **Viewport calculation was the hardest part.** Grid cells → physical pixels →
  logical DIP → CEF dimensions. Getting this chain right took many experiments.
- **sRGB double-correction was a major bug.** CEF outputs sRGB pixel data. If
  the texture view is declared as linear (`Bgra8Unorm`), the GPU applies gamma
  correction again, washing out colors. Fix: use `Bgra8UnormSrgb` so the GPU
  knows the data is already sRGB-encoded.
- **Dimension mismatch during resize** caused visual glitches. When the pane
  resizes, the old texture doesn't match the new viewport. ts3 logged mismatches
  and had debounce logic, but never fully solved dynamic resize.
- **`set_viewport()` was the compositing mechanism.** Rather than positioning
  the texture with vertex coordinates, ts3 set the GPU viewport to clip the
  render pass to the pane's rectangle. A normalized fullscreen quad then
  stretched to fill the viewport.

#### ts4 (Chromium Content API experiments)

ts4 proved in-process Chromium rendering at 60fps. Key lessons:

- **Metal IOSurface textures are zero-copy.**
  `device.makeTexture(descriptor:
  iosurface: plane:)` creates an MTLTexture
  that directly references IOSurface GPU memory. No pixel copying.
- **Retina scaling: always use physical pixels.** Multiply logical dimensions by
  `backingScaleFactor`. The drawable size is physical, not logical.
- **Metal bytesPerRow alignment.** IOSurface-backed textures require 16-byte row
  alignment: `(width * 4 + 15) & ~15`. Odd widths crash without this.
- **Split-screen via MTLViewport.** Each pane gets its own viewport rectangle
  within the same render pass. Simple, efficient, no extra framebuffers.

### Ghostty's Renderer (ts5)

ts5 is a lightly modified Ghostty fork. The renderer is a multi-pass Metal
pipeline:

```
IOSurface-backed Target (MTLTexture)
  ↓
Background (solid color or image)
  ↓
Kitty images (below text)
  ↓
Cell backgrounds (opaque)
  ↓
Kitty images (below text, above bg)
  ↓
Text (instanced: 4 vertices × N cells)
  ↓
Kitty images (above text)
  ↓
Debug overlay
  ↓
Custom shader passes (optional, ping-pong textures)
  ↓
Present (IOSurface → CALayer.contents)
```

**Key files:**

| File                                     | Purpose                                       |
| ---------------------------------------- | --------------------------------------------- |
| `ts5/src/renderer/generic.zig`           | Main render logic, `drawFrame()` at line 1393 |
| `ts5/src/renderer/metal/Target.zig`      | IOSurface-backed MTLTexture render target     |
| `ts5/src/renderer/metal/Frame.zig`       | Command buffer and completion handler         |
| `ts5/src/renderer/metal/RenderPass.zig`  | Render pass descriptor and encoder            |
| `ts5/src/renderer/metal/Pipeline.zig`    | MTLRenderPipelineState wrapper                |
| `ts5/src/renderer/metal/shaders.zig`     | Pipeline definitions and shader params        |
| `ts5/src/renderer/shaders/shaders.metal` | Metal shader source                           |
| `ts5/src/renderer/size.zig`              | Coordinate systems and size conversions       |
| `ts5/src/renderer/Metal.zig`             | Metal API wrapper, surface size, presentation |

**Coordinate systems** (from `size.zig`):

- **Surface coordinates:** (0,0) = top-left of window, units = physical pixels
  (after DPI scaling).
- **Terminal coordinates:** (0,0) = top-left of grid (padding removed), units =
  physical pixels.
- **Grid coordinates:** (0,0) = top-left of grid, units = cells (column, row).

**Existing pipelines** (from `metal/shaders.zig`):

- `bg_color` — Solid background fill
- `bg_image` — Background image
- `cell_bg` — Cell background colors
- `cell_text` — Text rendering (instanced)
- `image` — Kitty image protocol

Each pipeline has a vertex function, fragment function, and optional vertex
attributes. Adding a new pipeline for the pink overlay follows the same pattern.

## Architecture

The pink texture overlay is a new render pass step inserted into Ghostty's
`drawFrame()` function, after text rendering and before custom shaders. It draws
a solid-color rectangle at specific pixel coordinates within the terminal pane.

```
... existing render steps ...
  ↓
Text (instanced)
  ↓
Kitty images (above text)
  ↓
★ Pink overlay (NEW) — quad rendered at exact pixel coordinates of browser region
  ↓
Custom shader passes (if any)
  ↓
Present
```

### Why After Text

The pink texture must be drawn **on top of** the terminal content. The `web` TUI
renders its chrome (URL bar, status bar, borders) as terminal text. The browser
viewport area contains terminal text too (the coordinates display). The pink
overlay covers the viewport area, obscuring the terminal text beneath it — which
is exactly what a real browser texture would do.

### Positioning Strategy: Custom Escape Sequence

The `web` TUI knows its viewport in **grid coordinates** (column, row, width in
columns, height in rows). Ghostty knows how to convert grid coordinates to
physical pixels (cell size × grid position + padding). The natural communication
channel is the terminal itself — `web` writes to stdout, Ghostty parses it.

**Flow:**

```
web TUI (ratatui)                          Ghostty (renderer)
─────────────────                          ─────────────────
Layout computes viewport
  inner rect: col=1, row=3,
  width=78, height=20
        │
        ▼
Emit OSC escape sequence:
  \x1b]133337;1,3,78,20\x07  ──stdout──▶  Parse OSC 133337
        │                                    │
        │                                    ▼
        │                                  Store overlay rect
        │                                  in grid coordinates
        │                                    │
        │                                    ▼
        │                                  drawFrame() converts
        │                                  grid → physical pixels:
        │                                    x = col × cell_w + pad_left
        │                                    y = row × cell_h + pad_top
        │                                    w = cols × cell_w
        │                                    h = rows × cell_h
        │                                    │
        │                                    ▼
        │                                  Render pink quad at
        │                                  computed pixel rect
        │
Terminal resizes
        │
        ▼
ratatui recomputes layout
  new rect: col=1, row=3,
  width=118, height=40
        │
        ▼
Emit new OSC:
  \x1b]133337;1,3,118,40\x07 ──stdout──▶  Update overlay rect
                                             │
                                             ▼
                                           Next drawFrame() uses
                                           new coordinates
```

**Why grid coordinates, not pixels:**

- Grid coordinates are resolution-independent. No DPI/Retina math in `web`.
- Ghostty already knows cell sizes, padding, and scale factor.
- The conversion happens once per frame in `drawFrame()`, using values Ghostty
  already has.
- If the font size changes (which changes cell size), the overlay automatically
  adjusts without `web` needing to know.

**Why OSC escape sequences:**

- Terminal apps already communicate with their terminal via escape sequences.
- No extra IPC channel needed (no Unix socket, no file, no XPC).
- Ghostty already has an OSC parser — we just add a new code.
- The sequence is emitted on every `web` TUI draw cycle (ratatui's event loop),
  so coordinates stay in sync with the visible chrome.

**OSC format:** `\x1b]133337;{col},{row},{width},{height}\x07`

- `133337` is a private-use OSC code (well above any standard codes).
- The four values are grid coordinates (0-indexed column, 0-indexed row, width
  in columns, height in rows).
- `\x07` (BEL) terminates the sequence (standard OSC terminator).

To clear the overlay (e.g., when `web` exits): `\x1b]133337;\x07` (empty
payload).

## Experiments

### Experiment 1: Dynamic Pink Quad via OSC

Add a new Metal shader pipeline (`pink_overlay`) that draws a solid pink
rectangle. The rectangle's position and size come from an OSC escape sequence
emitted by the `web` TUI. When the terminal resizes, `web` emits updated
coordinates and the pink overlay follows.

This experiment has three parts: the Metal shader, the OSC parser, and the `web`
TUI integration.

#### Changes

##### Part 1: Metal Shader Pipeline

###### `ts5/src/renderer/shaders/shaders.metal`

Add two new shader functions:

**Vertex shader (`pink_overlay_vertex`):**

Takes a uniform buffer with the overlay rectangle (x, y, width, height in
physical pixels) and the projection matrix. Emits 4 vertices (triangle strip)
positioned at the exact corners of the overlay rectangle.

The vertex shader converts pixel coordinates to clip space using the existing
orthographic projection matrix. This is the same approach the `image` shader
uses.

**Fragment shader (`pink_overlay_fragment`):**

Returns a solid pink color: `float4(1.0, 0.41, 0.71, 1.0)` (hot pink,
`#FF69B4`).

###### `ts5/src/renderer/metal/shaders.zig`

Add a new pipeline definition `pink_overlay` alongside the existing pipelines.
Define a `PinkOverlayParams` struct with the overlay rectangle dimensions:

```
x: f32,      // Left edge in physical pixels
y: f32,      // Top edge in physical pixels
width: f32,  // Width in physical pixels
height: f32, // Height in physical pixels
```

###### `ts5/src/renderer/generic.zig`

In `drawFrame()`, after the kitty images (above text) step and before custom
shaders, add a new step:

1. Check if an overlay rect is set (non-zero). If not, skip this step.

2. Convert the stored grid coordinates to physical pixel coordinates:
   ```
   x = overlay_col × cell_width + padding_left
   y = overlay_row × cell_height + padding_top
   w = overlay_cols × cell_width
   h = overlay_rows × cell_height
   ```

3. Populate `PinkOverlayParams` with the computed pixel coordinates.

4. Sync the params buffer to the GPU.

5. Add a render pass step with the `pink_overlay` pipeline.

##### Part 2: OSC Parser

###### `ts5/src/terminal/osc.zig` (or equivalent)

Add handling for OSC code `133337`. When Ghostty receives
`\x1b]133337;col,row,width,height\x07`:

1. Parse the four comma-separated integers.
2. Store them in the terminal/surface state as the overlay grid rect.
3. Mark the surface as needing redraw.

When Ghostty receives `\x1b]133337;\x07` (empty payload):

1. Clear the overlay rect (set to zero/null).
2. Mark the surface as needing redraw.

The overlay rect must be accessible from the renderer thread (where
`drawFrame()` runs). Use the same thread-safe communication pattern Ghostty uses
for other terminal state (e.g., the surface mailbox or shared state protected by
the draw mutex).

##### Part 3: `web` TUI Integration

###### `web/src/main.rs`

After each `terminal.draw()` call, compute the viewport inner rect and emit the
OSC escape sequence:

```rust
terminal.draw(|frame| ui(frame, &url, &profile, &mode))?;

// Emit overlay coordinates to Ghostty.
// inner_rect is computed during ui() via Block::inner().
print!("\x1b]133337;{},{},{},{}\x07",
    inner_rect.x, inner_rect.y,
    inner_rect.width, inner_rect.height);
```

The `inner_rect` values are already computed by ratatui's layout engine. When
the terminal resizes, ratatui automatically recomputes the layout on the next
`draw()` call, and the new coordinates are emitted.

On exit (after restoring the terminal), emit the clear sequence:

```rust
print!("\x1b]133337;\x07");
```

This tells Ghostty to stop rendering the overlay.

#### Pass Criteria

1. Ghostty builds without errors or warnings.
2. Running `web <url>` inside Ghostty shows a pink rectangle exactly covering
   the viewport area (inside the border, below the URL bar, above the status
   bar).
3. Resizing the terminal causes the pink rectangle to resize and reposition to
   match the new viewport dimensions. No lag, no stale positioning.
4. The pink rectangle is opaque and fully covers the terminal text beneath it.
5. The rest of the terminal (URL bar, border, status bar) renders normally.
6. Quitting `web` (Ctrl+C or `q`) clears the pink overlay — the terminal returns
   to normal with no pink residue.
7. The pink rectangle does not flicker or tear during resize.

## Sizing Lessons (Reference)

From four generations of texture overlay experiments, these are the sizing
rules:

1. **Always work in physical pixels.** Multiply logical coordinates by
   `backingScaleFactor` (typically 2.0 on Retina Macs). Ghostty's renderer
   already operates in physical pixels — the projection matrix and all
   coordinates in `drawFrame()` use physical pixel units.

2. **Use the existing projection matrix.** Ghostty creates an orthographic 2D
   projection in `generic.zig` (`math.ortho2d`). Pass pixel coordinates through
   this matrix and they map directly to screen positions.

3. **IOSurface alignment.** When creating IOSurface-backed textures, bytesPerRow
   must be 16-byte aligned. For the pink overlay this doesn't apply (we're
   drawing a solid color, not importing a texture), but it will matter when we
   replace the pink rectangle with a real browser IOSurface.

4. **sRGB handling.** Ghostty uses Display P3 color space for its render
   targets. When importing external textures (from Chromium), the texture view
   format must declare the correct color space to avoid double gamma correction.
   For the pink overlay (a constant in the shader), this isn't an issue.

5. **Stale frames during resize.** Ghostty's `IOSurfaceLayer.setSurface()`
   already validates that the IOSurface dimensions match the layer bounds and
   discards mismatched frames. This same pattern should be applied to browser
   texture overlays.
