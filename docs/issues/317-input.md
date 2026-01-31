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

### Experiment 1: Basic Keyboard Forwarding

**Goal:** Forward keyboard input from Browse mode to CEF so users can type in
text fields and navigate with the keyboard.

#### Background

ts2 handles keyboard forwarding in `keyevent.rs:720-815`. Key insights:

1. **CEF uses Windows virtual key codes** — Even on macOS, CEF expects Windows
   VK codes. Requires a conversion table from macOS native keycodes.

2. **Three event types** — KEYDOWN, KEYUP, and CHAR. For printable characters,
   both KEYDOWN and CHAR must be sent on key press.

3. **Skip KEYUP for action keys** — Keys like Tab, Enter, arrows trigger actions
   on both down and up. Sending both causes double actions. Only send KEYDOWN.

4. **Modifiers tracked separately** — Shift, Ctrl, Alt, Meta are bitflags.

The existing XPC infrastructure from resize (issue 308) provides the command
pathway. We add a new `key_event` action.

#### Approach

**Part A: Add send_key_event to XpcManager (webview_xpc.rs)**

```rust
/// Send a key event to the browser in the given pane
pub fn send_key_event(&self, pane_id: PaneId, key: &KeyEvent) {
    let msg = XpcDictionary::new();
    msg.set_string("action", "key_event");
    msg.set_bool("key_is_down", key.key_is_down);

    // Include raw keycode if available (for accurate VK conversion)
    if let Some(raw) = &key.raw {
        msg.set_i64("raw_code", raw.raw_code as i64);
    }

    // Serialize the key for character extraction
    match &key.key {
        KeyCode::Char(c) => {
            msg.set_string("key_type", "char");
            msg.set_i64("char_code", *c as i64);
        }
        KeyCode::LeftArrow => msg.set_string("key_type", "left"),
        KeyCode::RightArrow => msg.set_string("key_type", "right"),
        KeyCode::UpArrow => msg.set_string("key_type", "up"),
        KeyCode::DownArrow => msg.set_string("key_type", "down"),
        KeyCode::Home => msg.set_string("key_type", "home"),
        KeyCode::End => msg.set_string("key_type", "end"),
        KeyCode::PageUp => msg.set_string("key_type", "pageup"),
        KeyCode::PageDown => msg.set_string("key_type", "pagedown"),
        KeyCode::Insert => msg.set_string("key_type", "insert"),
        KeyCode::Function(n) => {
            msg.set_string("key_type", "function");
            msg.set_i64("function_num", *n as i64);
        }
        _ => msg.set_string("key_type", "unknown"),
    }

    // Serialize modifiers
    let mods = key.modifiers;
    msg.set_bool("shift", mods.contains(Modifiers::SHIFT));
    msg.set_bool("ctrl", mods.contains(Modifiers::CTRL));
    msg.set_bool("alt", mods.contains(Modifiers::ALT));
    msg.set_bool("meta", mods.contains(Modifiers::SUPER));

    if self.send_command(pane_id, &msg) {
        log::debug!(
            "[XPC] Sent key_event to pane {}: {:?} down={}",
            pane_id, key.key, key.key_is_down
        );
    }
}
```

**Part B: Forward keys in handle_webview_key_event (keyevent.rs)**

Modify the Browse mode handling to forward keys instead of just consuming them:

```rust
WebviewMode::Browse => {
    if is_ctrl_c {
        log::info!("[Webview] Ctrl+C in Browse mode → Control mode");
        overlay.mode = WebviewMode::Control;
        drop(overlays);
        if let Some(ref w) = self.window {
            w.invalidate();
        }
        return Some(true);
    }

    // Forward key to browser via XPC
    if let Some(xpc_manager) = crate::termwindow::webview_xpc::get_xpc_manager() {
        xpc_manager.send_key_event(pane_id, window_key);
    }

    // Consume the key (don't send to terminal)
    Some(true)
}
```

**Part C: Handle key_event in profile server (main.rs)**

Add handler in the XPC event handler (after resize_browser):

```rust
"key_event" => {
    let state_guard = deferred_for_handler.lock().unwrap();
    let Some(bs) = state_guard.as_ref() else {
        println!("Profile: key_event ignored (state not ready)");
        return;
    };

    let key_is_down = msg.get_bool("key_is_down");
    let key_type = msg.get_string("key_type").unwrap_or_default();
    let raw_code = msg.get_i64("raw_code") as u32;
    let char_code = msg.get_i64("char_code") as u32;

    let shift = msg.get_bool("shift");
    let ctrl = msg.get_bool("ctrl");
    let alt = msg.get_bool("alt");
    let meta = msg.get_bool("meta");

    let bs = Arc::clone(bs);
    drop(state_guard);

    // Post to CEF UI thread
    cef::post_task(cef::ThreadId::UI, move || {
        send_key_event_to_cef(
            &bs, key_is_down, &key_type, raw_code, char_code,
            shift, ctrl, alt, meta
        );
    });
}
```

