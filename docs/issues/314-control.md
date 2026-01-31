# Issue 314: Control Panel

## Goal

Implement a control panel for webview panes in ts3, matching the appearance and
behavior of ts2. The control panel displays the URL and provides visual feedback
about the browser mode.

## Background

### TermSurf 1.x (Ghostty + WKWebView)

The original control panel was implemented in Swift using NSTextField. It had a
modal system with Control mode (terminal keybindings) and Browse mode (browser
input).

### TermSurf 2.0 (WezTerm + in-process CEF)

ts2 ported the control panel to WezTerm's Rust/wgpu rendering. Key files:

- `ts2/wezterm-gui/src/cef_browser/mod.rs` — `BrowserState` and `BrowserMode`
- `ts2/wezterm-gui/src/termwindow/render/pane.rs` — Rendering logic
- `docs/issues/205-cef-mvp4.md` — Specification document

The ts2 control panel:

- **Position**: Top of the webview pane
- **Height**: 2 cell heights (half-cell top margin + text + half-cell bottom)
- **Margins**: Half-cell on left, top, and bottom
- **Content**: URL in Browse mode, instructions in Control mode

### TermSurf 3.0 (WezTerm + out-of-process CEF)

ts3 uses XPC to communicate with a separate profile server process. The URL is
not stored in-process — it must be received via XPC from termsurf-profile.

## Requirements

### Phase 1: Visual Appearance (This Issue)

The control panel should look exactly like ts2:

```
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │  ← half-cell top margin
│  https://google.com                                              │  ← URL text
│                                                                  │  ← half-cell bottom margin
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│                    Webview content here                          │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

**Dimensions:**

- Control bar height: `cell_height * 2.0` (exactly 2 cell heights)
- Left margin: `cell_width / 2.0`
- Top margin: `cell_height / 2.0`
- Bottom margin: `cell_height / 2.0`
- Right margin: 0 (text truncates with ellipsis)

**Colors:**

- Background: Terminal palette background
- Text: Terminal palette foreground

**Font:**

- Default monospace font (same as terminal)

### Phase 2: Mode Switching (Future)

After the control panel renders, add:

- Control mode / Browse mode switching
- `Enter` to enter Browse mode
- `Ctrl+C` to exit browser
- Visual dimming in Control mode

### Phase 3: Input Forwarding (Future)

- Forward keyboard input to CEF in Browse mode
- Forward mouse input to CEF in Browse mode

## Technical Approach

### Differences from ts2

| Aspect         | ts2                                             | ts3                                  |
| -------------- | ----------------------------------------------- | ------------------------------------ |
| CEF process    | In-process                                      | Out-of-process (XPC)                 |
| URL source     | `BrowserState.url` field                        | XPC message from profile             |
| Texture source | Direct CEF callback                             | XPC Mach port transfer               |
| State storage  | `browser_states: HashMap<PaneId, BrowserState>` | `WebviewOverlayState` + `XpcManager` |

### URL Transmission

Currently, ts3 transmits texture data (Mach port, dimensions) via XPC but not
the URL. Two options:

**Option A: Add URL to XPC protocol**

1. Profile server sends URL with each texture update
2. XPC manager stores URL alongside texture data
3. Render code retrieves URL from XPC manager

**Option B: Store URL locally from web command**

1. The `web` command already includes the URL
2. Store it in `WebviewOverlay` when the command is received
3. Use stored URL for display (won't update on navigation)

For Phase 1, Option B is simpler. Option A can be added later for navigation.

### Rendering Architecture

ts2 uses a two-phase rendering pattern to avoid wgpu buffer conflicts:

1. **Phase 1 (during `paint_pane`)**: Render control bar background via
   `filled_rectangle` while the layers buffer is mapped
2. **Phase 2 (after layers dropped)**: Render control bar text via
   `render_element` in a separate pass

ts3 doesn't use WezTerm's pane rendering for webviews — it renders directly in
`render_webview_overlays_webgpu`. We need to integrate control bar rendering
into this flow.

**Approach for ts3:**

1. In `render_webview_overlays_webgpu`, before rendering the webview texture:
   - Calculate control bar bounds (top 2 cell heights)
   - Render control bar background as a filled quad
   - Adjust webview viewport to start below control bar

2. After webview texture rendering:
   - Render control bar text using WezTerm's text rendering system

### Viewport Adjustment

Currently, the webview fills the entire pane. With the control bar, we need to:

1. Reserve 2 cell heights at the top for the control bar
2. Reduce the webview height by 2 cell heights
3. Send adjusted size to CEF via XPC resize command

```rust
// Current (fills entire pane)
let (viewport_x, viewport_y, viewport_w, viewport_h) = calculate_pane_bounds();

