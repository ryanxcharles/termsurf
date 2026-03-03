# Issue 698: Replace XPC with Unix Domain Sockets

## Goal

Eliminate XPC as the IPC mechanism and replace it with Unix domain sockets. This
removes the xpc-gateway daemon, the launchd plist, and the macOS-specific IPC
layer — enabling cross-platform IPC with a single codebase for macOS and Linux.

## Background

### The original XPC justification

XPC was chosen because IOSurface textures can only be transferred between
processes via Mach ports, and Mach ports can only be transferred via XPC. This
was a hard macOS kernel constraint. Every IPC channel used XPC because the
texture channel required it, and a second mechanism would add complexity for
zero benefit.

This constraint was documented in CLAUDE.md as a "settled architectural
decision" and proven across ts3 (Issues 303, 325-350) and ts4 (Issues 403, 407).

### CALayerHost eliminated the constraint

Issue 625 replaced `FrameSinkVideoCapturer` with `CALayerHost`. The old pipeline
transferred an IOSurface Mach port over XPC on every frame (~60fps). The new
pipeline sends a `ca_context_id` (uint32) once per tab. Window Server composites
directly from GPU VRAM — zero per-frame IPC.

The `ca_context_id` is just an integer. CALayerHost is a Window Server mechanism
— any process that knows the ID can create a `CALayerHost` and display the
remote content. No Mach port transfer, no XPC-specific capability required.

**The original justification for XPC no longer applies.** The CLAUDE.md "settled
architectural decision" was written for the IOSurface era and never revisited
after the CALayerHost migration (Issues 624-632).

### What XPC currently provides

Reviewing all 30 XPC message types across three processes (GUI, TUI, Chromium
Profile Server):

| Data type      | Count | Example messages                      |
| -------------- | ----- | ------------------------------------- |
| string         | 30/30 | URLs, pane IDs, profiles, actions     |
| int64          | 20/30 | tab IDs, cursor types, key codes      |
| uint64         | 14/30 | pixel dimensions, modifiers, progress |
| double         | 6/30  | mouse coordinates, scroll deltas      |
| bool           | 10/30 | focus, dark mode, browsing state      |
| xpc_endpoint_t | 1/30  | gateway connect reply                 |

Every message is a flat dictionary of basic types. The only XPC-specific feature
still in use is `xpc_endpoint_t` — which is only needed because XPC itself
requires it for the gateway handshake pattern.

### Why the gateway exists

A Mach service can only be claimed by the process launchd launched for that job.
When you launch TermSurf via `open TermSurf.app`, macOS assigns it an
application bundle identity that doesn't match the launchd plist's job identity.
The kernel rejects the app's attempt to claim the Mach service.

The gateway (~86 lines of Swift) is a tiny daemon that launchd launches to own
the `com.termsurf.xpc-gateway` Mach service. It handles exactly two messages:

1. `register_app` — TermSurf app deposits its anonymous listener endpoint
2. `connect` — `web` TUI (and Chromium Profile Server) claims the endpoint

After the handshake, the gateway is idle. All traffic flows directly between
processes.

**With Unix domain sockets, none of this is needed.** The app listens on a
well-known socket path. Clients connect directly. No launchd, no plist, no
gateway daemon, no endpoint transfer.

## Analysis

### What changes

**Eliminated entirely:**

- `gui/xpc-gateway/` — the entire gateway daemon (~86 lines Swift)
- `gui/macos/com.termsurf.xpc-gateway.plist` — launchd plist
- `gui/macos/com.termsurf.debug.xpc-gateway.plist` — debug launchd plist
- Gateway installation/registration in build and install scripts
- The `TERMSURF_XPC_SERVICE` environment variable (Issue 653)

**Rewritten:**

- `gui/src/apprt/xpc.zig` (~1800 lines) — replace XPC C API calls with Unix
  socket + message serialization. Rename to `ipc.zig`.
- `tui/src/xpc.rs` (~670 lines) — replace XPC FFI with Unix socket + message
  serialization. Rename to `ipc.rs`.
- `chromium/src/content/chromium_profile_server/browser/shell_browser_main_parts.cc`
  — replace XPC connection setup with Unix socket client.
