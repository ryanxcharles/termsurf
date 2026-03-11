# Issue 741: Split protocol into two channels

## Goal

Replace the single `termsurf.proto` with two protocols — a GUI protocol
(TUI↔GUI) and a browser protocol (GUI+TUI↔Browser) — and let the TUI talk
directly to the browser engine over its own socket, eliminating all message
proxying through the GUI.

## Background

### Process responsibilities

Each process should own one concern:

- **TUI** — Owns user intent. Browser chrome: URL bar, navigation, modes,
  commands. Talks to the GUI about layout (overlays, splits, mode changes).
  Talks to the browser about content (navigation, page state, dialogs,
  downloads). Direct browser client.
- **GUI** — Owns the window. Terminal rendering, pane layout, overlay
  compositing, input capture, process lifecycle. Tells the browser "create a tab
  at these dimensions" and "here are mouse/key events." Tells the TUI "browser
  is ready, here's how to connect." Does NOT relay content messages. Does NOT
  track URLs, titles, loading state, or color schemes.
- **Browser** — Owns web content. Renders pages, manages tabs, reports state.
  Accepts connections from anyone — GUI for input/compositing, TUI for content.
  Doesn't care who's asking, just handles messages.

Today the GUI violates this separation. It proxies five message types between
TUI and browser:

- **UrlChanged, LoadingState, TitleChanged** — Browser sends to GUI, GUI
  forwards verbatim to TUI. The GUI does nothing with the data.
- **Navigate** — TUI sends with `pane_id`, GUI swaps it for `tab_id` and
  forwards to browser. Pure ID translation.
- **SetColorScheme** — Same as Navigate, except the GUI also stores `pane.dark`
  (only used to populate `CreateTab.dark` for new tabs).

These dual-use messages have overloaded fields (`tab_id` for one direction,
`pane_id` for the other). The proxy pattern scales badly — every future browser
feature (JS dialogs, downloads, file uploads, auth challenges, permissions,
find-in-page, console capture) would need forwarding code in both Ghostboard
(Zig) and Wezboard (Rust). That's two implementations of the same do-nothing
relay, per message, forever.

### Current architecture

All communication flows through the GUI as a hub:

```
TUI ──socket──> GUI ──socket──> Browser
```

This was inherited from the XPC era (ts5), where the GUI was necessarily the
hub. With Unix sockets there is no such constraint.

### Target architecture

The TUI connects directly to the browser for content-level communication:

```
TUI ──socket──> GUI        (overlay geometry, mode changes, queries)
TUI ──socket──> Browser    (navigation, page state, content features)
GUI ──socket──> Browser    (input, compositing, tab lifecycle, focus)
```

The GUI stops being a router. It becomes purely a rendering/input layer. The TUI
becomes the browser's direct client. The browser becomes a multi-client server.

### Connection handoff

The GUI still manages process lifecycle. The TUI discovers the browser through
the GUI:

1. TUI sends `SetOverlay` to GUI (as today).
2. GUI launches Roamium (if needed) and sends `CreateTab` to the browser.
3. Browser responds with `TabReady { pane_id, tab_id }` to the GUI.
4. GUI sends `BrowserReady { tab_id, browser_socket }` to the TUI.
5. TUI connects directly to Roamium's listening socket.
6. All content-level messages now flow directly: TUI↔Browser.

### Roamium socket model

Today Roamium connects to the GUI's socket as a client (`--ipc-socket={path}`).
It needs its own listening socket so TUIs can connect to it:

1. GUI spawns Roamium with `--ipc-socket={gui_socket}` (as today) plus a new
   `--listen-socket={browser_socket}` argument.
2. Roamium connects to the GUI socket (for input/compositing/lifecycle) and
   listens on its own socket (for TUI content connections).
3. GUI sends the browser socket path to the TUI in `BrowserReady`.
4. TUI connects to the browser socket directly.

The browser socket path follows the existing convention:
`$TMPDIR/termsurf/termsurf-roamium-{pid}.sock`.

### ID model

With a direct connection, the TUI learns `tab_id` from `BrowserReady` and uses
it in all messages to the browser. No more `pane_id` in browser messages, no
more `tab_id` in TUI↔GUI messages. Each protocol uses its own natural
identifier:

- **TUI↔GUI:** `pane_id` (string, assigned by TUI)
- **GUI↔Browser:** `tab_id` (int64, assigned by Chromium)
- **TUI↔Browser:** `tab_id` (int64, learned from `BrowserReady`)

### Two protocols, not three

The browser doesn't need separate protocols for GUI and TUI connections — a
CreateTab is a CreateTab regardless of who sends it. The browser receives
protobuf messages and acts on them; it doesn't restrict which client can send
which message.

