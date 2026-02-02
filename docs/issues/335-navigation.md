# Issue 335: Browser Navigation (Back/Forward)

## Product Requirements

### User Story

As a user browsing the web in TermSurf, I want to navigate back and forward
through my browsing history using familiar keyboard shortcuts, so that I can
revisit pages I've already viewed without retyping URLs.

### Acceptance Criteria

1. **Cmd+[** navigates back in browser history (like Safari/Chrome)
2. **Cmd+]** navigates forward in browser history (like Safari/Chrome)
3. Navigation only works when a webview pane is focused
4. Navigation works in both Browse mode and Control mode
5. If there's no history to navigate (e.g., can't go back on first page), the
   command does nothing silently

### Keybindings

| Shortcut | Action           | Notes                           |
| -------- | ---------------- | ------------------------------- |
| Cmd+[    | Navigate back    | Standard macOS browser shortcut |
| Cmd+]    | Navigate forward | Standard macOS browser shortcut |

### Non-Requirements (Out of Scope)

- History list/menu (showing all visited pages)
- Visual feedback when navigation occurs (URL will change naturally)
- Configurable keybindings (hardcoded for now, like ts2)
- Mouse back/forward buttons (future issue)

## Technical Context

### ts2 Implementation

In ts2, CEF runs in-process. Navigation is handled by:

1. Intercepting Cmd+[/] in `keyevent.rs` (lines 480-500) as special cases in
   Browse mode
2. Calling `browser.go_back()` / `browser.go_forward()` directly on the CEF
   browser object

### ts3 Challenge

In ts3, CEF runs **out-of-process** in `termsurf-profile`. The GUI cannot call
CEF methods directly — it must send a message to the profile server via IPC.

### IPC Options

**Option A: Unix socket protocol**

Extend the existing socket protocol (used for `open_webview`) with new commands:

```json
{"action": "go_back", "pane_id": 123}
{"action": "go_forward", "pane_id": 123}
```

**Option B: XPC messages**

Send navigation commands via the direct XPC connection between GUI and profile
server.

## Files Involved

### GUI Side

- `ts3/wezterm-gui/src/termwindow/keyevent.rs` — Intercept Cmd+[/] and send
  navigation command
- `ts3/wezterm-gui/src/termwindow/webview_socket.rs` — Add protocol support for
  navigation commands (if using socket)
- `ts3/wezterm-gui/src/termwindow/webview_xpc.rs` — Add XPC message types (if
  using XPC)

### Profile Server Side

- `ts3/termsurf-web/src/main.rs` — Handle navigation commands, call CEF methods

### CEF Bindings

- `cef-rs/cef/src/bindings/` — Already has `go_back()` and `go_forward()` on
  Browser object

---

## Experiments

(To be designed)
