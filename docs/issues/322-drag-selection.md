# 322: Drag Selection

Click-and-drag to select text in webview panes.

## Status

Not started.

## Product Requirements

Users need to select arbitrary text ranges by clicking and dragging:

1. **Click-and-drag selection** — Press mouse button, drag across text, release
   to select. The selection should highlight as the user drags.

2. **Visual feedback** — Selected text should display with standard highlight
   color during and after the drag operation.

3. **Selection persistence** — After releasing the mouse, the selection should
   remain until the user clicks elsewhere or performs another action.

4. **Copy support** — Selected text should be copyable via Cmd+C (already
   working from keyboard input issue 317).

5. **Extend selection** — Shift-click after an initial selection should extend
   the selection to the new click position (requires modifier key support).

## Background

### What Works (from Issues 317, 319, 320, 321)

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

### How Drag Selection Works in Browsers

Drag selection requires tracking mouse button state across move events:

1. **Mouse down** — Start potential selection at click position
2. **Mouse move (while button held)** — Extend selection to current position
3. **Mouse up** — Finalize selection

CEF receives these as separate events and handles selection internally. The key
is that mouse move events must indicate when the button is pressed.

### Current Mouse Move Implementation

The current `send_mouse_move()` doesn't track button state:

```rust
// From webview_xpc.rs
pub fn send_mouse_move(
    &self,
    pane_id: PaneId,
    x: i32,
    y: i32,
    modifiers: u32,  // Currently always 0
) -> bool
```

CEF's `send_mouse_move_event` signature:

```c
void send_mouse_move_event(
    const cef_mouse_event_t* event,  // x, y, modifiers
    int mouse_leave                   // 1 if mouse left the view
);
```

The `modifiers` field in `cef_mouse_event_t` can include mouse button flags:

| Flag                            | Value  | Meaning                  |
| ------------------------------- | ------ | ------------------------ |
| `EVENTFLAG_LEFT_MOUSE_BUTTON`   | 1 << 4 | Left button is pressed   |
| `EVENTFLAG_MIDDLE_MOUSE_BUTTON` | 1 << 5 | Middle button is pressed |
| `EVENTFLAG_RIGHT_MOUSE_BUTTON`  | 1 << 6 | Right button is pressed  |

### Architecture Reference

```
Drag Selection Flow:

1. Mouse Press (start selection)
    │
    ▼
send_mouse_click(is_up=false)
    │
    ├─ [NEEDED] Track that left button is now held
    │
    ▼
CEF positions cursor at click point

2. Mouse Move (extend selection)
    │
    ▼
send_mouse_move(modifiers with LEFT_MOUSE_BUTTON flag)
    │
    ▼
CEF extends selection to new position

3. Mouse Release (end selection)
    │
    ▼
send_mouse_click(is_up=true)
    │
    ├─ [NEEDED] Clear left button held state
    │
    ▼
CEF finalizes selection
```

## Implementation Approach

### 1. Track Button State

Add state to track which mouse buttons are currently pressed:

```rust
// In TermWindow or as part of webview state
mouse_buttons_held: RefCell<HashSet<MousePress>>,
```

Update on press/release events.

### 2. Include Button State in Move Events

When sending mouse move, check if buttons are held and set modifiers:

```rust
let mut modifiers = 0u32;
if self.mouse_buttons_held.borrow().contains(&MousePress::Left) {
    modifiers |= 0x10; // EVENTFLAG_LEFT_MOUSE_BUTTON
}
xpc_manager.send_mouse_move(pane_id, cef_x, cef_y, modifiers);
```

### 3. Verify CEF Receives Button State

The profile server's `MouseMoveTask` already passes modifiers to CEF:

```rust
let mouse_event = cef::MouseEvent {
    x: self.x,
    y: self.y,
    modifiers: self.modifiers,  // Just need to populate this
};
host.send_mouse_move_event(Some(&mouse_event), 0);
```

## Success Criteria

- [ ] Click-and-drag selects text
- [ ] Selection highlights during drag
- [ ] Selection persists after mouse release
- [ ] Can copy selection with Cmd+C
- [ ] Multiple drag selections work (new drag clears old selection)

## Next Steps (Other Mouse Input)

After drag selection, these features remain:

| Feature         | Priority | Notes                                                |
| --------------- | -------- | ---------------------------------------------------- |
| Modifier keys   | Medium   | Shift-click to extend selection, Cmd-click for links |
| Right-click     | Medium   | Context menus                                        |
| Middle-click    | Low      | Paste or open in new tab                             |
| Cursor feedback | Low      | Change cursor shape over links, text                 |

## Experiments

_No experiments yet._

## References

- `docs/issues/319-mouse.md` — Basic mouse input (completed)
- `docs/issues/320-double-click.md` — Double/triple click (completed)
- `docs/issues/321-scroll.md` — Scroll support (completed)
- CEF event flags: `cef_event_flags_t` in cef-rs bindings
- `ts3/wezterm-gui/src/termwindow/mouseevent.rs` — Mouse event handling
- `ts3/wezterm-gui/src/termwindow/webview_xpc.rs` — XPC mouse methods
- `ts3/termsurf-profile/src/main.rs` — CEF event handlers
