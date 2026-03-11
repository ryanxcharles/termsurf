# Issue 741: Split protocol into three channels

## Goal

Replace the single `termsurf.proto` with three independent protocols — TUI↔GUI,
GUI↔Browser, and TUI↔Browser — and let the TUI talk directly to the browser
engine over its own socket, eliminating all message proxying through the GUI.

## Background

The current protocol is a single `TermSurfMessage` oneof with 30 message types.
Five of these are proxied through the GUI:

- **UrlChanged, LoadingState, TitleChanged** — Browser sends to GUI, GUI
  forwards verbatim to TUI. The GUI does nothing with the data.
- **Navigate** — TUI sends with `pane_id`, GUI swaps it for `tab_id` and
  forwards to browser. Pure ID translation.
- **SetColorScheme** — Same as Navigate, except the GUI also stores `pane.dark`
  (only used to populate `CreateTab.dark` for new tabs).

These dual-use messages have overloaded fields (`tab_id` for one direction,
`pane_id` for the other), which is a design smell. Worse, the proxy pattern
scales badly — every future browser feature (JS dialogs, downloads, file
uploads, auth challenges, permissions, find-in-page, console capture) would need
forwarding code in both Ghostboard (Zig) and Wezboard (Rust). That's two
implementations of the same do-nothing relay, per message, forever.

### The hub-and-spoke assumption

The current architecture routes all communication through the GUI:

```
TUI ──socket──> GUI ──socket──> Browser
```

This was inherited from the XPC era (ts5), where the GUI was necessarily the
hub. With Unix sockets there is no such constraint. The TUI can connect directly
to the browser:

```
TUI ──socket──> GUI        (overlay geometry, mode changes, queries)
TUI ──socket──> Browser    (navigation, page state, content features)
GUI ──socket──> Browser    (input, compositing, tab lifecycle, focus)
```

### What changes

The GUI remains responsible for:

- **Process lifecycle** — Launching and killing browser engine processes.
- **Overlay rendering** — CALayerHost setup, pixel coordinates, resize.
- **Input forwarding** — Keyboard/mouse events come from the GUI's window.
- **Tab lifecycle** — CreateTab, CloseTab, Resize (the GUI knows pixel
  dimensions from overlay geometry).
- **Focus and cursor** — FocusChanged, CursorChanged (the GUI owns window
  focus).
- **Configuration queries** — Hello, QueryLast, QueryDevtools (the GUI knows
  what browsers and profiles exist).

The TUI takes over:

- **Navigation** — Navigate (the TUI already knows the URL, now sends it
  directly to the browser with `tab_id`).
- **Page state** — UrlChanged, LoadingState, TitleChanged (the browser sends
  directly to the TUI).
- **Color scheme** — SetColorScheme (the TUI sends directly to the browser).
- **All future content features** — JS dialogs, downloads, file uploads, auth,
  permissions, find-in-page, console capture. These are all TUI↔Browser
  conversations that the GUI has no business intermediating.

### Connection handoff

The key mechanism is the GUI telling the TUI how to connect to the browser:

1. TUI sends `SetOverlay` to GUI (as today).
2. GUI launches Roamium (if needed) and sends `CreateTab` to the browser.
3. Browser responds with `TabReady { pane_id, tab_id }` to the GUI.
4. GUI sends a new `BrowserReady { tab_id, browser_socket }` message to the TUI.
5. TUI connects directly to Roamium's socket using the provided path.
6. TUI registers itself with the browser via a new `TuiRegister { tab_id }`
   message so the browser knows which connection owns which tab.
7. All content-level messages now flow directly: TUI↔Browser.

The browser needs to accept multiple connections — one from the GUI (for input,
compositing, lifecycle) and one or more from TUIs (for content). Today Roamium
has a single connection to the GUI. It would need to listen on its own socket
(or accept multiple connections on the GUI's socket — but a dedicated browser
socket is cleaner).

### Roamium socket model

Today Roamium connects to the GUI's socket as a client (`--ipc-socket={path}`).
For TUI↔Browser direct communication, Roamium needs its own listening socket so
TUIs can connect to it:

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

### QueryTabs

`QueryTabsRequest/Reply` currently flows TUI→GUI→Browser→GUI→TUI. The GUI asks
the browser for tab counts, merges in its own pane count, and replies to the
TUI. With the split:

- The TUI can query the browser directly for tab info (TUI↔Browser).
- The TUI can query the GUI for pane info (TUI↔GUI).
- The TUI assembles the combined view itself.

Or `QueryTabs` could stay on TUI↔GUI for now and be refactored later. Either
way, it's no longer a three-hop relay.

### Proto file structure

Three proto files, three wrapper messages, zero overlap:

**`proto/termsurf_tui_gui.proto`** — TUI↔GUI channel

```
SetOverlay, SetDevtoolsOverlay, OpenSplit     (TUI → GUI)
ModeChanged                                    (GUI → TUI)
BrowserReady                                   (GUI → TUI) — NEW
HelloRequest/Reply                             (TUI ↔ GUI)
QueryLastRequest/Reply                         (TUI ↔ GUI)
QueryDevtoolsRequest/Reply                     (TUI ↔ GUI)
Shutdown                                       (GUI → Browser, stays)
```

**`proto/termsurf_gui_browser.proto`** — GUI↔Browser channel

```
CreateTab, CreateDevtoolsTab, CloseTab, Resize (GUI → Browser)
MouseEvent, MouseMove, ScrollEvent, KeyEvent   (GUI → Browser)
FocusChanged                                   (GUI → Browser)
ServerRegister                                 (Browser → GUI)
TabReady                                       (Browser → GUI)
CaContext                                      (Browser → GUI)
CursorChanged                                  (Browser → GUI)
```

**`proto/termsurf_tui_browser.proto`** — TUI↔Browser channel

```
TuiRegister                                    (TUI → Browser) — NEW
Navigate                                       (TUI → Browser)
SetColorScheme                                 (TUI → Browser)
UrlChanged                                     (Browser → TUI)
LoadingState                                   (Browser → TUI)
TitleChanged                                   (Browser → TUI)
QueryTabsRequest/Reply                         (TUI ↔ Browser)
```

Navigate and SetColorScheme lose their dual-use fields — no more `pane_id` in
Navigate, no more `tab_id`-or-`pane_id` ambiguity. Each message has exactly the
fields it needs for its channel.

### What the GUI loses

The GUI no longer sees UrlChanged, TitleChanged, LoadingState, Navigate, or
SetColorScheme. Examining each:

- **UrlChanged, TitleChanged, LoadingState** — The GUI never used these. Pure
  relay today.
- **Navigate** — The GUI never used the URL. Pure relay with ID swap.
- **SetColorScheme** — The GUI stored `pane.dark` to pass to `CreateTab`. Fix:
  the TUI already sends `dark` information — either include it in `SetOverlay`,
  or have the TUI send `SetColorScheme` to the browser after connecting. The GUI
  doesn't need to track dark mode state.

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
