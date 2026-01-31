# Issue 317: Input Forwarding to Webview

## Goal

Forward keyboard and mouse input from the terminal to the webview so users can
interact with web pages — click links, scroll, type in forms, and use standard
browser navigation.

## Background

### Current Behavior

When a webview pane is visible in Browse mode:

- The webview renders correctly
- All keyboard input is intercepted and consumed (315-mode)
- No input reaches the browser
- Users cannot interact with the page

The webview is effectively display-only. This is the biggest gap in
functionality.

### Desired Behavior

In Browse mode:

- Keyboard input is forwarded to CEF
- Mouse clicks are forwarded to CEF
- Mouse movement is forwarded to CEF (for hover effects)
- Scroll events are forwarded to CEF
- Users can interact with web pages naturally

### Reference Implementation (ts2)

ts2 implements input forwarding in `ts2/wezterm-gui/src/cef_browser/mod.rs`. Key
patterns:

1. **Keyboard events** — Converted to CEF `KeyEvent` struct with:

   - `key_type`: Press, Release, or Char
   - `modifiers`: Shift, Ctrl, Alt, Meta
   - `windows_key_code`: Virtual key code
   - `native_key_code`: Platform-specific code
   - `character`: Unicode character (for Char events)

2. **Mouse events** — Converted to CEF mouse API with:

   - `x`, `y`: Logical coordinates relative to browser view
   - `button`: Left, Middle, Right
   - `modifiers`: Shift, Ctrl, Alt
   - `click_count`: 1 for click, 2 for double-click

3. **Scroll events** — Converted to CEF scroll with:
   - `x`, `y`: Cursor position
   - `delta_x`, `delta_y`: Scroll amount

### XPC Command Infrastructure

Issue 308 established bidirectional XPC for resize commands:

```rust
// GUI sends command to profile
xpc_manager.send_resize(pane_id, width, height);

// Profile receives via event handler
set_event_handler(&*gui, move |event| {
    let action = msg.get_string("action");
    match action.as_str() {
        "resize_browser" => { ... }
    }
});
```

Input forwarding uses the same pattern with new action types.

## Product Requirements

### Keyboard Input

**In Browse mode**, all keyboard input (except mode-switching keys) should be
forwarded to the browser:

| Input                      | Action                                 |
| -------------------------- | -------------------------------------- |
| Regular keys (a-z, 0-9)    | Forward to browser                     |
| Arrow keys                 | Forward to browser (navigation)        |
| Enter                      | Forward to browser (form submit)       |
| Tab                        | Forward to browser (focus next)        |
| Backspace, Delete          | Forward to browser                     |
| Ctrl+C (in text field)     | Forward to browser (copy)              |
| Ctrl+V                     | Forward to browser (paste)             |
| Ctrl+A                     | Forward to browser (select all)        |
| Ctrl+C (not in text field) | Switch to Control mode (315 rule)      |
| WezTerm keybindings        | Execute WezTerm action (not forwarded) |

**Key insight:** Ctrl+C behavior depends on context. If the browser has a text
selection or is in a text field, Ctrl+C should copy. Otherwise, it switches to
Control mode. For Phase 1, we can simplify: always switch to Control mode on
Ctrl+C. Phase 2 can add context-aware behavior.

### Mouse Input

**In Browse mode**, all mouse input within the webview bounds should be
forwarded:

| Input        | Action                              |
| ------------ | ----------------------------------- |
| Left click   | Forward to browser                  |
| Right click  | Forward to browser (context menu)   |
| Middle click | Forward to browser                  |
| Mouse move   | Forward to browser (hover)          |
| Scroll       | Forward to browser                  |
| Drag         | Forward to browser (text selection) |

**Coordinate translation:** Mouse coordinates from WezTerm are in physical
pixels relative to the window. CEF expects logical pixels relative to the
browser view. Translation required:

```
cef_x = (window_x - pane_left) / scale_factor
cef_y = (window_y - pane_top - control_bar_height) / scale_factor
```

### Control Mode

In Control mode, input is NOT forwarded. This is already implemented in 315:

- Keys are consumed or passed to WezTerm keybindings
- Enter returns to Browse mode
- Ctrl+C exits the browser

### Focus Indication

Users need visual feedback about which mode is active:

- **Browse mode**: Webview at full brightness (current)
- **Control mode**: Webview dimmed (implemented in 316)

No additional visual changes needed for input forwarding.

## Technical Approach

### Phase 1: Keyboard Forwarding

**Goal:** Forward keyboard input from Browse mode to CEF.

#### 1. Modify key interception (keyevent.rs)

Currently `handle_webview_key_event` consumes all keys in Browse mode:

```rust
WebviewMode::Browse => {
    if is_ctrl_c { /* switch to Control mode */ }
    // In Browse mode, consume all keys (future: forward to CEF)
    Some(true)
}
```

