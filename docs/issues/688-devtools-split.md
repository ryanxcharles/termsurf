# Issue 688: DevTools Split Command

Add a `:devtools` command to the TUI command bar that opens DevTools in a new
split pane. Typing `:devtools right` in a browser pane creates a split to the
right and runs `web devtools` in it, automatically inspecting the current tab.

## Background

Issue 687 enforced one DevTools per tab and locked DevTools panes from
navigating. DevTools now works reliably, but opening it requires manually
creating a split pane and typing `web devtools`. This issue automates that
workflow into a single command.

The end goal is a keyboard shortcut like `Opt+Cmd+I` that means "devtools right"
— but the shortcut is a separate issue. This issue builds the underlying command
infrastructure.

## How It Works

1. User is browsing in a `web` pane (e.g., `web google.com`)
2. User presses `:` to enter Command mode
3. User types `devtools right` and presses Enter
4. The TUI sends an XPC message to the GUI: "create a split to the right of my
   pane, and run this command in it"
5. The GUI creates the split using existing Ghostty split infrastructure
6. The new pane runs `web devtools`, which auto-targets the browser tab from
   step 1

### Why the full executable path matters

The command sent to the new pane must use the full path of the currently running
`web` binary (`std::env::current_exe()`), not just `web`. In development, the
user may have multiple builds — a release `web` in `$PATH` and a debug `web` in
the cargo target directory. Running the wrong one leads to confusing version
mismatches. Using the exact path of the current process guarantees the same
binary runs in the new pane.

## Design

### New TUI command: `:devtools <direction>`

Extend the command dispatcher in `main.rs` (the `dispatch()` function) to
recognize `devtools` commands:

- `:devtools right` — open DevTools in a split to the right
- `:devtools down` — open DevTools in a split below
- `:devtools left` — open DevTools in a split to the left
- `:devtools up` — open DevTools in a split above
- `:devtools` (no direction) — default to `right`

The command returns a new `CommandResult` variant that carries the direction.

### New XPC message: `open_split`

The TUI sends a new XPC message to the GUI:

```
{
  action: "open_split",
  pane_id: "...",
  direction: "right",          // "right", "down", "left", "up"
  command: "/full/path/to/web devtools"
}
```

The GUI receives this, finds the surface for the pane, and triggers a split
using the existing `termsurf_surface_split` API with a custom command in the
`SurfaceConfiguration`.

### GUI handler: `handleOpenSplit`

The GUI handler needs to:

1. Look up the surface for `pane_id` (via `surface_to_pane` / `panes` map)
2. Map the direction string to `SplitDirection` (right=0, down=1, left=2, up=3)
3. Create a split on that surface with the given command

The existing split flow is:

```
termsurf_surface_split(surface, direction)
  → TermSurf.App.newSplit() posts notification with SurfaceConfiguration
  → BaseTerminalController.newSplit() creates new SurfaceView
```

The `SurfaceConfiguration` has a `command` field. The challenge is threading a
custom command through this flow — `termsurf_surface_split()` doesn't currently
accept a command parameter. Options:

1. **Use `initialInput`** — create a normal split, then "type" the command into
   it via the `initialInput` field on `SurfaceConfiguration`. This sends the
   command text as keyboard input after the shell starts. Simpler but depends on
   shell prompt timing.
2. **Use `command`** — set `SurfaceConfiguration.command` to the full command
   string. The new pane runs `web devtools` directly without a shell. Cleaner
   but requires modifying the split flow to accept a custom command.
3. **Use a new XPC-to-surface path** — bypass `termsurf_surface_split` and
   create the split directly from the XPC handler, posting the notification with
   a custom `SurfaceConfiguration`.

Option 3 is the most direct — the XPC handler already has access to the surface
and can post the same notification that `termsurf_surface_split` would, but with
a custom `SurfaceConfiguration` that includes the command.

### Getting the executable path

In `main.rs`, capture the current executable path early:

```rust
let current_exe = std::env::current_exe()
    .ok()
    .and_then(|p| p.to_str().map(String::from))
    .unwrap_or_else(|| "web".to_string());
```

When building the `open_split` command, construct: `"{current_exe} devtools"`

### Error cases

Two cases must be caught before sending the `open_split` XPC message:

1. **`:devtools` typed in a DevTools pane.** You can't open DevTools for
   DevTools. Checked locally — `is_devtools` is already a flag in the TUI. No
   XPC needed.
2. **`:devtools` typed in a browser tab that already has DevTools open.** The
   `query_devtools` message (Issue 687) already checks for duplicates. The TUI
   calls it before sending `open_split`. If it returns an error, the command bar
   shows the error instead of splitting.

Both cases are validated before any split is attempted.

### Command bar error display

When a command fails, the command bar turns red and shows an error message on a
footer line below the input. This is a general-purpose error mechanism for
command mode — any command can use it.

**Visual behavior:**

- The command bar border turns red (replacing the normal yellow)
- A single-line error message appears below the command input, inside the bar's
  bottom border area (e.g., `"Tab 4 already has DevTools open"`)
- The error persists until the user types another character or exits command
  mode (Esc)

**Implementation:**

Add a `CommandResult::Error(String)` variant to the `CommandResult` enum. When
`dispatch()` returns an error, the event loop stores the error message in a
`command_error: Option<String>` variable. The `ui()` function checks this
variable — if set, it renders the command bar with a red border and the error
text as a bottom title. Any subsequent keystroke in command mode clears the
error.

This pattern generalizes beyond DevTools — unrecognized commands, invalid
arguments, or any future command that can fail will use the same red-bar
mechanism.

## Relevant Code

- `tui/src/main.rs` — `dispatch()` function (command mode), `CommandResult` enum
- `tui/src/xpc.rs` — XPC message sending
- `gui/src/apprt/xpc.zig` — XPC message handling, `panes` map, `surface_to_pane`
  map
- `gui/src/apprt/embedded.zig` — `termsurf_surface_split`, C API exports
- `gui/src/apprt/action.zig` — `SplitDirection` enum
- `gui/macos/Sources/TermSurf/TermSurf.App.swift` — `newSplit()`, notification
  posting
- `gui/macos/Sources/Features/Terminal/BaseTerminalController.swift` —
  `newSplit()`, `SurfaceConfiguration`
- `gui/macos/Sources/TermSurf/Surface View/SurfaceView.swift` —
  `SurfaceConfiguration` struct with `command` and `initialInput` fields
