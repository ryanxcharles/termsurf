# CEF MVP3: Browser Fundamentals

This document details the implementation plan for MVP3 - making the CEF browser
fully interactive with control panel, keyboard/mouse input, resize handling, and
user profiles.

## Current State

**What Already Works (from MVP1 + MVP2):**

- CEF initialization and shutdown
- Browser creation via `web-open` command
- Browser texture rendering (accelerated OSR on macOS)
- Ctrl+C closes the browser and returns to terminal
- Basic resize handling (`was_resized()`)

**What's Missing:**

1. No control panel UI (URL bar, mode indicator)
2. No mode system (browse/control/insert)
3. No keyboard forwarding to CEF (keys are consumed but not sent)
4. No mouse/cursor interaction
5. No proper resize handling (browser doesn't resize with pane)
6. No user profile support

## Goal

A browser that behaves like TermSurf 1.x's WKWebView integration:

- Control panel with URL bar and mode indicator
- Three modes: browse, control, insert
- Full keyboard and mouse interactivity
- Proper resize behavior
- User profile isolation

## Implementation Checklist

### 1. Browser Mode System

Implement a modal system like TermSurf 1.x (see `docs/keybindings.md`).

#### 1.1 Define Browser Mode Enum

- [ ] Add `BrowserMode` enum to `cef_browser/mod.rs`:
  ```rust
  #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
  pub enum BrowserMode {
      #[default]
      Control,  // Terminal keybindings work, browser doesn't receive input
      Browse,   // Browser has focus, WezTerm keybindings still work
      Insert,   // URL field has focus for editing
  }
  ```

#### 1.2 Add Mode to BrowserState

- [ ] Add `mode: RefCell<BrowserMode>` field to `BrowserState`
- [ ] Add methods: `get_mode()`, `set_mode()`, `is_control_mode()`,
      `is_browse_mode()`, `is_insert_mode()`
- [ ] Emit focus events: `set_focus(true)` when entering browse mode,
      `set_focus(false)` when leaving

### 2. Control Panel UI

Render a control panel overlay at the top of the browser area.

#### 2.1 Control Panel Data Structure

- [ ] Create `cef_browser/control_panel.rs`:
  ```rust
  pub struct ControlPanel {
      pub url: String,
      pub mode: BrowserMode,
      pub editing_url: Option<String>,  // Some when in insert mode
  }
  ```

#### 2.2 Control Panel Rendering

- [ ] Add `paint_browser_control_panel()` method to render pass
- [ ] Render background bar (height ~24px, dark gray background)
- [ ] Render URL text on the left (monospace, truncate with ellipsis)
- [ ] Render mode hint on the right:
  - Control: "i to edit, Enter to browse, Ctrl+C to close"
  - Browse: "Ctrl+C to control"
  - Insert: "Enter to go, Esc to cancel"
- [ ] In insert mode, render URL in editable style (highlighted background)

#### 2.3 Control Panel Layout

- [ ] Offset browser texture rendering down by control panel height
- [ ] Update browser size to account for control panel height
- [ ] Ensure click coordinates are adjusted for control panel offset

### 3. Keyboard Interactivity

#### 3.1 Mode-Aware Key Handling

Update `keyevent.rs` to route keys based on browser mode:

- [ ] **Control Mode** - Handle special keys, forward others to terminal:
  - `Enter` → Switch to browse mode
  - `i` → Switch to insert mode
  - `Ctrl+C` → Close browser
  - All other keys → Forward to terminal (WezTerm keybindings work)
- [ ] **Browse Mode** - Forward most keys to browser:
  - `Ctrl+C` → Switch to control mode
  - Check WezTerm keybindings first (like ts1's `processKeyBindingIfMatched`)
  - Other keys → Forward to CEF browser
- [ ] **Insert Mode** - Handle URL editing:
  - `Enter` → Navigate to URL, switch to browse mode
  - `Esc` → Cancel, revert URL, switch to control mode
  - Other keys → Update URL string (basic text editing)

#### 3.2 CEF Key Event Conversion

Improve the key event conversion in `cef_browser/mod.rs`:

- [ ] Extend `keycode_to_windows_vk()` mapping for all common keys
- [ ] Handle character input properly (`character`, `unmodified_character`)
- [ ] Map modifiers correctly:
  - Shift → `EVENTFLAG_SHIFT_DOWN`
  - Ctrl → `EVENTFLAG_CONTROL_DOWN`
  - Alt/Option → `EVENTFLAG_ALT_DOWN`
  - Cmd → `EVENTFLAG_COMMAND_DOWN`
- [ ] Send proper key event sequence:
  - KeyDown (`KEYEVENT_RAWKEYDOWN`)
  - Char (`KEYEVENT_CHAR`) - for printable characters
  - KeyUp (`KEYEVENT_KEYUP`)

#### 3.3 Text Input for Insert Mode

- [ ] Track cursor position in URL string
- [ ] Handle character insertion
- [ ] Handle backspace/delete
- [ ] Handle arrow keys for cursor movement
- [ ] Handle Cmd+A (select all), Cmd+C/V (copy/paste)

### 4. Mouse/Cursor Interactivity

#### 4.1 Mouse Event Routing

- [ ] In `mouseevent.rs`, check if click is in browser pane area
- [ ] Adjust coordinates for browser position within pane
- [ ] Adjust coordinates for control panel offset
- [ ] Route mouse events to `BrowserState::send_mouse_event()`

#### 4.2 CEF Mouse Event Methods

Add methods to `BrowserState`:

- [ ] `send_mouse_move(x, y, modifiers)` - Mouse movement
- [ ] `send_mouse_down(x, y, button, modifiers, click_count)` - Button press
- [ ] `send_mouse_up(x, y, button, modifiers)` - Button release
- [ ] `send_mouse_wheel(x, y, delta_x, delta_y, modifiers)` - Scroll events

#### 4.3 Mouse Event Conversion

- [ ] Map WezTerm mouse buttons to CEF `MouseButtonType`:
  - Left → `MBT_LEFT`
  - Right → `MBT_RIGHT`
  - Middle → `MBT_MIDDLE`
- [ ] Create `MouseEvent` struct with proper fields:
  - `x`, `y` - coordinates
  - `modifiers` - modifier flags

#### 4.4 Cursor Updates

- [ ] Implement CEF's `on_cursor_change` in render handler
- [ ] Map CEF cursor types to WezTerm cursor types
- [ ] Update window cursor when hovering over browser

### 5. Browser Resize

#### 5.1 Window Resize Handling

- [ ] In `TermWindow::apply_scale_change()` or resize handler, detect pane size
      changes
- [ ] For panes with browsers, call
      `BrowserState::resize(new_width, new_height)`
- [ ] Account for control panel height when calculating browser height

#### 5.2 Pane Layout Changes

- [ ] When splits are added/removed, recalculate browser sizes
- [ ] When tab changes, pause/resume browser if needed
- [ ] Handle minimized window state

#### 5.3 HiDPI/Retina Support

- [ ] Pass correct `device_scale_factor` to CEF
- [ ] Scale browser dimensions by DPI factor
- [ ] Ensure texture rendering accounts for scale

### 6. User Profiles

User profiles provide session isolation (cookies, localStorage, cache).

#### 6.1 Profile Configuration

- [ ] Define profile configuration structure:
  ```rust
  pub struct BrowserProfile {
      pub name: String,
      pub cache_path: Option<PathBuf>,
  }
  ```
- [ ] Store profiles in `~/.config/wezterm/browser-profiles/` or similar

#### 6.2 Request Context Per Profile

- [ ] Modify `BrowserState::new()` to accept optional profile name
- [ ] Create `RequestContextSettings` with profile-specific `cache_path`:
  ```rust
  let settings = RequestContextSettings {
      cache_path: profile.cache_path.unwrap_or_else(|| {
          dirs::cache_dir()
              .unwrap_or_default()
              .join("termsurf")
              .join("profiles")
              .join(&profile.name)
      }).into(),
      ..Default::default()
  };
  ```
- [ ] Store request context per profile (reuse across browsers with same
      profile)

#### 6.3 CLI Integration

- [ ] Add `--profile <name>` flag to `web-open` command
- [ ] Pass profile to `WebOpen` PDU and `MuxNotification::WebOpen`
- [ ] Default profile: "default"

#### 6.4 Profile Management

- [ ] List profiles: `wezterm cli web-profile list`
- [ ] Clear profile data: `wezterm cli web-profile clear <name>`
- [ ] Delete profile: `wezterm cli web-profile delete <name>`

## File Summary

| File                                           | Changes                                       |
| ---------------------------------------------- | --------------------------------------------- |
| `wezterm-gui/src/cef_browser/mod.rs`           | Add BrowserMode, mouse events, key conversion |
| `wezterm-gui/src/cef_browser/control_panel.rs` | New file - control panel data structure       |
| `wezterm-gui/src/termwindow/keyevent.rs`       | Mode-aware key routing                        |
| `wezterm-gui/src/termwindow/mouseevent.rs`     | Mouse routing to browser                      |
| `wezterm-gui/src/termwindow/render/draw.rs`    | Control panel rendering                       |
| `wezterm-gui/src/termwindow/render/pane.rs`    | Resize handling, layout adjustments           |
| `wezterm-gui/src/termwindow/mod.rs`            | Profile storage, resize coordination          |
| `mux/src/lib.rs`                               | Add profile to WebOpen notification           |
| `codec/src/lib.rs`                             | Add profile to WebOpen PDU                    |
| `wezterm/src/cli/web_open.rs`                  | Add --profile flag                            |

## Testing Plan

### Manual Testing

1. **Modes:**
   - Open browser, verify starts in control mode
   - Press Enter → verify browse mode (can click links)
   - Press Ctrl+C → verify returns to control mode
   - Press i → verify insert mode (URL editable)
   - Press Esc → verify returns to control mode
   - Press Enter after editing URL → verify navigation

2. **Keyboard:**
   - In browse mode, type in a search box
   - Verify Tab moves between form fields
   - Verify Ctrl+C always escapes to control mode
   - Verify WezTerm keybindings (splits, tabs) work in control mode

3. **Mouse:**
   - Click links in browse mode
   - Right-click for context menu (if implemented)
   - Scroll with mouse wheel
   - Verify hover cursor changes

4. **Resize:**
   - Resize window, verify browser resizes
   - Split pane, verify browser adjusts
   - Close split, verify browser expands

5. **Profiles:**
   - Open browser with `--profile work`
   - Log into a site
   - Close browser
   - Open browser with `--profile personal`
   - Verify not logged in
   - Open browser with `--profile work`
   - Verify still logged in

### Automated Tests

- [ ] Unit tests for key event conversion
- [ ] Unit tests for mouse coordinate transformation
- [ ] Unit tests for URL normalization
- [ ] Integration test: mode state transitions

## Implementation Order

Recommended order to minimize risk and enable incremental testing:

1. **Browser Mode System** (1.1, 1.2) - Foundation for everything else
2. **Basic Key Routing** (3.1) - Makes browser usable with keyboard
3. **CEF Key Events** (3.2) - Actually sends keys to browser
4. **Mouse Events** (4.1-4.3) - Makes browser clickable
5. **Control Panel UI** (2.1-2.3) - Visual feedback for mode
6. **Resize Handling** (5.1-5.3) - Proper layout
7. **Insert Mode Text Input** (3.3) - URL editing
8. **User Profiles** (6.1-6.4) - Session isolation

## Dependencies

- MVP1 complete (CEF loads and initializes)
- MVP2 complete (browser renders and closes)
- `cef-rs` at `../../cef-rs` with mouse/keyboard APIs

## References

- [docs/keybindings.md](keybindings.md) - TermSurf 1.x keybinding architecture
- [docs/cef-mvp.md](cef-mvp.md) - MVP1 execution log
- [docs/cef-mvp2.md](cef-mvp2.md) - MVP2 execution log
- [cef-rs documentation](../cef-rs/README.md) - CEF Rust bindings
