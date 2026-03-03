# Issue 700: Replace TUI↔GUI XPC with Unix Sockets

## Goal

Replace the TUI↔GUI IPC channel with Unix domain sockets + protobuf. The
GUI↔Chromium channel stays on XPC (a separate issue). This eliminates the
xpc-gateway daemon for TUI connections and removes all ObjC FFI from the TUI.

## Background

Issue 698 proved the full stack across four experiments:

1. Protobuf works in all three languages (Experiment 1)
2. The full 30-message schema compiles and round-trips (Experiment 2)
3. Unix sockets work across Zig and Rust (Experiment 3)
4. Production integration was blocked only by build system (Experiment 4)

Issue 699 solved the build blocker: protobuf-c compiles and links into the GUI's
final macOS binary using the stb.c pattern (`gui/src/protobuf/`).

### What changes

**TUI (`tui/src/xpc.rs` → `tui/src/ipc.rs`):** Replace 710 lines of unsafe XPC
FFI with pure Rust sockets + prost. No more `block2`, no more `extern "C"`, no
more `CString`. The public API stays identical — `CompositorConnection` with the
same methods.

**GUI (`gui/src/apprt/xpc.zig`):** Add a Unix socket listener alongside the
existing XPC listener. TUI messages arrive over the socket; Chromium messages
still arrive over XPC. Both feed into the same handler functions. The TUI's
`web_peer` (currently an `xpc_object_t`) becomes a socket fd for sending replies
and events back.

**Gateway:** No longer needed for TUI connections. Still needed for Chromium
(until that channel is also migrated). The gateway can be removed entirely once
both channels use sockets.

### Architecture

```
Before:  TUI --XPC--> Gateway --endpoint--> GUI
After:   TUI --socket+protobuf--> GUI (direct)
```

The GUI listens on a Unix domain socket. The TUI connects by path. No
intermediary, no endpoint handshake, no launchd dependency.

### Socket path

```
$TMPDIR/termsurf/gui.sock          (macOS release)
$TMPDIR/termsurf/gui-debug.sock   (macOS debug)
$XDG_RUNTIME_DIR/termsurf/gui.sock (Linux)
```

The GUI creates the socket on startup, removes it on shutdown. Stale sockets
(from crashes) are detected by attempting connect — if refused, the file is
stale and can be replaced.

### Wire format

Same as Issue 698 Experiment 3: 4-byte little-endian length prefix before each
serialized `TermSurfMessage`.

```
[4 bytes: message length (u32 LE)] [N bytes: serialized TermSurfMessage]
```

### Message inventory

TUI → GUI (commands and queries):

| Message                | Type        | Fields                                                      |
| ---------------------- | ----------- | ----------------------------------------------------------- |
| `SetOverlay`           | fire-forget | pane_id, col, row, width, height, url, profile, browsing    |
| `SetDevtoolsOverlay`   | fire-forget | pane_id, col, row, width, height, inspected_tab_id, profile |
| `Navigate`             | fire-forget | pane_id, url                                                |
| `SetColorScheme`       | fire-forget | pane_id, dark                                               |
| `ModeChanged`          | fire-forget | pane_id, browsing                                           |
| `OpenSplit`            | fire-forget | pane_id, direction, command                                 |
| `HelloRequest`         | sync query  | pane_id                                                     |
| `QueryLastRequest`     | sync query  | pane_id, profile                                            |
| `QueryDevtoolsRequest` | sync query  | pane_id, inspected_tab_id, profile                          |
| `QueryTabsRequest`     | sync query  | pane_id, profile                                            |

GUI → TUI (replies and events):

| Message              | Type        | Fields                                                                     |
| -------------------- | ----------- | -------------------------------------------------------------------------- |
| `HelloReply`         | sync reply  | homepage                                                                   |
| `QueryLastReply`     | sync reply  | pane_id, tab_id, profile, error                                            |
| `QueryDevtoolsReply` | sync reply  | tab_id, error                                                              |
| `QueryTabsReply`     | sync reply  | gui_panes, chromium_tabs, chromium_browser, chromium_devtools, tabs, error |
| `ModeChanged`        | async event | browsing                                                                   |
| `UrlChanged`         | async event | tab_id (ignored by TUI), url                                               |
| `LoadingState`       | async event | tab_id (ignored by TUI), state, progress                                   |
| `TitleChanged`       | async event | tab_id (ignored by TUI), title                                             |