- `scripts/install.sh` — remove gateway bundling and plist registration steps.

**Unchanged:**

- CALayerHost/CAContext pipeline — unaffected, it's a Window Server mechanism
- All message semantics — same 30 message types, same fields, same directions
- Process architecture — still three processes (GUI, TUI, Chromium server)
- Profile server lifecycle — GUI still spawns and manages Chromium servers

### Service discovery

Replace launchd Mach service with a well-known socket path:

```
$XDG_RUNTIME_DIR/termsurf/termsurf.sock          (release)
$XDG_RUNTIME_DIR/termsurf/termsurf-debug.sock    (debug, replaces Issue 653)
```

On macOS, `$XDG_RUNTIME_DIR` is typically not set. TermSurf already has XDG
handling (Issue 615) — it would fall back to `$TMPDIR/termsurf/` or
`/tmp/termsurf-$UID/`.

The GUI creates the socket on startup and removes it on shutdown. Stale socket
files (from crashes) are detected by attempting to connect — if the connection
is refused, the file is stale and can be replaced.

### Message serialization

XPC dictionaries are replaced with Protocol Buffers (protobuf). All 30 messages
are defined once in a `.proto` file, and code generation produces type-safe
serializers for all three languages automatically.

**Why protobuf:**

- **Single schema, three languages.** One `.proto` file generates Rust, Zig, and
  C++ code. No hand-written parsers, no field ordering bugs, no type mismatches.
- **Extensible.** Adding new messages or fields to existing messages is
  backward-compatible. The message count will grow significantly as features are
  added (downloads, file uploads, JS dialogs, permissions, camera/mic, etc.).
