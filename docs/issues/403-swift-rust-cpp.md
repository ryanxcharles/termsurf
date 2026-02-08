# Issue 403: Swift + Rust + C++ Architecture Prototype

## Goal

Build the simplest possible application that proves the TermSurf 4.0
multi-process, multi-language architecture works. No terminal emulation. No
browser. Just three processes, three languages, and two colored rectangles
composited into one window at 60fps.

## Product Requirements

1. A macOS window opens showing two panes side by side.
2. The left pane is **blue**, rendered by a **Rust** process.
3. The right pane is **green**, rendered by a **C++** process.
4. The window itself is a **Swift** application.
5. Each colored rectangle is a GPU texture (IOSurface) created by its respective
   process and shared with the Swift window via XPC Mach port transfer.
6. The window composites both textures into a single frame at 60fps.
7. Resizing the window resizes both panes proportionally.

## Rationale

### Why this prototype matters

TermSurf 4.0's architecture (Issue 400) requires three languages working
together across process boundaries:

- **Swift** for the window (native macOS, Metal, XPC)
- **Rust** for the terminal (WezTerm crates, wgpu)
- **C++** for the browser (Chromium Content API)

Before building any real functionality, we must prove that:

1. A Swift window can receive and composite GPU textures from foreign processes.
2. A Rust process can render to an IOSurface and send it via XPC.
3. A C++ process can render to an IOSurface and send it via XPC.
4. The compositing runs at 60fps with no visible artifacts.
5. The three build systems (Xcode/SwiftPM, Cargo, CMake/Make) can coexist.

If any of these fail, we discover it now — not after building a terminal or
embedding Chromium.

### Why Swift for the window

Issue 401 (programming-language.md) recommended Rust for the window with
terminal in-process. Issue 402 (wezterm-vs-alacritty.md) moved the terminal
out-of-process, making the window a pure compositor. With no terminal library
embedded, the window process has no Rust dependency. This reopens the language
choice.

Swift wins for the window because:

1. **XPC is native.** `xpc_connection_create`, `xpc_dictionary_set_mach_send`,
   `xpc_connection_send_message` — all C API, callable from Swift with zero
   bridging. No `block2` crate, no unsafe FFI.

2. **Metal is native.** `MTLDevice`, `MTLTexture`, `IOSurface` — Swift has
   first-class access. Creating a Metal texture from an IOSurface is a
   one-liner: `device.makeTexture(descriptor:iosurface:plane:)`.

3. **AppKit is native.** `NSWindow`, `NSView`, `CAMetalLayer` — no winit
   abstraction layer, no platform quirks. Window management, fullscreen, split
   view, menu bar, notifications — all free.

4. **The window is simple.** It's a compositor and input router. It doesn't
   parse VTE sequences, shape fonts, or run a JavaScript engine. The complexity
   lives in the terminal and browser processes. The window is glue — and Swift
   is excellent glue for macOS.

5. **macOS-first is accepted.** Issue 401 established that macOS-first is fine.
   If we port to Linux/Windows later, only the window process needs rewriting
   (to Rust + winit + wgpu or similar). The terminal and browser processes are
   unchanged.

### Why not Rust for the window

With the terminal out-of-process (Issue 402), the window doesn't embed any Rust
library. The only Rust code it would use is `termsurf-xpc` — XPC bindings that
wrap the same C API Swift calls natively. Using Rust for the window means:

- winit for window management (adequate but not native)
- wgpu for compositing (works but adds a layer over Metal)
- `termsurf-xpc` for XPC (1,417 lines wrapping what Swift does in 0 lines)
- Objective-C FFI via `block2` and `objc` crates for any macOS-native feature

All of this is overhead to avoid writing Swift. With the terminal
out-of-process, that overhead buys nothing.

## Architecture

