# 324: Cursor Feedback

Display appropriate cursor shapes when hovering over webview content.

## Status

Not started.

## Product Requirements

Users need visual cursor feedback when interacting with webviews:

1. **Text cursor (I-beam)** — When hovering over selectable text, display the
   text selection cursor to indicate the user can select.

2. **Pointer cursor (hand)** — When hovering over clickable links, display the
   pointer/hand cursor to indicate the element is clickable.

3. **Default cursor (arrow)** — When hovering over non-interactive content,
   display the standard arrow cursor.

4. **Other cursors** — Support additional cursor types as needed (resize
   handles, wait/busy, not-allowed, etc.).

5. **Smooth transitions** — Cursor should change immediately as the mouse moves
   between different element types.

## Background

### What Works (from Issues 317-323)

Previous issues established input infrastructure for ts3 webviews:

| Feature                    | Status  | Issue |
| -------------------------- | ------- | ----- |
| Keyboard input             | Working | 317   |
| Mouse move                 | Working | 319   |
| Left click                 | Working | 319   |
| Hover effects              | Working | 319   |
| Double-click (word select) | Working | 320   |
| Triple-click (line select) | Working | 320   |
| Scroll (trackpad)          | Working | 321   |
| Drag selection             | Working | 322   |
| Shift-click extend         | Working | 323   |

### Current Behavior

Currently, the cursor does not change when hovering over webview content. It
likely stays as the default arrow regardless of what element is under the mouse.

### CEF Cursor Change API

CEF provides cursor feedback through the `CefRenderHandler::OnCursorChange`
callback:

```cpp
void OnCursorChange(
    CefRefPtr<CefBrowser> browser,
    CefCursorHandle cursor,      // Platform-specific cursor handle
    cef_cursor_type_t type,      // Cursor type enum
    const CefCursorInfo& info    // Custom cursor info (if type is CT_CUSTOM)
);
```

The `cef_cursor_type_t` enum includes:

| Type             | Value | Description                    |
| ---------------- | ----- | ------------------------------ |
| `CT_POINTER`     | 0     | Default arrow                  |
| `CT_CROSS`       | 1     | Crosshair                      |
| `CT_HAND`        | 2     | Hand/pointer for links         |
| `CT_IBEAM`       | 3     | I-beam for text                |
| `CT_WAIT`        | 4     | Busy/wait                      |
| `CT_HELP`        | 5     | Help cursor                    |
| `CT_EASTRESIZE`  | 6     | Resize east                    |
| `CT_NORTHRESIZE` | 7     | Resize north                   |
| `CT_MOVE`        | 13    | Move cursor                    |
| `CT_NOTALLOWED`  | 28    | Not allowed                    |
| ...              | ...   | Many more resize/special types |

### Architecture Reference

```
Cursor Feedback Flow:

CEF detects hover over link
    │
    ▼
CefRenderHandler::OnCursorChange(type=CT_HAND)
    │
    ▼
Profile Server receives callback
    │
    ├─ [NEEDED] Send cursor type to GUI via XPC
    │
    ▼
XPC message: { action: "cursor_change", cursor_type: 2 }
    │
    ▼
GUI receives message
    │
    ├─ [NEEDED] Map CEF cursor type to WezTerm/window cursor
    │
    └─ context.set_cursor(MouseCursor::Hand)
            │
            ▼
        Window displays hand cursor
```

### WezTerm Cursor Types

WezTerm's `window` crate provides `MouseCursor` enum:

```rust
pub enum MouseCursor {
    Arrow,
    Hand,
    Text,
    SizeUpDown,
    SizeLeftRight,
    // ... others
}
```

We need to map CEF's `cef_cursor_type_t` to WezTerm's `MouseCursor`.

### Current XPC Infrastructure

The GUI already receives XPC messages from the profile server for IOSurface
updates. We can add a new message type for cursor changes:

```rust
// Profile server sends:
{
    "action": "cursor_change",
    "pane_id": 123,
    "cursor_type": 2  // CT_HAND
}

// GUI receives and applies cursor
```

## Implementation Approach

### 1. Profile Server: Handle OnCursorChange

In `termsurf-profile`, implement the CEF callback to send cursor type to GUI:

```rust
fn on_cursor_change(
    &self,
    _browser: &Browser,
    cursor: cef::CursorHandle,
    type_: cef::CursorType,
    _custom_cursor_info: &cef::CursorInfo,
) {
    // Send cursor type to GUI via XPC
    if let Some(connection) = self.gui_connection.as_ref() {
        let msg = xpc_dictionary_create(null(), null(), 0);
        xpc_dictionary_set_string(msg, "action", "cursor_change");
        xpc_dictionary_set_int64(msg, "cursor_type", type_ as i64);
        xpc_connection_send_message(connection, msg);
    }
}
```

### 2. GUI: Receive Cursor Change Messages

In `webview_xpc.rs`, handle the new message type:

```rust
"cursor_change" => {
    let cursor_type = xpc_dictionary_get_int64(event, "cursor_type") as u32;
    // Store cursor type for this pane
    // Trigger window invalidation to apply cursor
}
```

### 3. GUI: Apply Cursor in Mouse Handler

When processing mouse events over webviews, apply the stored cursor:

```rust
fn handle_webview_mouse_event(&mut self, event: &MouseEvent) -> bool {
    // ... existing code ...

    // Apply cursor feedback
    if let Some(cursor) = self.get_webview_cursor(pane_id) {
        context.set_cursor(Some(cursor));
    }

    // ... rest of handler ...
}
```

### 4. Cursor Type Mapping

Map CEF cursor types to WezTerm cursors:

```rust
fn cef_cursor_to_wezterm(cef_type: u32) -> MouseCursor {
    match cef_type {
        0 => MouseCursor::Arrow,      // CT_POINTER
        2 => MouseCursor::Hand,       // CT_HAND
        3 => MouseCursor::Text,       // CT_IBEAM
        6 | 8 => MouseCursor::SizeLeftRight,  // CT_EASTRESIZE, CT_WESTRESIZE
        7 | 9 => MouseCursor::SizeUpDown,     // CT_NORTHRESIZE, CT_SOUTHRESIZE
        _ => MouseCursor::Arrow,      // Default for unsupported types
    }
}
```

## Success Criteria

- [ ] I-beam cursor appears when hovering over selectable text
- [ ] Hand/pointer cursor appears when hovering over links
- [ ] Arrow cursor appears when hovering over non-interactive areas
- [ ] Cursor changes are immediate (no noticeable delay)
- [ ] Cursor reverts to arrow when leaving the webview

## Next Steps (Other Mouse Input)

After cursor feedback, these features remain:

| Feature      | Priority | Notes                                                  |
| ------------ | -------- | ------------------------------------------------------ |
| Cmd-click    | Medium   | Open links (modifiers now passed, needs link handling) |
| Right-click  | Medium   | Context menus                                          |
| Middle-click | Low      | Paste or open in new tab                               |

## Experiments

_No experiments yet._

## References

- `docs/issues/323-shift-click.md` — Shift-click (completed)
- CEF `cef_cursor_type_t` enum in cef-rs bindings
- WezTerm `MouseCursor` enum in `window` crate
- `ts3/wezterm-gui/src/termwindow/mouseevent.rs` — Mouse event handling
- `ts3/wezterm-gui/src/termwindow/webview_xpc.rs` — XPC communication
- `ts3/termsurf-profile/src/main.rs` — CEF render handler