**Part D: CEF key event sending (main.rs)**

Add the key event conversion and sending:

```rust
fn send_key_event_to_cef(
    state: &BrowserState,
    key_is_down: bool,
    key_type: &str,
    raw_code: u32,
    char_code: u32,
    shift: bool,
    ctrl: bool,
    alt: bool,
    meta: bool,
) {
    use cef::{KeyEvent, KeyEventType};

    let browser = match state.browser.lock().unwrap().as_ref() {
        Some(b) => b.clone(),
        None => return,
    };
    let host = match browser.host() {
        Some(h) => h,
        None => return,
    };

    // Build CEF modifiers
    let mut modifiers = 0u32;
    if shift { modifiers |= cef::EVENTFLAG_SHIFT_DOWN; }
    if ctrl { modifiers |= cef::EVENTFLAG_CONTROL_DOWN; }
    if alt { modifiers |= cef::EVENTFLAG_ALT_DOWN; }
    if meta { modifiers |= cef::EVENTFLAG_COMMAND_DOWN; }

    // Convert to Windows VK code
    let windows_vk = macos_keycode_to_windows_vk(raw_code);
    let native_code = raw_code as i32;

    // Determine if this is an action key (skip KEYUP to avoid double actions)
    let is_action_key = matches!(
        key_type,
        "left" | "right" | "up" | "down" | "home" | "end" |
        "pageup" | "pagedown" | "insert"
    ) || (key_type == "char" && matches!(
        char_code,
        0x08 | 0x7f | 0x09 | 0x1b | 0x0d | 0x20  // BS, DEL, TAB, ESC, ENTER, SPACE
    ));

    if is_action_key && !key_is_down {
        return; // Skip KEYUP for action keys
    }

    // Send KEYDOWN or KEYUP
    let event_type = if key_is_down {
        KeyEventType::KEYDOWN
    } else {
        KeyEventType::KEYUP
    };

    let key_event = KeyEvent {
        size: std::mem::size_of::<KeyEvent>(),
        type_: event_type,
        modifiers,
        windows_key_code: windows_vk,
        native_key_code: native_code,
        is_system_key: 0,
        character: 0,
        unmodified_character: 0,
        focus_on_editable_field: 0,
    };
    host.send_key_event(Some(&key_event));

    // For key-down of printable characters, also send CHAR event
    if key_is_down && key_type == "char" && char_code > 0 && char_code < 0x10000 {
        let char_event = KeyEvent {
            size: std::mem::size_of::<KeyEvent>(),
            type_: KeyEventType::CHAR,
            modifiers,
            windows_key_code: char_code as i32,
            native_key_code: 0,
            is_system_key: 0,
            character: char_code as u16,
            unmodified_character: char_code as u16,
            focus_on_editable_field: 0,
        };
        host.send_key_event(Some(&char_event));
    }

    println!(
        "Profile: key_event {:?} vk={} native={} char={}",
        event_type, windows_vk, native_code, char_code
    );
}

/// Convert macOS keycode to Windows virtual key code (from ts2)
fn macos_keycode_to_windows_vk(code: u32) -> i32 {
    match code {
        // Letters
        0x00 => 0x41, // A
        0x0B => 0x42, // B
        0x08 => 0x43, // C
        0x02 => 0x44, // D
        0x0E => 0x45, // E
        0x03 => 0x46, // F
        0x05 => 0x47, // G
        0x04 => 0x48, // H
        0x22 => 0x49, // I
        0x26 => 0x4A, // J
        0x28 => 0x4B, // K
        0x25 => 0x4C, // L
        0x2E => 0x4D, // M
        0x2D => 0x4E, // N
        0x1F => 0x4F, // O
        0x23 => 0x50, // P
        0x0C => 0x51, // Q
        0x0F => 0x52, // R
        0x01 => 0x53, // S
        0x11 => 0x54, // T
        0x20 => 0x55, // U
        0x09 => 0x56, // V
        0x0D => 0x57, // W
        0x07 => 0x58, // X
        0x10 => 0x59, // Y
        0x06 => 0x5A, // Z
        // Numbers
        0x1D => 0x30, // 0
        0x12 => 0x31, // 1
        0x13 => 0x32, // 2
        0x14 => 0x33, // 3
        0x15 => 0x34, // 4
        0x17 => 0x35, // 5
        0x16 => 0x36, // 6
        0x1A => 0x37, // 7
        0x1C => 0x38, // 8
        0x19 => 0x39, // 9
        // Special keys
        0x24 => 0x0D, // Return -> VK_RETURN
        0x30 => 0x09, // Tab -> VK_TAB
        0x31 => 0x20, // Space -> VK_SPACE
        0x33 => 0x08, // Delete (backspace) -> VK_BACK
        0x35 => 0x1B, // Escape -> VK_ESCAPE
        0x75 => 0x2E, // Forward Delete -> VK_DELETE
        // Arrow keys
        0x7B => 0x25, // Left -> VK_LEFT
        0x7C => 0x27, // Right -> VK_RIGHT
        0x7E => 0x26, // Up -> VK_UP
        0x7D => 0x28, // Down -> VK_DOWN
        // Navigation
        0x73 => 0x24, // Home -> VK_HOME
        0x77 => 0x23, // End -> VK_END
        0x74 => 0x21, // PageUp -> VK_PRIOR
        0x79 => 0x22, // PageDown -> VK_NEXT
        // Function keys
        0x7A => 0x70, // F1
        0x78 => 0x71, // F2
        0x63 => 0x72, // F3
        0x76 => 0x73, // F4
        0x60 => 0x74, // F5
        0x61 => 0x75, // F6
        0x62 => 0x76, // F7
        0x64 => 0x77, // F8
        0x65 => 0x78, // F9
        0x6D => 0x79, // F10
        0x67 => 0x7A, // F11
        0x6F => 0x7B, // F12
        // Punctuation (common ones)
        0x27 => 0xBA, // ; -> VK_OEM_1
        0x18 => 0xBB, // = -> VK_OEM_PLUS
        0x2B => 0xBC, // , -> VK_OEM_COMMA
        0x1B => 0xBD, // - -> VK_OEM_MINUS
        0x2F => 0xBE, // . -> VK_OEM_PERIOD
        0x2C => 0xBF, // / -> VK_OEM_2
        0x32 => 0xC0, // ` -> VK_OEM_3
        0x21 => 0xDB, // [ -> VK_OEM_4
        0x2A => 0xDC, // \ -> VK_OEM_5
        0x1E => 0xDD, // ] -> VK_OEM_6
        0x27 => 0xDE, // ' -> VK_OEM_7
        _ => 0,
    }
}
```

#### Files to Modify

| File                                            | Changes                                      |
| ----------------------------------------------- | -------------------------------------------- |
| `ts3/wezterm-gui/src/termwindow/webview_xpc.rs` | Add `send_key_event()` method                |
| `ts3/wezterm-gui/src/termwindow/keyevent.rs`    | Forward keys in Browse mode                  |
| `ts3/termsurf-profile/src/main.rs`              | Handle `key_event`, CEF key conversion       |

#### Verification

```bash
cd ts3 && ./scripts/build-debug.sh --open