```
┌──────────────────────────────────────────────────────────┐
│                                                          │
│  Swift Window Process                                    │
│  ├── NSWindow + CAMetalLayer                             │
│  ├── MTLDevice + MTLCommandQueue                         │
│  ├── XPC connections to child processes                  │
│  ├── IOSurface → MTLTexture import                       │
│  ├── Metal render pass (composite two textures)          │
│  └── Input forwarding (future: keyboard/mouse via XPC)   │
│       │                    │                             │
│       │ XPC                │ XPC                         │
│       ▼                    ▼                             │
│  Rust Process         C++ Process                        │
│  ├── wgpu device      ├── wgpu or Metal device           │
│  ├── Render blue      ├── Render green                   │
│  │   to IOSurface     │   to IOSurface                   │
│  ├── Create Mach      ├── Create Mach                    │
│  │   port from        │   port from                      │
│  │   IOSurface        │   IOSurface                      │
│  └── Send via XPC     └── Send via XPC                   │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

### Process Lifecycle

1. Swift window launches and creates two XPC anonymous listeners.
2. Swift spawns the Rust process, passing the first listener's endpoint as a
   launch argument (serialized via `xpc_endpoint_create` or Mach bootstrap).
3. Swift spawns the C++ process, passing the second listener's endpoint.
4. Each child connects to its assigned listener, establishing a direct XPC
   channel.
5. Each child renders its color to an IOSurface, creates a Mach port, and sends
   it to the Swift window.
6. Swift imports the IOSurface, creates a Metal texture, and composites both
   textures in a render pass.
7. On resize, Swift sends new dimensions to each child via XPC. Each child
   creates a new IOSurface at the new size and sends the updated Mach port.

### XPC Protocol

All messages are XPC dictionaries. The protocol is identical for both children.

**Window → Child:**

| Key      | Type   | Description                         |
| -------- | ------ | ----------------------------------- |
| `action` | string | `"resize"`                          |
| `width`  | int64  | Pane width in pixels                |
| `height` | int64  | Pane height in pixels               |
| `scale`  | string | Device scale factor (e.g., `"2.0"`) |

**Child → Window:**

| Key              | Type      | Description              |
| ---------------- | --------- | ------------------------ |
| `action`         | string    | `"frame"`                |
| `iosurface_port` | mach_send | Mach port for IOSurface  |
| `width`          | int64     | Texture width in pixels  |
| `height`         | int64     | Texture height in pixels |

Future extensions (keyboard, mouse, cursor) will use the same channel.

### Endpoint Bootstrapping

XPC anonymous endpoints can't be passed as CLI arguments directly. Instead:

**Option A: Mach bootstrap (preferred)**

1. Swift registers a named Mach service (e.g., `com.termsurf.window.<pid>`).
2. Children connect to the named service.
3. Swift sends each child its dedicated anonymous endpoint via the named
   channel.
4. Children create a new connection from the endpoint for direct communication.

**Option B: Launcher pattern (from ts3)**

1. A launcher process (`com.termsurf.launcher`) acts as a rendezvous point.
2. Swift sends endpoints to the launcher with session IDs.
3. Children claim their endpoints from the launcher.

For this prototype, Option A is simpler. No launcher needed.

**Option C: File descriptor passing**

1. Swift creates a Unix socket pair before spawning each child.
2. Pass one end to the child as a file descriptor.
3. Use the socket for initial handshake, then upgrade to XPC.

Option A is most natural for macOS.

## Implementation Plan

### Swift Window Process

**Framework:** AppKit + Metal (no SwiftUI for the rendering layer)

**Key components:**

```
termsurf-window/
├── Package.swift           (or Xcode project)
├── Sources/
│   ├── main.swift          Entry point
│   ├── WindowController.swift
│   ├── MetalView.swift     NSView subclass with CAMetalLayer
│   ├── Compositor.swift    Metal render pass (two textured quads)
│   ├── XPCManager.swift    Manage child connections
│   └── IOSurfaceImport.swift  IOSurface → MTLTexture
```

**Metal compositing** is straightforward: two textured quads covering the left
and right halves of the viewport. Vertex shader positions them; fragment shader
samples the IOSurface-backed textures.

**IOSurface import** in Swift:

```swift
let surface = IOSurfaceLookupFromMachPort(machPort)
let descriptor = MTLTextureDescriptor.texture2DDescriptor(
    pixelFormat: .bgra8Unorm,
    width: Int(IOSurfaceGetWidth(surface)),
    height: Int(IOSurfaceGetHeight(surface)),
    mipmapped: false
)
let texture = device.makeTexture(
    descriptor: descriptor,
    iosurface: surface,
    plane: 0
)
```

### Rust Child Process (Blue)

**Framework:** wgpu with Metal backend

**Key components:**

```
termsurf-terminal/
├── Cargo.toml
└── src/
    ├── main.rs         Entry point, XPC connection, render loop
    ├── renderer.rs     wgpu setup, blue rectangle render
    └── iosurface.rs    Create IOSurface, render to it, create Mach port
```

**Rendering to IOSurface from wgpu:**

wgpu's Metal backend can create textures from IOSurface via
`hal::metal::Device::texture_from_raw`. The flow:

1. Create an IOSurface (via CoreFoundation C API)
2. Create a Metal texture backed by the IOSurface
3. Wrap it as a wgpu texture via unsafe HAL access
4. Render to it with a standard wgpu render pass (clear to blue)
5. Create a Mach port from the IOSurface
6. Send the Mach port via XPC

**XPC:** Reuse `termsurf-xpc` crate from ts3.

### C++ Child Process (Green)

**Framework:** wgpu-native (C API) or raw Metal (Objective-C++)

**Key components:**

```
termsurf-browser/
├── CMakeLists.txt      (or Makefile)
└── src/
    ├── main.cpp        Entry point, XPC connection, render loop
    ├── renderer.mm     Metal setup, green rectangle render (Obj-C++)
    └── iosurface.mm    Create IOSurface, render to it, create Mach port
