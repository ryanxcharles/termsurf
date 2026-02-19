# Issue 601: XPC in Ghost

## Goal

Add XPC support to Ghost so it can receive messages from `web`. No Chromium, no
IOSurface — just the XPC connection lifecycle and message handling, all in Zig.

## Background

In ts5, all XPC communication lives in Swift (CompositorXPC.swift, ~500 lines).
Ghost moves this to Zig. This issue proves the integration works by handling the
`web` TUI's messages — the simplest half of the XPC protocol.

### The `web` protocol

The `web` TUI connects to Ghost through the xpc-gateway:

```
web                          xpc-gateway              Ghost
 │                               │                      │
 │  connect to Mach service      │                      │
 │──────────────────────────────▶│                      │
 │                               │                      │
 │                               │◀─── register_app ────│
 │                               │     (endpoint)       │
 │                               │                      │
 │  { action: "connect" }        │                      │
 │──────────────────────────────▶│                      │
 │                               │                      │
 │  reply: { endpoint }          │                      │
 │◀──────────────────────────────│                      │
 │                               │                      │
 │  connect to endpoint ─────────────────────────────▶  │
 │                                                      │
 │  { action: "set_overlay",                            │
 │    pane_id, col, row,                                │
 │    width, height, url,                               │
 │    profile, browsing }                               │
 │─────────────────────────────────────────────────────▶│
 │                                                      │
 │  { action: "mode_changed",                           │
 │    pane_id, browsing }                               │
 │─────────────────────────────────────────────────────▶│
 │                                                      │
 │  disconnect                                          │
 │─────────────────────────────────────────────────────▶│
```

Messages from `web`:

| Action         | Fields                                                   |
| -------------- | -------------------------------------------------------- |
| `set_overlay`  | pane_id, col, row, width, height, url, profile, browsing |
| `mode_changed` | pane_id, browsing                                        |

Messages from Ghost to `web`:

| Action         | Fields   |
| -------------- | -------- |
| `mode_changed` | browsing |
| `url_changed`  | url      |

### Blocks in Zig

XPC uses C blocks for event handlers. Ghostty's `zig-objc` dependency
(`objc.Block`) already handles this — it constructs the block struct (isa,
flags, invoke, descriptor, captures) in pure Zig. The Metal renderer uses it for
command buffer completion and IOSurface layer callbacks. No C shim needed.

### Where the code lives

In ts5, CompositorXPC.swift is a standalone class instantiated by AppDelegate.
In Ghost, the XPC logic belongs in Zig's core — likely as a new module alongside
the existing Surface, renderer, and apprt code. The exact file structure will be
determined in Experiment 1.

## Experiments

### Experiment 1: Gateway connection and anonymous listener

#### Goal

Ghost connects to the xpc-gateway on startup, creates an anonymous XPC listener,
and registers its endpoint. The `web` TUI can connect and Ghost logs the
connection. No message parsing yet — just proving the XPC plumbing works from
Zig.

#### Changes

Add a new Zig source file (e.g., `ghost/src/apprt/xpc.zig` or similar) that:

1. Imports `<xpc/xpc.h>` via `@cImport`.
2. Creates a named XPC connection to `com.termsurf.xpc-gateway`.
3. Sets an event handler using `objc.Block`.
4. Resumes the connection.
5. Creates an anonymous XPC listener (`xpc_connection_create`).
6. Sends `{ action: "register_app", endpoint: <listener endpoint> }` to the
   gateway.
7. Accepts incoming peer connections on the listener.
8. Logs when a peer connects and disconnects.

Wire this into Ghost's startup path (called from the embedded apprt or
AppDelegate — whichever is simpler for a first pass).

#### Verification

```bash
cd ghost && zig build
open ghost/zig-out/Ghostty.app --stderr ~/dev/termsurf/logs/ghost.log

# In another terminal:
export TERMSURF_PANE_ID=$(uuidgen)
cargo run -p web -- https://example.com
```

Pass: Ghost logs show "Peer connected" when `web` starts, "Peer disconnected"
when `web` exits. The `web` TUI renders normally (it doesn't care who handles
its messages). No crashes.

### Experiment 2: Parse `set_overlay` and `mode_changed`

#### Goal

Ghost parses the `set_overlay` and `mode_changed` messages from `web` and logs
the values. This proves XPC dictionary parsing works from Zig.

#### Prerequisites

Experiment 1 must pass.

#### Changes

In the peer event handler, parse incoming dictionaries:

1. Read `action` string from the XPC dictionary.
2. For `set_overlay`: extract pane_id, col, row, width, height, url, profile,
   browsing. Log all values.
3. For `mode_changed`: extract pane_id, browsing. Log the values.
4. On peer disconnect: log which pane_id disconnected.

#### Verification

```bash
cd ghost && zig build
open ghost/zig-out/Ghostty.app --stderr ~/dev/termsurf/logs/ghost.log

# In a Ghost terminal pane:
cargo run -p web -- https://example.com
```

Pass: Ghost logs show the pane ID, grid coordinates (col, row, width, height),
URL, profile, and browsing state received from `web`. Pressing Esc/Enter in
`web` generates `mode_changed` messages that Ghost logs correctly.

### Experiment 3: Send messages to `web`

#### Goal

Ghost sends `mode_changed` and `url_changed` messages back to `web`. This proves
bidirectional XPC communication works.

#### Prerequisites

Experiment 2 must pass.

#### Changes

1. Store the peer connection per pane_id (so Ghost can send messages to a
   specific `web` instance).
2. When `web` sends `mode_changed` with `browsing: true`, Ghost echoes back
   `{ action: "mode_changed", browsing: true }` to confirm.
3. Add a test: Ghost sends `{ action: "url_changed", url: "https://test.com" }`
   after receiving `set_overlay`.

#### Verification

```bash
cd ghost && zig build
open ghost/zig-out/Ghostty.app --stderr ~/dev/termsurf/logs/ghost.log

# In a Ghost terminal pane:
cargo run -p web -- https://example.com
```

Pass: the `web` TUI receives the `mode_changed` echo and updates its mode
indicator. The `url_changed` message updates the URL bar in `web`. Full
bidirectional communication works.
