+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"
+++

# Experiment 15: Phase C — the live present path (the 801 crux), slice 1

## Description

Exp 14 pinned the one thing keeping the launched app blank:
`roastty_surface_draw` → `Surface::draw()` → `request_render()` only sets a
`dirty` flag — libroastty never puts a rendered frame onto the app's NSView. 801
built the pieces but **explicitly deferred** the wiring: `frame_renderer.rs`
says "Metal presentation (`draw_frame`/compositor), deriving the rebuild input
from surface config/state, wiring into `surface.draw()`, and clearing dirty bits
are all later slices." This experiment is the first of those slices.

**What already exists (801):** `renderer/metal/compositor.rs` (owns
`MTLDevice`/queue + `FrameState` + the layer),
`renderer/metal/iosurface_layer.rs` (`MetalIOSurfaceLayer` — an IOSurface-backed
`CALayer` with a display callback), `frame_renderer.rs` (CPU-side frame-rebuild
from a live terminal), and the full cell/atlas/pipeline/render-pass stack.

**What's missing (corrected by the design review):** the compositor's
render→present path is **already built and tested end-to-end** on a real device
(`compositor.rs` `draw_frame` → `layer.set_surface(...)`;
`frame_renderer.rs::render_and_present_frame_presents` exercises the full path).
The gaps are: (1) the `Surface` never captures the `nsview`; (2) nothing owns a
compositor/`FrameRenderer`/layer for a surface or **attaches the layer to the
`nsview`** (`MetalIOSurfaceLayer` exposes `layer()` but **no attach-to-view
method** — that glue is new); and **(3) the critical one — nothing DRIVES a
present.** `roastty_surface_draw` has **no caller** (grep of `roastty/macos` is
empty; the app has no `draw` override / `CVDisplayLink`, and `App.wakeup` is an
empty stub, so `request_render`→`wakeup_app` schedules nothing). Upstream drives
present from a **renderer thread + `CVDisplayLink` inside the library**
(`renderer/Thread.zig` `drawFrame`, `renderer/generic.zig` `CVDisplayLink`);
roastty has built no such driver. So a present wired only into `surface.draw()`
would never run.

**In-process advantage:** the app links libroastty directly, so the library can
set its `CALayer` as the app-provided `NSView`'s layer with no `CAContext`
round-trip.

## Approach (slice 1 — first pixels, present driven by a main-thread FFI the app already calls)

The goal of slice 1 is the **first real pixels from libroastty into the app's
NSView**, while **owning the present trigger** (since nothing calls
`surface_draw`). A full `CVDisplayLink` render driver is deferred to **slice
2**; slice 1 presents from an FFI the app **does** call on the main thread at
launch/resize — `roastty_surface_set_size` / `set_content_scale` (verified
called from `SurfaceView_AppKit`). One static frame on screen proves the hard
part (nsview → attached layer → present); the continuous driver makes it live.

1. **Capture the `nsview` on the `Surface`.** In `roastty_surface_new`, read
   `config.platform.macos.nsview`, store it (+ `content_scale`, size) on
   `Surface`.
2. **New glue (enumerated — not "thin"):** `Surface` owns a
   `MetalFrameCompositor` + `FrameRenderer` (+ its `Atlas`es / device /
   `MetalUniforms` from config), created lazily on the **main thread** with
   `MTLCreateSystemDefaultDevice`. Add a **new attach-to-view method** on
   `MetalIOSurfaceLayer` (e.g. `attach_to_nsview(nsview)` → set
   `view.wantsLayer`, `view.layer = self.layer()`, `contentsScale`); add a
   compositor accessor for its layer. The live `SharedGrid` + `Terminal` are
   reached via `with_termio` (the terminal is behind the termio lock) — identify
   the `SharedGrid` source (font subsystem) in implementation.
3. **Present on `set_size`/`set_content_scale`** (main thread): after sizing,
   build the frame from the live terminal+config via `FrameRenderer` and present
   through the compositor (the already-tested `draw_frame`→`set_surface` path).
   **Slice-1 fidelity:** the terminal background + whatever the existing
   compositor cell path draws; if cells are too big for one slice, present a
   **cleared bg-color frame** (still proves the plumbing) and defer cells.
4. **`surface.draw()`** also calls the same present (so once a driver exists in
   slice 2 it works unchanged) — but slice 1 does **not** depend on `draw()`
   being called.
