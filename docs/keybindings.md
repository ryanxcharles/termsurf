# Keybindings

All TermSurf-specific keybindings. Anything not listed here falls back to
Ghostty defaults or user-configured Ghostty keybindings.

## `web` TUI keybindings

These are handled by the `web` TUI process (`tui/src/main.rs`) via crossterm key
events received through the terminal PTY.

| Key    | Mode    | Action            | Notes                                        |
| ------ | ------- | ----------------- | -------------------------------------------- |
| Esc    | Browse  | Switch to Control | Sends `mode_changed(browsing: false)` to GUI |
| Enter  | Control | Switch to Browse  | Sends `mode_changed(browsing: true)` to GUI  |
| q      | Control | Quit              |                                              |
| Ctrl+C | Any     | Force quit        |                                              |

## GUI keybindings

These are handled in Ghost's Zig core (`gui/src/Surface.zig`), intercepted in
`keyCallback` before keybinding processing.

| Key      | Mode   | Action            | Notes                                                                                                                                                                    |
| -------- | ------ | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Ctrl+Esc | Browse | Switch to Control | Calls `notifyNonOverlayClicked` which sends `mode_changed(browsing: false)` to `web` and `focus_changed(false)` to Chromium. Gated on `isOverlayForwarding` (Issue 607). |

## Modes

The `web` TUI has two modes. The GUI only tracks a boolean (`browsing` or not).

| Mode        | GUI sees | Behavior                                                |
| ----------- | -------- | ------------------------------------------------------- |
| **Browse**  | `true`   | Keyboard/mouse goes to the browser (default on startup) |
| **Control** | `false`  | Terminal keybindings active, browser input paused       |

The GUI does not distinguish between non-browse modes. If `web` adds more modes
in the future (insert, search, etc.), they all map to `browsing: false` from the
GUI's perspective.

## Mode synchronization

Mode state is shared between the GUI and `web` via `mode_changed` XPC messages
on the existing direct connection (Issue 513). Both sides send and receive:

- **`web` changes mode** (Esc, Enter) → sends `mode_changed` to GUI
- **GUI changes mode** (Ctrl+Esc) → sends `mode_changed` to `web`
- **Initial mode** is set via the `browsing` field in the `set_overlay` message

## Ghostty fallbacks

All other keybindings (splits, tabs, copy/paste, font size, etc.) are Ghostty
defaults. See the [Ghostty documentation](https://ghostty.org/docs) for the full
list. Users can customize these via Ghostty's configuration file.
