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

### The real problem: macOS menu system eats Cmd+C/V/X before Rust code sees them

The macOS event flow for Cmd+C:

1. macOS calls `performKeyEquivalent` on the view
2. Wezboard returns `Bool::NO` — "I didn't handle this"
3. macOS checks the menu bar, finds **Edit → Copy to clipboard** (Cmd+C)
4. macOS invokes `wezboardPerformKeyAssignment:` → `CopyTo(Clipboard)`
5. Terminal copies its selection to the system clipboard
6. **`keyDown` is never called** → `key_common` never runs → `try_forward_key`
   never runs → the browser never sees the event

The Rust key processing pipeline (`raw_key_event_impl` → `process_key` →
`try_forward_key`) never executes for Cmd+C/V because the macOS menu system
consumes the event at step 3. This is the same problem Ghostboard had (Issue 609
Experiment 2) — just manifesting through the menu bar instead of
`performKeyEquivalent` bindings.

### Two-part fix

The fix requires changes in two layers:

1. **`perform_key_equivalent`** (window crate, `window.rs`): When the Cmd
   modifier is held and the character is one of `c`, `v`, `x`, `a`, `z`, route
   the event through `key_common` and return `Bool::YES`. This prevents the
   macOS menu system from intercepting the event. We only intercept these
   specific keys — other Cmd+key combos (Cmd+T, Cmd+W, Cmd+N) must still reach
   the menu system.

2. **`try_forward_key`** (wezboard-gui, `input.rs`): Check browse mode before
   the `only_key_bindings` guard. When browsing, forward to the browser. When
   not browsing, return `None` so the normal keybinding lookup handles Cmd+C →
   `CopyTo` for terminal copy.

Together: `perform_key_equivalent` ensures the event reaches Rust code, and
`try_forward_key` routes it to either the browser (browse mode) or the terminal
keybinding system (control mode).

## Experiments

### Experiment 1: Bypass menu system for clipboard keys in browse mode

#### Description

Two changes that work together to route Cmd+C/V/X/A/Z to the browser when in
browse mode, while preserving terminal clipboard behavior in control mode.

#### Changes

**`wezboard/window/src/os/macos/window.rs`** — In `perform_key_equivalent`,
before the `Bool::NO` fallthrough, add a check for clipboard-related Cmd+key
events. Route them through `key_common` to bypass the menu system:

```rust
if modifiers == Modifiers::SUPER
    && matches!(chars, "a" | "c" | "v" | "x" | "z")
{
    Self::key_common(this, nsevent, true);
    return Bool::YES;
}
```

This goes after the existing Cmd+period / Ctrl+Esc / Ctrl+Tab / Shift+Tab block
and before the `Bool::NO` return. It uses the same `key_common` + `Bool::YES`
pattern that already exists for Ctrl+Esc.

This unconditionally intercepts these five keys — it doesn't check browse mode
because the `window` crate has no access to TermSurf state. The browse mode
routing happens in the next layer.

**`wezboard/wezboard-gui/src/termsurf/input.rs`** — In `try_forward_key()`,
check browse mode before the `only_key_bindings` guard:

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

When browsing, the browser gets the key. When not browsing, `try_forward_key`
returns `None`, and `process_key` falls through to the keybinding lookup which
finds Cmd+C → `CopyTo(Clipboard)` — terminal copy still works.

#### Verification

```bash
scripts/build.sh wezboard
scripts/install.sh wezboard
```

Launch Wezboard, open a web page with `web`, click a text field to enter browse
mode.

| #  | Test                      | Steps                                             | Expected                      |
| -- | ------------------------- | ------------------------------------------------- | ----------------------------- |
| 1  | Cmd+C copies browser text | Select text on page, Cmd+C, paste in terminal     | Browser selection pasted      |
| 2  | Cmd+V pastes into browser | Copy text in terminal, click browser field, Cmd+V | Text appears in browser field |
| 3  | Cmd+X cuts browser text   | Select text in browser field, Cmd+X               | Text removed, on clipboard    |
| 4  | Cmd+A selects all         | Click browser field with text, Cmd+A, type "X"    | All text replaced with "X"    |
| 5  | Cmd+Z undoes              | Type "hello", Cmd+A, type "X", Cmd+Z              | "hello" restored              |
| 6  | Regular typing works      | Type "hello" in browser field                     | "hello" appears               |
| 7  | Esc exits browse mode     | Press Esc                                         | Returns to control mode       |
| 8  | Cmd+C in control mode     | Exit browse mode, Cmd+C                           | Copies terminal selection     |
| 9  | Cmd+V in control mode     | Exit browse mode, Cmd+V                           | Pastes into terminal          |
| 10 | Cmd+T still works         | Press Cmd+T                                       | Opens new tab                 |
| 11 | Cmd+W still works         | Press Cmd+W                                       | Closes current tab            |

Tests 8-9 verify terminal clipboard still works in control mode. Tests 10-11
verify that non-clipboard Cmd+key shortcuts still route through the menu system.
