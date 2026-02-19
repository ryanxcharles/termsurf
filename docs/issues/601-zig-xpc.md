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

## Ideas for experiments

1. **Gateway connection and anonymous listener** — Ghost connects to the
   xpc-gateway, creates a listener, registers its endpoint. `web` connects and
   Ghost logs the connection. Proves XPC plumbing and `objc.Block` work from
   Zig.

2. **Parse `set_overlay` and `mode_changed`** — Ghost parses incoming XPC
   dictionaries from `web` and logs the values. Proves dictionary reading works.

3. **Send messages to `web`** — Ghost sends `mode_changed` and `url_changed`
   back to `web`. Proves bidirectional communication.
