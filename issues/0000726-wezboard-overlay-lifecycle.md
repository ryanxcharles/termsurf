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

The function takes a `HashSet<String>` of all active pane IDs across all
windows. A pane is shown if its ID is in the set, hidden otherwise. This
correctly handles multiple windows — each window contributes its active pane to
the set.

```rust
use std::collections::HashSet;

#[cfg(target_os = "macos")]
pub fn sync_overlay_visibility(active_pane_ids: &HashSet<String>) {
    let Some(state) = super::shared_state() else {
        return;
    };
    let st = state.lock().unwrap();
    for (pane_id, pane) in &st.panes {
        if pane.ca_layer_flipped == 0 {
            continue;
        }
        let is_active = active_pane_ids.contains(pane_id);
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

**`wezboard/wezboard-gui/src/termwindow/mod.rs`** — Call sync on
`WindowInvalidated` (line 1298):

On every `WindowInvalidated`, iterate all mux windows and collect each window's
active pane ID into a `HashSet`. The TermSurf `pane_id` is the mux pane ID as a
string (WezTerm sets `WEZBOARD_PANE` env var → TUI reads it → sends as
`HelloRequest.pane_id`).

```rust
MuxNotification::WindowInvalidated(_) => {
    window.invalidate();
    self.update_title_post_status();

    // Gather active pane IDs across all windows
    let mux = Mux::get();
    let mut active_ids = std::collections::HashSet::new();
    for window_id in mux.iter_windows() {
        if let Some(w) = mux.get_window(window_id) {
            if let Some(tab) = w.get_active() {
                if let Some(pane) = tab.get_active_pane() {
                    active_ids.insert(pane.pane_id().to_string());
                }
            }
        }
    }
    crate::termsurf::conn::sync_overlay_visibility(&active_ids);
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
9. Open a second window with a webview — both windows' overlays visible
   simultaneously

**Result:** Fail

Switching to a new tab correctly hides the overlay. But switching back to the
tab with the webview does not restore it — the overlay stays hidden.

The hide works because `sync_overlay_visibility` sets `setHidden:YES` on every
pane whose `pane_id` is not in the active set. The show fails because
`active_pane_ids.contains(pane_id)` returns false even when the tab is active.

The most likely cause is a **pane_id mismatch**. The TermSurf state stores panes
keyed by the string the TUI sends in `HelloRequest.pane_id`. The
`WindowInvalidated` handler builds the active set from
`pane.pane_id().to_string()` (the mux's `PaneId` as a string). If these two
strings don't match, the pane is never recognized as active.

In Ghostboard, `TERMSURF_PANE_ID` is set to the surface's pane ID. In Wezboard,
`WEZBOARD_PANE` is set to the mux pane ID at `mux/src/domain.rs:482` — but the
TUI reads `TERMSURF_PANE_ID`, not `WEZBOARD_PANE`. If `TERMSURF_PANE_ID` isn't
set, the TUI may send a different value (or empty string) that doesn't match the
mux pane ID.

#### Conclusion

Research confirmed the root cause: Wezboard never sets `TERMSURF_PANE_ID`. The
TUI reads this env var to get its pane identity — without it, either the TUI
doesn't connect at all, or it sends a pane_id that doesn't match the mux pane
ID. Either way, `sync_overlay_visibility` can never match the TermSurf pane keys
against the mux active pane set.

### Experiment 2: Set TERMSURF_PANE_ID in Wezboard

#### Background

Ghostboard sets `TERMSURF_PANE_ID` at `Surface.zig:662` when spawning child
processes. The TUI reads it at `webtui/src/main.rs:223` to identify itself to
the board. Without this env var, the TUI either cannot connect or sends an
unrecognized pane_id.

Wezboard already sets `WEZBOARD_PANE` to the mux pane ID at
`mux/src/domain.rs:482`. We just need to also set `TERMSURF_PANE_ID` to the same
value. This ensures:

1. The TUI connects and sends `HelloRequest.pane_id` matching the mux pane ID
2. The TermSurf state stores panes keyed by the mux pane ID string
3. `sync_overlay_visibility` can match these keys against the active pane set

**Hypothesis:** This single-line fix will make Experiment 1's tab switching
logic work — overlays will hide on tab switch away and reappear on switch back.

#### Changes

**`wezboard/mux/src/domain.rs`** — Add `TERMSURF_PANE_ID` after line 482:

```rust
cmd.env("WEZBOARD_PANE", pane_id.to_string());
cmd.env("TERMSURF_PANE_ID", pane_id.to_string());
```

#### Verification

1. `cd wezboard && cargo build -p wezboard-gui` — zero errors
2. Launch Wezboard, run `web google.com` in the first tab
3. Open a new tab (Cmd+T)
4. **Expected:** browser overlay disappears
5. Switch back to the first tab
6. **Expected:** browser overlay reappears

**Result:** Pass

Overlays hide on tab switch away and reappear on switch back. The missing
`TERMSURF_PANE_ID` env var was the root cause of Experiment 1's failure —
without it, the TUI couldn't identify itself to the board, so the TermSurf pane
keys never matched the mux active pane set.

#### Conclusion

Setting `TERMSURF_PANE_ID` alongside `WEZBOARD_PANE` in `domain.rs` completes
the pane identity bridge between WezTerm's mux and the TermSurf protocol.
Combined with Experiment 1's `sync_overlay_visibility` (called from the
`WindowInvalidated` handler), tab switching now correctly hides and shows
browser overlays.

### Experiment 3: Fix overlay visibility for splits and multiple windows

#### Background

Experiments 1–2 solved tab switching: overlays hide when switching to a tab
without a webview and reappear when switching back. But opening a second pane
(split or new window) with a webview exposes a bug — the second overlay flashes
briefly then disappears, while the first remains visible.

The root cause is in the `WindowInvalidated` handler at `mod.rs:1298`. It builds
the active pane set using `tab.get_active_pane()`, which returns only the single
**focused** pane per tab. In a split layout, both panes are visible on screen,
but only one is focused. The non-focused pane's overlay gets hidden by
`sync_overlay_visibility`.

The flash occurs because:

1. The second TUI connects and sends `SetOverlay` → CALayer created → overlay
   visible
2. `WindowInvalidated` fires (triggered by the new pane or focus change)
3. `sync_overlay_visibility` runs — the focused pane is still the first one, so
   the second pane's overlay is hidden

The fix is to collect **all panes in each window's active tab**, not just the
focused pane. WezTerm's `Tab::iter_panes()` returns a `Vec<PositionedPane>` with
every pane in the tab's split tree. Each `PositionedPane` has a `.pane` field
(`Arc<dyn Pane>`) with a `.pane_id()` method.

This correctly handles all cases:

- **Single pane**: one pane in the active tab → its overlay is visible
- **Split panes**: all panes in the active tab → all overlays visible
- **Tab switch**: panes on inactive tabs are not iterated → their overlays
  hidden
- **Multiple windows**: each window contributes all panes from its active tab

#### Changes

**`wezboard/wezboard-gui/src/termwindow/mod.rs`** — In the `WindowInvalidated`
handler (~line 1298), replace `get_active_pane()` with `iter_panes()`:

```rust
// Before (only the focused pane):
if let Some(tab) = w.get_active() {
    if let Some(pane) = tab.get_active_pane() {
        active_ids.insert(pane.pane_id().to_string());
    }
}

// After (all panes in the active tab):
if let Some(tab) = w.get_active() {
    for positioned in tab.iter_panes() {
        active_ids.insert(positioned.pane.pane_id().to_string());
    }
}
```

#### Verification

1. `cd wezboard && cargo build -p wezboard-gui` — zero errors
2. Launch Wezboard, run `web google.com` in the first pane
3. Open a split pane, run `web google.com` again
4. **Expected:** both overlays visible simultaneously
5. Switch to a new tab (Cmd+T)
6. **Expected:** both overlays disappear
7. Switch back
8. **Expected:** both overlays reappear
9. Open a second window, run `web google.com`
10. **Expected:** all three overlays visible across both windows

**Result:** Fail

The second pane's webview does not appear. The first webview remains visible
after a brief flash. Behavior is identical to before the change — replacing
`get_active_pane()` with `iter_panes()` had no effect.

This means the problem is not about which pane IDs are in the active set. The
second pane IS in the set (since `iter_panes()` returns all panes in the active
tab), yet its overlay still disappears. The root cause must be elsewhere —
likely in the overlay creation or CALayer setup path for the second pane, not in
the visibility sync logic.

#### Conclusion

The `get_active_pane()` vs `iter_panes()` distinction was not the cause. The
second overlay flashes (briefly appears then vanishes) regardless of which pane
IDs are in the active set. The bug is upstream of `sync_overlay_visibility` —
something in the overlay creation, CALayer tree setup, or the interaction
between two concurrent pane connections is preventing the second overlay from
persisting.

### Experiment 4: Debug logs for second pane overlay lifecycle

#### Background

Experiment 3 proved the bug is not in `sync_overlay_visibility`. The second
overlay flashes (appears briefly then vanishes) regardless of which pane IDs are
in the active set. Static analysis of the code shows the overlay creation path
_should_ work for multiple panes — each gets its own 3-layer hierarchy under a
shared root layer. But something is wrong at runtime.

Three hypotheses remain:

1. **The second `CaContext` never arrives.** If `TabReady` for the second tab is
   delayed or lost, `handle_ca_context` logs `"unknown tab_id"` and skips layer
   creation. The "flash" could be the first overlay flickering during a
   `WindowInvalidated` redraw, not the second overlay appearing at all.

2. **The second `SetOverlay` hits the resize path instead of the new-pane
   path.** If somehow the second pane's `pane_id` collides with an existing key
   in `state.panes`, line 248 (`!st.panes.contains_key`) returns false and it
   takes the resize-only path, never creating a tab.

3. **`sync_overlay_visibility` runs between layer creation and the
   `CATransaction` commit.** The second pane's `ca_layer_flipped` is set at line
   734 but the transaction doesn't commit until line 764. If `WindowInvalidated`
   fires on the main thread during this window, sync could set `hidden: YES` on
   the new layer before it becomes visible.

This experiment adds targeted debug logs at every critical point in the second
pane's lifecycle to determine which hypothesis (or what other cause) is correct.

#### Changes

**`wezboard/wezboard-gui/src/termsurf/conn.rs`** — Add logs at these points:

1. **`handle_set_overlay` entry** (~line 229): Log whether the pane is new or
   existing, and the pane_id:

   ```rust
   log::info!(
       "SetOverlay: pane_id={} is_new={} pixel={}x{}",
       overlay.pane_id, is_new, pixel_w, pixel_h
   );
   ```

2. **`handle_tab_ready`** (~line 378): Log the tab_to_pane mapping:

   ```rust
   log::info!(
       "TabReady: tab_id={} pane_id={} tab_to_pane_count={}",
       ready.tab_id, ready.pane_id, st.tab_to_pane.len()
   );
   ```

3. **`handle_ca_context` pane lookup** (~line 672): Log whether the pane was
   found and its current layer state:

   ```rust
   log::info!(
       "handle_ca_context: tab_id={} pane_id={} has_layers={}",
       ca_context.tab_id, pane_id, pane.ca_layer_host != 0
   );
   ```

4. **`handle_ca_context` after layer creation** (~line 738): Log the layer
   pointers:

   ```rust
   log::info!(
       "CALayerHost created: pane_id={} flipped={:#x} host={:#x}",
       pane_id, pane.ca_layer_flipped, pane.ca_layer_host
   );
   ```

5. **`sync_overlay_visibility`** (~line 818): Log what's being shown/hidden:

   ```rust
   log::info!(
       "sync_overlay_visibility: active_ids={:?} pane_count={}",
       active_pane_ids, st.panes.len()
   );
   ```

   And inside the loop:

   ```rust
   log::info!(
       "  pane_id={} is_active={} ca_layer_flipped={:#x}",
       pane_id, is_active, pane.ca_layer_flipped
   );
   ```

#### Verification

1. `cd wezboard && cargo build -p wezboard-gui` — zero errors
2. Launch Wezboard with logs: redirect stderr to
   `~/dev/termsurf/logs/wezboard.log`
3. Run `web google.com` in the first pane — confirm overlay appears
4. Open a split pane, run `web google.com` again
5. Check the log for the second pane's message sequence:
   - Does `SetOverlay` arrive with `is_new=true`?
   - Does `TabReady` arrive with the correct pane_id?
   - Does `handle_ca_context` find the pane and create layers?
   - Does `sync_overlay_visibility` hide the second pane's layer?
6. The log output will identify the exact failure point

**Result:** Partial

The debug logs revealed that the board-side overlay code works correctly for
multiple panes. The second pane's full lifecycle completes successfully:

```
SetOverlay: pane_id=1 is_new=true pixel=1014x1980
sent CreateTab: pane_id=1 url=https://ryanxcharles.com
TabReady: pane_id=1 tab_id=2 tab_to_pane_count=2
CaContext: tab_id=2 context_id=2345823755
handle_ca_context: tab_id=2 pane_id=1 has_layers=false
CALayerHost created: pane_id=1 contextId=2345823755 flipped=0xbbf61c000 host=0xbbf4d99c0
```

The overlay IS created and briefly visible. Then ~2 seconds later, the TUI
disconnects:

```
TermSurf client disconnected (Tui)
removed pane 1 on TUI disconnect
```

The "flash" is not a visibility sync issue — it's the overlay appearing, the TUI
process dying, and `handle_disconnect` cleaning up the pane and its CALayers.

Further testing revealed the problem is even broader: closing the first TUI and
reopening a new one also fails. This is not a concurrency issue — it's a state
issue. Any TUI after the first one crashes.

The root cause is that Wezboard only implements 11 of 30 TermSurf protocol
messages. The TUI sends synchronous query messages that expect replies, but
Wezboard silently drops them (line 177-178 of `handle_message`). The TUI blocks
on `recv_reply()` with a 5-second timeout, then crashes.

**Missing message handlers that cause TUI crashes (synchronous, need replies):**

| Message                | What Ghostboard does                                              | Wezboard |
| ---------------------- | ----------------------------------------------------------------- | -------- |
| `QueryLastRequest`     | Finds last browser pane by profile, replies with `QueryLastReply` | Missing  |
| `QueryDevtoolsRequest` | Validates DevTools tab, replies with `QueryDevtoolsReply`         | Missing  |
| `QueryTabsRequest`     | Counts panes/tabs, replies with `QueryTabsReply`                  | Missing  |

**Missing message handlers (fire-and-forget, won't crash but incomplete):**

| Message              | What Ghostboard does             | Wezboard |
| -------------------- | -------------------------------- | -------- |
| `SetDevtoolsOverlay` | Creates/resizes DevTools overlay | Missing  |
| `OpenSplit`          | Opens a split pane               | Missing  |
| `CursorChanged`      | Updates mouse cursor type        | Missing  |

**Not needed yet (input forwarding — both boards missing):**

| Message       | Status              |
| ------------- | ------------------- |
| `KeyEvent`    | Both boards missing |
| `MouseEvent`  | Both boards missing |
| `MouseMove`   | Both boards missing |
| `ScrollEvent` | Both boards missing |

Ghostboard handles 18 of 30 messages (60%). Wezboard handles 11 (37%). The three
missing query handlers are the immediate cause of the TUI crash — the TUI sends
a synchronous request, Wezboard drops it, the TUI blocks waiting for a reply
that never comes.

#### Conclusion

The debug logs disproved the original hypotheses about CALayer setup or
visibility sync. The overlay creation path works correctly for multiple panes.
The actual bug is that the TUI process dies because Wezboard doesn't implement
the full TermSurf protocol — specifically the three query request/reply pairs
(`QueryLastRequest`, `QueryDevtoolsRequest`, `QueryTabsRequest`). Implementing
the remaining protocol messages will fix the multi-pane issue and complete the
Wezboard PoC.
