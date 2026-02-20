# Issue 604: Two Panes

## Goal

Two terminal panes, each running `web`, both rendering live Chromium frames
simultaneously. Same profile, same server process, two independent browser tabs
streaming at 60fps to two different grid overlays.

## Background

Issue 603 proved the full Ghost pipeline works for a single pane: `web` sends
`set_overlay`, Ghost spawns a Chromium Profile Server, receives IOSurface Mach
ports at 60fps, and composites them as a Metal overlay at the correct grid
coordinates. Dynamic resize works. Clean exit kills the server.

But `xpc.zig` has a single `var pane: Pane = .{}`. A second `web` process
connecting to the same Ghost instance overwrites the first pane's state. Two
panes cannot coexist.

### What we have

**Ghost (from Issues 601–603):**

- XPC gateway connection, anonymous listener, endpoint registration
- `Pane` struct with per-pane mutex, web/server peer tracking
- Server lifecycle: spawn, `server_register` → `create_tab` → `display_surface`
- IOSurface → Metal texture pipeline (zero-copy)
- Dynamic resize via `sendResize`
- `setOverlay()` / `setOverlayIOSurface()` / `clearOverlay()` with `draw_mutex`
- Surface lookup by pane ID (`app.findSurfaceByPaneId`)

**Chromium Profile Server (from ts5 Issues 503–515):**

- Multi-tab support: one process hosts N WebContents, each with an independent
  `FrameSinkVideoCapturer` streaming at 60fps
- `CreateTab` adds a new Shell + VideoConsumer + per-tab XPC connection
- `CloseTab` fires when a tab's connection drops (server continues if tabs
  remain)
- `display_surface` messages carry `pane_id` — the server stamps each tab's
  frames with the pane UUID from `create_tab`
- `resize` routes by `pane_id` to the correct tab
- Server auto-exits when the last tab closes
- Profile lock on `--user-data-dir`: a second server with the same path will
  crash. Server reuse is a correctness requirement, not an optimization.

**`web` TUI:**

- Already sends `pane_id`, `url`, `profile` in `set_overlay`
- Each `web` instance gets its own `TERMSURF_PANE_ID` from the shell environment
- No changes needed to `web`

### What needs to change

**1. Multi-pane state in `xpc.zig`.**

Replace `var pane: Pane = .{}` with a data structure that maps pane UUIDs to
`Pane` structs. When `set_overlay` arrives with a new `pane_id`, create a new
`Pane`. When a web peer disconnects, clean up only that pane.

**2. Server reuse by profile.**

Currently Ghost spawns a new server for every `set_overlay` with a URL. With two
panes on the same profile, the second pane must reuse the first pane's server.
Need a profile → server mapping so that `handleSetOverlay` can detect an
existing server and send `create_tab` on its control connection instead of
spawning.

**3. `display_surface` routing.**

Currently `handleDisplaySurface` writes to `pane.overlay_surface` — a single
surface. With multiple panes, the handler must read `pane_id` from each
`display_surface` message and route the IOSurface to the correct pane's surface.

**4. Per-pane disconnect.**

Currently `handleDisconnect` kills the server and cleans up everything. With
multiple panes sharing a server, disconnecting one pane should only remove that
pane's tab. The server should be killed only when all panes for that profile
have disconnected (or the server auto-exits when its last tab closes).

**5. Per-pane resize.**

Currently `sendResize` sends on `pane.server_peer`. With multiple panes sharing
one server, resize messages go on the shared control connection but must include
`pane_id` so the server routes to the correct tab. This already works — the
`resize` message already includes `pane_id`.

### What should work without changes

- **xpc-gateway** — Stateless rendezvous. No pane or profile awareness.
- **Metal renderer** — Each surface independently receives IOSurface frames.
- **`web` TUI** — Already sends unique `pane_id` per instance.
- **Chromium server multi-tab** — Proven in Issue 503 Experiment 3, used in
  Issue 511.
