# Issue 407: In-Process Chromium Proof of Concept

## Goal

Prove that Chromium can be embedded in-process inside a Swift macOS application
with multiple browser profiles rendering simultaneously at high framerate. No
custom XPC protocol. No out-of-process browser host. Chromium runs inside the
same process as the Swift window.

## What This Proves

This PoC validates three things that the entire ts4 architecture depends on:

1. **In-process Chromium embedding works on macOS.** We can embed Chromium's
   browser process directly inside a Swift application — not via CEF, not via a
   separate process, but as an in-process library that the Swift app links
   against.

2. **Multiple profiles coexist in one process.** Two `BrowserContext` instances
   with different storage paths run simultaneously, each with isolated cookies,
   localStorage, and cache. Issue 406 established that this is architecturally
   possible. This PoC proves it works in practice.

3. **High framerate rendering.** Chromium delivers frames at 120fps or higher to
   the Swift window. Issue 403 proved the compositing pipeline is fast (0.04ms
   per frame). Issues 325–350 proved CEF cannot sustain 60fps headless. This PoC
   proves the Content API path achieves what CEF could not.

## The Test Page

A Bun application in `ts4/` serves a single HTML page:

- **Blue spinning square.** A square rendered via WebGL (or Canvas 2D) that
  rotates 360 degrees per second (1 Hz). The rotation rate is chosen so that
  smoothness differences between 60fps and 120fps are visible to the eye — at
  60fps the square moves 6 degrees per frame, at 120fps it moves 3 degrees.

- **localStorage identity.** On first load, the page generates a random string
  and stores it in `localStorage`. On subsequent loads, it reads the stored
  string. The string is displayed above the spinning square. This serves as
  proof that profiles are isolated — the left and right panes will show
  different random strings because they use different `BrowserContext` storage
  paths.

- **FPS counter.** The page displays its own measured framerate (frames per
  `requestAnimationFrame` callback per second). This is the content-side
  framerate — the rate at which the page's animation loop runs inside Chromium.

- **High framerate target.** The animation loop should run faster than 60fps.
  The target is 120fps or higher, ideally 240fps. This may require disabling
  Chromium's internal vsync throttling or using `requestAnimationFrame` with
  appropriate display configuration.

- **No external dependencies.** The page is fully self-contained — no CDN, no
  npm packages, no network requests. It works offline.

## The Swift Application

A macOS Swift application in `ts4/` that embeds Chromium and displays two
browser panes side by side:

- **Single window, two panes.** The window is split vertically — left half and
  right half — similar to the blue/green split from Issue 403 Phase 7.

- **Left pane: Profile A.** Loads the test page using one `BrowserContext` with
  its own storage directory (e.g., `~/.config/termsurf/poc/profile-a/`).

- **Right pane: Profile B.** Loads the same test page using a different
  `BrowserContext` with a separate storage directory (e.g.,
  `~/.config/termsurf/poc/profile-b/`).

- **In-process Chromium.** Chromium's browser process runs inside the Swift
  application's process. Chromium may still spawn its own renderer and GPU
  sub-processes internally (this is Chromium's own multi-process architecture
  and is expected). The point is that we are not inventing a custom XPC or IPC
  protocol between our Swift app and a separate browser host process — the
  browser host is in-process.

- **Compositing.** Each Chromium pane renders to a surface (IOSurface or
  similar) that the Swift app composites into its Metal render pass. The
  compositing approach is flexible — whatever works for in-process embedding.

- **FPS measurement.** The Swift app measures and displays the frame delivery
  rate for each pane — how many new frames Chromium delivers per second to each
  pane's surface.

## Success Criteria

### Must have

- Two panes rendering the same webpage simultaneously in one window.
- Each pane shows a different localStorage string, proving profile isolation.
- The localStorage strings persist across app restarts (profiles are stored on
  disk, not in-memory).
- Both panes render the spinning square smoothly at 60fps or higher.
- Chromium is embedded in-process — no custom XPC/IPC protocol between the Swift
  app and a separate browser host.

### Target

- Frame delivery at 120fps for both panes (matching ProMotion displays).
- Frame-to-frame intervals are consistent (not bursty — no bimodal clustering).
- CPU usage is reasonable (no 100% busy-wait loops).

### Stretch

- Frame delivery at 240fps.
- Resize both panes dynamically without dropping frames.
- Keyboard input forwarded to the focused pane (typing in a text field).

## What This Does NOT Need

- No tab management, no URL bar, no navigation controls.
- No terminal pane — this is a pure Chromium embedding PoC.
- No Ghostty integration — that comes after this PoC succeeds.
- No custom XPC protocol — Chromium's internal IPC between its own browser,
  renderer, and GPU processes is fine (that is Chromium's concern, not ours).
- No production-quality error handling or crash recovery.

## Test Page Server

The Bun app is minimal:

- `ts4/server.ts` (or similar) — serves `ts4/public/index.html` on
  `localhost:<port>`.
- The Swift app loads `http://localhost:<port>` in both panes.
- The server exists only because Chromium needs an HTTP origin for localStorage
  to work (file:// URLs have localStorage restrictions in some configurations).

## Relationship to Other Issues

| Issue | Relationship                                                                     |
| ----- | -------------------------------------------------------------------------------- |
| 403   | Proved IOSurface compositing at 60fps with colored rectangles                    |
| 404   | Selected Ghostty as the terminal emulator                                        |
| 405   | Chose Ghostty fork + out-of-process Chromium architecture                        |
| 406   | Proved multiple profiles work in one Chromium process; ruled out CEF             |
| 407   | This issue — proves in-process Chromium with multiple profiles at high framerate |

If this PoC succeeds, we proceed to integrate Chromium into the Ghostty fork
(the architecture from Issue 405). If it fails, we revisit the embedding
strategy before touching Ghostty.