```

**Rendering to IOSurface from C++:**

On macOS, the simplest path is Objective-C++ with Metal directly:

1. Create IOSurface via `IOSurfaceCreate()`
2. Create `MTLTexture` from IOSurface via
   `[device newTextureWithDescriptor:iosurface:plane:]`
3. Create a render pass that clears to green
4. Encode and commit the command buffer
5. Create Mach port via `IOSurfaceCreateMachPort()`
6. Send via `xpc_dictionary_set_mach_send()`

**XPC from C++:** libxpc is a C API — call directly from C++ with no wrappers.

Using raw Metal instead of wgpu-native for the C++ process is pragmatic:
Chromium's eventual integration will use Chromium's own GPU pipeline, not wgpu.
The C++ process just needs to produce an IOSurface. How it renders internally
doesn't matter to the window.

## Build System

Three independent builds, no cross-dependencies:

```bash
# Swift window
cd ts4/termsurf-window && swift build
# or: xcodebuild -scheme TermSurf

# Rust terminal (blue pane for now)
cd ts4/termsurf-terminal && cargo build

# C++ browser (green pane for now)
cd ts4/termsurf-browser && make
# or: cmake --build build
```

**Directory structure:**

```
ts4/
├── termsurf-window/        Swift (SwiftPM or Xcode)
│   ├── Package.swift
│   └── Sources/
├── termsurf-terminal/      Rust
│   ├── Cargo.toml
│   └── src/
├── termsurf-browser/       C++/Obj-C++
│   ├── Makefile
│   └── src/
└── termsurf-xpc/           Rust XPC library (shared, from ts3)
    ├── Cargo.toml
    └── src/
```

The three binaries are standalone executables. The Swift window spawns the other
two as child processes.

## Success Criteria

1. **Window opens** with a blue left pane and green right pane.
2. **60fps** compositing with no tearing or flickering.
3. **Resize works** — dragging the window edge resizes both panes.
4. **Three processes** visible in Activity Monitor.
5. **Clean shutdown** — closing the window terminates all three processes.
6. **No shared code** between Swift, Rust, and C++ beyond the XPC protocol.

## What This Proves

| Question                                              | Answer                                    |
| ----------------------------------------------------- | ----------------------------------------- |
| Can Swift composite IOSurface from foreign processes? | Yes — Metal + IOSurfaceLookupFromMachPort |
| Can Rust render to IOSurface via wgpu?                | Yes — Metal HAL texture_from_raw          |
| Can C++ render to IOSurface via Metal?                | Yes — standard Metal API                  |
| Can XPC transfer Mach ports between Swift/Rust/C++?   | Yes — libxpc is language-agnostic C API   |
| Is 60fps achievable cross-process?                    | Measured (target: <2ms per frame for IPC) |
| Can three build systems coexist?                      | SwiftPM + Cargo + Make, no conflicts      |

## What This Does NOT Prove

- Terminal emulation performance (no VTE, no PTY)
- Chromium embedding feasibility (no Content API)
- Text rendering quality (no fonts, no glyphs)
- Input forwarding latency (no keyboard/mouse events yet)
- Multi-profile process management (no launcher, no profile isolation)

These are separate issues. This prototype validates only the process model, IPC
mechanism, and GPU texture sharing — the foundation everything else builds on.

## Relationship to Other Issues

| Issue                      | Dependency                                                           |
| -------------------------- | -------------------------------------------------------------------- |
| 400 (A New Hope)           | This implements Phase 1 (window + compositor)                        |
| 401 (Chromium feasibility) | Independent — browser research continues in parallel                 |
| 401 (Programming language) | This supersedes 401's recommendation: Swift replaces Rust for window |
| 402 (WezTerm vs Alacritty) | Terminal is out-of-process as recommended; WezTerm crates used later |

## After This Prototype

1. **Replace blue with terminal.** The Rust process uses `wezterm-term` +
   `wezterm-font` + wgpu to render a real terminal instead of a blue rectangle.
   Same IOSurface output, same XPC protocol.

2. **Replace green with browser.** The C++ process uses Chromium Content API to
   render a real webpage instead of a green rectangle. Same IOSurface output,
   same XPC protocol.

3. **Add input forwarding.** Swift sends keyboard and mouse events to the
   focused pane's process via XPC. Same channel, new message types.

4. **Add pane management.** Split, resize, focus, multiple tabs.

The colored rectangles are placeholders. The architecture is the product.
