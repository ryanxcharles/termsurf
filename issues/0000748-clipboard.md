# Issue 748: Browser clipboard (copy/cut/paste)

## Goal

Copy, cut, and paste work inside browser overlays in Wezboard. Cmd+C copies
selected text to the system clipboard. Cmd+V pastes from the system clipboard
into the browser. Cmd+X cuts. This should work on any web page — text fields,
content-editable regions, and selecting text on a read-only page.

## Background

### The Chromium side is already solved

Issue 609 (Ghostboard) solved clipboard for Chromium. The key learning:
**Chromium's renderer does not re-interpret raw keyboard events.** On macOS,
Chromium's input path works like this:

1. NSEvent → `interpretKeyEvents:` → Cocoa calls `doCommandBySelector:`
2. The selector is converted to an editing command string (`"copy"`, `"paste"`)
3. `ForwardKeyboardEventWithCommands` sends the key event AND the editing
   commands to the renderer
4. The renderer applies the commands directly — it never looks at the raw key
   event

Issue 609 Experiment 4 fixed this by using `ForwardKeyboardEventWithCommands`
with explicit editing commands for Cmd+key combinations. This code was carried
forward into `libtermsurf_chromium` — the current
`TsBrowserMainParts::ForwardKeyEvent` in `ts_browser_main_parts.cc` already uses
`ForwardKeyboardEventWithCommands` with the correct command mapping:

- Cmd+A → `"selectAll"`
- Cmd+C → `"copy"`
- Cmd+V → `"paste"`
- Cmd+X → `"cut"`
- Cmd+Z → `"undo"`

The Chromium side needs no changes.

### Prior art: CEF (Issues 206, 318)

The ts2/ts3 CEF architecture used direct `frame.copy()` / `frame.paste()` /
`frame.cut()` FFI calls. Key learnings:

- **macOS clipboard asymmetry:** Background processes can WRITE to the clipboard
  but cannot READ it. Paste required proxying through the GUI.
- **Key event synthesis is unsafe:** Simulating key events through message loops
  caused re-entrancy panics. Direct API calls are safer.
- **Menu shortcuts intercept before key events:** `performKeyEquivalent` on
  macOS consumes Cmd+key before `keyDown` fires.

### The real problem: Wezboard intercepts Cmd+C/V before the browser sees them

The key event pipeline in Wezboard has two stages:

1. **`raw_key_event_impl()`** — Called first. Passes `OnlyKeyBindings::Yes` to
   `process_key()`. This causes `try_forward_key()` to return `None` (line 74 of
   `input.rs`), skipping the browser forward. Then `process_key()` checks
   keybindings and finds Cmd+C → `CopyTo(Clipboard)`, which copies the terminal
   selection. The event is marked handled.

2. **`key_event_impl()`** — Called second, but only if the raw handler didn't
   consume the event. Passes `OnlyKeyBindings::No`. Would correctly forward to
   the browser, but never runs because step 1 already consumed the event.

The flow:

```
Cmd+C pressed
  → raw_key_event_impl()
    → process_key(OnlyKeyBindings::Yes)
      → try_forward_key(only_key_bindings=true)
        → returns None (skips browser)           ← BUG
      → lookup_key() finds Cmd+C → CopyTo
      → copies terminal text, marks handled
  → key_event_impl() never called
  → browser never sees the event
```

The fix: when `only_key_bindings` is true but the pane is in browse mode,
forward to the browser instead of returning `None`. Browse mode should take
priority over terminal keybindings for keys that the browser needs.

### macOS `performKeyEquivalent`

Wezboard's `perform_key_equivalent` (in `window.rs:3085`) returns `Bool::NO` for
Cmd+C/V, letting macOS route them to `keyDown`. This is fine — the problem is
entirely in the Rust key processing pipeline, not in the macOS event layer.
(Ghostboard had the opposite problem — its `performKeyEquivalent` consumed
Cmd+key events before they reached the forwarding code.)

## Experiments

### Experiment 1: Forward Cmd+key in browse mode

#### Description

Change `try_forward_key()` in `input.rs` to forward key events to the browser
when the pane is in browse mode, even when `only_key_bindings` is true. This is
a one-line change: remove the early `return None` when `only_key_bindings` is
true, and instead check browse mode first.

The current code:

```rust
if only_key_bindings {
    return None;
}
// ... check browsing, forward to browser
```

Should become:

```rust
// Check browse mode BEFORE checking only_key_bindings.
// When browsing, the browser gets the key — even if this is the
// keybindings-only pass (raw_key_event_impl). This prevents terminal
// bindings like Cmd+C → CopyTo from stealing clipboard shortcuts.
```

#### Changes

**`wezboard/wezboard-gui/src/termsurf/input.rs`** — In `try_forward_key()`, move
the browse mode check before the `only_key_bindings` check:

```rust
pub fn try_forward_key(
    pane_id: usize,
    keycode: &KeyCode,
    modifiers: Modifiers,
    is_down: bool,
    key_event: Option<&::window::KeyEvent>,
    only_key_bindings: bool,
) -> Option<bool> {
    let pane_id_str = pane_id.to_string();
    let state = super::shared_state()?;
    let browsing = {
        let st = state.lock().unwrap();
        let pane = st.panes.get(&pane_id_str)?;
        pane.browsing
    };

    if !browsing {
        return None;
    }

    // Pane is browsing — forward to browser regardless of only_key_bindings.

    // Esc key press (no Ctrl) exits browse mode
    // ... rest unchanged
```

When not browsing, return `None` so terminal keybindings work normally. When
browsing, fall through to send the key to Chromium — `only_key_bindings` is
ignored because the browser should receive all key events in browse mode.

#### Verification

```bash
scripts/build.sh wezboard
scripts/install.sh wezboard
```

Launch Wezboard, open a web page with `web`, click a text field to enter browse
mode.

| # | Test                      | Steps                                             | Expected                      |
| - | ------------------------- | ------------------------------------------------- | ----------------------------- |
| 1 | Cmd+C copies browser text | Select text on page, Cmd+C, paste in terminal     | Browser selection pasted      |
| 2 | Cmd+V pastes into browser | Copy text in terminal, click browser field, Cmd+V | Text appears in browser field |
| 3 | Cmd+X cuts browser text   | Select text in browser field, Cmd+X               | Text removed, on clipboard    |
| 4 | Cmd+A selects all         | Click browser field with text, Cmd+A, type "X"    | All text replaced with "X"    |
| 5 | Cmd+Z undoes              | Type "hello", Cmd+A, type "X", Cmd+Z              | "hello" restored              |
| 6 | Regular typing works      | Type "hello" in browser field                     | "hello" appears               |
| 7 | Esc exits browse mode     | Press Esc                                         | Returns to control mode       |
| 8 | Cmd+C in control mode     | Exit browse mode, Cmd+C                           | Copies terminal selection     |
| 9 | Cmd+V in control mode     | Exit browse mode, Cmd+V                           | Pastes into terminal          |

Tests 8-9 are regression checks — Cmd+C/V must still work normally outside
browse mode.
