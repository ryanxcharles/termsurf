# Issue 513: Ctrl+Esc and Window-Side Mode Tracking

## Background

The `web` TUI has two modes: Browse and Control. Pressing Esc switches from
Browse to Control mode. The status bar displays
`[ctrl+esc] force exit browse
mode` as a hint — the intent was always for
Ctrl+Esc to be the primary mode switch, with bare Esc as a temporary
convenience.

## Problem

Ctrl+Esc works in WezTerm but not in TermSurf (Ghostty fork). Bare Esc works in
both. The `web` code hasn't changed — the issue is in how the terminal encodes
Ctrl+Esc and how crossterm parses it.

### What Ghostty sends

Ghostty's `function_keys.zig` (line 226) encodes Ctrl+Escape as:

```
\x1b[27;5;27~
```

This is the xterm "modify-other-keys" format (`CSI 27 ; modifier ; keycode ~`).
The entry has `modify_other_keys: .any`, meaning Ghostty sends this sequence
regardless of whether the application has requested modify-other-keys mode.

Bare Escape sends `\x1b` — a single byte that crossterm handles fine.

### What crossterm expects

The `web` TUI uses crossterm 0.28.1 **without enabling keyboard enhancement
flags**. Without `PushKeyboardEnhancementFlags`, crossterm's legacy parser
handles standard CSI sequences (arrows, function keys, etc.) but does not
recognize the `CSI 27 ; 5 ; 27 ~` format — key number 27 is not in the standard
function key table. The sequence is either silently dropped or misinterpreted.

### Why WezTerm works

WezTerm likely sends a different encoding for Ctrl+Esc — either the same bare
`\x1b` as unmodified Escape, or a sequence that crossterm's legacy parser
recognizes. The exact encoding WezTerm uses has not been verified.

## Architectural decision: window handles input in browse mode

The original Options section proposed fixing this inside the `web` TUI (enabling
the kitty keyboard protocol, parsing raw sequences, etc.). But a broader
analysis of the input forwarding architecture changes the picture.

### Why the window must handle keyboard in browse mode

When browser input forwarding is implemented, the window (TermSurf) will need to
forward keypresses to the Chromium Profile Server. The window has access to
`NSEvent`, which provides:

- KeyDown and KeyUp events (terminals only signal "key pressed")
- Left Shift vs Right Shift distinction
- Key repeat vs separate presses
- IME composition sequences
- Dead keys for accented characters

The terminal PTY is a lossy channel — it cannot faithfully transmit the full
range of keyboard events that browsers need. Issue 513 itself is proof: Ctrl+Esc
doesn't survive the Ghostty → PTY → crossterm encoding.

Mouse input must also go through the window, because the terminal lacks sub-cell
pixel coordinates needed for fine-grained mouse control. Since both keyboard and
mouse must go through the window in browse mode, the window is the single input
authority for browser interaction.

### Mode must be shared

Both the window and the `web` TUI need to know the current mode:

- **The window** needs the mode to decide whether to forward keypresses to the
  Chromium Profile Server (browse mode) or let them pass through to the terminal
  (control mode).
- **The `web` TUI** needs the mode to render the correct UI state (border
  colors, status bar hints, URL bar focus).

Mode state is shared between the two processes. When the mode changes, the
window must notify `web` (via the existing XPC connection through
CompositorXPC), and vice versa.

### How Ctrl+Esc works under this architecture

1. User presses Ctrl+Esc while in browse mode.
2. The window intercepts the keypress via `NSEvent` (before it reaches the PTY).
3. The window recognizes Ctrl+Esc as the "exit browse mode" keybinding.
4. The window transitions its mode state from Browse to Control.
5. The window notifies `web` of the mode change via XPC.
6. The window stops forwarding keypresses to Chromium and lets them pass through
   to the terminal.
7. `web` updates its UI to reflect control mode (border colors, status bar).

Bare Esc continues to work as it does today — `web` receives it via crossterm
and transitions to control mode locally, then notifies the window.

## Implementation scope

This issue requires two things:

### 1. Window-side mode tracking and Ctrl+Esc handling

Add mode state (Browse/Control) to CompositorXPC, per pane. When the window
receives a Ctrl+Esc keypress in browse mode:

- Transition mode to Control
- Notify `web` via XPC
- Stop intercepting keypresses (let them flow to the terminal)

When the window receives a mode change notification from `web` (e.g., `web`
detected bare Esc or Enter):

- Update mode state
- Start or stop intercepting keypresses accordingly

### 2. Mode synchronization protocol

Add XPC messages for mode changes on the existing `web` ↔ CompositorXPC
connection:

- `mode_changed` (from window to `web`): window changed mode (e.g., Ctrl+Esc)
- `mode_changed` (from `web` to window): `web` changed mode (e.g., bare Esc,
  Enter)

Both sides update their local mode state on receipt.

## Future: full input forwarding

This issue lays the groundwork for full browser input forwarding (keyboard and
mouse). Once the window can intercept keypresses in browse mode, forwarding them
to the Chromium Profile Server is a natural next step. The XPC channel from the
window to the profile server already exists (CompositorXPC manages it). Adding
`key_event` and `mouse_event` messages completes the input pipeline.

The `web` TUI remains responsible for browser chrome (URL bar, status bar,
viewport border) and control mode keybindings (`q` to quit, Enter to browse).
The window is responsible for browser input (all keypresses and mouse events in
browse mode).