### Synchronous queries

The TUI has 4 synchronous queries (`hello`, `query_last`, `query_devtools`,
`query_tabs`). With XPC, these use
`xpc_connection_send_message_with_reply_sync`.

With sockets, the TUI spawns a reader thread that reads all incoming messages.
Reply messages route to a pending-reply channel; event messages route to the
existing `LoopEvent` mpsc channel. Since the TUI sends queries one at a time
(single-threaded main loop), there is at most one pending query. The query
method writes the request, then blocks on the reply channel.

### Thread safety

The GUI already uses a serial GCD queue (`xpc_queue`) for all XPC handlers. The
socket reader uses `dispatch_source` (`DISPATCH_SOURCE_TYPE_READ`) targeting the
same queue. Same serialization guarantee — no additional mutexes needed.

### SetColorScheme field mapping

The current XPC `set_color_scheme` sends a `scheme` string (`"dark"`, `"light"`,
`"system"`). The protobuf `SetColorScheme` message has a `dark` bool. The TUI
will convert: `"dark"` → `dark=true`, `"light"` → `dark=false`, `"system"` →
`dark=false` (let the GUI decide). The GUI handler already converts the scheme
string to a bool internally, so this simplifies the path.

### ModeChanged field mapping

The current XPC `mode_changed` from TUI→GUI sends `pane_id` + `browsing`. The
protobuf `ModeChanged` message only has `browsing`. The `pane_id` needs to be
added to the protobuf message — update `proto/termsurf.proto` to add
`string pane_id = 2` to `ModeChanged`.

Similarly, the GUI→TUI `ModeChanged` currently only sends `browsing` (no
pane_id), which matches the protobuf schema. No change needed for that
direction.

## Experiments

### Experiment 1: TUI ipc.rs (socket client)

Create `tui/src/ipc.rs` with the same public API as `xpc.rs` but using Unix
sockets + prost. Switch `main.rs` from `mod xpc` to `mod ipc`. Verify it
compiles.

#### Changes

**1. Update `proto/termsurf.proto`**

Add `string pane_id = 2` to `ModeChanged` (needed for TUI→GUI direction).

Regenerate: run `proto/generate.sh` and `cd tui && cargo build` (prost
regenerates automatically via `build.rs`).

**2. Add prost to `tui/Cargo.toml`**

```toml
[dependencies]
prost = "0.14"

[build-dependencies]
prost-build = "0.14"
```

Remove `block2` (no longer needed — XPC ObjC blocks are gone).

**3. Create `tui/build.rs`**

```rust
fn main() {
    prost_build::Config::new()
        .compile_protos(&["../proto/termsurf.proto"], &["../proto/"])
        .unwrap();
}
```

**4. Create `tui/src/ipc.rs`**

Same public API as `xpc.rs`:

```rust
pub enum CompositorMessage {
    ModeChanged { browsing: bool },
    UrlChanged { url: String },
    LoadingState { state: String, _progress: u8 },
    TitleChanged { title: String },
}

pub struct CompositorConnection { ... }

impl CompositorConnection {
    pub fn connect(tx: Sender<LoopEvent>) -> Option<Self> { ... }
    pub fn send_set_overlay(&self, ...) { ... }
    pub fn send_set_devtools_overlay(&self, ...) { ... }
    pub fn send_hello(&self, pane_id: &str) -> Option<String> { ... }
    pub fn send_query_last(&self, ...) -> Option<(String, String, i64)> { ... }
    pub fn send_query_devtools(&self, ...) -> Result<i64, String> { ... }
    pub fn send_query_tabs(&self, ...) -> Result<String, String> { ... }
    pub fn send_navigate(&self, pane_id: &str, url: &str) { ... }
    pub fn send_set_color_scheme(&self, pane_id: &str, scheme: &str) { ... }
    pub fn send_open_split(&self, ...) { ... }
    pub fn send_mode_changed(&self, pane_id: &str, browsing: bool) { ... }
}
```