- **Overlay shader** — Reads pixel dimensions per IOSurface.

## Experiment 1: Multi-pane XPC routing

### Goal

Two terminal panes, each running `web http://localhost:9407`, both rendering the
box demo simultaneously at 60fps. Same profile, one Chromium server process.

### Design

**Data structures** in `xpc.zig`:

Replace the single `var pane: Pane = .{}` with:

```zig
/// Active panes, keyed by pane UUID string.
var panes: std.StringHashMap(*Pane) = undefined;
var panes_mutex: std.Thread.Mutex = .{};

/// Active servers, keyed by profile name.
var servers: std.StringHashMap(*Server) = undefined;
```

Where `Server` holds the shared server state:

```zig
const Server = struct {
    process: std.process.Child,
    peer: xpc_object_t = null,
    profile: []const u8,
    pane_count: usize = 0,
};
```

And `Pane` changes to reference its server:

```zig
const Pane = struct {
    mutex: std.Thread.Mutex = .{},
    web_peer: xpc_object_t = null,
    overlay_surface: ?*CoreSurface = null,
    server: ?*Server = null,
    pane_id: [36]u8 = undefined,
    pending_url_buf: [2048]u8 = undefined,
    pending_url_len: usize = 0,
    pending_pixel_w: u64 = 0,
    pending_pixel_h: u64 = 0,
};
```

**`handleSetOverlay` flow:**

1. Extract `pane_id` from message
2. Lock `panes_mutex`, look up or create `Pane` for this pane ID
3. Lock `pane.mutex`, store overlay surface, URL, pixel dimensions
4. Look up `servers` by profile:
   - **No server:** spawn one, store URL as pending, increment `pane_count`
   - **Server exists, peer not yet connected:** store URL as pending (server is
     still starting up), increment `pane_count`
   - **Server exists, peer connected:** send `create_tab` immediately, increment
     `pane_count`
5. If server already running and dimensions changed: send `resize`

**`handleServerRegister` flow:**

1. Extract `profile` from message, look up `Server`
2. Store the peer connection on the `Server`
3. Iterate all panes whose server matches and have pending URLs → send
   `create_tab` for each

**`handleDisplaySurface` flow:**

1. Extract `pane_id` from message
2. Look up `Pane` by pane ID
3. Import IOSurface, route to that pane's surface

**Disconnect flow:**

When a web peer disconnects:

1. Find which pane this peer belongs to (by connection identity)
2. Send `close_tab` or let the server detect the tab connection drop
3. Decrement server's `pane_count`
4. If `pane_count == 0`: kill server, remove from `servers`
5. Clean up pane, remove from `panes`

When a server peer disconnects (server crashed or exited):

1. Find which server this peer belongs to
2. Clean up all panes that reference this server
3. Remove server from `servers`

**Peer identification:**

Currently `handleDisconnect` doesn't know which pane disconnected — it just
cleans up the single pane. With multiple panes, the disconnect handler must
identify the disconnecting peer. XPC provides
`xpc_dictionary_get_remote_connection` on messages, but disconnect events are
errors, not dictionaries.

Option: store the connection pointer (address) as a key in a reverse lookup map
(`connection_ptr → pane_id`). When a peer connects (first message), register the
mapping. On disconnect, look up by connection pointer.

### Verification

```bash
cd box-demo && bun run server.ts &
cd ghost && zig build
open ghost/zig-out/Ghostty.app --stderr ~/dev/termsurf/logs/ghost.log
# In pane 1:
cargo run -p web -- http://localhost:9407
# Split pane (Cmd+D or equivalent), in pane 2:
cargo run -p web -- http://localhost:9407
```

Pass criteria:

- Both panes render the box demo simultaneously at 60fps
- Only one `chromium_profile_server` process running (same profile)
- Closing one pane doesn't affect the other
- Closing the last pane kills the server
- Resize works independently per pane