Change to forward keys:

```rust
WebviewMode::Browse => {
    if is_ctrl_c {
        overlay.mode = WebviewMode::Control;
        return Some(true);
    }

    // Forward to CEF via XPC
    if window_key.key_is_down {
        if let Some(xpc_manager) = get_xpc_manager() {
            xpc_manager.send_key_event(pane_id, window_key);
        }
    }
    Some(true) // Still consume (don't send to terminal)
}
```

#### 2. Add send_key_event to XpcManager (webview_xpc.rs)

```rust
pub fn send_key_event(&self, pane_id: PaneId, key: &KeyEvent) {
    let msg = XpcDictionary::new();
    msg.set_string("action", "key_event");
    msg.set_bool("key_is_down", key.key_is_down);

    // Serialize key code
    let key_code = match &key.key {
        KeyCode::Char(c) => format!("char:{}", c),
        KeyCode::Function(n) => format!("f:{}", n),
        KeyCode::LeftArrow => "left".to_string(),
        KeyCode::RightArrow => "right".to_string(),
        KeyCode::UpArrow => "up".to_string(),
        KeyCode::DownArrow => "down".to_string(),
        KeyCode::Enter => "enter".to_string(),
        KeyCode::Backspace => "backspace".to_string(),
        KeyCode::Tab => "tab".to_string(),
        KeyCode::Escape => "escape".to_string(),
        // ... other keys
        _ => format!("raw:{:?}", key.key),
    };
    msg.set_string("key_code", &key_code);

    // Serialize modifiers
    let mods = key.modifiers;
    msg.set_bool("shift", mods.contains(Modifiers::SHIFT));
    msg.set_bool("ctrl", mods.contains(Modifiers::CTRL));
    msg.set_bool("alt", mods.contains(Modifiers::ALT));
    msg.set_bool("meta", mods.contains(Modifiers::SUPER));

    self.send_command(pane_id, &msg);
}
```

#### 3. Handle key_event in profile server (main.rs)

```rust
"key_event" => {
    let key_is_down = msg.get_bool("key_is_down");
    let key_code = msg.get_string("key_code").unwrap_or_default();
    let shift = msg.get_bool("shift");
    let ctrl = msg.get_bool("ctrl");
    let alt = msg.get_bool("alt");
    let meta = msg.get_bool("meta");

    // Convert to CEF KeyEvent
    let cef_event = convert_to_cef_key_event(
        key_is_down, &key_code, shift, ctrl, alt, meta
    );

    // Send to browser on UI thread
    let bs = Arc::clone(browser_state);
    cef::post_task(cef::ThreadId::UI, move || {
        if let Some(browser) = bs.browser.lock().unwrap().as_ref() {
            if let Some(host) = browser.host() {
                host.send_key_event(&cef_event);
            }
        }
    });
}
```

#### 4. CEF key event conversion

Reference ts2 for the conversion logic. Key points:

- CEF uses Windows virtual key codes even on macOS
- Need to handle both key down/up and character events
- Character events are separate from key events in CEF

### Phase 2: Mouse Forwarding

**Goal:** Forward mouse clicks and movement to CEF.

#### 1. Intercept mouse events (mod.rs or new file)

WezTerm handles mouse events in `termwindow/mod.rs`. Add webview check:

```rust
fn mouse_event(&mut self, event: MouseEvent, window: &Window) {
    // Check if mouse is over a webview pane
    if let Some((pane_id, local_x, local_y)) = self.webview_hit_test(&event) {
        self.forward_mouse_to_webview(pane_id, local_x, local_y, &event);
        return; // Don't process as terminal event
    }

    // ... existing terminal mouse handling
}
```

#### 2. Hit test for webview panes

```rust
fn webview_hit_test(&self, event: &MouseEvent) -> Option<(PaneId, f32, f32)> {
    let server = get_server()?;
    let overlays = server.state().read().unwrap();

    for (pane_id, _) in overlays.overlays.iter() {
        // Get pane bounds
        let pos = self.get_panes_to_render()
            .iter()
            .find(|p| p.pane.pane_id() == *pane_id)?;

        // Check if mouse is within pane bounds
        let pane_x = pos.left as f32 * self.render_metrics.cell_size.width as f32;
        let pane_y = pos.top as f32 * self.render_metrics.cell_size.height as f32;
        let pane_w = pos.pixel_width as f32;
        let pane_h = pos.pixel_height as f32;

        if event.x >= pane_x && event.x < pane_x + pane_w
            && event.y >= pane_y && event.y < pane_y + pane_h
        {
            // Translate to local coordinates
            let local_x = (event.x - pane_x) / scale;
            let local_y = (event.y - pane_y - control_bar_height) / scale;
            return Some((*pane_id, local_x, local_y));
        }
    }
    None
}
```

