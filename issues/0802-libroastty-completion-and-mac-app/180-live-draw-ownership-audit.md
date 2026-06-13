# Experiment 180: Phase C — live draw ownership audit

## Description

Close the remaining Phase C ambiguity around `surface_draw` ownership and the
interim `render_state` pull path by auditing the copied app's actual render
loop.

Earlier Phase C experiments built the live Metal renderer, attached it to the
app-provided `NSView`, proved CoreVideo display-link presentation with live
smoke, threaded cursor blink state through live frame rendering, and propagated
focus / visibility / config options. The roadmap still has two unchecked items:

- `surface_draw` owns a Metal renderer bound to the app `NSView` / `CALayer`;
  attach the layer and present on-screen.
- Retire the interim `render_state` pull divergence.

Current source evidence suggests these are now mostly proof/documentation gaps:
the copied Swift app creates each surface with its `NSView`, drives size/content
scale/focus/input through the surface ABI, and does not call
`roastty_surface_render_state_update` or the C `render_state` row/cell iterator
APIs. The live Rust path stores `SurfaceLiveRenderer` on `Surface`,
`roastty_surface_draw` calls `Surface::draw`, and `Surface::present_live` lazily
builds the Metal compositor, attaches the IOSurface layer to the `NSView`, and
presents live frames.

This experiment should prove those statements against the current tree and
update the Issue 802 roadmap only when the evidence is strong enough. It should
not delete the generic C `render_state` ABI, because upstream still exposes
terminal render-state helpers through `lib_vt`; the Phase C concern is the
copied app render loop relying on an interim pull path instead of the live
surface renderer.

## Changes

- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Link this experiment as `Designed`.
  - After verification, mark it `Pass`, `Partial`, or `Fail`.
  - Check the `surface_draw` ownership item only if source inspection and live
    smoke prove the copied app path creates a live renderer owned by `Surface`,
    attached to the app `NSView`, and presents on-screen without a separate
    Swift renderer.
  - Check the `render_state` pull divergence item only if source inspection
    proves the copied app render path no longer calls
    `roastty_surface_render_state_update` or lower-level C render-state
    iterator/cell APIs.

- `issues/0802-libroastty-completion-and-mac-app/180-live-draw-ownership-audit.md`
  - Record the source evidence, command output, live artifact paths, result,
    conclusion, and AI completion review.

- `roastty/src/lib.rs`
  - No production change is expected.
  - Add a narrow test only if the design review identifies a missing,
    deterministic assertion needed to prove the ownership claim.

## Verification

Before implementation:

- Codex-native adversarial design review approves this experiment.
- Commit the reviewed plan separately from the result.

Source audit:

- Prove the copied Swift app has no render-state pull usage:

  ```bash
  rg -n "render_state|RenderState|surface_render_state_update|roastty_render_state" \
    roastty/macos/Sources
  ```

- Prove the copied Swift app creates surfaces with the AppKit view and routes
  resize/scale/focus through the surface ABI:

  ```bash
  rg -n "roastty_surface_new|roastty_surface_set_size|roastty_surface_set_content_scale|roastty_surface_set_focus" \
    "roastty/macos/Sources/Roastty/Surface View"
  ```

- Prove the Rust surface draw path owns and presents through the live renderer:

  ```bash
  rg -n "struct SurfaceLiveRenderer|fn build_live_renderer|fn draw\\(&mut self\\)|fn present_live|roastty_surface_draw|attach_to_nsview|render_and_present_frame" \
    roastty/src/lib.rs roastty/src/renderer
  ```

- Compare against upstream embedded draw ownership:

  ```bash
  sed -n '760,785p' vendor/ghostty/src/apprt/embedded.zig
  sed -n '875,887p' vendor/ghostty/src/Surface.zig
  ```

Regression checks:

