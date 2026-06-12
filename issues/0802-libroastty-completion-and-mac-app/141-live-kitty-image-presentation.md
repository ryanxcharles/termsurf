# Experiment 141: Phase H — live Kitty image presentation

## Description

Wire Kitty graphics into the live Metal presentation path.

Roastty already has the pieces needed for Kitty-image rendering:

- terminal-side Kitty image storage and render-placement snapshots;
- `renderer::image::ImageState`, which tracks Kitty placements, converts image
  payloads into pending uploads, buckets placements by upstream z layers,
  uploads pending images, and produces image draw calls;
- Metal image shaders, image texture upload support, and render-pass tests for
  the image pipeline.

The live app path still stops short of using those pieces.
`Surface::present_live` collects the terminal and calls
`FrameRenderer::render_and_present_frame`, which ultimately asks
`MetalFrameCompositor` to draw only the background, cell backgrounds, and text.
`MetalRenderPass::draw_cells` explicitly documents that background-image and
Kitty/overlay draws are deferred.

This experiment connects the existing Kitty image state to the live compositor
and preserves the upstream draw order from `renderer/generic.zig`:

1. background color;
2. Kitty images below cell backgrounds;
3. opaque cell backgrounds;
4. Kitty images between cell backgrounds and text;
5. cell text;
6. Kitty images above text.

The first slice should not attempt background images, custom shaders, or debug
overlay rendering. Those have separate config/file-loading and pass-order
concerns and should stay separate.

## Changes

- `roastty/src/lib.rs`
  - Add persistent Kitty image render state to `SurfaceLiveRenderer`.
  - During `Surface::present_live`, derive current Kitty render placements from
    the live terminal with the existing `kitty_render_placement_snapshots`
    helper, update the renderer image state, and pass that state into the live
    presentation call.
  - Preserve existing no-NSView/no-renderer/no-worker no-op behavior.
- `roastty/src/renderer/frame_renderer.rs`
  - Add a live presentation entry point that accepts the caller-owned
    `ImageState<MetalTexture>` and forwards it to the compositor after the
    normal frame rebuild.
  - Keep existing non-image `render_and_present_frame` behavior available for
    tests and callers that do not pass image state.
- `roastty/src/renderer/frame_rebuild.rs`
  - Extend the presentation-only input with optional mutable image state, or add
    a parallel image-aware presentation path, so the frame rebuild remains the
    owner of terminal/cell contents while image state remains persistent across
    frames.
  - Do not make image state part of `Contents`; Kitty images are separate
    renderer state upstream.
- `roastty/src/renderer/metal/compositor.rs`
  - Upload pending image data using `MetalImageUploadBackend`.
  - Draw image buckets in the upstream order around the existing cell draw
    sequence.
  - Add or call an image-aware compositor path that interleaves explicit render
    pass steps as:
    `draw_background_color -> draw_kitty_below_background -> draw_cell_backgrounds -> draw_kitty_below_text -> draw_cell_text -> draw_kitty_above_text`.
    Do not call the current monolithic `draw_cells` helper from this path,
    because that helper draws all three cell stages contiguously and leaves no
    insertion point for Kitty buckets.
  - Preserve target resize, IOSurface presentation, and cell-only rendering when
    no image placements exist.
- `roastty/src/renderer/metal/render_pass.rs`
  - Split the existing cell-draw sequence into reusable helpers for background
    color, opaque cell backgrounds, and cell text, while keeping
    `draw_cells`/`draw_frame` as cell-only compatibility wrappers.
  - Add the smallest Metal `ImageDrawBackend`/render-pass helper needed to bind
    one image vertex, the image texture, the image pipeline, uniforms, and issue
    the draw call produced by `ImageState`.
  - Keep zero-instance and missing/not-ready image behavior non-fatal, matching
    the existing `ImageState::draw` contract.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - After the result, update Phase H notes to distinguish live Kitty image
    presentation from still-open background image, custom shader,
    link-highlight, and debug-overlay work.

Out of scope:

- Background-image config loading, upload, and draw.
- Custom shader screen pass.
- Link-highlight matcher.
- Debug overlay image rendering.
- Public C ABI changes.
- UI automation for image protocols. This experiment proves the live renderer
  path with focused renderer/compositor tests and existing Kitty placement
  snapshot coverage.

## Verification

- Format markdown:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/141-live-kitty-image-presentation.md issues/0802-libroastty-completion-and-mac-app/README.md`
- Format Rust:
  - `cargo fmt`
- Run focused renderer/image tests:
  - `cargo test -p roastty renderer_image_state`
  - `cargo test -p roastty image_render_pass`
  - `cargo test -p roastty compositor`
  - `cargo test -p roastty live_kitty_image`
- Run Kitty graphics snapshot / ABI-adjacent coverage:
  - `cargo test -p roastty kitty_graphics`
- Run full Roastty Rust coverage:
  - `cargo test -p roastty -- --test-threads=1`
- Run hosted app coverage to confirm live drawing still builds and presents:
  - `cd roastty && macos/build.nu --action test --only-testing RoasttyTests/SurfaceViewAppKitTests`
  - `cd roastty && macos/build.nu --action test`
- Run checks:
  - `cargo fmt --check`
  - `git diff --check`
  - `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/141-live-kitty-image-presentation.md issues/0802-libroastty-completion-and-mac-app/README.md`

**Pass** = live presentation owns persistent Kitty image state, updates it from
the terminal's current render placements, uploads pending images to Metal, draws
Kitty images in the upstream z-bucket order around cell backgrounds/text, keeps
cell-only presentation unchanged when no images exist, and the focused plus full
Rust/macOS test gates pass.

Required new proof:

- A compositor or render-pass pixel-readback test named with `live_kitty_image`
  proves bucket ordering. It must construct ready `ImageState<MetalTexture>`
  placements for all three Kitty buckets and assert target pixels distinguish:
  - below-background images are covered by opaque cell backgrounds;
  - below-text images appear above cell backgrounds but below text where a text
    glyph covers the same pixel;
  - above-text images appear over both cell backgrounds and text.
- A frame-renderer/live-present test named with `live_kitty_image` drives the
  image-aware presentation path from terminal Kitty render placements, not just
  hand-written placements. It must create or feed a terminal with a visible
  Kitty image placement, update persistent `ImageState<MetalTexture>`, present
  through `MetalFrameCompositor`, and assert target readback contains the image
  color. A second frame with no visible placements should prove the persistent
  image state unload/removal path returns the affected pixel to the cell-only
  background.

**Partial** = Kitty render placements update and upload, but image draw cannot
be proven in the live Metal pass without a larger compositor seam or UI harness.

**Fail** = the existing Kitty image state cannot be connected to the live
renderer without reworking terminal storage, image snapshot generation, or the
compositor architecture beyond one experiment.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Descartes`, fresh
context.

**Verdict:** Approved after fixes.

**Findings and fixes:**

- **Required:** The first design did not explicitly address the current
  monolithic `MetalRenderPass::draw_cells` sequence. Because that helper draws
  background color, opaque cell backgrounds, and text contiguously, it leaves no
  insertion points for upstream's Kitty image buckets. Fixed by requiring either
  split render-pass helpers or a dedicated image-aware compositor path with the
  explicit upstream order:
  `draw_background_color -> draw_kitty_below_background -> draw_cell_backgrounds -> draw_kitty_below_text -> draw_cell_text -> draw_kitty_above_text`.
- **Required:** The first verification plan did not name concrete tests proving
  live Kitty image presentation; it listed existing test groups plus an
  unspecified new focused test. Fixed by requiring `live_kitty_image` tests with
  target-pixel readback that proves bucket order relative to cell backgrounds
  and text, plus a frame-renderer/live-present test that drives the path from
  terminal Kitty render placements through persistent `ImageState<MetalTexture>`
  into `MetalFrameCompositor`.

Codex-native adversarial reviewer `Pascal` re-reviewed only those fixes and
approved the design with no required, optional, or nit findings.
