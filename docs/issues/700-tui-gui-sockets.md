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

## Ideas for experiments

- **TUI socket client.** Create `tui/src/ipc.rs` with the same public API as
  `xpc.rs` but using Unix sockets + prost. Replace `block2` ObjC FFI with pure
  Rust. Switch `main.rs` from `mod xpc` to `mod ipc`.

- **GUI socket listener.** Add a Unix domain socket listener to `xpc.zig`.
  Accept TUI connections, read length-prefixed protobuf messages, dispatch to
  the existing handler functions. Send replies and events back over the socket.
  Add `web_fd` to `Pane` alongside `web_peer`.

- **Proto schema update.** Add `string pane_id = 2` to `ModeChanged` (needed for
  TUI→GUI direction). Regenerate GUI and TUI protobuf files.

- **End-to-end integration.** Both sides compiled and working together. Full
  verification of all 10 TUI→GUI and 8 GUI→TUI message types.

## Experiments

### Experiment 1: TUI socket client

Replace `tui/src/xpc.rs` (710 lines of unsafe XPC FFI) with `tui/src/ipc.rs`
(pure Rust sockets + prost). Same public API — `CompositorConnection` with
identical methods. Verify it compiles.

#### Changes

**1. Update `proto/termsurf.proto`**

Add `string pane_id = 2` to `ModeChanged`. The TUI→GUI direction needs `pane_id`
to identify which pane changed mode. The GUI→TUI direction ignores it (the TUI
only has one pane).

**2. Regenerate protobuf files**

Run `proto/generate.sh` to update `gui/src/protobuf/termsurf.pb-c.{c,h}`.

**3. Update `tui/Cargo.toml`**

Replace `block2` with `prost`. Add `prost-build` to build-dependencies.

```toml
[dependencies]
prost = "0.14"

[build-dependencies]
prost-build = "0.14"
```

**4. Create `tui/build.rs`**

Prost code generation — compiles `proto/termsurf.proto` at build time.

```rust
fn main() {
    prost_build::Config::new()
        .compile_protos(&["../proto/termsurf.proto"], &["../proto/"])
        .unwrap();
}
```

**5. Create `tui/src/ipc.rs`**

Same public types and methods as `xpc.rs`:

- `CompositorMessage` enum (4 variants: `ModeChanged`, `UrlChanged`,
  `LoadingState`, `TitleChanged`)
- `CompositorConnection` struct
- `connect(tx)` → builds socket path from `$TMPDIR`, connects via `UnixStream`,
  spawns reader thread
- 6 fire-and-forget methods: `send_set_overlay`, `send_set_devtools_overlay`,
  `send_navigate`, `send_set_color_scheme`, `send_open_split`,
  `send_mode_changed`
- 4 sync query methods: `send_hello`, `send_query_last`, `send_query_devtools`,
  `send_query_tabs`

Internals:

- Socket path: `$TMPDIR/termsurf/gui.sock`
- Wire format: 4-byte LE length prefix + serialized `TermSurfMessage`
- Reader thread: reads length-prefixed messages in a loop, routes reply messages
  to a `reply_tx` channel and event messages to the `LoopEvent` mpsc channel
- Sync queries: write request → block on `reply_rx` → return result
- `send_set_color_scheme`: converts scheme string (`"dark"`, `"light"`,
  `"system"`) to `dark` bool before sending
- `Drop`: closes the `UnixStream` (reader thread exits on EOF)

**6. Switch `main.rs` from `xpc` to `ipc`**

- `mod xpc` → `mod ipc`
- `xpc::CompositorMessage` → `ipc::CompositorMessage`
- `xpc::CompositorConnection` → `ipc::CompositorConnection`
- `LoopEvent::Xpc` → `LoopEvent::Ipc`

#### Verification

```bash
cd tui && cargo build
```

**Pass criterion:** Compiles with zero errors. No runtime test — the GUI socket
listener doesn't exist yet.

#### Result: PASS

`cargo build` compiles with zero errors and zero warnings. The TUI is now 100%
pure Rust — no `block2`, no `extern "C"`, no `CString`, no `unsafe`. The
`ipc.rs` module (265 lines) replaces `xpc.rs` (710 lines) with the same public
API.

The GUI also rebuilds cleanly with the regenerated protobuf files (adding
`pane_id` to `ModeChanged`).
