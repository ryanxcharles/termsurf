# TermSurf 3.0 XPC Architecture

## Background

This document continues the work from [ts3-2-webview.md](./ts3-2-webview.md),
which explored cross-process GPU texture sharing between the profile server
(CEF) and the GUI (wezterm-gui).

### The Problem

TermSurf 3.0 uses a multi-process architecture for browser isolation:

```
wezterm-gui (terminal + renderer)
    │
    └── profile server (CEF browser engine)
            │
            └── CEF GPU helper process
                    │
                    └── IOSurface (GPU texture)
```

The profile server renders web content to an IOSurface. The GUI needs to display
that texture. The challenge: how do you share a GPU texture between separate
processes on macOS?

## Why Earlier Attempts Failed

### Attempt 1: IOSurface Global ID Lookup

Our first approach used `IOSurfaceGetID()` and `IOSurfaceLookup()`:

```rust
// Profile server
let id = IOSurfaceGetID(handle);  // Get numeric ID
send_to_gui(id);                   // Send ID over socket

// GUI
let handle = IOSurfaceLookup(id);  // Look up by ID → NULL
```

**Result:** `IOSurfaceLookup()` returned NULL.

**Root cause:** Apple deprecated `kIOSurfaceIsGlobal` in macOS 10.11 (2015).
Without this flag, IOSurfaces are not globally registered and cannot be looked
up by ID across process boundaries. This flag was removed for security reasons —
globally accessible screen buffers are a security hole.

### Attempt 2: Process Ancestry

We hypothesized that `IOSurfaceLookup()` might work if the GUI was an ancestor
of the process that created the IOSurface (similar to how Chromium's browser
process can access IOSurfaces from its GPU subprocess).

We re-architected so the GUI spawns the profile server:

```
wezterm-gui (spawns profile server)
    └── profile server
            └── CEF GPU helper (creates IOSurface)
```

**Result:** `IOSurfaceLookup()` still returned NULL.

**Conclusion:** The `kIOSurfaceIsGlobal` deprecation is absolute. Process
ancestry does not enable IOSurface lookup. The "global ID" mechanism is
fundamentally broken for cross-process sharing on modern macOS.

### The Bootstrap API Dead End

After the global ID approach failed, we considered using Mach ports directly:

```rust
// Profile server
let port = IOSurfaceCreateMachPort(handle);
// Send port to GUI somehow...

// GUI
let handle = IOSurfaceLookupFromMachPort(port);  // Works!
```

The APIs exist and work. The problem: how do you send a Mach port between
processes?

**Attempt: Bootstrap server registration**

```rust
// GUI
bootstrap_register("com.termsurf.gui", port);  // Register port with name

// Profile server
bootstrap_look_up("com.termsurf.gui", &port);  // Look up by name
```

**Problem:** `bootstrap_register()` is deprecated (since macOS 10.5).
`bootstrap_check_in()` only works for launchd-managed services. There is no
non-deprecated way to register an arbitrary Mach port for lookup by name.

## Why XPC Is Required

XPC (Cross-Process Communication) is Apple's modern replacement for raw Mach IPC
and bootstrap server registration. It provides:

1. **Mach port transfer:** XPC handles Mach port passing transparently via
   `xpc_dictionary_set_mach_send()` and `xpc_dictionary_copy_mach_send()`

2. **Service discovery:** XPC services are registered with launchd and
   discoverable by name without deprecated bootstrap APIs

3. **Endpoint passing:** Anonymous XPC listeners can create endpoints that are
   passed over existing XPC connections, enabling dynamic peer-to-peer
   communication

4. **Process lifecycle:** launchd manages XPC service lifecycle (launch
   on-demand, restart on crash)

### How Chromium Solved This

Chromium faced the same problem internally. Their GPU process creates
IOSurfaces, and the browser process needs to access them.

