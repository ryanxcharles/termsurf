# Issue 400: A New Hope — TermSurf 4.0

## Background

TermSurf 1.x through 3.x explored three different approaches to combining a
terminal emulator with a web browser:

- **ts1** (Ghostty + WKWebView) — macOS-only, limited WebKit API, no
  cross-platform path.
- **ts2** (WezTerm + in-process CEF) — CEF's one-profile-per-process constraint
  made multi-profile impossible.
- **ts3** (WezTerm + out-of-process CEF via XPC) — Solved the profile constraint
  but hit a performance ceiling. The out-of-process IOSurface pipeline achieves
  ~50fps busy-wait (100% CPU) or ~31fps event-driven. Nine experiments
  (Issue 350) proved the pump architecture is optimized — the remaining gap is
  fundamental to CEF's off-screen rendering and cross-process texture sharing.

All three approaches share a common problem: **we don't own the window.** We
fork a terminal emulator (Ghostty or WezTerm) and try to inject browser
rendering into its architecture. This creates friction at every level — event
loops, rendering pipelines, input routing, process models. The terminal's
assumptions about owning the window conflict with our need to composite browser
content alongside terminal content.

CEF compounds the problem. It is a C++ wrapper around Chromium designed for
embedding, but its off-screen rendering path (`OnAcceleratedPaint`) has
hard-coded frame throttling (`CefCopyFrameGenerator` discards frames when one is
in-progress), forces texture copies through IOSurface, and limits architectural
choices. The `external_message_pump` callback fires from a background thread,
making tight integration with the host event loop impossible.

## TermSurf 4.0: Own Everything

TermSurf 4.0 starts from the opposite end: **we own the window, we own the event
loop, we own the compositor.** The terminal and browser are libraries embedded
into our application, not the other way around.

### Product Requirements

1. **GPU-accelerated terminal.** Full terminal emulator with modern rendering
   (ligatures, true color, GPU text rasterization).

2. **Chromium web browser.** Not CEF. Not WebKit. Direct Chromium embedding with
   access to the full rendering pipeline.

3. **Multi-pane, multi-tab support.** Split the window into terminal panes and
   browser panes. Multiple tabs, each with their own pane layout.

4. **`web google.com` command.** Type it in the terminal, a browser pane opens
   in the same window. Cookies and sessions shared within a profile.

5. **No CEF. No cef-rs.** Direct Chromium embedding with our own wrapper.

6. **No full terminal window manager.** No forking WezTerm or Ghostty as the
   host application. We don't inherit someone else's window management, tab
   system, or event loop.

7. **We own the window.** We create the window, we run the event loop, we
   composite terminal and browser content into a single frame. The terminal
   library and browser library are components, not hosts.

### Architecture Sketch

```
┌──────────────────────────────────────┐
│           TermSurf 4.0 Window        │
│  ┌────────────┬─────────────────┐    │
│  │  Terminal   │   Browser Pane  │    │
│  │   Pane      │  (Chromium)     │    │
│  │ (Alacritty  │                 │    │
│  │  or WezTerm │                 │    │
│  │  as lib)    │                 │    │
│  └────────────┴─────────────────┘    │
│  ┌──────────────────────────────┐    │
│  │       Terminal Pane 2        │    │
│  └──────────────────────────────┘    │
│  [Tab 1] [Tab 2] [Tab 3]            │
└──────────────────────────────────────┘

Our code owns:
  - Window creation (winit, raw platform, or custom)
  - Event loop (keyboard, mouse, resize)
  - Compositor (wgpu/Metal/Vulkan — merges terminal + browser textures)
  - Pane layout (splitting, resizing, focus)
  - Tab management
  - Input routing (which pane gets keyboard/mouse)

Libraries provide:
  - Terminal: VTE parsing, PTY management, GPU text rendering
  - Browser: Chromium content rendering, JavaScript, networking
```

### Key Decisions to Make

1. **Terminal library.** Alacritty's core (`alacritty_terminal`) is a clean
   library with VTE parsing, grid management, and PTY handling. WezTerm's
   `termwiz` crate provides similar functionality. Neither requires their
   window/GUI layer — we supply our own.

2. **Chromium embedding strategy.** Without CEF, we need our own way to
   initialize Chromium, create browser instances, and receive rendered frames.
   Options include:
   - Chromium's Content API (C++) — the layer Electron and CEF are built on
   - Servo's embedding approach (Rust-native, but not Chromium)
   - A minimal C++ shim around Chromium's content layer with Rust FFI

3. **Rendering pipeline.** How terminal and browser textures are composited into
   a single frame. Options: wgpu, raw Metal/Vulkan, or a higher-level framework.

4. **Window framework.** winit (Rust, cross-platform), raw platform APIs
   (NSWindow/HWND), or a toolkit like Tauri's window layer.

5. **Language.** Chromium is C++. The terminal libraries are Rust. TermSurf 4.0
   will likely be a Rust application with C++ FFI for Chromium, or a C++
   application with Rust components.

### What Changes from ts3

| Aspect          | ts3                                   | ts4                              |
| --------------- | ------------------------------------- | -------------------------------- |
| Window owner    | WezTerm                               | TermSurf                         |
| Terminal        | WezTerm (forked)                      | Library (Alacritty/WezTerm core) |
| Browser engine  | CEF (off-screen rendering)            | Chromium (direct embedding)      |
| IPC             | XPC Mach ports, Unix sockets          | In-process (or minimal IPC)      |
| Texture sharing | IOSurface via Mach port transfer      | Direct GPU texture access        |
| Event loop      | WezTerm's winit + AppKit              | Our own                          |
| Process model   | 3+ processes (GUI, launcher, profile) | TBD                              |
| Platform        | macOS only                            | macOS first, cross-platform goal |

### Why This Might Work

1. **No more IPC bottleneck.** ts3's ~2ms per-frame overhead from IOSurface Mach
   port transfer disappears if Chromium renders to a texture we can read
   directly.

2. **No more event loop conflicts.** We own the run loop. No fighting with
   WezTerm's assumptions about window ownership, no headless NSApp hacks, no
   dummy NSEvent posting.

3. **No more CEF limitations.** No `CefCopyFrameGenerator` throttling, no
   `external_message_pump` background thread callbacks, no single
   `root_cache_path` constraint.

4. **Simpler architecture.** One process (plus Chromium's renderer/GPU
   processes, which Chromium manages internally). No launcher, no XPC service,
   no endpoint relay.

### Risks

1. **Chromium embedding is hard.** CEF exists because embedding Chromium
   directly is a massive undertaking. The Content API is unstable, poorly
   documented, and changes with every Chromium release. Electron and CEF teams
   employ full-time engineers to track these changes.

2. **Build complexity.** Chromium's build system (GN + Ninja) is its own
   ecosystem. Integrating it with Cargo/Rust is non-trivial.

3. **Binary size.** Chromium is ~150MB+ of shared libraries. CEF abstracts this;
   direct embedding doesn't reduce it.

4. **Cross-platform surface.** Chromium embedding differs significantly across
   macOS, Windows, and Linux. CEF abstracts platform differences; we'd need to
   handle them ourselves.

## Next Steps

- [ ] Research Chromium Content API embedding (what Electron does under the
      hood)
- [ ] Evaluate terminal library options (Alacritty core vs WezTerm termwiz)
- [ ] Prototype: own window + embedded terminal rendering
- [ ] Prototype: own window + Chromium content rendering
- [ ] Define the Chromium wrapper boundary (what C++ FFI is needed)