The browser doesn't need to know what kind of client is connected. Every
connection gets every event — the browser broadcasts all outbound messages
(TabReady, CaContext, UrlChanged, etc.) to all connections. Each client ignores
what it doesn't care about. The GUI ignores UrlChanged. The TUI ignores
CursorChanged. No registration, no connection types, no routing logic.

`ServerRegister` stays as the only identification message — it tells the browser
which profile this process serves, not what kind of client is connecting.

This future-proofs the protocol. If the GUI ever needs UrlChanged (e.g., for a
window title), it already receives it. If the TUI ever needs to send Resize
directly, it just sends it.

**`proto/termsurf_gui.proto`** — TUI↔GUI channel

```
SetOverlay, SetDevtoolsOverlay, OpenSplit     (TUI → GUI)
ModeChanged                                    (GUI → TUI)
BrowserReady                                   (GUI → TUI) — NEW
HelloRequest/Reply                             (TUI ↔ GUI)
QueryLastRequest/Reply                         (TUI ↔ GUI)
QueryDevtoolsRequest/Reply                     (TUI ↔ GUI)
```

**`proto/termsurf_browser.proto`** — Browser channel (GUI and TUI both connect)

```
ServerRegister                                 (Client → Browser)
CreateTab, CreateDevtoolsTab, CloseTab, Resize (Client → Browser)
MouseEvent, MouseMove, ScrollEvent, KeyEvent   (Client → Browser)
FocusChanged                                   (Client → Browser)
Navigate                                       (Client → Browser)
SetColorScheme                                 (Client → Browser)
QueryTabsRequest                               (Client → Browser)
TabReady                                       (Browser → all clients)
CaContext                                      (Browser → all clients)
CursorChanged                                  (Browser → all clients)
UrlChanged                                     (Browser → all clients)
LoadingState                                   (Browser → all clients)
TitleChanged                                   (Browser → all clients)
QueryTabsReply                                 (Browser → all clients)
Shutdown                                       (Client → Browser)
```

Navigate and SetColorScheme lose their dual-use fields — no more `pane_id` in
Navigate, no more `tab_id`-or-`pane_id` ambiguity. Each message has exactly the
fields it needs for its channel.

### Process management

The GUI remains the process manager:

- **Launching:** GUI spawns Roamium with both `--ipc-socket` (GUI connection)
  and `--listen-socket` (TUI connection). Same as today plus one argument.
- **Shutdown:** GUI sends `Shutdown` message to browser (Issue 732/733). No
  change.
- **Crash detection:** GUI monitors child processes. If Roamium dies, GUI
  notifies all TUIs that had tabs on that browser (new message or error on
  existing queries).
- **Reuse:** GUI tracks which profile/browser combinations already have a
  running Roamium. When a TUI requests a new overlay on an existing profile, the
  GUI sends `CreateTab` to the existing Roamium and returns the same browser
  socket path to the TUI.

The TUI does NOT launch or kill browser processes. It asks the GUI (via
`SetOverlay`), gets back a `BrowserReady` with the socket path and tab_id, and
connects directly.

### Why direct sockets, not a proxy envelope

An alternative approach would keep the hub-and-spoke topology and add a generic
proxy envelope (`ProxyToBrowser { pane_id, bytes }` /
`ProxyToTui { tab_id, bytes }`). The GUI would relay opaque bytes between TUI
and browser, replacing per-message forwarding with a single generic function.
This is less upfront work — no new sockets, no multi-connection handling.

However, the proxy envelope is a detour, not a stepping stone. The work does not
carry over to direct sockets:

- The generic relay code in both GUIs would be written and then deleted.
- The `tab_to_pane` / `pane_to_tab` ID mapping would be maintained and then
  deleted.
- The TUI would wrap messages in envelopes and then stop wrapping them.
- The browser would receive unwrapped messages from the GUI and then switch to
  receiving them from a TUI connection.

The direct socket approach has three concrete pieces of work:

1. **Roamium listener** (~50 lines of Rust) — Add `--listen-socket=`, accept
   connections, broadcast events to all. Same pattern as the existing
   `ipc::connect` but in reverse.
2. **GUI sends `BrowserReady` to TUI** — One new message sent after `TabReady`
   arrives. A few lines in each GUI.
3. **TUI opens a second connection** — Connect to browser socket, spawn a second
   reader thread. The event loop already multiplexes GUI events via `mpsc` — the
   browser reader thread sends to the same channel.

After that, forwarding code is deleted from both GUIs — a net reduction in
complexity. No intermediate state, no throwaway work.

### Staged implementation

Prove the architecture on one GUI first, then port. Wezboard is the right
starting point — it's Rust (like the TUI and Roamium), under active development,
and easier to iterate on. Ghostboard (Zig) gets ported after the design is
proven.

