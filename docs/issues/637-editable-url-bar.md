# Issue 637: Editable URL Bar

## Goal

Make the URL bar in the `web` TUI editable using edtui, a Vim-inspired text
editor widget for ratatui. Users should be able to edit the URL with full Vim
keybindings and press Enter to navigate.

## Background

The URL bar (`tui/src/main.rs`) is currently a read-only `Paragraph` widget that
displays the current URL. It updates via `CompositorMessage::UrlChanged` from
the compositor but cannot be edited by the user. There is no way to navigate to
a new URL by typing — the only navigation is via links clicked in the browser
pane.

[edtui](https://github.com/preiter93/edtui) is a Vim-inspired text editor widget
for ratatui. It provides full Vim keybindings (normal, insert, visual modes),
horizontal scrolling for long lines, undo/redo, cursor management, and a
customizable key handler. A local copy lives at `vendor/edtui/`.

## Current state

- **URL bar**: `Paragraph` widget in `layout[0]`, displays `url: String`
- **Modes**: `Browse` and `Control` (enum at line 24)
- **Mode switching**: `Esc` exits Browse, `Enter` enters Browse
- **Key dispatch**: lines 136–155, per-mode match
- **Compositor sync**: `send_set_overlay()` sends URL, `UrlChanged` receives it
- **No text input state**: no cursor, no edit buffer, no text input crate

## Mode design

This is the hardest part. The TUI currently has two modes. Adding an editable
URL bar introduces a third mode that itself contains Vim sub-modes. The
transitions must feel natural to a Vim user.

### Current modes

```
Browse ──Esc──> Control ──Enter──> Browse
```

- **Browse**: Keys forwarded to Chromium. URL bar is read-only.
- **Control**: Keys handled by TUI. Can quit (`q`), enter Browse (`Enter`).

### New mode: UrlEdit

```
Browse ──Ctrl+Esc──> Control ──e──> UrlEdit ──Enter──> Browse
                         ^                       │
                         └───────Ctrl+Esc────────┘
```

- **UrlEdit**: Keys handled by edtui. URL bar is editable with Vim keybindings.

Inside UrlEdit, edtui manages its own Vim modes (Normal, Insert, Visual). The
TUI does not need to track these — edtui handles them internally. The TUI only
needs to intercept two keys at the boundary:

- **Enter** (from any edtui mode): Navigate to the edited URL. Switch to Browse.
- **Ctrl+Esc** (from any edtui mode): Cancel edit. Switch to Control.

`Ctrl+Esc` is already the universal "escape to Control" key — it exits Browse
mode the same way. Using it in UrlEdit too keeps one consistent key for "get me
back to Control from anywhere." Escape (without Ctrl) always goes to edtui,
which uses it for its own Vim mode transitions (Insert → Normal, Visual →
Normal). No conditional interception needed.

### Entering UrlEdit

From Control mode, pressing `e` enters UrlEdit. The edit buffer is initialized
with the current URL. edtui starts in Normal mode with the cursor at the end.

A typical flow:

1. Press Esc to leave Browse and enter Control
2. Press `e` to enter UrlEdit (edtui Normal mode, cursor at end of URL)
3. Press `A` to append (edtui Insert mode)
4. Type the new URL
5. Press Enter to navigate (switches to Browse)

Or to edit a URL already in the bar:

1. Esc → `e` → use `w`/`b`/`h`/`l` to navigate → `ciw` to change a word → type →
   Enter

Or to cancel:

1. Esc → `e` → edit some text → Ctrl+Esc (back to Control)

### Mode summary

| Mode    | URL bar          | Keys go to | Enter           | Ctrl+Esc  | Escape        |
| ------- | ---------------- | ---------- | --------------- | --------- | ------------- |
| Browse  | read-only        | Chromium   | —               | → Control | —             |
| Control | read-only        | TUI        | → Browse        | —         | —             |
| UrlEdit | editable (edtui) | edtui      | navigate+Browse | → Control | → edtui (Vim) |

## edtui configuration

### Single-line mode

edtui has no built-in single-line mode. We enforce it by:

1. **Removing newline keybindings** from the `KeyEventHandler`:
   - Remove Enter → `LineBreak(1)` (Insert mode)
   - Remove `o` → `AppendNewline(1)` (Normal mode)
   - Remove `O` → `InsertNewline(1)` (Normal mode)
2. **Rebinding Enter** to trigger navigation (handled by the TUI, not edtui).
3. **Patching `insert_char`** in edtui's `helper.rs` to strip `\n` from pasted
   text. This prevents multi-line pastes from creating new lines.
4. **Setting `wrap(false)`** on `EditorView` to enable horizontal scrolling
   instead of line wrapping.

### Horizontal scrolling

With `wrap(false)`, edtui's `update_viewport_horizontal()` in `state/view.rs`
shifts the viewport to keep the cursor visible. Long URLs that don't fit in the
bar will scroll left/right as the cursor moves. This works out of the box.

### Theme

edtui's `EditorView` accepts a `theme()` to match the Tokyo Night palette used
by the TUI.

## Rendering

In UrlEdit mode, replace the `Paragraph` widget with edtui's `EditorView` widget
in `layout[0]`. The `EditorView` renders the edit buffer with cursor and handles
all display internally (cursor position, horizontal scroll, selection
highlighting).

In Browse and Control modes, continue rendering the read-only `Paragraph` as
before.

## Navigation

When Enter is pressed in UrlEdit mode:

1. Extract the edited URL from edtui's `EditorState`
2. Update the TUI's `url` string
3. Send the new URL to the compositor (either via `send_set_overlay()` with the
   new URL, or via a new `send_navigate()` XPC action)
4. Switch to Browse mode and notify the compositor (`send_mode_changed(true)`)

## Dependencies

Add edtui as a path dependency to `vendor/edtui/` (so we can patch `insert_char`
later for paste-with-newline). This requires upgrading the TUI from
`ratatui = "0.29"` + `crossterm = "0.28"` to `ratatui = "0.30"` +
`crossterm = "0.29"` to match edtui's dependency versions.

## Experiment 1: edtui URL bar with mode transitions

### Hypothesis

Adding edtui as the URL bar editor with a `UrlEdit` mode, Ctrl+Esc/Enter
interception, and single-line enforcement will produce a working editable URL
bar with Vim keybindings. The user can press `e` from Control mode, edit the
URL, and press Enter to navigate.

### Changes

#### 1. Upgrade dependencies (`tui/Cargo.toml`)

edtui 0.11.1 depends on `crossterm = "0.29"` and `ratatui-core = "0.1"`. The TUI
currently uses `ratatui = "0.29"` + `crossterm = "0.28"`. Upgrade to match:

```toml
[dependencies]
crossterm = "0.29"
ratatui = "0.30"
edtui = { path = "../vendor/edtui", default-features = false, features = ["arboard"] }
libc = "0.2"
block2 = "0.6"
```

Disable `default-features` to skip `syntax-highlighting` (not needed for a URL
bar). Keep `arboard` for system clipboard support.

#### 2. Add `UrlEdit` mode (`tui/src/main.rs`)

Add a third variant to the `Mode` enum:

```rust
#[derive(PartialEq)]
enum Mode {
    Browse,
    Control,
    UrlEdit,
}
```

Add edtui state alongside existing state in `main()`:

```rust
use edtui::{EditorEventHandler, EditorMode, EditorState, EditorTheme,
            EditorView, Lines};
use edtui::events::{KeyEventHandler, KeyEventRegister, KeyInput};

let mut editor_state = EditorState::new(Lines::from(url.as_str()));
let mut editor_handler = {
    let mut kh = KeyEventHandler::vim_mode();
    // Remove newline keybindings for single-line mode.
    kh.remove(&KeyEventRegister::i(vec![KeyInput::new(KeyCode::Enter)]));
    kh.remove(&KeyEventRegister::n(vec![KeyInput::new('o')]));
    kh.remove(&KeyEventRegister::n(vec![KeyInput::shift('O')]));
    EditorEventHandler::new(kh)
};
```

#### 3. Key dispatch for UrlEdit mode

In the event loop, add a `Mode::UrlEdit` arm. The TUI intercepts Enter and
Ctrl+Esc. Everything else (including plain Escape) goes to edtui:

```rust
Mode::UrlEdit => {
    match key.code {
        KeyCode::Enter => {
            // Extract URL from editor, navigate, switch to Browse.
            let new_url: String = editor_state.lines
                .get(jagged::index::RowIndex::new(0))
                .map(|line| line.iter().collect())
                .unwrap_or_default();
            url = new_url;
            mode = Mode::Browse;
            if let (Some(ref conn), Some(ref pid)) = (&compositor, &pane_id) {
                conn.send_mode_changed(pid, true);
            }
            // Force viewport update to send new URL.
            last_viewport = Rect::default();
        }
        _ => {
            // Pass everything else to edtui (including Escape).
            editor_handler.on_key_event(key, &mut editor_state);
        }
    }
}
```

Note: `Ctrl+Esc` is already handled before the mode dispatch (it always switches
to Control from any mode). See the existing `Ctrl+C` pattern — `Ctrl+Esc` is
added at the same level.

#### 4. Enter UrlEdit from Control mode

Add `e` keybinding in Control mode:

```rust
Mode::Control => match key.code {
    KeyCode::Char('q') => break,
    KeyCode::Char('e') => {
        // Initialize editor with current URL, cursor at end.
        editor_state = EditorState::new(Lines::from(url.as_str()));
        let len = url.len();
        editor_state.cursor = edtui::Index2::new(0, len.saturating_sub(1));
        mode = Mode::UrlEdit;
        if let (Some(ref conn), Some(ref pid)) = (&compositor, &pane_id) {
            conn.send_mode_changed(pid, false);
        }
    }
    KeyCode::Enter => {
        mode = Mode::Browse;
        if let (Some(ref conn), Some(ref pid)) = (&compositor, &pane_id) {
            conn.send_mode_changed(pid, true);
        }
    }
    _ => {}
},
```

#### 5. Render edtui in UrlEdit mode (`ui()` function)

Pass `editor_state` to `ui()` and render `EditorView` when in UrlEdit mode:

```rust
fn ui(
    frame: &mut Frame,
    url: &str,
    profile: &str,
    mode: &Mode,
    editor_state: &mut EditorState,
) -> Rect {
    // ... layout unchanged ...

    let (url_border, viewport_border) = match mode {
        Mode::Browse => (BORDER, CYAN),
        Mode::Control => (CYAN, BORDER),
        Mode::UrlEdit => (CYAN, BORDER),
    };

    if *mode == Mode::UrlEdit {
        let theme = EditorTheme::default()
            .base(Style::default().fg(FG).bg(BG))
            .cursor_style(Style::default().fg(BG).bg(FG))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Chromium ")
                    .title_top(profile_title.alignment(Alignment::Right))
                    .border_style(Style::default().fg(url_border).bg(BG))
                    .title_style(Style::default().fg(url_border))
                    .style(Style::default().bg(BG)),
            )
            .hide_status_line();
        EditorView::new(editor_state)
            .theme(theme)
            .wrap(false)
            .render(layout[0], frame.buffer_mut());
    } else {
        // Existing Paragraph rendering.
        let url_bar = Paragraph::new(url)
            .style(Style::default().fg(FG))
            .block(/* ... existing block ... */);
        frame.render_widget(url_bar, layout[0]);
    }

    // ... rest unchanged ...
}
```

#### 6. Update status bar hints

Add UrlEdit hints:

```rust
Mode::UrlEdit => Line::from(vec![
    Span::styled("<", d),
    Span::styled("enter", f),
    Span::styled("> ", d),
    Span::styled("navigate  ", f),
    Span::styled("<", d),
    Span::styled("ctrl+esc", f),
    Span::styled("> ", d),
    Span::styled("control", f),
]),
```

And a mode label:

```rust
let label = match mode {
    Mode::Browse => "\u{F059F} BROWSE",
    Mode::Control => "\u{F11C} CONTROL",
    Mode::UrlEdit => "\u{F040} EDIT",
};
```

### Verification

1. `cd tui && cargo build` — must compile with upgraded deps
2. Launch TermSurf, open a `web` pane
3. Press Esc to enter Control mode
4. Press `e` — URL bar becomes editable, cursor visible at end of URL
5. Press `A` then type a new URL — characters appear
6. Press Enter — browser navigates to the new URL, mode switches to Browse
7. Press Ctrl+Esc from UrlEdit — cancels edit, returns to Control
8. Ctrl+Esc → `e` → `0` → `w` → `ciw` → type → Enter — Vim motions work
9. Long URL: horizontal scrolling keeps cursor visible

### Success criteria

- `e` from Control enters UrlEdit with edtui rendering
- Vim keybindings work (h/l/w/b/i/a/A/x/dd/ciw etc.)
- Enter extracts the URL and navigates
- Ctrl+Esc exits UrlEdit to Control from any edtui sub-mode
- Escape handled by edtui for Vim mode transitions (Insert → Normal)
- No newlines can be inserted (Enter and o/O removed)
- Horizontal scrolling works for long URLs
- Status bar shows correct hints and mode label