#### 3. Add send_mouse_event to XpcManager

```rust
pub fn send_mouse_event(
    &self,
    pane_id: PaneId,
    x: f32,
    y: f32,
    event_type: &str, // "move", "down", "up", "scroll"
    button: Option<MouseButton>,
    scroll_delta: Option<(f32, f32)>,
    modifiers: Modifiers,
) {
    let msg = XpcDictionary::new();
    msg.set_string("action", "mouse_event");
    msg.set_string("event_type", event_type);
    // XPC has no set_f32, use string encoding
    msg.set_string("x", &x.to_string());
    msg.set_string("y", &y.to_string());

    if let Some(btn) = button {
        msg.set_string("button", match btn {
            MouseButton::Left => "left",
            MouseButton::Right => "right",
            MouseButton::Middle => "middle",
            _ => "none",
        });
    }

    if let Some((dx, dy)) = scroll_delta {
        msg.set_string("scroll_x", &dx.to_string());
        msg.set_string("scroll_y", &dy.to_string());
    }

    msg.set_bool("shift", modifiers.contains(Modifiers::SHIFT));
    msg.set_bool("ctrl", modifiers.contains(Modifiers::CTRL));
    msg.set_bool("alt", modifiers.contains(Modifiers::ALT));

    self.send_command(pane_id, &msg);
}
```

#### 4. Handle mouse_event in profile server

```rust
"mouse_event" => {
    let event_type = msg.get_string("event_type").unwrap_or_default();
    let x: f32 = msg.get_string("x").unwrap_or_default().parse().unwrap_or(0.0);
    let y: f32 = msg.get_string("y").unwrap_or_default().parse().unwrap_or(0.0);

    match event_type.as_str() {
        "move" => {
            host.send_mouse_move_event(x as i32, y as i32, false);
        }
        "down" => {
            let button = parse_button(&msg);
            host.send_mouse_click_event(x as i32, y as i32, button, false, 1);
        }
        "up" => {
            let button = parse_button(&msg);
            host.send_mouse_click_event(x as i32, y as i32, button, true, 1);
        }
        "scroll" => {
            let dx: f32 = msg.get_string("scroll_x")...;
            let dy: f32 = msg.get_string("scroll_y")...;
            host.send_mouse_wheel_event(x as i32, y as i32, dx as i32, dy as i32);
        }
        _ => {}
    }
}
```

## Files to Modify

| File                                            | Changes                                    |
| ----------------------------------------------- | ------------------------------------------ |
| `ts3/wezterm-gui/src/termwindow/keyevent.rs`    | Forward keys in Browse mode                |
| `ts3/wezterm-gui/src/termwindow/webview_xpc.rs` | Add send_key_event, send_mouse_event       |
| `ts3/wezterm-gui/src/termwindow/mod.rs`         | Intercept mouse events for webview panes   |
| `ts3/termsurf-profile/src/main.rs`              | Handle key_event, mouse_event XPC commands |

## Success Criteria

### Phase 1: Keyboard

- [ ] Keys forwarded to browser in Browse mode
- [ ] Can type in Google search box
- [ ] Can press Enter to submit form
- [ ] Arrow keys navigate (scroll page, move cursor)
- [ ] Tab key moves focus between elements
- [ ] Ctrl+C still switches to Control mode
- [ ] WezTerm keybindings still work

### Phase 2: Mouse

- [ ] Click on links navigates to new page
- [ ] Click in text field focuses it
- [ ] Mouse hover shows hover effects (underlines, tooltips)
- [ ] Scroll wheel scrolls the page
- [ ] Right-click shows context menu
- [ ] Drag to select text works
- [ ] Double-click selects word

## Verification

```bash
cd ts3 && ./scripts/build-debug.sh --open

# Test 1: Keyboard input
web google.com
# Type "hello" in search box
# Expected: Text appears in search box

# Test 2: Form submission
# Press Enter
# Expected: Google search results appear

# Test 3: Arrow key navigation
# Press Tab to focus a link, Enter to click
# Expected: Navigation works

# Test 4: Mode switching still works
# Press Ctrl+C
# Expected: Switch to Control mode, webview dims

# Test 5: Mouse click (Phase 2)
# Click on a link
# Expected: Navigates to new page

# Test 6: Scroll (Phase 2)
# Scroll with mouse wheel
# Expected: Page scrolls smoothly
```

## References

- `docs/issues/315-mode.md` — Key interception implementation
- `ts2/wezterm-gui/src/cef_browser/mod.rs` — ts2 input handling
- `ts2/wezterm-gui/src/termwindow/keyevent.rs` — ts2 key forwarding
- CEF documentation on `send_key_event`, `send_mouse_click_event`

---

## Experiments

(To be added as implementation progresses)
