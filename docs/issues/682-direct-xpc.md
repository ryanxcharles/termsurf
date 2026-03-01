# Issue 682: Direct TUI → Chromium XPC Connection

The TUI (`web`) sends several messages to Chromium that the GUI simply relays
without touching. Adding a direct XPC connection from the TUI to the Chromium
profile server would eliminate these relay hops and simplify the architecture.

## Background

### Current Architecture

All XPC flows through the GUI as a hub:

```
TUI ←→ GUI ←→ Chromium
```

The GUI dispatches TUI messages in `xpc.zig` and Chromium messages in the same
file. Some messages require the GUI (they touch GUI state like the renderer,
focus, or input coordinates). Others are pure pass-through — the GUI reads
fields, rebuilds the message, and forwards it unchanged.

### Pass-Through Messages

**TUI → GUI → Chromium** (GUI just relays):

| Message            | What GUI does                              |
| ------------------ | ------------------------------------------ |
| `navigate`         | Reads pane_id/url, rebuilds, forwards      |
| `set_color_scheme` | Reads scheme, resolves dark bool, forwards |

**Chromium → GUI → TUI** (GUI just relays):

| Message         | What GUI does                        |
| --------------- | ------------------------------------ |
| `url_changed`   | Reads url, rebuilds, forwards to TUI |
| `loading_state` | Reads state/progress, forwards       |
| `title_changed` | Reads title, forwards                |
| `mode_changed`  | Reads browsing, forwards             |

That's 6 relay hops that add latency and code for no functional reason.

### Messages That Need the GUI

These messages genuinely touch GUI state and must stay in the GUI:

| Message          | Why it needs GUI                                       |
| ---------------- | ------------------------------------------------------ |
| `create_tab`     | GUI creates the pane, manages server lifecycle         |
| `resize`         | GUI calculates pixel dimensions from cell grid         |
| `focus_changed`  | GUI tracks focus across surfaces                       |
| `mouse_event`    | GUI translates surface coordinates to overlay-relative |
| `scroll_event`   | GUI translates coordinates, reads NSEvent fields       |
| `mouse_move`     | GUI translates coordinates                             |
| `key_event`      | GUI translates key codes, handles Cmd bypass           |
| `ca_context`     | GUI creates CALayerHost in the renderer                |
| `cursor_changed` | GUI sets the NSCursor on the window                    |
| `set_overlay`    | GUI positions the overlay in the surface               |

### Proposed Architecture

```
TUI ←→ GUI ←→ Chromium
 ↑                ↑
 └────────────────┘
    direct link
```

The TUI connects to Chromium directly for messages that don't need the GUI. The
GUI connection remains for everything that touches rendering, input, or pane
lifecycle.

### Open Questions

1. **How does the TUI discover the Chromium server?** Currently, the GUI manages
   server lifecycle — it launches Chromium, receives `server_register`, and
   stores the server's peer connection. The TUI would need either:
   - The GUI to forward the server's XPC endpoint to the TUI
   - The TUI to connect to the same gateway and receive the endpoint directly
   - A new mach service name for the Chromium server

2. **Pane ID mapping.** The TUI knows its `TERMSURF_PANE_ID` but Chromium uses
   the same pane ID. As long as both use the same ID, routing works. But the GUI
   currently creates the tab (via `create_tab`) before the TUI can talk to
   Chromium. The TUI would need to wait for the tab to exist.

3. **`set_color_scheme` resolution.** The TUI currently sends `scheme` as a
   string (`"dark"`, `"light"`, `"system"`). The GUI resolves `"system"` by
   reading the surface's `config_conditional_state.theme`. If the TUI sends
   directly to Chromium, it would need to resolve `"system"` itself — either by
   querying the GUI first, or by receiving system theme notifications directly.

4. **Message ordering.** If `navigate` goes direct but `create_tab` goes through
   the GUI, there's a race: the TUI might send `navigate` before the GUI's
   `create_tab` arrives. The TUI would need to know the tab is ready before
   sending direct messages.

5. **Is it worth the complexity?** The relay adds microseconds of latency. The
   code savings are modest (removing ~60 lines of forwarding in xpc.zig). The
   architectural benefit is cleaner separation of concerns, but the cost is a
   second XPC connection to manage and new synchronization requirements.

## Experiment 1: Research and decide

### Hypothesis

Analyzing the trade-offs in detail will reveal whether a direct connection is
worth the added complexity, or whether the relay pattern is good enough.

### Plan

1. Measure the actual relay latency (timestamp before TUI send, after Chromium
   receive) to quantify the cost
2. Prototype the simplest direct path: GUI forwards the Chromium server's XPC
   endpoint to the TUI after `server_register`
3. Evaluate whether the synchronization complexity (tab readiness, system theme
   resolution) outweighs the relay elimination
4. Decide: direct connection, hybrid (some direct, some relayed), or keep
   current architecture