# Test 1: Type in search box
web google.com
# Click in search box (not yet implemented - use Tab to focus)
# Type "hello"
# Expected: Letters appear in search box

# Test 2: Arrow keys
# Press Tab several times to navigate
# Press Enter to activate focused link
# Expected: Navigation works

# Test 3: Backspace
# Type "helloo", then Backspace
# Expected: Last character deleted

# Test 4: Mode switching still works
# Press Ctrl+C
# Expected: Switch to Control mode

# Test 5: WezTerm keybindings
# Press Ctrl+Shift+T
# Expected: New tab opens (not sent to browser)

# Check logs
cat /tmp/termsurf-gui.log | grep "key_event"
cat /tmp/termsurf-profile-*.log | grep "key_event"
```

#### Success Criteria

- [ ] `send_key_event` sends XPC messages with key data
- [ ] Profile server receives and processes `key_event` action
- [ ] CEF `send_key_event` is called with correct KeyEvent
- [ ] Printable characters appear when typing in text fields
- [ ] Backspace deletes characters
- [ ] Tab moves focus between elements
- [ ] Enter submits forms / activates links
- [ ] Arrow keys work for navigation
- [ ] Ctrl+C still switches to Control mode
- [ ] WezTerm keybindings still work (not forwarded)

#### Known Limitations (Phase 1)

1. **Focus** — Without mouse click support, the only way to focus a text field
   is Tab navigation. Phase 2 (mouse) will fix this.

2. **Ctrl+C ambiguity** — Always switches to Control mode, even in text fields
   where it should copy. Acceptable for Phase 1.

3. **Complex input** — IME (Chinese, Japanese input) not supported. Standard
   keyboard input only.
