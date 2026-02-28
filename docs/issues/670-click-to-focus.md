# Issue 670: Click-to-Focus Without Pass-Through

When you click an unfocused split pane, the click both activates the pane and
passes through to the content. This causes accidental link clicks, text
selections, and other unintended interactions. macOS windows don't work this way
— clicking an inactive window only activates it; you must click again to
interact.

## Current Behavior

Clicking an unfocused split pane:

1. `becomeFirstResponder()` fires → pane gains focus
2. `mouseDown` fires → `termsurf_surface_mouse_button` → `mouseButtonCallback`
3. The click is forwarded to content (terminal or Chromium)

All three happen on the same click. The user wanted to focus the pane but
accidentally clicked a link, started a selection, or triggered some other
action.

## Desired Behavior

macOS-style click-to-focus:

1. First click on unfocused pane → only activates it, click is consumed
2. Second click → interacts with content normally

## Analysis

### Click chain

```
NSEvent mouseDown
  → SurfaceView_AppKit.swift mouseDown() (line 844)
    → termsurf_surface_mouse_button (C API)
      → Surface.zig mouseButtonCallback() (line 4030)
        → hit-test overlay → forward to Chromium (xpc.sendMouseEvent)
        → or: forward to terminal (mouse reporting / selection)
```

Focus happens separately via `becomeFirstResponder()` (line 769), which sets
`self.focused = true` and calls `focusDidChange(true)` → Zig's `focusCallback()`
(Surface.zig line 3399) → sets `self.focused = true`.

### Existing precedent

Issue 606 Experiment 10 added `overlay_activation` — a flag that suppresses the
activation click on Chromium overlays. When the overlay transitions from
not-forwarding to forwarding (i.e. activating browse mode), the press and
release are consumed. This proves the pattern works for overlays. But it doesn't
cover terminal content clicks, and it only applies to the overlay activation
case, not split pane focus changes.

### Where to fix

The Zig layer (`mouseButtonCallback`) is the right place. It already has
`self.focused` which tracks focus state. The fix:

1. When `mouseButtonCallback` receives a press and `self.focused` was `false`
   just before this click, consume the click.
2. The focus change (`focusCallback`) happens before or concurrently with the
   mouse event, so we need a flag that persists across the focus→click sequence.

The tricky part: `focusCallback` sets `self.focused = true` before
`mouseButtonCallback` runs (because `becomeFirstResponder` fires first). So by
the time the click arrives, `self.focused` is already `true`. We need a separate
flag.

## Experiment 1: Activation click suppression

### Hypothesis

A `pane_activation` flag on `Surface.mouse` — set by `focusCallback` when
gaining focus, cleared by the subsequent mouse release — will suppress the
pass-through click without affecting normal interactions.

### Changes

#### 1. Surface.zig — add `pane_activation` flag

In the `mouse` struct (near `overlay_activation`):

```zig
/// Set when this surface just gained focus. The next press+release
/// is consumed (click-to-focus without pass-through).
pane_activation: bool = false,
```

#### 2. Surface.zig — set flag in `focusCallback`

In `focusCallback()`, when gaining focus:

```zig
if (focused) {
    self.mouse.pane_activation = true;
}
```

#### 3. Surface.zig — consume click in `mouseButtonCallback`

At the top of `mouseButtonCallback`, after recording click state but before any
forwarding logic:

```zig
// Suppress activation click — click-to-focus without pass-through (Issue 670).
if (self.mouse.pane_activation) {
    if (action == .release) {
        self.mouse.pane_activation = false;
    }
    return true; // Consume the event.
}
```

This goes before the overlay hit-test block (line 4046), so it applies to both
terminal and browser clicks.

### Test

1. `cd gui && zig build` — compiles without errors.
2. Open TermSurf, create a split.
3. Click unfocused pane → pane activates, no content interaction.
4. Click again → content interaction works normally.
5. Verify terminal: clicking unfocused pane doesn't start a selection.
6. Verify browser: clicking unfocused pane doesn't click a link.
7. **Resize the window** — still works.
8. **Focus-follows-mouse** — if enabled, mouse hover focuses without consuming
   subsequent clicks (hover sets focus without `pane_activation`). Verify.
9. **Keyboard focus** — switching panes via keybinding doesn't set
   `pane_activation`, so the next click works immediately.