From
[Chromium code review 1532813002](https://codereview.chromium.org/1532813002):

> **"Replace IOSurfaceManager by directly passing IOSurface Mach ports over
> Chrome IPC"**

Chromium switched from global IDs to Mach ports sent over their internal IPC
layer. Their IPC layer is analogous to XPC — it can transfer Mach port rights
between processes.

CEF inherits this mechanism internally. When your code receives an IOSurface
handle in `on_accelerated_paint`, CEF has already used Mach ports to transfer it
from the GPU process to your browser process. The handle is valid because you're
in the same process.

For TermSurf 3.0, we need to perform the same transfer again — from the profile
server process to the GUI process. XPC is Apple's supported mechanism for this.

### The XPC Requirement Is Fundamental

This is not a matter of preference or convenience. On modern macOS:

- **Global IOSurface IDs don't work** (deprecated 2015)
- **Bootstrap registration doesn't work** (deprecated 2005)
- **Task port insertion requires entitlements** (security hardening)
- **Unix sockets can't transfer Mach ports** (only file descriptors via
  SCM_RIGHTS)

XPC is the only non-deprecated, non-entitled mechanism for transferring Mach
port rights between unrelated processes on macOS.

## Cross-Platform Considerations

XPC is macOS-specific. Other platforms have their own GPU texture sharing
mechanisms, and importantly, they're often simpler.

### Platform-Specific Transfer Mechanisms

| Platform    | Handle Type               | Transfer Mechanism         | Complexity |
| ----------- | ------------------------- | -------------------------- | ---------- |
| **macOS**   | IOSurface (`*mut c_void`) | XPC + Mach ports           | High       |
| **Linux**   | DMA-BUF (file descriptor) | Unix socket + `SCM_RIGHTS` | Low        |
| **Windows** | DXGI handle (`HANDLE`)    | `DuplicateHandle`          | Medium     |

### Linux Is Easier

On Linux, CEF renders to DMA-BUF textures, which are represented as file
descriptors. File descriptors can be sent over Unix domain sockets using
`SCM_RIGHTS` ancillary data. This means the existing Unix socket infrastructure
can carry texture handles directly — no new IPC mechanism needed.

From cef-rs `dmabuf.rs`:

```rust
pub struct DmaBufImporter {
    fds: Vec<std::os::fd::RawFd>,  // File descriptors - can use SCM_RIGHTS
    // ...
}
```

### Windows Is Medium Complexity

On Windows, DXGI shared handles can be duplicated into another process using
`DuplicateHandle()`. This requires the target process handle, which the GUI has
for profile servers it spawns.

### Architectural Consistency

Despite different transfer mechanisms, the overall architecture remains
identical across platforms:

```
Profile Server                      GUI
──────────────                      ───
on_accelerated_paint(handle)
        │
        ├── macOS: XPC + Mach port
        ├── Linux: Unix socket + SCM_RIGHTS
        └── Windows: DuplicateHandle
                                    │
                                    └── Import texture, render
```

The profile server sends; the GUI receives and imports. Only the transfer
mechanism differs.

### Implementation Strategy

1. **macOS first** — XPC/Mach ports are the most complex; solving this first
   de-risks the architecture
2. **Linux second** — Add `SCM_RIGHTS` support to existing Unix socket code
3. **Windows third** — `DuplicateHandle` is straightforward once we have the
   process handle

## Summary

| Approach               | Status   | Reason                                 |
| ---------------------- | -------- | -------------------------------------- |
| IOSurface global ID    | Failed   | `kIOSurfaceIsGlobal` deprecated        |
| Process ancestry       | Failed   | Deprecation is absolute                |
| Bootstrap registration | Blocked  | `bootstrap_register()` deprecated      |
| Raw Mach IPC           | Blocked  | No way to establish initial connection |
| XPC                    | Required | Apple's supported mechanism            |

The next step is to design and implement XPC-based communication between the GUI
and profile servers.

## Experiments

