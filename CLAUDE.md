# Agent Development Guide

A file for [guiding coding agents](https://agents.md/).

## AI Guidance

Do exactly what I say. No more, no less. NEVER assume I want something I didn't
ask for. NEVER under any circumstances change the code unless I explicitly ask
to change the code.

## Settled Architectural Decisions

Do NOT suggest alternatives to these. They were chosen after extensive
experimentation across five generations (ts1–ts5) and hundreds of experiments.

### XPC is the only IPC mechanism on macOS

All inter-process communication on macOS MUST use XPC. There are no exceptions.
Do not suggest Unix domain sockets, named pipes, shared memory, or any other IPC
mechanism as an alternative.

IOSurface textures can only be transferred between processes via Mach ports, and
Mach ports can only be transferred via XPC. This is not a preference — it is a
hard macOS kernel constraint. Every IPC channel in TermSurf uses XPC because the
texture channel requires it, and using a second IPC mechanism for non-texture
messages would add complexity for zero benefit.

This was proven in ts3 (Issues 303, 325–350) and ts4 (Issues 403, 407).

### Every Chromium issue gets its own branch

When modifying the Chromium fork (`chromium/src/`), ALWAYS create a new branch
for the current issue. Never commit directly to an existing issue's branch.

1. Find the most relevant recent branch (usually the one with the latest
   TermSurf modifications).
2. Create a new branch from it: `{version}-issue-{N}` (e.g.,
   `146.0.7650.0-issue-625`).
3. Add the new branch to the Branches table in `docs/chromium.md`.

This keeps every issue's Chromium changes isolated and traceable.

## Project Overview

TermSurf is a terminal emulator with an integrated web browser. Users type
`web google.com` in their terminal and a webpage renders directly in the
terminal pane, sharing cookies and sessions across tabs within the same browser
profile.

The project has evolved through six generations:

- **ts1** (Ghostty + WKWebView) — macOS-only. WKWebView had limited API and no
  cross-platform path. Abandoned in favor of CEF.
- **ts2** (WezTerm + in-process CEF) — Embedded CEF directly in WezTerm. CEF
  allows only one `root_cache_path` per process, meaning one browser profile per
  application. Multiple profiles required moving CEF out-of-process. Abandoned.
- **ts3** (WezTerm + out-of-process CEF via XPC) — Each browser profile gets its
  own CEF process, solving the one-profile-per-process limitation. Processes
  communicate with the GUI via XPC Mach port transfer. Superseded by ts4 after
  26 experiments (Issues 325–350) proved CEF's headless off-screen rendering
  caps at ~31fps on macOS.
- **ts4** (Chromium Content API experiments) — Proved in-process Chromium works:
  multiple browser profiles coexisting in one process, 60fps rendering. PoC only
  — used content_shell inside the Chromium source tree. Superseded by ts5.
- **ts5** (Ghostty fork + out-of-process Chromium) — Proved end-to-end Chromium
  streaming works: IOSurface overlay pipeline, XPC communication, mouse/keyboard
  forwarding, focus lifecycle, text selection. All logic lived in Swift
  (CompositorXPC). Superseded by TermSurf GUI.
- **gui** (Ghostty fork, Zig-first) — **Active development.** Ghostty fork with
  all browser integration logic in Zig instead of Swift. XPC communication,
  CALayerHost compositing, keyboard/mouse forwarding — all in Zig, matching
  Ghostty's architecture where Swift is a thin macOS wrapper.

**Directory structure:**

- `gui/` — TermSurf GUI (Ghostty fork, Zig-first). **Active development.**
- `tui/` — `web` TUI (Rust/ratatui). Browser chrome in the terminal pane.
- `chromium/` — Chromium fork build workspace (gitignored).
- `docs/issues/` — All documentation across all generations.
- `docs/early-prototypes.md` — Archived prototype documentation (ts1–ts5,
  cef-rs).

## TermSurf GUI (gui/) — Active Development

### Architecture

TermSurf GUI forks Ghostty with all browser integration logic in Zig. Swift
remains a thin macOS wrapper — window creation, menu bar, application lifecycle
— matching Ghostty's own architecture. This is a clean break from ts5, where
browser integration lived in Swift (CompositorXPC.swift).

Key architectural decisions:

- **XPC in Zig.** XPC is a C API (`<xpc/xpc.h>`). Zig calls it directly via
  `@cImport`. No Swift intermediary needed.
- **CALayerHost in Zig.** Browser panes render via `CALayerHost` — a CALayer
  subclass that displays a remote `CAContext` from Chromium's GPU process.
  Window Server composites directly from GPU VRAM. Zero per-frame IPC, zero
  texture copies. The Metal renderer sets up the CALayerHost layer tree in Zig.
- **Input routing in Zig.** Zig already receives all keyboard/mouse events
  through `Surface.keyCallback()` and `mouseButtonCallback()`. In browse mode,
  these route to Chromium via XPC instead of to the terminal.
- **Single source of truth.** Browse mode, focus state, pane profiles, overlay
  coordinates — all live in Zig's Surface struct.

### Current State

TermSurf GUI is a Ghostty fork with browser integration built in Zig. Current
TermSurf additions: XPC gateway connection and anonymous listener (Issue 601),
pink texture proof-of-concept (Issue 602), live Chromium streaming at 60fps with
dynamic resize (Issue 603), multi-pane multi-profile server reuse (Issues
604–605), mouse input forwarding with cursor changes and text selection (Issue
606), keyboard input forwarding with Cmd+key bypass (Issues 607–609), TermSurf
branding and app icon (Issues 611–612), directory rename from ghost/web to
gui/tui (Issue 613), XDG directory compliance (Issue 615), loading progress
indicator and browser navigation keybindings (Issue 616), CALayerHost migration
replacing FrameSinkVideoCapturer with zero-copy Window Server compositing
(Issues 624–632).

### Directory Structure

- `gui/src/` — Shared Zig core (libghostty)
- `gui/src/Surface.zig` — Core surface (holds browser state)
- `gui/src/renderer/Metal.zig` — Metal renderer
- `gui/src/renderer/metal/` — Metal pipeline, shaders, IOSurface layer
- `gui/src/apprt/embedded.zig` — C API exports
- `gui/include/ghostty.h` — libghostty C API headers
- `gui/macos/` — macOS app (Swift, thin wrapper)
- `gui/build.zig` — Build system
- `gui/build.zig.zon` — Dependencies

### Build Commands

```bash
cd gui && zig build
```

### Launching

```bash
open gui/zig-out/TermSurf.app
```

### Upstream Merges

Same approach as ts5 (Issue 418 Experiment 3):

```bash
git fetch upstream
git subtree pull --prefix=gui upstream main -m "Merge upstream Ghostty into gui"
```

## History

TermSurf evolved through six generations. The prototypes (ts1–ts5) and the
cef-rs dependency have been archived from the working tree. Full documentation
is in [docs/early-prototypes.md](docs/early-prototypes.md).

- **ts1** (Ghostty + WKWebView) — WKWebView had limited API and no
  cross-platform path. Abandoned in favor of CEF.
- **ts2** (WezTerm + in-process CEF) — CEF allows only one `root_cache_path`
  per process, meaning one browser profile per application. Abandoned.
- **ts3** (WezTerm + out-of-process CEF via XPC) — Proved XPC and IOSurface
  patterns work. CEF caps at ~31fps on macOS (Issues 325–350). Superseded.
- **ts4** (Chromium Content API experiments) — Proved in-process Chromium works:
  multiple profiles, 60fps. PoC only. Superseded.
- **ts5** (Ghostty fork + out-of-process Chromium) — Proved end-to-end Chromium
  streaming: IOSurface overlay, XPC, mouse/keyboard, focus, text selection. All
  logic in Swift. Superseded by gui (Zig-first).
- **cef-rs** — CEF Rust bindings used by ts3. Archived.

## Documentation

### TermSurf GUI (active)

- `docs/issues/600-termsurf-ghost.md` — GUI vision, Zig-first architecture,
  Ghostty fork
- `docs/issues/601-zig-xpc.md` — XPC in Zig (gateway, listener, message parsing)
- `docs/issues/602-pink-texture.md` — Pink texture overlay (GPU quad via XPC)
- `docs/issues/603-box-demo.md` — Live Chromium streaming at 60fps
- `docs/issues/604-two-panes.md` — Multi-pane Chromium streaming
- `docs/issues/605-two-profiles.md` — Multi-profile server reuse
- `docs/issues/606-mouse-input.md` — Mouse clicks, drag, scroll, cursor changes
- `docs/issues/607-keyboard-input.md` — Basic key forwarding to Chromium
- `docs/issues/608-search-input.md` — Search input (Google, address bar)
- `docs/issues/609-keyboard-input-2.md` — Cmd+key bypass, clipboard, Tab
- `docs/issues/610-app-icon.md` — App icon (blocked by bundle ID, resolved)
- `docs/issues/611-rename.md` — Rename Ghostty → TermSurf
- `docs/issues/612-icon.md` — App icon pipeline (release + debug icons)
- `docs/issues/613-rename-directories.md` — Rename ghost/ → gui/, web/ → tui/
- `docs/issues/614-docs-review.md` — Documentation review
- `docs/issues/615-xdg.md` — XDG directory compliance
- `docs/issues/616-web-features.md` — Missing web features inventory
- `docs/issues/617-alpha.md` — Alpha release planning
- `docs/issues/618-url-sync.md` — URL sync
- `docs/issues/619-input-latency.md` — Input latency measurement and analysis
- `docs/issues/620-zig-content-shell.md` — Zig Content Shell (in-process
  attempt)
- `docs/issues/621-single-process.md` — Single-process multi-profile performance
- `docs/issues/622-javascript-is-slow.md` — JavaScript causes 2fps in
  multi-profile
- `docs/issues/623-viz-display-serialization.md` — Viz Display serialization
  theory (debunked)
- `docs/issues/624-chromium-ipc.md` — Chromium IPC architecture research
- `docs/issues/625-calayerhost.md` — CALayerHost migration (replaced
  FrameSinkVideoCapturer)
- `docs/issues/626-x-y-calayerhost.md` — CALayerHost X/Y positioning fix
- `docs/issues/627-resize-calayerhost.md` — CALayerHost resize fix
- `docs/issues/628-navigation-calayerhost.md` — CALayerHost navigation (first
  attempt, 8 experiments failed)
- `docs/issues/629-understand-nav-calayerhost.md` — Navigation blank diagnosis
- `docs/issues/630-nav-calayerhost-6.md` — Navigation blank fix (7 coordinated
  fixes)
- `docs/issues/631-continue-nav-calayerhost.md` — Navigation flicker
  investigation
- `docs/issues/632-nav-flicker-calayerhost.md` — Navigation flicker diagnosis
- `docs/issues/633-persistent-compositor.md` — Persistent compositor for stable
  CAContext
- `docs/xdg.md` — XDG directory pattern and conventions

### Early Prototypes (ts1–ts5)

Issue docs for all prototype generations are indexed in
[docs/early-prototypes.md](docs/early-prototypes.md#issue-documentation-index).

### General

- `docs/issues/002-merge-upstream.md` — How to merge changes from upstream repos
- `docs/issues/001-competitors.md` — Terminal-browser hybrid comparison
- `docs/issues/003-website.md` — termsurf.com website

## AI Reminder

NEVER change the code unless I explicitly ask. NEVER make unrequested changes.
Always do EXACTLY what I ask - no more, no less.
