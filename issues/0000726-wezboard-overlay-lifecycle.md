# Issue 726: Wezboard overlay lifecycle and remaining protocol

## Goal

Make the browser overlay respond correctly to window and pane changes (resize,
splits, tab switching), then handle all remaining TermSurf protocol messages to
complete the Wezboard PoC.

## Background

Issue 725 solved overlay rendering: browser content is visible in the Wezboard
terminal window with correct size and position. But the overlay is static — it
doesn't respond to window resizes, split pane changes, or tab switches. And
Wezboard only handles 11 of 30 TermSurf protocol messages.

### Overlay lifecycle issues

Four overlay behaviors need to work:

1. **Window resize** — When the window resizes, the webview should resize with
   it. The metrics bridge (`metrics.rs`) updates on resize, but `conn.rs`
   doesn't re-read the metrics or call `update_ca_layer_frame()`. There's no
   notification path from TermWindow to the connection code.
2. **Split pane resize** — When opening or closing a split pane, the terminal
   pane shrinks or grows. The webview should resize to match the new pane
   dimensions.
3. **Tab switch away** — When opening a new tab or switching to a tab without a
   webview, the overlay should hide.
4. **Tab switch back** — When navigating back to a tab with an active webview,
   the overlay should reappear.

### Remaining protocol messages

Wezboard currently handles 11 of 30 TermSurf protocol messages. The remaining 19
fall into four categories:

**Input forwarding (4 messages):**

- `KeyEvent` — Keyboard input to browser
- `MouseEvent` — Mouse clicks to browser
- `MouseMove` — Mouse movement to browser
- `ScrollEvent` — Scroll wheel to browser

Without input forwarding, the browser overlay is view-only. This is the most
important missing piece after overlay lifecycle.

**Tab queries (6 messages):**

- `QueryLastRequest` / `QueryLastReply` — Get last active tab for session
  restore
- `QueryDevtoolsRequest` / `QueryDevtoolsReply` — Get DevTools tab
- `QueryTabsRequest` / `QueryTabsReply` — Get all tabs for a profile

**DevTools (2 messages):**

- `CreateDevtoolsTab` — Create DevTools tab
- `SetDevtoolsOverlay` — Create/resize DevTools overlay

**Other (3 messages):**

- `FocusChanged` — Tab focus state
- `CursorChanged` — Browser cursor type updates
- `OpenSplit` — Open split pane

**Already handled (11 messages):**

- `HelloRequest` / `HelloReply` — Handshake
- `ServerRegister` — Chromium process registration
- `SetOverlay` — Create/resize browser overlay
- `CreateTab` (sent, not received) — Create tab in browser
- `TabReady` — Tab initialized
- `CaContext` — CALayerHost context for compositing
- `Navigate` — URL navigation forwarding
- `UrlChanged` / `LoadingState` / `TitleChanged` — State forwarding to TUI
- `SetColorScheme` — Dark/light mode
- `ModeChanged` — Browse/edit mode toggle

### Priority order

