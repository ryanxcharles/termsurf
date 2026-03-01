# Issue 689: Tab Lifecycle — Close Tabs When Panes Close

## Problem

When a GUI pane is removed (user closes a split, TUI exits, etc.), the
corresponding Chromium tab is never closed. The tab persists inside the profile
server — Shell, WebContents, compositor, renderer — all stay alive, consuming
memory and GPU resources. This is a silent leak for browser tabs and a crash
trigger for DevTools tabs.

### How to reproduce

1. Open `web google.com`
2. Open DevTools: `web devtools`
3. Close the DevTools pane (`:q` or close the split)
4. Open DevTools again: `web devtools`
5. Resize the terminal window
6. **Crash.** The main browser tab crashes.

The crash happens because the first DevTools tab was never closed in Chromium.
Two `InspectorOverlayAgent` instances attach to the same renderer, and on
resize, the duplicate triggers a `PaintController` DCHECK (Issue 686).

### Scope

This affects **all tabs**, not just DevTools:

- **Browser tabs:** Every closed pane leaks its Chromium tab. Orphaned tabs
  accumulate memory and GPU resources for the lifetime of the profile server.
  Masked by `killServer` — when the last pane on a profile closes, the entire
  server process is killed, destroying all tabs (orphaned or not). So single-tab
  workflows never notice. Multi-tab workflows silently leak.
- **DevTools tabs:** Same leak, but visible because orphaned DevTools crash when
  a new inspector attaches to the same renderer.

### Root cause

Two XPC connections exist per tab:

- **Connection A** (TUI ↔ GUI): Created by the TUI via the gateway. Stored as
  `web_peer` on the Pane struct. Drops when the TUI exits.
- **Connection B** (Profile Server → GUI): Created by the profile server in
  `CreateTab`/`CreateDevToolsTab`. Stored as `tab_connection` in `TabState`.
  Stays alive when the TUI exits because nobody cancels it.

When Connection A drops, `handleDisconnect` cleans up the GUI pane (overlay,
maps, focus state) and decrements the server's pane count. But it never tells
the profile server to close the Chromium tab. The profile server has no idea the
pane is gone.

### Prior art (Issue 688)

Issue 688 attempted three approaches to fix this. All failed:

1. **Experiment 1:** Built `:devtools` command. Orphaned tabs crashed on reopen.
2. **Experiment 2:** Cancelled `xpc_dictionary_get_remote_connection(msg)` — but
   that returns the shared control connection, killing all tabs.
3. **Experiment 3:** Added explicit `close_tab` XPC message with
   `CloseTabByPaneId`. Crashed on first invocation for unknown reasons.

The failures showed we don't understand the tab lifecycle well enough. Before
fixing, we need to **measure**: see exactly how many tabs Chromium thinks are
alive vs how many the GUI thinks are alive, and watch the counts change in real
time.

## Plan

### Phase 1: Measure — `query_tabs` diagnostic

Add a diagnostic query that returns the tab count from both the GUI and Chromium
in a single round trip. This lets us observe the leak directly and verify any
fix.

#### 1a. Chromium: `query_tabs` action handler

Add a new action in the control connection handler (`StartDynamicMode` in
`shell_browser_main_parts.cc`). When the profile server receives `query_tabs`,
it replies with the number of tabs and a summary of each:

```cpp
} else if (action && std::string_view(action) == "query_tabs") {
    xpc_object_t reply = xpc_dictionary_create_reply(event);
    if (reply) {
        content::GetUIThreadTaskRunner({})->PostTask(
            FROM_HERE,
            base::BindOnce([](ShellBrowserMainParts* self,
                              xpc_object_t reply) {
                xpc_dictionary_set_int64(reply, "count",
                                         self->tabs_.size());
                // Include per-tab summary for debugging.
                for (size_t i = 0; i < self->tabs_.size(); i++) {
                    auto& tab = self->tabs_[i];
                    std::string key = "tab_" + std::to_string(i);
                    std::string val = "id=" + std::to_string(tab->tab_id)
                        + " inspected=" + std::to_string(tab->inspected_tab_id)
                        + " pane=" + tab->pane_id;
                    xpc_dictionary_set_string(reply, key.c_str(),
                                              val.c_str());
                }
                xpc_connection_send_message(
                    xpc_dictionary_get_remote_connection(reply), reply);
                xpc_release(reply);
            }, base::Unretained(self), reply));
    }
}
```