### Experiment 1: XPC IOSurface Transfer with Test Texture

**Status:** PLANNED

**Goal:** Validate the complete XPC architecture by displaying a test texture in
the terminal pane. Running `web google.com` will display a pink 100x100 texture
stretched to fill the pane, proving the entire IPC pipeline works before
integrating CEF.

#### What the User Sees

```
$ web google.com
```

- Terminal pane fills with solid pink (stretched from 100x100 texture)
- Resizing the pane stretches the pink to fit
- Ctrl+C exits and restores the terminal

#### Why Pink?

- **Not purple** — Purple often indicates uninitialized GPU memory or errors
- **Not black** — Could be confused with "nothing rendered"
- **Not white** — Could be confused with a blank page
- **Bright pink (#FF69B4)** — Unmistakably intentional, clearly a test texture

#### Architecture

```
web CLI                    GUI                         Launcher (XPC)           Test Sender
───────                    ───                         ──────────────           ───────────
    │                       │                               │                        │
    │── open_webview ──────>│                               │                        │
    │                       │                               │                        │
    │                       │── connect ───────────────────>│                        │
    │                       │                               │                        │
    │                       │── spawn_profile ─────────────>│                        │
    │                       │   + XPC endpoint              │                        │
    │                       │                               │── spawn ──────────────>│
    │                       │                               │   --session-id UUID    │
    │                       │                               │                        │
    │                       │                               │<── claim_session ──────│
    │                       │                               │                        │
    │                       │                               │── GUI endpoint ───────>│
    │                       │                               │                        │
    │                       │<══════════ XPC connection (direct) ═══════════════════>│
    │                       │                               │                        │
    │                       │<── IOSurface Mach port ───────────────────────────────│
    │                       │    (100x100 pink texture)     │                        │
    │                       │                               │                        │
    │                       │── render stretched ──>        │                        │
    │                       │   to pane size                │                        │
```

#### Components

##### 1. Launcher XPC Service (`ts3/termsurf-launcher/`)

Minimal XPC service that relays endpoints between GUI and spawned processes:

```rust
// Pseudocode
fn handle_spawn_profile(endpoint, profile, session_id) {
    pending_sessions.insert(session_id, endpoint);
    spawn("termsurf-test-sender", ["--session-id", session_id]);
}

fn handle_claim_session(session_id) -> endpoint {
    pending_sessions.remove(session_id)
}
```

**Info.plist:** Registers as `com.termsurf.launcher`

**Sandbox Note:** XPC services are sandboxed by default and may not be able to
spawn child processes. The launcher needs one of:

- **Option A:** Disable sandbox via entitlements:
  ```xml
  <!-- termsurf-launcher.entitlements -->
  <key>com.apple.security.app-sandbox</key>
  <false/>
  ```

- **Option B:** Keep sandbox but add process execution entitlement:
  ```xml
  <key>com.apple.security.temporary-exception.mach-lookup.global-name</key>
  <array>
      <string>com.apple.runningboard</string>
  </array>
  ```

- **Option C:** Restructure so GUI spawns test sender directly (launcher only
  relays endpoints, never spawns). This avoids sandbox issues entirely but
  changes the architecture slightly.

For this experiment, **Option A** (disable sandbox) is simplest. Production may
want to revisit with tighter security.

##### 2. Test Sender (`ts3/termsurf-test-sender/`)

Minimal binary that creates and sends a test IOSurface:

```rust
fn main() {
    let session_id = args.session_id;

    // 1. Connect to launcher, claim session, get GUI endpoint
    let launcher = xpc_connect("com.termsurf.launcher");
    let gui_endpoint = claim_session(launcher, session_id);

    // 2. Connect to GUI
    let gui = xpc_connect_from_endpoint(gui_endpoint);

    // 3. Create pink IOSurface
    let surface = create_iosurface(100, 100);
    fill_with_color(surface, 0xFF69B4);  // Hot pink

    // 4. Send Mach port to GUI
    let port = IOSurfaceCreateMachPort(surface);
    send_iosurface_port(gui, port, pane_id, 100, 100);

    // 5. Keep alive until GUI disconnects
    run_loop();
}
```

##### 3. GUI Modifications (`ts3/wezterm-gui/`)

**New file:** `src/termwindow/webview_xpc.rs`

- Create anonymous XPC listener on startup
- Connect to launcher
- Handle `open_webview` by sending spawn request with endpoint
- Receive IOSurface Mach port from test sender
- Call `IOSurfaceLookupFromMachPort()` to get handle
- Store handle for rendering

**Render changes:**

- Check for active webview overlay on pane
- Import IOSurface as wgpu texture
- Render fullscreen quad stretched to pane dimensions
- Re-stretch on resize (no re-request to sender)

##### 4. Web CLI

Existing code already sends `open_webview` to GUI. No changes needed — GUI
handles everything internally.

#### Files to Create

| File                                            | Purpose                    |
| ----------------------------------------------- | -------------------------- |
| `ts3/termsurf-launcher/Cargo.toml`              | Launcher crate manifest    |
| `ts3/termsurf-launcher/src/main.rs`             | XPC service implementation |
| `ts3/termsurf-launcher/Info.plist`              | XPC service registration   |
| `ts3/termsurf-test-sender/Cargo.toml`           | Test sender crate manifest |
| `ts3/termsurf-test-sender/src/main.rs`          | IOSurface creation + send  |
| `ts3/wezterm-gui/src/termwindow/webview_xpc.rs` | XPC client + listener      |

#### Files to Modify

| File                                               | Changes                         |
| -------------------------------------------------- | ------------------------------- |
| `ts3/wezterm-gui/src/termwindow/mod.rs`            | Initialize XPC manager          |
| `ts3/wezterm-gui/src/termwindow/webview_socket.rs` | Route to XPC for `open_webview` |
| `ts3/wezterm-gui/src/termwindow/render/*.rs`       | Render stretched texture        |
| `ts3/Cargo.toml`                                   | Add workspace members           |
| Build scripts                                      | Bundle XPC service in app       |

#### Success Criteria

- [ ] `web google.com` displays solid pink in the pane
- [ ] Pink fills entire pane (stretched from 100x100)
- [ ] Resizing pane re-stretches the pink (no flicker, no re-fetch)
- [ ] Ctrl+C exits cleanly, terminal restored
- [ ] Logs show: "IOSurfaceLookupFromMachPort returned valid handle"

#### What This Proves

1. **XPC service bundling works** — launchd finds and launches our service
2. **Endpoint relay works** — GUI endpoint successfully passed through launcher
3. **Direct XPC connection works** — Test sender connects to GUI via endpoint
4. **Mach port transfer works** — IOSurface port sent over XPC
5. **IOSurfaceLookupFromMachPort works** — Handle valid in receiving process
6. **Texture import works** — IOSurface → wgpu texture pipeline
7. **Rendering works** — Stretched quad displays correctly

#### Failure Modes

| Symptom                     | Likely Cause                                            |
| --------------------------- | ------------------------------------------------------- |
| `web` hangs                 | Launcher not starting, check Console.app for XPC errors |
| Black pane                  | IOSurfaceLookupFromMachPort failed, check logs          |
| Purple pane                 | Texture import failed, uninitialized GPU memory         |
| Pink square (not stretched) | Render quad not using pane dimensions                   |
| Crash on resize             | Texture lifecycle issue                                 |

#### After This Experiment

If successful, Experiment 2 replaces `termsurf-test-sender` with the real
profile server:

1. Profile server uses same XPC flow
2. Instead of pink IOSurface, uses `on_accelerated_paint` IOSurface
3. Sends Mach port on first paint and on resize
4. Real webpage appears in pane
