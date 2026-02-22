# Issue 618: URL sync

## Goal

When the browser navigates to a new page (link click, JavaScript redirect,
back/forward), update the URL bar in the `web` TUI to reflect the current URL.

## Background

The URL currently flows in one direction: the TUI sends the initial URL to the
GUI via `set_overlay`, the GUI forwards it to the Chromium server via
`create_tab`, and Chromium loads the page. After that initial load, the URL bar
is static — it always shows whatever the user typed on the command line.

The TUI already handles incoming `url_changed` messages (`tui/src/xpc.rs:190`,
`tui/src/main.rs:170`). The `UrlChanged` variant in `CompositorMessage` updates
the `url` variable and triggers a redraw. This was built during Issue 616 in
anticipation of this feature.

What's missing is the other end of the pipeline:

1. **Chromium server** — Does not notify the GUI when the URL changes after a
   navigation. The server needs to detect URL changes (via
   `DidNavigateMainFrame` or equivalent Content API callback) and send a message
   to the GUI.
2. **GUI (xpc.zig)** — Does not forward URL change messages from the Chromium
   server to the TUI. The GUI needs to receive the URL change from the server
   and relay it to the correct TUI pane.

### Current message flow

```
TUI → GUI → Chromium    (set_overlay with URL, create_tab with URL)
Chromium → GUI           (loading_state, iosurface frames)
GUI → TUI                (mode_changed, loading_state)
```

### Desired message flow

```
Chromium → GUI → TUI     (url_changed when navigation occurs)
```

## Architecture

Three changes are needed across three processes:

1. **Chromium server** (`chromium/src/`): Send a `url_changed` XPC message to
   the GUI whenever the main frame navigates to a new URL. The Content API
   provides `WebContentsObserver::DidFinishNavigation` for this.
2. **GUI** (`gui/src/apprt/xpc.zig`): Handle the incoming `url_changed` message
   from the Chromium server, look up which TUI pane owns the tab, and forward
   the message to that pane's XPC connection.
3. **TUI** (`tui/src/`): Already implemented — `UrlChanged` message handler
   updates the URL bar and redraws.