Wait — `xpc_dictionary_create_reply` must be called on the handler thread, and
the reply sent from the same connection. The control connection handler runs on
XPC's dispatch queue. But `tabs_` must be accessed on the UI thread. This needs
careful threading:

1. Create the reply dictionary on the handler thread (where the message
   arrives).
2. Post to UI thread to read `tabs_` and populate the reply.
3. Send the reply from the UI thread (XPC supports this).

Alternatively, keep it simple: have the profile server log `tabs_.size()` to
stdout on every `query_tabs`, and have the GUI read the reply count. The per-tab
detail can be in the log.

#### 1b. GUI: forward `query_tabs` to profile server

Add a `query_tabs` handler in xpc.zig that:

1. Receives the synchronous XPC request from the TUI.
2. Counts GUI-side panes (from the `panes` map) for the requested profile.
3. Forwards a synchronous `query_tabs` to the profile server's control
   connection.
4. Returns both counts in the reply: `gui_panes` and `chromium_tabs`.

#### 1c. TUI: `:tabs` command

Add a `:tabs` command in the TUI command bar that calls `query_tabs` and
displays the result:

```
Chromium: 3 tabs | GUI: 1 pane
```

If the numbers don't match, we have orphans. This gives instant visibility.

### Phase 2: Fix — `close_tab` on pane cleanup

Once we can measure the leak, fix it. The approach from Issue 688 Experiment 3
is the right direction — an explicit `close_tab` XPC message on the control
connection — but we need to debug why it crashed. With the `query_tabs`
diagnostic, we can:

1. Open a browser tab → verify `Chromium: 1, GUI: 1`
2. Open DevTools → verify `Chromium: 2, GUI: 2`
3. Close DevTools pane → send `close_tab` → verify `Chromium: 1, GUI: 1`
4. Reopen DevTools → verify `Chromium: 2, GUI: 2` (no crash)

If `close_tab` crashes again, the diagnostic tells us exactly what state
Chromium was in before and after.

#### 2a. Chromium: `close_tab` action + `CloseTabByPaneId`

Same design as Issue 688 Experiment 3 — add `close_tab` action handler that
dispatches `CloseTabByPaneId(pane_id)` on the UI thread. Finds the tab by
`pane_id`, destroys Shell, cancels per-tab connection, erases from `tabs_`.

#### 2b. GUI: send `close_tab` in `cleanupPane` and `handleDisconnect`

When a pane is cleaned up and the server still has other panes alive, send
`close_tab` on the control connection with the pane's ID.

### Phase 3: Verify — automated leak test

After the fix, use `:tabs` to verify:

1. Single tab open/close cycle: counts go 1→0
2. DevTools open/close/reopen: counts go 1→2→1→2 (no crash)
3. Multi-tab same profile: close one, other survives, counts decrease correctly
4. Close last tab: server killed as before

## Relevant Code

- `chromium/src/content/chromium_profile_server/browser/shell_browser_main_parts.cc`
  — `tabs_` vector, `CreateTab`, `CreateDevToolsTab`, `CloseTab`,
  `StartDynamicMode` handler
- `chromium/src/content/chromium_profile_server/browser/shell_browser_main_parts.h`
  — `TabState` struct, method declarations
- `gui/src/apprt/xpc.zig` — `panes` map, `handleDisconnect`, `cleanupPane`,
  message handlers
- `tui/src/main.rs` — command dispatcher, `:tabs` command (new)
- `tui/src/xpc.rs` — `send_query_tabs` (new)