The four experiments, each independently testable:

1. **Roamium listener** — Add `--listen-socket=`, accept connections, broadcast
   events to all. Shared across both GUIs — the browser doesn't care which GUI
   launched it. The GUI still works as before. Nothing is removed yet. Verify: a
   test client can connect and receive events.
2. **Wezboard + TUI direct connection** — Wezboard sends `BrowserReady` after
   `TabReady`. TUI connects to browser socket. Content messages flow directly.
   Wezboard forwarding still exists but is now unused for these messages.
   Verify: navigation works end-to-end over the direct socket with Wezboard.
3. **Remove Wezboard forwarding** — Delete proxy code from Wezboard, remove ID
   maps, split proto files. Verify: everything still works with Wezboard, GUI
   code is smaller.
4. **Port to Ghostboard** — Implement `BrowserReady` in Ghostboard, remove its
   forwarding code. The proto files and Roamium are already done. Verify:
   everything works with Ghostboard.

## Experiments

### Experiment 1: Roamium listener

#### Description

Add a listening socket to Roamium so clients can connect directly. Today Roamium
has a single outbound connection to the GUI. This experiment adds an inbound
listener that accepts any number of connections and broadcasts all outbound
events (TabReady, CaContext, UrlChanged, LoadingState, TitleChanged,
CursorChanged, QueryTabsReply) to every connected client. Inbound messages from
any connection are dispatched the same way. The GUI connection and all existing
behavior remain unchanged.

#### Changes

**`roamium/src/main.rs`** — Parse `--listen-socket=`

Add a new `OnceLock<String>` for the listen socket path. Parse
`--listen-socket=` from argv alongside the existing `--ipc-socket=`. In
`on_initialized`, after connecting to the GUI, call `ipc::listen()` to start the
listener.

**`roamium/src/ipc.rs`** — Add listener and broadcast

Current state: `ipc.rs` has a single global `WRITER` (the GUI connection) and
`send()` always writes to it. Change this to broadcast to all connections.

Rename `WRITER` to `WRITERS: Mutex<Vec<UnixStream>>`. The GUI connection is the
first entry. Each accepted listener connection adds another entry.

- `connect()` — Same as today, but pushes to `WRITERS` instead of setting a
  single `WRITER`.
- `listen(path: &str)` — Bind a `UnixListener`, spawn an accept thread. Each
  accepted connection clones the write half into `WRITERS` and spawns a reader
  thread (same `reader_loop` pattern as the GUI connection).
- `send()` — Iterate `WRITERS`, write the message to each. Remove any connection
  that errors (client disconnected). This replaces the single-writer send.

The accept thread and reader threads post messages to the UI thread via
`ts_post_task`, same as the GUI reader thread. The reader thread for a listener
connection does NOT quit the process on EOF — only the GUI connection EOF
triggers shutdown (a disconnected client is not fatal).

**`roamium/src/dispatch.rs`** — No changes

All callbacks already call `ipc::send()`. Since `send()` now broadcasts, every
connected client receives every event. Inbound message dispatch is unchanged —
`handle_message()` doesn't care which connection the message came from.

#### Verification

1. Build Roamium: `./scripts/build.sh roamium`
2. The existing system works unchanged — launch Wezboard, browse a page, verify
   UrlChanged/LoadingState/TitleChanged still arrive at the TUI (via GUI proxy).
3. Write a test script (`scripts/test-browser-socket.sh`) that: a. Finds the
   Roamium listen socket in `$TMPDIR/termsurf/`. b. Connects to it with `socat`
   or a small Rust program. c. Navigates to a page in the browser. d. Observes
   that TabReady, CaContext, UrlChanged, LoadingState, TitleChanged, and
   CursorChanged all arrive on the direct connection.
4. Verify Roamium logs show the connection accepted.

**Result:** Fail

The code changes are correct — Roamium accepts `--listen-socket=`, binds a
listener, accepts connections, and broadcasts to all writers. However, the
experiment cannot be verified because Wezboard (the GUI) does not pass
`--listen-socket=` when spawning Roamium. Without that argument, Roamium never
starts its listener, so there is no socket to connect to. Verification requires
modifying Wezboard's spawn command to pass the flag, which is a GUI change —
outside the scope of this experiment.

#### Conclusion

The Roamium-side implementation is complete but untestable in isolation. The
experiment's scope was too narrow: it assumed the listener could be verified
without any GUI changes, but Roamium is always launched by the GUI and the GUI
controls its arguments. The `--listen-socket=` flag must be passed by Wezboard
for the listener to activate. Fold this into Experiment 2, which modifies
Wezboard anyway.