1. Overlay lifecycle (this issue's primary focus)
2. Input forwarding (makes the browser usable)
3. Tab queries (session restore, DevTools discovery)
4. Auxiliary features (focus, cursor, DevTools, splits)

## Proposed solutions

### Overlay lifecycle

For **resize**, TermWindow could send a notification through the TermSurf shared
state or a channel whenever dimensions change. Alternatively, `conn.rs` could
poll the metrics atomics periodically — but that's wasteful. A better approach:
when `SetOverlay` arrives with updated dimensions, re-read metrics and update
the CALayer frame.

For **tab switching**, the overlay NSView or its sublayers need to be
shown/hidden based on which tab is active. The mux (WezTerm's tab/pane manager)
knows which pane is focused. When the focused pane changes, the board needs to
hide overlays for inactive panes and show overlays for the active pane.

### Input forwarding

The TUI already captures keyboard and mouse events and sends them as protobuf
messages. The board needs to receive these messages and forward them to the
correct Chromium process based on pane-to-tab mapping.

## Experiments

### Experiment 1: Hide overlay on tab switch

#### Background

ts3 (Issue 310) hit this exact bug: browser overlay from Tab A leaked into Tab
B. ts3 rendered browser content as IOSurface textures, so the fix was filtering
at render time — skip drawing overlays whose `tab_id != active_tab_id`. Wezboard
uses CALayerHost (zero-copy GPU compositing), so there's no render loop to
filter. Instead we toggle the `hidden` property on each pane's
`ca_layer_flipped` (the per-pane root in the overlay layer tree).

Ghostboard removes/adds layers on focus change, but it owns the surface
lifecycle in Zig. In Wezboard, the overlay code runs in async connection tasks
(`conn.rs`) on a different thread from TermWindow. Currently the only bridge is
`metrics.rs` global atomics — a write-only path from TermWindow to conn.rs.

To toggle layer visibility we need TermWindow to access the TermSurf shared
state (which holds all pane CALayer pointers). Currently `SharedState` is
created in `main.rs` and passed to the listener, but TermWindow has no access to
it. We fix this by making `SharedState` globally accessible via `OnceLock` — the
same global pattern as `metrics.rs` but for the full state. This also simplifies
the existing code (listener.rs and conn.rs can read the global instead of
threading state through function arguments).

Every tab switch in WezTerm flows through `Window::set_active_without_saving()`,
which fires `MuxNotification::WindowInvalidated`. TermWindow already handles
this at `mod.rs:1298`. We add a `sync_overlay_visibility()` call there that
reads the active mux pane ID and toggles `setHidden:` on each pane's
`ca_layer_flipped`.

#### Changes

**`wezboard/wezboard-gui/src/termsurf/state.rs`** — Make SharedState globally
accessible:

Add a `OnceLock` global and accessor functions:

```rust
use std::sync::OnceLock;

static GLOBAL_STATE: OnceLock<SharedState> = OnceLock::new();

pub fn init_global(state: SharedState) {
    GLOBAL_STATE.set(state).ok();
}

pub fn global() -> Option<&'static SharedState> {
    GLOBAL_STATE.get()
}
```

**`wezboard/wezboard-gui/src/termsurf/mod.rs`** — Re-export the global accessor:

```rust
pub use state::global as shared_state;
```

**`wezboard/wezboard-gui/src/main.rs`** — Initialize the global after creating
state (line 429):

```rust
let termsurf_state = Arc::new(std::sync::Mutex::new(termsurf::state::TermSurfState::new()));
termsurf::state::init_global(termsurf_state.clone());
```

**`wezboard/wezboard-gui/src/termsurf/conn.rs`** — Add
`sync_overlay_visibility`:

```rust
#[cfg(target_os = "macos")]
pub fn sync_overlay_visibility(active_pane_id: &str) {
    let Some(state) = super::shared_state() else {
        return;
    };
    let st = state.lock().unwrap();
    for (pane_id, pane) in &st.panes {
        if pane.ca_layer_flipped == 0 {
            continue;
        }
        let is_active = pane_id == active_pane_id;
        unsafe {
            use objc2::msg_send;
            use objc2::runtime::Bool;
            let layer = pane.ca_layer_flipped as *mut objc2::runtime::AnyObject;
            let hidden = if is_active { Bool::NO } else { Bool::YES };
            let _: () = msg_send![layer, setHidden: hidden];
        }
    }
}
```

Also call it at the end of `handle_ca_context()` (after `update_ca_layer_frame`
on line 761) to hide newly created layers if they belong to an inactive tab:

```rust
// After update_ca_layer_frame(pane, root_layer);
// Read the active pane from the caller's pane_id context — but we don't
// know the mux active pane here. Instead, just let the next
// WindowInvalidated sync handle it. The layer will flash briefly on
// creation but will be hidden on the next paint cycle.
```

Actually, `handle_ca_context` doesn't know the active mux pane. But
`WindowInvalidated` fires frequently (on every paint), so the flash will be
imperceptible. No change needed in `handle_ca_context`.

**`wezboard/wezboard-gui/src/termwindow/mod.rs`** — Call sync on
`WindowInvalidated` (line 1298):

The TermSurf `pane_id` is the mux pane ID as a string (WezTerm sets
`WEZBOARD_PANE` env var with the mux pane ID → TUI reads it → sends it as
`HelloRequest.pane_id`). So we get the active mux pane ID and convert to string:

```rust
MuxNotification::WindowInvalidated(_) => {
    window.invalidate();
    self.update_title_post_status();

    // Sync TermSurf overlay visibility with active pane
    if let Some(pane) = self.get_active_pane_or_overlay() {
        crate::termsurf::conn::sync_overlay_visibility(
            &pane.pane_id().to_string(),
        );
    }
}
```

#### Verification

1. `cd wezboard && cargo build -p wezboard-gui` — zero errors
2. Launch Wezboard, run `web google.com` in the first tab
3. Open a new tab (Cmd+T)
4. **Expected:** browser overlay disappears — new tab shows only terminal
5. Switch back to the first tab
6. **Expected:** browser overlay reappears
7. Open a third tab with another `web` instance
8. Switch between all three tabs — each shows only its own overlay (or no
   overlay)
