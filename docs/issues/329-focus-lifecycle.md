# 329: Focus Lifecycle

The webview caret blinks even when the webview should not have focus.

## Status

**Open.** Follow-up to issue 328 (caret visibility).

## Problem

After implementing issue 328 (focus toggle on first paint), the caret now
appears correctly. However, it continues blinking in situations where the
webview should not have focus:

1. **Control mode** — When switching from Browse mode to Control mode (Ctrl+C),
   the caret continues blinking. The webview should be unfocused.

2. **Pane switching** — When switching to another pane, the webview caret keeps
   blinking. Only the active pane's webview should have focus.

**Expected behavior:**

| Scenario                    | Caret State  |
| --------------------------- | ------------ |
| Browse mode, active pane    | Blinking     |
| Browse mode, inactive pane  | Not blinking |
| Control mode, active pane   | Not blinking |
| Control mode, inactive pane | Not blinking |

## Background

Issue 328 added focus initialization on first paint:

```rust
// on_accelerated_paint
if !self.inner.state.initial_focus_set.load(Ordering::Relaxed) {
    host.set_focus(0);
    host.set_focus(1);
    self.inner.state.initial_focus_set.store(true, Ordering::Relaxed);
}
```

This correctly enables the caret on initial load, but there's no mechanism to
update focus state when:

- Mode changes (Browse ↔ Control)
- Active pane changes

## Proposed Solution

Send focus commands to the profile server via XPC when focus state should
change.

### Focus Events

| Event                             | Action                                |
| --------------------------------- | ------------------------------------- |
| Enter Browse mode                 | Send `set_focus(1)` to profile server |
| Enter Control mode                | Send `set_focus(0)` to profile server |
| Pane becomes active (Browse mode) | Send `set_focus(1)` to profile server |
| Pane becomes inactive             | Send `set_focus(0)` to profile server |

### Implementation Approach

**1. Add `focus` XPC command to profile server**

In `ts3/termsurf-profile/src/main.rs`, handle a new `focus` action:

```rust
"focus" => {
    let focused = msg.get_bool("focused");
    if let Some(browser) = browser_state.browser.lock().unwrap().as_ref() {
        if let Some(host) = browser.host() {
            println!("[FOCUS] Setting focus to {}", focused);
            host.set_focus(if focused { 1 } else { 0 });
        }
    }
}
```

**2. Add `send_focus` to XpcManager**

In `ts3/wezterm-gui/src/termwindow/webview_xpc.rs`:

```rust
pub fn send_focus(&self, pane_id: PaneId, focused: bool) {
    let msg = XpcDictionary::new();
    msg.set_string("action", "focus");
    msg.set_bool("focused", focused);
    self.send_command(pane_id, &msg);
}
```

**3. Send focus on mode change**

In `ts3/wezterm-gui/src/termwindow/keyevent.rs`, when mode changes:

```rust
// Ctrl+C in Browse mode → Control mode
WebviewMode::Browse => {
    if is_ctrl_c {
        overlay.mode = WebviewMode::Control;
        // Unfocus webview
        if let Some(xpc) = get_xpc_manager() {
            xpc.send_focus(pane_id, false);
        }
        // ... existing code ...
    }
}

// Enter in Control mode → Browse mode
WebviewMode::Control => {
    if is_enter {
        overlay.mode = WebviewMode::Browse;
        // Refocus webview
        if let Some(xpc) = get_xpc_manager() {
            xpc.send_focus(pane_id, true);
        }
        // ... existing code ...
    }
}
```

**4. Send focus on pane change**

This is more complex. Need to detect when the active pane changes and send
focus/unfocus commands accordingly. Possible locations:

- `MuxNotification::PaneFocused` handler in `termwindow/mod.rs`
- Mouse click handler when clicking on a different pane
- Keyboard navigation between panes

For each webview pane:

- If it becomes active AND is in Browse mode → `send_focus(true)`
- If it becomes inactive → `send_focus(false)`

## Files to Modify

| File                                            | Changes                   |
| ----------------------------------------------- | ------------------------- |
| `ts3/termsurf-profile/src/main.rs`              | Handle `focus` XPC action |
| `ts3/wezterm-gui/src/termwindow/webview_xpc.rs` | Add `send_focus()` method |
| `ts3/wezterm-gui/src/termwindow/keyevent.rs`    | Send focus on mode change |
| `ts3/wezterm-gui/src/termwindow/mod.rs`         | Send focus on pane change |

## Verification

```bash
cd ts3 && ./scripts/build-debug.sh --open

# Test 1: Control mode unfocuses
web google.com
# Caret should blink in search box
# Press Ctrl+C to enter Control mode
# Expected: Caret stops blinking, webview dims

# Test 2: Browse mode refocuses
# Press Enter to return to Browse mode
# Expected: Caret resumes blinking

# Test 3: Pane switch unfocuses
# Open a second terminal pane (Ctrl+Shift+E or similar)
# Click on the terminal pane
# Expected: Webview caret stops blinking

# Test 4: Return to webview pane refocuses
# Click on the webview pane
# Expected: Caret resumes blinking (if in Browse mode)

# Test 5: Multiple webviews
# Open two webview panes
# Switch between them
# Expected: Only the active webview has a blinking caret
```

## Success Criteria

- [ ] Caret stops blinking when entering Control mode
- [ ] Caret resumes blinking when entering Browse mode
- [ ] Caret stops blinking when switching to another pane
- [ ] Caret resumes blinking when switching back to webview in Browse mode
- [ ] Multiple webviews: only active one has blinking caret
- [ ] No regression in keyboard/mouse input

## References

- Issue 328 — Initial caret fix (focus toggle on first paint)
- Issue 315 — Mode system (Browse/Control modes)
- `ts3/wezterm-gui/src/termwindow/keyevent.rs` — Mode switching logic
- `ts3/wezterm-gui/src/termwindow/mod.rs` — Pane focus handling
