# Issue 315: Control Mode

## Goal

Implement mode switching for webview panes. When focused on a webview pane, the
user is in one of two modes:

- **Browse mode** — Browser is focused, receiving input (future)
- **Control mode** — Control panel is focused, browser dimmed

This issue implements the mode state machine and key interception. Actual input
forwarding to the browser is deferred to a future issue.

## Background

### Current Behavior

When a webview pane is visible:

- The control panel displays the URL
- The webview renders below it
- All keyboard input goes to the terminal underneath
- Ctrl+C sends SIGINT to the terminal process

### Desired Behavior

When a webview pane is visible:

- No keyboard input reaches the terminal underneath
- Keys are intercepted by the control panel, webview, or WezTerm GUI
- Mode determines which component receives input

## Product Requirements

### Mode State Machine

```
                ┌─────────────┐
                │             │
     ┌──────────│ Browse Mode │◄─────────┐
     │          │  (default)  │          │
     │          │             │          │
     │          └─────────────┘          │
     │                                   │
Ctrl+C                               Enter
     │                                   │
     │          ┌─────────────┐          │
     │          │             │          │
     └─────────►│Control Mode │──────────┘
                │             │
                └──────┬──────┘
                       │
                  Ctrl+C
                       │
                       ▼
                ┌─────────────┐
                │             │
                │ Exit Browser│
                │             │
                └─────────────┘
```

### Browse Mode

**Default mode** when entering a webview pane.

| Input               | Action                                     |
| ------------------- | ------------------------------------------ |
| Ctrl+C              | Switch to Control mode                     |
| WezTerm keybindings | Execute (e.g., Ctrl+Shift+T for new tab)   |
| All other keys      | No-op for now (future: forward to browser) |
| Mouse input         | No-op for now (future: forward to browser) |

**Visual appearance:**

- Control panel shows URL
- Webview renders normally (full brightness)

### Control Mode

**Activated** by pressing Ctrl+C in Browse mode.

| Input               | Action                                           |
| ------------------- | ------------------------------------------------ |
| Enter               | Switch to Browse mode                            |
| Ctrl+C              | Exit browser (close webview, return to terminal) |
| WezTerm keybindings | Execute (e.g., Ctrl+Shift+T for new tab)         |
| All other keys      | No-op                                            |
| Mouse input         | No-op                                            |

**Visual appearance:**

- Control panel shows instructions: "Enter to browse. Ctrl+C to exit."
- Webview renders dimmed (reduced opacity or overlay)

### Key Interception

**Critical requirement:** While a webview is visible, NO keys should reach the
terminal process underneath. This prevents:

- Accidental input to shell while browsing
- Ctrl+C sending SIGINT to terminal process
- Any keystrokes appearing in terminal

Keys are handled in this priority order:

1. **WezTerm keybindings** — Ctrl+Shift+T, Ctrl+Tab, etc.
2. **Mode-specific actions** — Ctrl+C, Enter (as defined above)
3. **Browser input** — Future: forwarded to CEF in Browse mode
4. **Dropped** — All remaining keys are discarded

### Exit Behavior

When exiting the browser (Ctrl+C in Control mode):

1. Close the webview overlay
2. Remove the control panel
3. Return focus to the terminal pane underneath
4. Terminal resumes normal operation

This matches the current Ctrl+C behavior, but only triggers from Control mode.

## Technical Approach

### Mode State Storage

Store the current mode per webview pane:

```rust
pub enum WebviewMode {
    Browse,
    Control,
}

// In WebviewOverlay or separate state
pub struct WebviewModeState {
    mode: WebviewMode,
}
```

### Key Event Interception

Intercept key events before they reach the terminal:

1. Check if the focused pane has a webview overlay
2. If yes, route the key through the mode state machine
3. Only WezTerm keybindings and mode actions are processed
4. All other keys are consumed (not forwarded)

Location in WezTerm: `termwindow/mod.rs` key event handling.

### Visual Feedback

**Control mode text** (matching ts2):

```
"Enter to browse. Ctrl+C to exit."
```

**Dimming in Control mode:**

- Option A: Reduce webview opacity
- Option B: Overlay semi-transparent layer
- Option C: Apply CSS filter via CEF (future)

For Phase 1, Option B is simplest — render a semi-transparent overlay on top of
the webview texture.

## Implementation Plan

### Step 1: Add Mode State

Add `WebviewMode` enum and storage to track current mode per pane.

### Step 2: Intercept Key Events

Modify key handling to check for webview overlay and route through mode logic.

### Step 3: Implement Mode Transitions

- Ctrl+C in Browse mode → Control mode
- Enter in Control mode → Browse mode
- Ctrl+C in Control mode → Exit browser

### Step 4: Update Control Panel Text

Show different text based on mode:

- Browse mode: URL
- Control mode: "Enter to browse. Ctrl+C to exit."

### Step 5: Add Visual Dimming

Render semi-transparent overlay on webview in Control mode.

## Files to Modify

| File                                               | Changes                         |
| -------------------------------------------------- | ------------------------------- |
| `ts3/wezterm-gui/src/termwindow/webview_socket.rs` | Add WebviewMode enum and state  |
| `ts3/wezterm-gui/src/termwindow/mod.rs`            | Key event interception          |
| `ts3/wezterm-gui/src/termwindow/render/pane.rs`    | Mode-aware control panel text   |
| `ts3/wezterm-gui/src/termwindow/render/draw.rs`    | Dimming overlay in Control mode |

## Verification

```bash
cd ts3 && ./scripts/build-debug.sh --open

# 1. Open webview (starts in Browse mode)
web google.com

# 2. Verify Browse mode
# - Control panel shows URL
# - Type random keys — nothing appears in terminal
# - Webview is full brightness

# 3. Press Ctrl+C — switch to Control mode
# - Control panel shows "Enter to browse. Ctrl+C to exit."
# - Webview is dimmed
# - Type random keys — nothing appears in terminal

# 4. Press Enter — switch back to Browse mode
# - Control panel shows URL again
# - Webview is full brightness

# 5. Press Ctrl+C twice — exit browser
# - First Ctrl+C: Control mode
# - Second Ctrl+C: Browser closes, terminal visible

# 6. Verify WezTerm keybindings work in both modes
# - Ctrl+Shift+T opens new tab
# - Ctrl+Tab switches tabs
```

## Success Criteria

1. [ ] `WebviewMode` enum exists (Browse, Control)
2. [ ] Mode state stored per webview pane
3. [ ] Keys intercepted when webview is visible
4. [ ] No keys reach terminal underneath
5. [ ] Ctrl+C in Browse mode → Control mode
6. [ ] Enter in Control mode → Browse mode
7. [ ] Ctrl+C in Control mode → Exit browser
8. [ ] Control panel text changes based on mode
9. [ ] Visual dimming in Control mode
10. [ ] WezTerm keybindings work in both modes

## References

- `docs/issues/314-control.md` — Control panel implementation
- `ts2/wezterm-gui/src/cef_browser/mod.rs` — ts2 BrowserMode enum
- `ts1/src/apprt/surface.zig` — ts1 mode implementation

---

## Experiments

(To be added during implementation)