- **Mature libraries.** C++ has Google's official `libprotobuf` (already in
  Chromium at `third_party/protobuf/`). Rust has
  [prost](https://github.com/tokio-rs/prost) (`prost = "0.14"`). Zig has
  [zig-protobuf](https://github.com/Arwalk/zig-protobuf) or can use
  [protobuf-c](https://github.com/allyourcodebase/protobuf-c) via C interop.
- **Compact binary encoding.** Smaller than JSON or msgpack for typed messages.
  No float precision issues.

**Wire format:** Length-prefixed protobuf messages over the Unix socket:

```
[4 bytes: message length][protobuf payload]
```

The `.proto` file lives at the repo root (e.g., `proto/termsurf.proto`) and is
shared by all three build systems.

### Synchronous request/reply

5 messages use XPC's synchronous request/reply (`xpc_dictionary_create_reply`):
`hello`, `query_last`, `query_devtools`, `query_tabs`, and gateway `connect`.

With Unix sockets, synchronous request/reply is implemented by including a
sequence ID in the request and blocking until a response with the same sequence
ID arrives. Or, since these are all TUI→GUI queries, the TUI can simply block on
`recv()` after sending the request — the GUI responds on the same connection.

### Connection topology

**Current (XPC):**

```
GUI ←──xpc_endpoint──→ Gateway ←──xpc_endpoint──→ TUI
GUI ←──xpc_endpoint──→ Gateway ←──xpc_endpoint──→ Chromium Server
GUI ←── direct XPC ──→ TUI (after endpoint handshake)
GUI ←── direct XPC ──→ Chromium Server (after endpoint handshake)
```

**Proposed (Unix sockets):**

```
GUI ←── unix socket ──→ TUI        (direct, no intermediary)
GUI ←── unix socket ──→ Chromium Server  (direct, no intermediary)
```

Each client gets its own connection (accept() returns a new fd). The GUI's event
loop polls all connected fds.

### Chromium server connection

The Chromium Profile Server currently connects to the gateway, requests an
endpoint, then connects directly to the GUI. With Unix sockets, it connects
directly to the socket path. The `--xpc-service` command-line flag becomes
`--ipc-socket` pointing to the socket file path.

### Event loop integration

**GUI (Zig):** Currently uses a GCD serial dispatch queue (`xpc_queue`) for all
XPC event handlers. With Unix sockets, the GUI can either:

- Use `poll()`/`kqueue()` on the socket fds, integrated into the existing event
  loop
- Continue using a GCD dispatch source (`dispatch_source_create` with
  `DISPATCH_SOURCE_TYPE_READ`) to maintain the serial-queue-no-mutex pattern

**TUI (Rust):** Currently uses `block2` crate for ObjC block callbacks. With
Unix sockets, it uses standard `std::net::UnixStream` — pure Rust, no FFI, no
ObjC blocks. The `block2` dependency can be removed entirely.

**Chromium (C++):** Currently uses XPC C API with ObjC blocks. With Unix
sockets, it uses standard POSIX `connect()`/`send()`/`recv()`. No ObjC
dependency for IPC.

### Cross-platform implications

Unix domain sockets work identically on macOS and Linux. This means:

- The IPC layer is platform-agnostic from day one
- On Linux, the same socket-based IPC works without any adaptation
- The GPU compositing layer (CALayerHost vs Wayland subsurfaces) remains
  platform-specific, but IPC is shared

### GPU compositing on Linux

On macOS, CALayerHost lets Window Server composite Chromium's GPU output
directly into TermSurf's window — zero copies, zero per-frame IPC. The closest
Linux analog is **Wayland subsurfaces**.

Wayland subsurfaces work the same way: Chromium renders to its own Wayland
surface, and the Wayland compositor composites that surface at specific
coordinates within TermSurf's window. The compositor handles GPU buffer sharing
directly from VRAM, same as Window Server.

Chromium already has full Wayland support via `--ozone-platform=wayland`. The
rendering side is solved upstream. The integration question is how to get
Chromium's Wayland surface reparented as a subsurface of TermSurf's window —
similar to the CALayerHost `contextId` handshake, but using Wayland's
`wl_subsurface` protocol.

**Wayland is the default display server on every major desktop Linux distro:**

- Fedora — default since 2016 (GNOME) and 2021 (KDE)
- Ubuntu — default since 22.04 LTS (2022)
- Debian — default since Debian 10 (2019)
- Linux Mint — switched to Wayland in 2026
- KDE Plasma — going Wayland-only, dropping X11 support entirely

X11 is effectively legacy. Targeting Wayland only for a Linux port is safe.

### What could go wrong

1. **Stale socket files.** If the app crashes, the socket file persists. Fix:
   check if the socket is live before creating a new one (attempt connect, if
   refused → stale → unlink and recreate).

2. **Permissions.** Socket file inherits directory permissions. The
   `$XDG_RUNTIME_DIR` directory is user-owned (mode 0700 on Linux), so this is
   fine. On macOS with `$TMPDIR`, same situation.

3. **Multiple instances.** Currently the gateway stores one app endpoint — if
   two TermSurf instances run, the second overwrites the first. With Unix
   sockets, the second instance would fail to bind. Fix: include PID or instance
   ID in socket name, or use a directory of sockets.

4. **macOS sandbox.** TermSurf is not sandboxed or notarized. If it ever is,
   Unix sockets in `$TMPDIR` require explicit sandbox entitlements. XPC
   integrates with the sandbox natively. This is a future concern, not a current
   blocker.

5. **File descriptor limits.** Each connection uses one fd. With ~3 connections
   (TUI, Chromium server, maybe a second TUI), this is negligible.

### Scope estimate

| Component                   | Current lines       | Work required                                                      |
| --------------------------- | ------------------- | ------------------------------------------------------------------ |
| xpc-gateway (Swift)         | 86                  | Delete entirely                                                    |
| launchd plists              | 24 each             | Delete                                                             |
| xpc.zig (GUI)               | ~1800               | Rewrite connection setup (~200 lines), keep message handlers       |
| xpc.rs (TUI)                | ~670                | Rewrite connection (~100 lines), simplify (remove block2/ObjC FFI) |
| shell_browser_main_parts.cc | ~50 lines XPC setup | Rewrite to Unix socket client                                      |
| install.sh                  | ~10 lines gateway   | Delete gateway steps                                               |
| build scripts               | gateway build steps | Remove                                                             |

Most of the existing code is message handling logic (parsing fields, looking up
panes, calling surface methods) — that stays the same. Only the transport layer
changes.

## Experiments

### Experiment 1: Protobuf schema and code generation

Define all 30 IPC messages in a `.proto` file and verify that code generation
produces compilable output for all three languages (Rust, Zig, C++). This
validates the serialization layer before touching any transport code.

#### Changes

**1. Create `proto/termsurf.proto`**

Define a proto3 schema with all 30 current message types. Use a wrapper `oneof`
pattern so every IPC message is a single `TermSurfMessage` type:

```protobuf
syntax = "proto3";
package termsurf;

message TermSurfMessage {
  oneof msg {
    CreateTab create_tab = 1;
    CreateDevtoolsTab create_devtools_tab = 2;
    TabReady tab_ready = 3;
    Resize resize = 4;
    CloseTab close_tab = 5;
    Navigate navigate = 6;
    CaContext ca_context = 7;
    UrlChanged url_changed = 8;
    LoadingState loading_state = 9;
    TitleChanged title_changed = 10;
    MouseEvent mouse_event = 11;
    MouseMove mouse_move = 12;
    ScrollEvent scroll_event = 13;
    KeyEvent key_event = 14;
    FocusChanged focus_changed = 15;
    SetColorScheme set_color_scheme = 16;
    ModeChanged mode_changed = 17;
    CursorChanged cursor_changed = 18;
    SetOverlay set_overlay = 19;
    SetDevtoolsOverlay set_devtools_overlay = 20;
    ServerRegister server_register = 21;
    HelloRequest hello_request = 22;
    HelloReply hello_reply = 23;
    QueryLastRequest query_last_request = 24;
    QueryLastReply query_last_reply = 25;
    QueryDevtoolsRequest query_devtools_request = 26;
    QueryDevtoolsReply query_devtools_reply = 27;
    QueryTabsRequest query_tabs_request = 28;
    QueryTabsReply query_tabs_reply = 29;
    OpenSplit open_split = 30;
  }
}
```

Each inner message type has fields matching the current XPC dictionary keys.
Request/reply pairs are separate message types (no XPC reply mechanism needed).

**2. Rust — add prost to `tui/Cargo.toml` and create `tui/build.rs`**

```toml
[dependencies]
prost = "0.14"

[build-dependencies]
prost-build = "0.14"
```

`tui/build.rs` runs `prost_build::compile_protos(&["../proto/termsurf.proto"])`.
The generated Rust code lands in `target/` and is included via
`include!(concat!(env!("OUT_DIR"), "/termsurf.rs"))` in a new `tui/src/proto.rs`
module.

**3. C++ — run `protoc` manually and check in the generated code**

```bash
protoc --cpp_out=chromium/src/content/chromium_profile_server/proto \
  proto/termsurf.proto
```

This generates `termsurf.pb.h` and `termsurf.pb.cc`. Check them into the
Chromium fork so the Chromium build doesn't need a `protoc` step (it already has
`libprotobuf` at `third_party/protobuf/`). Add the generated files to the
profile server's `BUILD.gn` sources list.

**4. Zig — evaluate protobuf-c via C interop**

Zig can call C directly. Use the
[protobuf-c](https://github.com/allyourcodebase/protobuf-c) library:

```bash
protoc --c_out=gui/src/apprt/proto proto/termsurf.proto
```

This generates `termsurf.pb-c.h` and `termsurf.pb-c.c`. Add them to the Zig
build via `addCSourceFile()` and link against `libprotobuf-c`. The Zig code
calls the C serialization functions directly — no Zig-native protobuf library
needed.

If protobuf-c integration proves difficult (build system conflicts with
Ghostty's build.zig), fall back to
[zig-protobuf](https://github.com/Arwalk/zig-protobuf) which is a pure Zig
implementation.

#### Verification

1. `cd tui && cargo build` — compiles with prost-generated Rust code, no errors.
2. Zig build (`cd gui && zig build`) — compiles with protobuf-c generated C
   code, no errors.
3. Write a minimal round-trip test in Rust: create a `TermSurfMessage` with a
   `CreateTab` payload, serialize to bytes, deserialize back, assert fields
   match. Run with `cargo test`.
4. Verify the `.proto` schema covers all 30 current XPC message types by
   cross-referencing against the XPC message inventory in `xpc.zig` and
   `xpc.rs`.
