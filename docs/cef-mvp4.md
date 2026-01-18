# CEF MVP4: Control Panel and Modes

## Goal

Restore the control panel UI and modal keybinding system from TermSurf 1.x,
adapted for CEF and WezTerm.

## Background

TermSurf 1.x has a modal system for browser panes:

- **Control mode**: Terminal keybindings work, browser is inactive
- **Browse mode**: Browser has focus, terminal keybindings intercepted
- **Insert mode**: URL field is editable

This provides a clear UX where users always know whether their keystrokes go to
the terminal or browser, with a guaranteed escape hatch (Ctrl+C).

## Requirements

### Modes

1. **Control Mode** (default when switching to browser pane)

   - Terminal keybindings work (pane navigation, splits, etc.)
   - Browser is visually dimmed
   - Keybindings:
     - `Enter` → browse mode
     - `i` → insert mode
     - `Ctrl+C` → close browser
   - Status bar: "i to edit, Enter to browse, Ctrl+C to close"

2. **Browse Mode**

   - Browser receives all input
   - Control bar is visually dimmed
   - `Ctrl+C` always exits to control mode (cannot be overridden by page)
   - Terminal keybindings still work (intercepted before browser)
   - Status bar: "Ctrl+C to control"

3. **Insert Mode**

   - URL field is editable, text selected
   - `Enter` → navigate to URL, switch to browse mode
   - `Esc` → cancel, restore URL, switch to control mode
   - Status bar: "Enter to go, Esc to cancel"

### Control Bar

A horizontal bar at the bottom of the browser pane:

```
┌─────────────────────────────────────────────────────────────┐
│ (1/2) https://example.com/path/to/page    Ctrl+C to control │
│ └─┬─┘ └──────────────┬──────────────────┘ └───────┬───────┘ │
│ stack        URL (truncates)                  mode hint     │
└─────────────────────────────────────────────────────────────┘
```

- **Stack indicator**: "(1/2)" when multiple browsers in pane (future feature)
- **URL field**: Current URL, truncates with ellipsis, editable in insert mode
- **Mode hint**: Context-sensitive help text

### Visual Feedback

- **Dim overlay**: Inactive area (browser or control bar) is dimmed
- Clicking dimmed area switches to that mode
- Opacity matches WezTerm's `inactive_pane_hsb` setting

## Technical Approach

### 1. Control Bar Rendering

The control bar is rendered as part of the browser overlay in
`paint_browser_overlay`. It consists of:

- Background: Solid color matching terminal background or config
- URL text: Left-aligned, monospace, truncates
- Mode hint: Right-aligned, secondary color
- Height: ~24-28 pixels

**Rendering approach**: Render as quads/text using WezTerm's existing text
rendering, or as a separate wgpu pass after the browser texture.

### 2. Mode State

Add mode tracking to `BrowserState`:

```rust
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BrowserMode {
    Control,
    Browse,
    Insert,
}

pub struct BrowserState {
    // ... existing fields ...
    mode: RefCell<BrowserMode>,
    url: RefCell<String>,
    edit_url: RefCell<String>,  // URL being edited in insert mode
}
```

### 3. Key Event Routing

In `keyevent.rs`, the CEF browser key handling section needs mode-aware logic:

```rust
#[cfg(all(target_os = "macos", feature = "cef"))]
{
    let pane_id = pane.pane_id();
    if let Some(browser) = self.browser_states.borrow().get(&pane_id) {
        match browser.get_mode() {
            BrowserMode::Control => {
                // Handle Enter, i, Ctrl+C
                // Other keys: check if terminal keybinding, else ignore
            }
            BrowserMode::Browse => {
                // Ctrl+C always exits to control mode
                // Check terminal keybindings first
                // Forward remaining keys to CEF
            }
            BrowserMode::Insert => {
                // Handle Enter (submit), Esc (cancel)
                // Forward text input to URL field state
            }
        }
    }
}
```

### 4. CEF Input Forwarding

In browse mode, forward key events to CEF:

```rust
impl BrowserState {
    pub fn forward_key_event(&self, event: &KeyEvent) {
        // Convert WezTerm KeyEvent to CEF KeyEvent
        // Send via browser.host().send_key_event()
    }
}
```

### 5. Dim Overlay

Render a semi-transparent overlay on the inactive area:

- **Control/Insert mode**: Dim the browser area
- **Browse mode**: Dim the control bar

The overlay is rendered after the browser texture but uses the same viewport
bounds system from MVP3.

### 6. Click Handling

Mouse clicks need mode-aware routing:

- Click on dimmed browser → switch to browse mode
- Click on dimmed control bar → switch to control mode
- Click on URL field (in control mode) → switch to insert mode

## Implementation Plan

### Phase 1: Mode State and Basic Switching

1. Add `BrowserMode` enum and state to `BrowserState`
2. Update key handling in `keyevent.rs` for mode switching
3. Add mode-based key routing (control: Enter/i/Ctrl+C, browse: Ctrl+C)

### Phase 2: Control Bar Rendering

1. Calculate control bar bounds (bottom of pane, fixed height)
2. Render control bar background
3. Render URL text (truncated)
4. Render mode hint text
5. Adjust browser viewport to exclude control bar height

### Phase 3: Visual Feedback

1. Render dim overlay on inactive area
2. Implement click detection for mode switching
3. Match opacity to WezTerm inactive pane settings

### Phase 4: Insert Mode

1. Track edit URL separately from actual URL
2. Handle text input in insert mode
3. Render cursor in URL field
4. Handle Enter (navigate) and Esc (cancel)

### Phase 5: CEF Input Forwarding

1. Convert WezTerm key events to CEF format
2. Forward mouse events to CEF
3. Handle focus/blur for CEF browser

## Key Differences from TS1

| Aspect          | TS1 (Swift/WKWebView)            | TS2 (Rust/CEF)                 |
| --------------- | -------------------------------- | ------------------------------ |
| Event monitor   | NSEvent.addLocalMonitorForEvents | WezTerm key_event_impl         |
| First responder | NSView responder chain           | WezTerm pane focus             |
| Text rendering  | NSTextField                      | WezTerm text rendering or wgpu |
| Dim overlay     | NSView with alpha                | wgpu quad with alpha           |
| URL editing     | Native text field                | Custom text input handling     |

## Non-Goals for MVP4

- Multiple browser stack (future feature)
- Full text editing (selection, copy/paste in URL field)
- Navigation controls (back/forward buttons)
- Tab completion for URLs

## Success Criteria

The implementation is complete when:

- [ ] Pressing Enter in control mode switches to browse mode
- [ ] Pressing i in control mode switches to insert mode
- [ ] Pressing Ctrl+C in control mode closes the browser
- [ ] Pressing Ctrl+C in browse mode switches to control mode
- [ ] Pressing Enter in insert mode navigates and switches to browse mode
- [ ] Pressing Esc in insert mode cancels and switches to control mode
- [ ] Control bar displays current URL and mode hint
- [ ] Inactive area is visually dimmed
- [ ] Clicking dimmed area switches modes
- [ ] Terminal keybindings work in control mode
- [ ] Browser receives input in browse mode
