# Issue 726: Wezboard overlay lifecycle and remaining protocol

## Goal

Make the browser overlay respond correctly to window and pane changes (resize,
splits, tab switching), then handle all remaining TermSurf protocol messages to
complete the Wezboard PoC.

## Background

Issue 725 solved overlay rendering: browser content is visible in the Wezboard
terminal window with correct size and position. But the overlay is static — it
doesn't respond to window resizes, split pane changes, or tab switches. And
Wezboard only handles 11 of 30 TermSurf protocol messages.

### Overlay lifecycle issues

Four overlay behaviors need to work:

1. **Window resize** — When the window resizes, the webview should resize with
   it. The metrics bridge (`metrics.rs`) updates on resize, but `conn.rs`
   doesn't re-read the metrics or call `update_ca_layer_frame()`. There's no
   notification path from TermWindow to the connection code.
2. **Split pane resize** — When opening or closing a split pane, the terminal
   pane shrinks or grows. The webview should resize to match the new pane
   dimensions.
3. **Tab switch away** — When opening a new tab or switching to a tab without a
   webview, the overlay should hide.
4. **Tab switch back** — When navigating back to a tab with an active webview,
   the overlay should reappear.

### Remaining protocol messages

Wezboard currently handles 11 of 30 TermSurf protocol messages. The remaining 19
fall into four categories:

**Input forwarding (4 messages):**

- `KeyEvent` — Keyboard input to browser
- `MouseEvent` — Mouse clicks to browser
- `MouseMove` — Mouse movement to browser
- `ScrollEvent` — Scroll wheel to browser

Without input forwarding, the browser overlay is view-only. This is the most
important missing piece after overlay lifecycle.

**Tab queries (6 messages):**

- `QueryLastRequest` / `QueryLastReply` — Get last active tab for session
  restore
- `QueryDevtoolsRequest` / `QueryDevtoolsReply` — Get DevTools tab
- `QueryTabsRequest` / `QueryTabsReply` — Get all tabs for a profile

**DevTools (2 messages):**

- `CreateDevtoolsTab` — Create DevTools tab
- `SetDevtoolsOverlay` — Create/resize DevTools overlay

**Other (3 messages):**

- `FocusChanged` — Tab focus state
- `CursorChanged` — Browser cursor type updates
- `OpenSplit` — Open split pane

**Already handled (11 messages):**

- `HelloRequest` / `HelloReply` — Handshake
- `ServerRegister` — Chromium process registration
- `SetOverlay` — Create/resize browser overlay
- `CreateTab` (sent, not received) — Create tab in browser
- `TabReady` — Tab initialized
- `CaContext` — CALayerHost context for compositing
- `Navigate` — URL navigation forwarding
- `UrlChanged` / `LoadingState` / `TitleChanged` — State forwarding to TUI
- `SetColorScheme` — Dark/light mode
- `ModeChanged` — Browse/edit mode toggle

### Priority order

1. Overlay lifecycle (this issue's primary focus)
2. Input forwarding (makes the browser usable)
3. Tab queries (session restore, DevTools discovery)
4. Auxiliary features (focus, cursor, DevTools, splits)

## Proposed solutions

### Overlay lifecycle

For **resize**, TermWindow could send a notification through the TermSurf shared
state or a channel whenever dimensions change. Alternatively, `conn.rs` could
poll the metrics atomics periodically — but that's wasteful. A better approach:
when `SetOverlay` arrives with updated dimensions, re-read metrics and update
the CALayer frame.

For **tab switching**, the overlay NSView or its sublayers need to be
shown/hidden based on which tab is active. The mux (WezTerm's tab/pane manager)
knows which pane is focused. When the focused pane changes, the board needs to
hide overlays for inactive panes and show overlays for the active pane.

### Input forwarding

The TUI already captures keyboard and mouse events and sends them as protobuf
messages. The board needs to receive these messages and forward them to the
correct Chromium process based on pane-to-tab mapping.