- `cargo test -p roastty live_renderer_options -- --test-threads=1`
- `cargo test -p roastty live_cursor_blink -- --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt --check -p roastty`
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/180-live-draw-ownership-audit.md issues/0802-libroastty-completion-and-mac-app/README.md`
- `git diff --check`

Live sanity:

- Rebuild the copied app:

  ```bash
  cd roastty && macos/build.nu --action build
  ```

- Re-run live smoke:

  ```bash
  scripts/roastty-app/stop-app.sh
  TERMSURF_AB_HOLD_SECONDS=10 \
  ROASTTY_PRESENT_DRIVER_LOG=1 \
    scripts/roastty-app/live-ab-smoke.sh \
      --recipe smoke \
      --comparison-region content \
      --max-mismatch-ratio 1 \
      --max-mean-channel-delta 255
  ```

**Pass** = source audit proves the copied app render path is live
`SurfaceLiveRenderer` ownership rather than Swift/render-state pulling,
regression checks pass, the copied app rebuilds, live smoke renders with
`present-driver=display-link reason=core-video`, and both remaining Phase C
ownership/divergence checklist items can be checked.

**Partial** = live rendering remains healthy, but source evidence shows any
copied-app render path still depends on `roastty_surface_render_state_update` or
the ownership claim remains too indirect to check a roadmap item. Record the
exact missing proof.

**Fail** = source audit contradicts the ownership claim, app startup/rendering
regresses, or the verification gates fail.

## Design Review

**Reviewer:** Codex-native adversarial review subagent `Harvey`, fresh context.

**Verdict:** Approved.

Findings: None. The reviewer confirmed the README links Experiment 180 as
`Designed`, the experiment has the required sections, an audit-only scope is
legitimate for closing the remaining Phase C proof gaps, the design explicitly
preserves the public `render_state` ABI because upstream still exposes it
through `lib_vt`, and the verification includes source-audit, live smoke,
regression, formatting, Prettier, and `git diff --check` gates.

## Result

**Result:** Pass.

The source audit proved the copied app render path is live-renderer owned and no
longer uses the interim render-state pull path.

Copied Swift app render-state audit:

- `rg -n "render_state|RenderState|surface_render_state_update|roastty_render_state" roastty/macos/Sources`
  returned no matches. That proves the copied app source no longer calls
  `roastty_surface_render_state_update` or lower-level C `render_state` row/cell
  APIs.

Copied Swift app surface ABI routing:

```text
roastty/macos/Sources/Roastty/Surface View/SurfaceView_AppKit.swift:356:                roastty_surface_new(app, &surface_cfg_c)
roastty/macos/Sources/Roastty/Surface View/SurfaceView_AppKit.swift:420:            roastty_surface_set_focus(surface, focused)
roastty/macos/Sources/Roastty/Surface View/SurfaceView_AppKit.swift:459:            roastty_surface_set_size(surface, width, height)
roastty/macos/Sources/Roastty/Surface View/SurfaceView_AppKit.swift:858:            roastty_surface_set_content_scale(surface, xScale, yScale)
roastty/macos/Sources/Roastty/Surface View/SurfaceView_UIKit.swift:30:                roastty_surface_new(app, &surface_cfg_c)
roastty/macos/Sources/Roastty/Surface View/SurfaceView_UIKit.swift:50:            roastty_surface_set_focus(surface, focused)
roastty/macos/Sources/Roastty/Surface View/SurfaceView_UIKit.swift:66:            roastty_surface_set_content_scale(surface, scale, scale)
roastty/macos/Sources/Roastty/Surface View/SurfaceView_UIKit.swift:67:            roastty_surface_set_size(
```

The AppKit surface creation path passes `self` as the platform view via
`surface_cfg.withCValue(view: self)` before `roastty_surface_new`, so the Rust
surface captures the app `NSView` at creation.

Rust live ownership audit:

```text
roastty/src/lib.rs:2726:struct SurfaceLiveRenderer {
roastty/src/lib.rs:3237:fn build_live_renderer(
roastty/src/lib.rs:3276:    compositor.layer().attach_to_nsview(nsview, scale);
roastty/src/lib.rs:4017:    fn draw(&mut self) {
roastty/src/lib.rs:4025:    fn present_live(&mut self) {
roastty/src/lib.rs:4126:                frame_renderer.render_and_present_frame_with_images_and_link_ranges_and_cursor_options(
roastty/src/lib.rs:4152:                    .render_and_present_frame_with_images_and_custom_shaders_and_link_ranges_and_cursor_options(
roastty/src/lib.rs:17636:pub extern "C" fn roastty_surface_draw(surface: RoasttySurface) {
roastty/src/renderer/metal/iosurface_layer.rs:47:    pub(crate) fn attach_to_nsview(&self, nsview: *mut c_void, scale: f64) {
```

That chain proves the live renderer is stored on `Surface`,
`roastty_surface_draw` calls `Surface::draw`, `Surface::draw` calls
`present_live`, and `present_live` uses the owned `SurfaceLiveRenderer` to
render and present frames. `build_live_renderer` attaches the compositor
IOSurface layer to the captured `NSView`.

Upstream comparison:

- `vendor/ghostty/src/apprt/embedded.zig` routes `ghostty_surface_draw(surface)`
  to `surface.draw()`.
- `vendor/ghostty/src/Surface.zig` implements draw by forcing the renderer to
  draw a frame on the main thread: `try self.renderer.drawFrame(true);`.

Roastty's path is structurally equivalent for the embedded/copy-app use case,
except that the renderer state is in-process Rust live renderer state rather
than Ghostty's Zig renderer object.

Regression verification:

- `cargo test -p roastty live_renderer_options -- --test-threads=1` — **Pass**,
  6 tests passed.
- `cargo test -p roastty live_cursor_blink -- --test-threads=1` — **Pass**, 4
  tests passed.
- `cargo test -p roastty --test abi_harness` — **Pass**, 1 test passed. The
  existing 10 enum-conversion warnings and `[unknown](scope): message` remained.
- `cargo fmt --check -p roastty` — **Pass**.
- `prettier --check --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/180-live-draw-ownership-audit.md issues/0802-libroastty-completion-and-mac-app/README.md`
  — **Pass**.
- `git diff --check` — **Pass**.

Live sanity:

- `cd roastty && macos/build.nu --action build` — **Pass**. The copied app build
  completed with `** BUILD SUCCEEDED **`; only existing Swift actor, retroactive
  Sendable, linker deployment-target, and terminfo warnings appeared.
- `scripts/roastty-app/stop-app.sh && TERMSURF_AB_HOLD_SECONDS=10 ROASTTY_PRESENT_DRIVER_LOG=1 scripts/roastty-app/live-ab-smoke.sh --recipe smoke --comparison-region content --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  — **Pass**. The harness launched Ghostty PID `23234` and Roastty PID `23242`
  with marker `ISSUE802_AB_SMOKE_20260613-022939` and returned JSON verdict
  `PASS`.
- Content-region diff metrics:

  ```text
  mismatch_ratio=0.005609027777777778
  mean_channel_delta=0.55804375
  compared_pixels=1440000
  mismatched_pixels=8077
  ```

- Full-window diff metrics:

  ```text
  mismatch_ratio=0.05834849683544304
  mean_channel_delta=1.5011784266218355
  compared_pixels=2022400
  mismatched_pixels=118004
  ```

- Screenshot artifacts:

  ```text
  /Users/ryan/.cache/termsurf/shots/ghostty-ab-content-20260613-022939.png
  /Users/ryan/.cache/termsurf/shots/roastty-ab-content-20260613-022939.png
  /Users/ryan/.cache/termsurf/shots/ghostty-ab-crop-20260613-022939.png
  /Users/ryan/.cache/termsurf/shots/roastty-ab-crop-20260613-022939.png
  /Users/ryan/.cache/termsurf/shots/roastty-ab-full-20260613-022939.png
  ```

- Present-driver log check:

  ```text
  /Users/ryan/.cache/termsurf/shots/roastty-ab-stderr-20260613-022939.log:1:[roastty] present-driver=display-link reason=core-video
  ```

- Cleanup check: `no debug Roastty app PID remains`.

The Phase C `surface_draw` ownership and interim `render_state` pull-divergence
items are now checked. The remaining Phase C item is the milestone: proving and
documenting that the app launches and shows a working ASCII terminal.

## Completion Review

Codex-native adversarial subagent `Heisenberg` reviewed the completed experiment
with fresh context before the result commit. It inspected the experiment file,
README update, uncommitted result diff, local source paths, upstream comparison
paths, and the claimed verification. Verdict: **APPROVED**. Findings: none.

## Conclusion

The live renderer is now the copied app's render path. Swift owns the view and
routes platform events into the surface ABI, while Rust owns the live renderer,
attaches its layer to the Swift-provided `NSView`, and presents frames. The old
C render-state pull API remains available for upstream `lib_vt` parity and
tests, but it is no longer part of the copied app rendering path. The next Phase
C experiment can focus on the final milestone wording: a working ASCII terminal
proof with the current live path and any remaining issue-level closure criteria.
