# 330: Multi-Webview Connection Bug

Closing one webview causes the profile server to exit, making other webviews in
the same profile inactive.

## Status

**Open.** Discovered during issue 329 testing.

## Problem

When two webviews are open in the same profile and one is closed, the profile
server exits even though the other webview should still be active. This causes
the remaining webview to become inactive (no longer receives frames).

**Steps to reproduce:**

1. Open a terminal
2. Run `web google.com` in pane 0
3. Split pane (Ctrl+Shift+E or similar)
4. Run `web google.com` in pane 1
5. Close pane 1's webview (Ctrl+C twice)
6. Observe: pane 0's webview becomes inactive

**Expected behavior:** Closing one webview should not affect other webviews in
the same profile. The profile server should remain running as long as at least
one webview is active.

## Background

The profile server tracks GUI connections with a counter:

```rust
static GUI_CONNECTION_COUNT: AtomicUsize = AtomicUsize::new(0);
```

When a connection is established, the count increments. When a connection closes
(error received), the count decrements. When the count reaches 0, the profile
server exits gracefully.

The launcher has profile reuse logic — when a second `web` command uses the same
profile as an existing profile server, the launcher forwards a `create_browser`
command to the existing server instead of spawning a new one.

## Observed Behavior

**Profile server logs:**

```
Profile: GUI connection established (total: 1)
Profile: GUI connection established (total: 2)
Profile: GUI disconnected (remaining: 1)
Profile: GUI disconnected (remaining: 0)
Profile: No more GUI connections, exiting gracefully
```

Both connections disconnect when only one webview is closed.

**GUI logs:**

```
13:23:15.279  [XPC-CONN] Stored connection for pane 0: 0x865940030
13:23:21.690  [XPC-CONN] Stored connection for pane 1: 0x8670b2810
13:23:27.279  [Webview] Ctrl+C in Control mode → Exit browser
13:23:27.279  [XPC] Removed connection for pane 1
13:23:27.279  [XPC] Removed invalidate callback for pane 1
13:23:27.279  ERROR [XPC Manager] Connection error: XPC connection invalid
13:23:27.279  [Webview] Closed webview for pane 1
```

The GUI correctly tracks separate connections for each pane, but an "XPC
connection invalid" error appears immediately after removing pane 1's
connection.

## Analysis

The GUI side properly maintains separate connections per pane:

- `peer_connections: HashMap<PaneId, Arc<XpcConnection>>`
- Each webview has its own connection stored by pane ID
- Removing one pane's connection should only affect that connection

The profile server also creates separate connections per browser:

- Each `create_browser_on_ui_thread` call creates a new `XpcConnection`
- Each connection has its own event handler that decrements the count on error

However, when one connection is dropped on the GUI side, both connections on the
profile server side appear to receive disconnect errors.

### Possible Causes

1. **XPC connection sharing** — The XPC library might share some state between
   connections created from endpoints on the same anonymous listener.

2. **Listener cleanup** — The GUI stores listeners in a `Vec<XpcListener>` but
   never removes them. When one webview closes, its listener might still be
   active, causing issues.

3. **macOS XPC behavior** — Closing one connection might invalidate the
   underlying Mach port in a way that affects other connections from the same
   process.

4. **Profile server browser cleanup** — When one browser's GUI connection
   closes, CEF or the browser cleanup code might be affecting the whole process.

## Files Involved

| File                                            | Role                               |
| ----------------------------------------------- | ---------------------------------- |
| `ts3/wezterm-gui/src/termwindow/webview_xpc.rs` | GUI-side XPC manager               |
| `ts3/wezterm-gui/src/termwindow/keyevent.rs`    | `close_webview_for_pane()`         |
| `ts3/termsurf-profile/src/main.rs`              | Profile server connection handling |
| `ts3/termsurf-xpc/src/lib.rs`                   | XPC connection wrapper             |

## Proposed Investigation

### Experiment 1: Add Connection Identifiers

Add unique identifiers to each connection on both GUI and profile server sides
to track which specific connection is receiving errors.

**Profile server:**

```rust
// In create_browser_on_ui_thread, assign a unique ID to each browser
static BROWSER_ID: AtomicU64 = AtomicU64::new(0);
let browser_id = BROWSER_ID.fetch_add(1, Ordering::Relaxed);
println!("[CONN-{}] GUI connection established", browser_id);

// In error handler
println!("[CONN-{}] GUI disconnected", browser_id);
```

**GUI:**

```rust
// In remove_connection, log which connection is being dropped
println!("[XPC] Dropping connection for pane {}: {:p}", pane_id, Arc::as_ptr(conn));
```

### Experiment 2: Delay Connection Removal

Test if the issue is timing-related by adding a delay before removing the
connection:

```rust
fn close_webview_for_pane(&mut self, pane_id: PaneId) {
    // ... remove overlay ...

    // Delay before removing XPC connection
    std::thread::sleep(Duration::from_millis(100));

    if let Some(xpc_manager) = get_xpc_manager() {
        xpc_manager.remove_connection(pane_id);
    }
}
```

### Experiment 3: Check Listener Lifecycle

Investigate whether listeners need to be removed when webviews close:

```rust
// Add method to XpcManager
pub fn remove_listener_for_session(&self, session_id: &str) {
    // Find and remove the listener associated with this session
}
```

## Success Criteria

- [ ] Closing one webview does not affect other webviews
- [ ] Profile server remains running while at least one webview is active
- [ ] Connection count accurately reflects active connections
- [ ] No "XPC connection invalid" errors when closing a single webview

## References

- Issue 329 — Where this bug was discovered
- Issue 326 — Profile server graceful shutdown (introduced connection counting)
- `CLAUDE.md` — Documents "Current gap" with multi-webview support
