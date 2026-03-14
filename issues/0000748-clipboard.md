# Issue 748: Browser clipboard (copy/cut/paste)

## Goal

Copy, cut, and paste work inside browser overlays in Wezboard. Cmd+C copies
selected text to the system clipboard. Cmd+V pastes from the system clipboard
into the browser. Cmd+X cuts. This should work on any web page — text fields,
content-editable regions, and selecting text on a read-only page.

## Background

Keyboard input is forwarded to Chromium via `ts_forward_key_event()` (Issue
728). When the user presses Cmd+C in browse mode, Wezboard sends a `KeyEvent`
to Roamium, which calls `ts_forward_key_event(wc, 0, VK_C, "c", CMD)`. Chromium
receives this and Blink processes it as an editing command.

However, clipboard operations fail silently. The most likely cause is that
Chromium's clipboard path goes through the `blink::mojom::ClipboardHost` Mojo
interface, and the Content Shell setup used by libtermsurf_chromium may not bind
this interface. This is the same class of problem identified in the Mojo
interface audit TODO item — the renderer crashes or silently fails when it calls
a Mojo API that the embedder hasn't registered.

### How Chromium clipboard works internally

1. Blink receives the Cmd+C editing command
2. Blink calls `ClipboardHost::WriteText()` via Mojo IPC (renderer → browser)
3. The browser process handles the Mojo call in `ClipboardHostImpl`
4. `ClipboardHostImpl` calls `ui::Clipboard::WriteText()`
5. On macOS, `ui::Clipboard` writes to `NSPasteboard`

For paste, the reverse: Blink calls `ClipboardHost::ReadText()`, the browser
reads from `NSPasteboard`, and sends the result back via Mojo.

### Prior art: CEF (Issue 206)

The old CEF-based architecture handled clipboard differently — it called
`frame.copy()` / `frame.paste()` / `frame.cut()` directly via FFI because CEF
ran in the same process. This isn't possible in TermSurf's multi-process
architecture where Roamium and Wezboard are separate processes communicating via
Unix sockets.

However, if the Mojo interface is properly bound, we may not need any protocol
changes at all. Chromium's browser process (running inside Roamium) has direct
access to `NSPasteboard`. The clipboard operations would be handled entirely
within the Roamium process — Blink sends clipboard data via Mojo to the browser
process, the browser process reads/writes `NSPasteboard`. No IPC to Wezboard
needed.

### Current protocol (no clipboard messages)

The TermSurf protocol has 31 message types. None are clipboard-related. If the
Mojo path works, no new messages are needed — clipboard stays internal to
Chromium/Roamium.

## Analysis

There are two possible failure modes:

### 1. Missing Mojo binding (most likely)

`ClipboardHostImpl` is registered in Chrome's `RenderFrameHostImpl` but may not
be bound in the Content Shell setup. If `ClipboardHost` isn't registered,
clipboard Mojo calls from the renderer are silently dropped. This would explain
why copy/paste fails without any crash.

**Fix:** Ensure `ClipboardHostImpl` is bound in the browser process. This may
already be handled by `content::RenderFrameHostImpl` (which does bind
`ClipboardHost` by default), in which case the problem is elsewhere.

### 2. Key event not triggering editing commands

`ts_forward_key_event()` injects events at the `WebInputEvent` level via
`RenderWidgetHost::ForwardKeyboardEvent()`. This should trigger Blink's editing
command handling, but there may be a missing step — Chrome normally processes
Cmd+C at the browser level (via accelerators) before it reaches the renderer.
If the editing command doesn't fire, no clipboard call happens at all.

**Fix:** Either ensure the key event reaches Blink's editing command handler, or
add a dedicated `ts_exec_command()` C API that calls
`web_contents->GetFocusedFrame()->ExecuteEditingCommand("copy")` directly.

### Proposed approach

1. **Diagnose first.** Add logging in Chromium to determine whether (a) the
   Cmd+C key event reaches Blink's editing command handler, and (b) the
   `ClipboardHost` Mojo interface is bound.

2. **Fix the Mojo binding** if that's the issue. This is the minimal fix — no
   protocol changes, no new C API functions.

3. **If key events don't trigger editing commands**, add `ts_exec_command()` to
   the C API and call it from Roamium when it receives a KeyEvent that matches
   Cmd+C/V/X. Alternatively, handle clipboard at the Wezboard level by
   intercepting Cmd+C/V/X before forwarding and using the system clipboard
   directly — but this would require new protocol messages
   (`ClipboardWrite`/`ClipboardRead`) and is more complex.

### Files likely involved

- `chromium/src/content/libtermsurf_chromium/libtermsurf_chromium.cc` — may need
  Mojo binding or new C API
- `chromium/src/content/libtermsurf_chromium/libtermsurf_chromium.h` — C API
  header if adding `ts_exec_command()`
- `roamium/src/dispatch.rs` — may need to intercept clipboard key events
- `roamium/src/ffi.rs` — FFI bindings if adding new C functions
