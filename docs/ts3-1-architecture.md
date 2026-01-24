# TermSurf 3.0 Architecture

## Overview

TermSurf 3.0 (ts3) is a terminal emulator with integrated browser capabilities,
built on WezTerm + CEF (Chromium Embedded Framework). This document describes
the core architecture, specifically the process model for browser integration.

## Background

### Why ts3?

TermSurf 2.0 (ts2) validated that WezTerm + CEF integration works:

- IOSurface texture sharing on macOS
- Keyboard, mouse, and scroll input handling
- Multiple browser instances in a single process
- Browser resize handling

However, Experiment 7 revealed a fundamental limitation: **CEF can only
initialize once per process with a single `root_cache_path`**. This means a
shared browser daemon cannot support multiple isolated profiles (different
cookies, storage, login sessions).

ts3 addresses this with a new process model.

### Lessons from Experiment 7

1. A shared CEF daemon forces all browsers to share one profile context
2. Event routing by pane ID across processes is error-prone
3. The `web` command should BE the browser, not a messenger to a daemon

## Process Model

### Architecture

```
termsurf (main terminal process)
    │
    ├── termsurf web --profile=default (browser subprocess)
    │       └── CEF helper processes (managed by CEF)
    │
    ├── termsurf web --profile=work (browser subprocess)
    │       └── CEF helper processes (managed by CEF)
    │
    └── termsurf web --profile=personal (browser subprocess)
            └── CEF helper processes (managed by CEF)
```

### Key Principles

1. **One CEF process per profile**: Each profile gets its own `termsurf web`
   subprocess with its own CEF context, enabling true isolation (separate
   cookies, storage, sessions).

2. **Multiple panes per profile**: A single `termsurf web` process can host
   multiple browser panes/tabs that share the same profile. This is efficient -
   you don't spawn a new process for each tab.

3. **The `web` command IS the browser**: Unlike Experiment 7's client-daemon
   model, the `web` command directly initializes CEF and renders browsers. No
   IPC to a separate daemon.

4. **Texture sharing via IOSurface**: Browser content is rendered off-screen by
   CEF and shared with the main terminal process via IOSurface (macOS). This
   allows compositing browser panes alongside terminal panes.

### Communication

The main terminal process and browser subprocesses communicate via:

- Unix domain sockets for commands (navigate, go back, reload, etc.)
- IOSurface handles for texture sharing (zero-copy)

### Profile Isolation

Each profile has:

- Its own CEF `root_cache_path` (cookies, local storage, cache)
- Its own `termsurf web` process
- Complete isolation from other profiles

Users can:

- Have multiple tabs open in the same profile (shared session)
- Have tabs in different profiles (isolated sessions)
- Log into the same site with different accounts in different profiles

## Components

### Main Terminal Process (WezTerm-based)

- Window management and compositing
- Terminal emulation
- Spawns and manages `termsurf web` subprocesses
- Receives textures from browser subprocesses
- Routes input events to appropriate subprocess

### Browser Subprocess (`termsurf web`)

- Initializes CEF with profile-specific cache path
- Manages one or more browser instances (tabs)
- Renders to off-screen textures (IOSurface)
- Handles browser-specific input (when pane is focused)
- Streams console output back to terminal (optional)

### CEF Helper Processes

- Managed internally by CEF
- GPU process, renderer processes, etc.
- No direct interaction with TermSurf code

## Validated Technology (from ts2/cef-rs)

The following has been validated and is ready for ts3:

| Component                | Status     | Notes                        |
| ------------------------ | ---------- | ---------------------------- |
| IOSurface texture import | Working    | Zero-copy texture sharing    |
| Keyboard input           | Working    | All key events handled       |
| Mouse input              | Working    | Click, move, scroll          |
| Multiple browsers        | Working    | Per-instance texture routing |
| Browser resize           | Working    | Dynamic resize support       |
| Context menu             | Suppressed | Prevents windowing conflicts |

## Open Questions

1. **Pane creation flow**: How does the main process signal a browser subprocess
   to create a new pane?
2. **Texture handle passing**: How are IOSurface handles passed from subprocess
   to main process?
3. **Focus management**: How does the main process know which browser pane has
   focus?
4. **Subprocess lifecycle**: When does a `termsurf web` process exit? When all
   its panes close?

## Future Considerations

- **Linux/Windows**: IOSurface is macOS-only. Will need platform-specific
  texture sharing (DMA-BUF on Linux, shared handles on Windows).
- **Profile management UI**: How users create, switch, and manage profiles.
- **DevTools**: Exposing Chrome DevTools for browser panes.