Internals:

- `connect()`: Build socket path from `$TMPDIR`, connect via
  `std::os::unix::net::UnixStream`, spawn reader thread.
- Reader thread: Loop reading 4-byte length prefix + protobuf payload. Dispatch
  by `oneof` type: reply messages → `reply_tx` channel, event messages →
  `LoopEvent` mpsc channel.
- Fire-and-forget methods: Serialize `TermSurfMessage` with prost, write
  length-prefixed to socket.
- Sync query methods: Write request, block on `reply_rx`, deserialize reply.

**5. Switch `main.rs` from `xpc` to `ipc`**

Change `mod xpc;` to `mod ipc;`. Change all `xpc::CompositorMessage` to
`ipc::CompositorMessage` and `xpc::CompositorConnection` to
`ipc::CompositorConnection`. Change `LoopEvent::Xpc` to `LoopEvent::Ipc`.

#### Verification

```bash
cd tui && cargo build
```

**Pass criterion:** Compiles with zero errors. No runtime test — the GUI socket
listener doesn't exist yet.

### Experiment 2: GUI socket listener

Add a Unix domain socket listener to `xpc.zig`. Accept TUI connections, read
length-prefixed protobuf messages, and dispatch to the existing handler
functions. Send replies and events back over the socket.

#### Changes

**1. Add socket listener to `init()`**

After the existing XPC setup:

- Build socket path from `$TMPDIR` (or `$XDG_RUNTIME_DIR` on Linux)
- `mkdir -p` the parent directory
- Unlink any stale socket
- Create `AF_UNIX` / `SOCK_STREAM` socket, bind, listen
- Create `dispatch_source` (`DISPATCH_SOURCE_TYPE_READ`) on the listen fd,
  targeting `xpc_queue`
- The handler calls `accept()`, stores the client fd, creates a per-connection
  `dispatch_source` for reading

**2. Add per-connection read buffer and message extraction**

Each TUI connection gets a read buffer. When the dispatch source fires:

- `read()` into the buffer
- Extract complete messages (4-byte LE length + payload)
- Deserialize with `termsurf__term_surf_message__unpack()`
- Switch on `msg_case` and call the existing handler function

**3. Replace `web_peer` with socket fd**

Currently `Pane.web_peer` is an `xpc_object_t` used to send events to the TUI.
Add a `web_fd: i32 = -1` field. When a TUI connects via socket,
`handleSetOverlay` stores the fd instead of the XPC connection. Event forwarding
(`forwardUrlChanged`, `forwardLoadingState`, `forwardTitleChanged`,
`sendModeChanged`) serializes a protobuf message and writes it to `web_fd`.

For query handlers (`handleHello`, `handleQueryLast`, `handleQueryDevtools`,
`handleQueryTabs`), serialize the reply and write to the client fd.

**4. Add socket cleanup to `deinit()`**

Close the listen fd, unlink the socket file, close all client fds, cancel
dispatch sources.

**5. Regenerate `gui/src/protobuf/` files**

Run `proto/generate.sh` after the proto schema change in Experiment 1.

#### Verification

1. `cd gui && zig build` — compiles without errors
2. `cd tui && cargo build` — compiles without errors
3. Launch TermSurf, open a split, type `web google.com`
4. Page loads (socket connection, hello, set_overlay all work)
5. URL bar updates as page navigates (url_changed forwarded via socket)
6. Loading indicator animates (loading_state forwarded)
7. Navigate via `:open https://github.com` (navigate works)
8. Open DevTools via `:devtools` (query_devtools, set_devtools_overlay work)
9. Dark mode via `:colorscheme dark` (set_color_scheme works)
10. Close pane — no crashes, no stale socket
11. Multiple splits — all work independently
12. `:tabs` — tab inventory works (query_tabs via socket)

**Pass criterion:** All TUI↔GUI communication works over Unix sockets +
protobuf. No regressions in any existing functionality.
