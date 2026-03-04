# Issue 704: Roamium, Zoomium, and Plusium — browser bindings for Chromium

## Goal

Create three browser binding packages — Roamium (Rust), Zoomium (Zig), and
Plusium (C++) — that wrap Chromium's Content API through a shared C library.
Each produces a standalone binary that speaks the TermSurf IPC protocol (Unix
sockets + length-prefixed protobuf). The TUI gains a `--browser` flag so users
can specify which binary to use: `web google.com --browser roamium`. After all
three work, Roamium becomes the default. The GUI treats browser binaries as
opaque — any protocol-compatible binary can be passed in.

## Background

### The C shim is proven

Issue 620 (Zig Content Shell) proved that Chromium's Content API can be driven
through a C function boundary. A thin C++ shim (~190 lines, 3 files) in
`chromium/src/content/zig_content_shell/` exports C functions for:

- Initialization: `ts_content_main()`
- Profile management: `ts_create_browser_context()`,
  `ts_destroy_browser_context()`
- Tab management: `ts_create_web_contents()`, `ts_destroy_web_contents()`
- Navigation: `ts_load_url()`, `ts_go_back()`, `ts_go_forward()`, `ts_reload()`
- Input forwarding: `ts_forward_mouse_event()`, `ts_forward_scroll_event()`,
  `ts_forward_key_event()`
- Display: `ts_get_ca_context_id()`, `ts_set_view_size()`
- Callbacks: `ts_set_on_url_changed()`, `ts_set_on_loading_state_changed()`,
  `ts_set_on_cursor_changed()`, `ts_set_on_title_changed()`,
  `ts_set_on_ca_context_id_changed()`

This C API subclasses `ContentMainDelegate`, `ContentBrowserClient`,
`BrowserMainParts`, `WebContentsDelegate`, and `WebContentsObserver` internally,
exposing only opaque handles and C function pointers to callers.

### Previous attempts at language bindings

- **Issue 642** (Zig Profile Server) — Proved the C shim architecture works
  end-to-end (profile creation, tab management, CAContext IDs). Failed on
  deployment: couldn't integrate a Zig binary into Chromium's app bundle due to
  code signing and path resolution mismatches between `zig build` and
  `autoninja`.
- **Issue 643** (Zig Profile Server Take 2) — Proposed moving Zig code inside
  `chromium/src/` and having GN build it. Abandoned before implementation.
- **Issue 644** (Simplified C++ Profile Server) — Pragmatic alternative: keep
  C++, strip Content Shell down to essentials. Baseline confirmed working,
  research complete, not yet implemented.

The key lesson: the C shim works, but the binding language's binary must be
buildable within or alongside Chromium's build system, or the deployment story
breaks.

### Current profile server

The current `Chromium Profile Server` is a Content Shell fork (~100 files, ~1050
lines TermSurf-specific) that:

1. Connects to the GUI via Unix socket (`--ipc-socket={path}`)
2. Sends `ServerRegister` with its profile name
3. Handles protobuf commands: CreateTab, Navigate, MouseEvent, KeyEvent,
   ScrollEvent, Resize, CloseTab, FocusChanged, SetColorScheme
4. Sends back: TabReady, CaContext, UrlChanged, LoadingState, TitleChanged,
   CursorChanged

### IPC protocol

All communication uses Unix domain sockets with 4-byte little-endian
length-prefixed protobuf messages (`termsurf.proto`). The GUI spawns server
processes with `--ipc-socket={path}` and `--user-data-dir={profile_path}`. The
server connects back to the GUI socket and registers itself.

### Relevant Chromium branches

| Branch                   | Issue | What                                                       |
| ------------------------ | ----- | ---------------------------------------------------------- |
| `146.0.7650.0-issue-620` | 620   | C shim PoC (zig_content_shell)                             |
| `146.0.7650.0-issue-642` | 642   | Zig profile server (3 files in content/zig_profile_server) |
| `146.0.7650.0-issue-643` | 643   | Zig profile server via GN (abandoned)                      |
| `146.0.7650.0-issue-644` | 644   | Simplified C++ profile server (research done)              |
| `146.0.7650.0-issue-702` | 702   | Current working branch (socket IPC)                        |

## Architecture

### Shared C library: `libtermsurf_content`

A C library built inside `chromium/src/` by autoninja. It wraps the Content API
and exports C functions for browser lifecycle, profile/tab management, input
forwarding, and state observation. This is an evolution of the Issue 620 C shim,
extended with:

- Socket IPC (connect to GUI, send/receive protobuf messages)
- Persistent compositor support (stable CAContext IDs across navigations)
- All message types from `termsurf.proto`
- DevTools tab creation
- Color scheme support

The C library does NOT contain a `main()` function. Each binding package
provides its own `main()` that calls into the C library. This means the C
library is a static or shared library, not an executable.

### Three binding packages

Each package links against `libtermsurf_content` and provides:

1. A `main()` function that parses CLI args and calls `ts_content_main()`
2. Socket connection logic (connect to `--ipc-socket`, length-prefixed protobuf)
3. Message dispatch (receive commands, call C API, send responses)
4. Callback registration (C function pointers that marshal state changes back
   over the socket)

All three must be **functionally equivalent** to the current Chromium Profile
Server. They are interchangeable — the GUI cannot tell which one it's talking
to.

| Package     | Language | Build system                                      | Notes                                |
| ----------- | -------- | ------------------------------------------------- | ------------------------------------ |
| **Roamium** | Rust     | Cargo (linked against libtermsurf_content)        | Default browser after completion     |
| **Zoomium** | Zig      | Zig build (linked against libtermsurf_content)    | Matches GUI's language               |
| **Plusium** | C++      | GN/autoninja (linked against libtermsurf_content) | Simplest, lives inside chromium/src/ |

### GUI changes: generic browser binary support

The GUI currently hardcodes the binary name `Chromium Profile Server`. This
changes to:

1. The TUI sends the browser binary path to the GUI as part of the overlay
   setup.
2. The GUI spawns whatever binary it receives, passing `--ipc-socket` and
   `--user-data-dir` as today.
3. The GUI does not care what language the binary is written in — only that it
   speaks the protobuf protocol.

### TUI changes: `--browser` flag

The TUI gains a `--browser` flag:

```
web google.com                        # Uses default (roamium)
web google.com --browser roamium      # Explicit roamium
web google.com --browser zoomium      # Zig version
web google.com --browser plusium      # C++ version
web google.com --browser /path/to/bin # Any protocol-compatible binary
```

Resolution order for named browsers:

1. Check if the value is an absolute path → use directly
2. Look in the app bundle: `{bundle}/Contents/Browsers/{name}`
3. Look in `$PATH`

The TUI passes the resolved binary path to the GUI in the `SetOverlay` message
(new field). The GUI uses this path instead of the hardcoded Chromium Profile
Server path.

### Protocol compatibility

Any binary is valid as long as it:

1. Accepts `--ipc-socket={path}` and `--user-data-dir={path}` CLI args
2. Connects to the Unix socket at the given path
3. Sends `ServerRegister` with the profile name
4. Handles all command messages from `termsurf.proto`
5. Sends back all response messages from `termsurf.proto`

This makes the browser binary a plugin point — third parties could write their
own protocol-compatible browser engines (Gecko, WebKit) and plug them in.

## Code locations

### Chromium side

| Location                                        | What                                                      |
| ----------------------------------------------- | --------------------------------------------------------- |
| `chromium/src/content/zig_content_shell/`       | Issue 620 C shim (starting point for libtermsurf_content) |
| `chromium/src/content/chromium_profile_server/` | Current working profile server                            |
| `chromium/src/content/zig_profile_server/`      | Issue 642 Zig profile server files                        |

### GUI side

| Location                         | What                                           |
| -------------------------------- | ---------------------------------------------- |
| `gui/src/apprt/xpc.zig` ~860-986 | `spawnServerProcess()` — hardcoded binary path |
| `gui/src/apprt/xpc.zig` ~146     | `TERMSURF_SOCKET` env var setup                |

### TUI side

| Location          | What                        |
| ----------------- | --------------------------- |
| `tui/src/main.rs` | CLI argument parsing (clap) |
| `tui/src/ipc.rs`  | Socket connection to GUI    |

### Proto

| Location               | What                         |
| ---------------------- | ---------------------------- |
| `proto/termsurf.proto` | Protobuf message definitions |

## Ideas for experiments

These are rough ideas, not commitments. Each experiment will be designed when
the previous one is complete.

1. **Extract `libtermsurf_content`** — Factor the C shim from Issue 620 into a
   proper C library within `chromium/src/`. Extend it with socket IPC, all
   message types, persistent compositor, DevTools. Build as a static library via
   GN. Verify by linking a minimal C `main()` that connects and serves one page.

2. **Build Plusium** — Create a minimal C++ binary in `chromium/src/` that links
   `libtermsurf_content`, implements socket IPC and message dispatch. This is
   the easiest binding since it stays inside the Chromium build. Verify it's
   functionally equivalent to the current profile server.

3. **Build Roamium** — Create a Rust crate that links `libtermsurf_content` via
   FFI (`bindgen` or manual declarations). Handle the build system integration
   (Cargo needs to find the Chromium-built library and headers). Verify
   equivalence.

4. **Build Zoomium** — Create a Zig package that links `libtermsurf_content` via
   `@cImport`. Same build system challenge as Roamium but for Zig. Verify
   equivalence.

5. **TUI `--browser` flag** — Add the CLI flag, browser resolution logic, and
   pass the binary path to the GUI via `SetOverlay`.

6. **GUI generic binary support** — Replace hardcoded binary path in
   `spawnServerProcess()` with the path received from the TUI.

7. **Make Roamium the default** — Once all three work, switch the default from
   Chromium Profile Server to Roamium.

8. **Retire the old profile server** — Remove `chromium_profile_server/` once
   Plusium fully replaces it.