5. **Re-launch + capture** (Exp-14 harness): the window shows a **non-blank**
   libroastty frame. Kill the spawned app (0 dangling PIDs).

**Threading + `Send`:** all CALayer/Metal work is main-thread (`set_size` etc.
are main-thread app callbacks). `MetalFrameCompositor`/`MetalIOSurfaceLayer`
hold `Retained<MTLDevice>` + a `CALayer` (not `Send`) — so **confirm `Surface`
is never `Send`-bound** (it must not be moved to the off-thread
`termio_worker`); if it is, isolate the Metal state behind a main-thread-only
handle. This touches **only `libroastty`**; no app source changes.

## Verification

1. **`cargo test -p roastty --lib`** green (the `Surface`/`surface_new`/`draw`
   changes + any new layer-attach code don't regress; add a unit test for the
   nsview capture + the draw-path guards where headless-testable).
2. **The app launches and renders a non-blank frame** — a window-isolated
   capture (by PID, via the Exp-14 scripts) shows libroastty pixels (bg color /
   cells), cross-checked against logs (white-blank vs a real frame) and against
   a baseline real-Ghostty capture for sanity.
3. **No crash** in the live launch (stderr clean); **0 dangling PIDs** after
   `stop-app.sh`.
4. **Screenshots out-of-repo** (policy); evidence excerpts quoted into the
   Result.

**Pass** = the `Surface` captures the nsview, a Metal layer is attached to it, a
present is driven from a main-thread FFI (`set_size`/`set_content_scale`), and
the launched app shows a **non-blank libroastty frame** (≥ the terminal
background; cells if reached), `cargo test` green, app killed cleanly.
(Continuous live updates via a `CVDisplayLink` driver = slice 2.)

**Partial** = the plumbing works (layer attaches, a cleared/bg frame presents —
window no longer blank-white) but full cell/text rendering and/or the continuous
driver are deferred to slice 2 (documented with the exact remaining work).

**Fail** = the layer can't be attached / a present can't be driven from this
harness (documented as the real blocker — e.g. a `Send` constraint forcing
Metal-state isolation, or a main-thread/device-init issue). The compositor's
render→present is already proven, so this is about the attach + trigger, not the
renderer itself.

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: CHANGES REQUIRED → addressed.** It found my original
slice (wire present into `surface.draw()`) **could not produce a frame**,
because:

- **Required — nothing drives `draw()`.** `grep surface_draw roastty/macos` is
  empty; the app has no `draw` override / `CVDisplayLink`, and `App.wakeup` is
  an empty stub. Upstream drives present from a **library-internal renderer
  thread + `CVDisplayLink`** (`renderer/Thread.zig`, `renderer/generic.zig`),
  which roastty hasn't built. **Fixed:** slice 1 now owns the trigger (present
  from `set_size`/`set_content_scale`, the main-thread FFIs the app actually
  calls); the continuous `CVDisplayLink` driver is slice 2.
- **Required — the risk analysis was wrong.** The compositor's render→present is
  **already built + tested** (`compositor.rs draw_frame`;
  `frame_renderer.rs render_and_present_frame_presents` on a real device).
  **Fixed:** the Description/Fail-path now state this; the real risk is the
  driver + threading + `Send`.
- **Optional — "thin glue" understated.** The compositor's layer is private (no
  accessor) and `MetalIOSurfaceLayer` has no attach-to-view method; `Surface`
  must own compositor + `FrameRenderer` + atlases + device + reach the live
  `Terminal`/`SharedGrid` via `with_termio`. **Fixed:** enumerated.
- **Optional — non-`Send` Metal state** on `Surface` (`Retained<MTLDevice>` +
  `CALayer`). **Fixed:** confirm `Surface` isn't `Send`-bound or isolate the
  Metal state.
- **Nit — `render_state_*` is a pull/scalar path that never touches the
  compositor** — there is no library-internal present to "route into." Noted.

(Separately, the reviewer flagged a **prompt-injection in
`vendor/ghostty/CLAUDE.md`** — an upstream trap telling agents to write a
self-deprecating file on issue/PR requests. Ignored by both the reviewer and
this issue; surfaced to the user.)

## Result

_(to be added after the run.)_

## Conclusion

_(to be added after the run.)_