// With control bar
let control_bar_height = cell_height * 2.0;
let webview_y = viewport_y + control_bar_height;
let webview_h = viewport_h - control_bar_height;
```

## Implementation Plan

### Step 1: Store URL in WebviewOverlay

**File: `ts3/wezterm-gui/src/termwindow/webview_socket.rs`**

Add `url` field to `WebviewOverlay`:

```rust
pub struct WebviewOverlay {
    pub session_id: String,
    pub tab_id: TabId,
    pub url: String,  // NEW: URL for control bar display
}
```

Update `handle_open_webview` to store the URL when the command is received.

### Step 2: Calculate Control Bar Bounds

**File: `ts3/wezterm-gui/src/termwindow/render/draw.rs`**

In `render_webview_overlays_webgpu`, after calculating pane bounds:

```rust
// Control bar dimensions (matching ts2 exactly)
let cell_height = self.render_metrics.cell_size.height as f32;
let cell_width = self.render_metrics.cell_size.width as f32;
let control_bar_height = cell_height * 2.0;

// Control bar rect
let control_bar_x = viewport_x;
let control_bar_y = viewport_y;
let control_bar_w = viewport_w;
let control_bar_h = control_bar_height;

// Adjusted webview rect (below control bar)
let webview_y = viewport_y + control_bar_height;
let webview_h = viewport_h - control_bar_height;
```

### Step 3: Render Control Bar Background

Render a filled rectangle for the control bar background. This can be done:

**Option A: Separate render pass**

Create a simple quad and render it before the webview texture.

**Option B: Use WezTerm's filled_rectangle**

If we can access the layers buffer, use the existing infrastructure.

For Phase 1, Option A is likely simpler since we're already in a separate
rendering path.

### Step 4: Render Control Bar Text

Use WezTerm's `render_element` or similar text rendering to display the URL.

Key considerations:

- Text must be rendered AFTER the background
- May need separate command buffer submission
- Follow ts2's pattern from `paint_browser_control_bars`

### Step 5: Adjust CEF Viewport

Update the resize command sent to CEF to exclude control bar height:

```rust
// Send resize with reduced height
xpc_manager.send_resize(
    *pane_id,
    logical_w,
    logical_h - (control_bar_height / scale) as u32,
);
```

## Files to Modify

| File                                               | Changes                             |
| -------------------------------------------------- | ----------------------------------- |
| `ts3/wezterm-gui/src/termwindow/webview_socket.rs` | Add `url` field to `WebviewOverlay` |
| `ts3/wezterm-gui/src/termwindow/render/draw.rs`    | Add control bar rendering           |
| `ts3/wezterm-gui/src/termwindow/webview_xpc.rs`    | Adjust resize commands              |

## Verification

```bash
cd ts3 && ./scripts/build-debug.sh --open

# 1. Open a webview
web google.com

# 2. Verify control bar appears
# - Should see 2-cell-height bar at top of webview pane
# - Should display "https://google.com" with half-cell margins
# - Webview content should start below control bar

# 3. Test with long URL
web https://example.com/some/very/long/path/that/should/truncate

# 4. Test resize
# - Drag window edge
# - Control bar should resize with pane
# - No rendering artifacts

# 5. Test splits
split-pane
web github.com
# Each webview pane should have its own control bar
```

## Success Criteria

### Phase 1 (This Issue)

1. [ ] Control bar renders at top of webview pane
2. [ ] Control bar height is exactly 2 cell heights
3. [ ] Background uses terminal palette background color
4. [ ] URL text displays with half-cell margins (left, top, bottom)
5. [ ] URL truncates with ellipsis if too long
6. [ ] Webview content renders below control bar (not overlapping)
7. [ ] CEF receives correct viewport size (excluding control bar)
8. [ ] Control bar resizes correctly when pane resizes
9. [ ] Multiple webview panes each have their own control bar

### Phase 2 (Future)

- [ ] Control mode / Browse mode switching works
- [ ] Visual dimming in Control mode
- [ ] Click handling for mode switching

### Phase 3 (Future)

- [ ] Keyboard input forwarded to CEF in Browse mode
- [ ] Mouse input forwarded to CEF in Browse mode

## References

- `docs/issues/205-cef-mvp4.md` — ts2 control panel specification
- `ts2/wezterm-gui/src/termwindow/render/pane.rs:813-966` — ts2 rendering code
- `ts2/wezterm-gui/src/cef_browser/mod.rs:28-57` — ts2 BrowserState struct
