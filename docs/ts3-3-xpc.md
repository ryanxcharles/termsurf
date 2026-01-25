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

### Experiment 1: XPC Test Bundle

**Status:** PLANNED

**Goal:** Validate the `termsurf-xpc` Rust bindings and XPC service architecture
with a minimal standalone test before integrating with wezterm/CEF. This
isolates XPC complexity from GUI/rendering complexity.

**Critical validation:** The launcher must be able to **spawn child processes**.
This is the entire reason the launcher exists — if it can't spawn profile
servers, the architecture fails.

#### What This Tests

| API / Capability                        | Purpose                                  |
| --------------------------------------- | ---------------------------------------- |
| `xpc_connection_create_mach_service()`  | Connect to named XPC service             |
| `xpc_connection_set_event_handler()`    | Handle incoming messages (via blocks)    |
| `XpcListener::new_anonymous()`          | Create endpoint for peer-to-peer         |
| `xpc_endpoint_create()`                 | Extract endpoint from listener           |
| `xpc_dictionary_set_endpoint()`         | Send endpoint in message                 |
| `xpc_connection_create_from_endpoint()` | Connect to anonymous listener            |
| `xpc_dictionary_set_mach_send()`        | Send Mach port in message                |
| `xpc_dictionary_copy_mach_send()`       | Receive Mach port from message           |
| `IOSurfaceCreateMachPort()`             | Create sendable port from IOSurface      |
| `IOSurfaceLookupFromMachPort()`         | Reconstruct IOSurface from received port |
| **Process spawning from XPC service**   | Launcher spawns sender (critical!)       |
| **Sandbox compatibility**               | XPC service can spawn with entitlements  |

#### Architecture

This test mirrors the production architecture:

| Test Component | Production Equivalent |
| -------------- | --------------------- |
| Receiver       | wezterm-gui           |
| Launcher       | termsurf-launcher     |
| Sender         | profile server        |

```
┌────────────────────────────────────────────────────────────────────────┐
│                     TermSurf XPC Test Bundle                           │
│                     (ts3/termsurf-xpc/)                                │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │
│  ┌─────────────┐          ┌─────────────┐          ┌─────────────┐    │
│  │  Receiver   │          │   Launcher  │ ──spawn──>   Sender    │    │
│  │   (Rust)    │          │   (XPC Svc) │          │   (Rust)    │    │
│  └──────┬──────┘          └──────┬──────┘          └──────┬──────┘    │
│         │                        │                        │           │
│    1. Create              2. Spawn sender            3. Create        │
│       anonymous              + pass endpoint            IOSurface     │
│       listener               + session ID               + send port   │
│                                                                       │
└───────────────────────────────────────────────────────────────────────┘
```

#### Test Flow

**Key difference from naive approach:** The launcher spawns the sender. This
validates that XPC services can spawn child processes (required for production).

```
Receiver                      Launcher                     Sender
────────                      ────────                     ──────
    │                            │
    │── connect ────────────────>│
    │                            │
    │── spawn_sender ───────────>│
    │   + my endpoint            │
    │   + session_id: "test-1"   │
    │                            │
    │                            │── spawn ──────────────────>│
    │                            │   args: --session test-1   │
    │                            │                            │
    │                            │<──────── connect ──────────│
    │                            │                            │
    │                            │<── claim_session ──────────│
    │                            │    session_id: "test-1"    │
    │                            │                            │
    │                            │── receiver endpoint ──────>│
    │                            │                            │
    │<════════════ direct XPC connection ════════════════════>│
    │                            │                            │
    │                            │    (IOSurface Mach port)   │
    │<─────────────────────────────────── send_surface ───────│
    │                            │                            │
    │   IOSurfaceLookupFromMachPort() → handle                │
    │   Verify: width=100, height=100, pixel[0,0]=0xFF69B4    │
    │                            │                            │
    │   print "SUCCESS"          │                            │
    │                            │                            │
```

#### Components

##### 1. XPC Service (Launcher)

**Location:** `ts3/termsurf-xpc/examples/launcher.rs`

XPC service that spawns child processes and relays endpoints. Written in pure
Rust using the `termsurf-xpc` bindings:

```rust
// examples/launcher.rs
use termsurf_xpc::*;
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::env;

fn main() -> Result<()> {
    println!("Launcher: Starting...");

    // Session storage: session_id -> receiver endpoint
    let sessions: Arc<Mutex<HashMap<String, XpcEndpoint>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // Path to sender binary (sibling in same directory)
    let exe_path = env::current_exe().expect("Failed to get exe path");
    let exe_dir = exe_path.parent().expect("Failed to get exe directory");
    let sender_path = exe_dir.join("sender");

    // Create listener for this XPC service
    let listener = XpcListener::new_mach_service("com.termsurf.xpc-test")?;

    // Handle incoming connections
    let sessions_clone = sessions.clone();
    let sender_path_clone = sender_path.clone();

    set_new_connection_handler(&listener, move |conn| {
        println!("Launcher: New connection");

        let sessions = sessions_clone.clone();
        let sender_path = sender_path_clone.clone();

        set_event_handler(&conn, move |event| {
            match event {
                Ok(msg) => {
                    let action = msg.get_string("action").unwrap_or_default();

                    match action.as_str() {
                        "spawn_sender" => {
                            let session_id = msg.get_string("session_id")
                                .expect("Missing session_id");
                            let endpoint = msg.get_endpoint("receiver_endpoint")
                                .expect("Missing receiver_endpoint");

                            // Store endpoint for sender to claim
                            {
                                let mut sessions = sessions.lock().unwrap();
                                sessions.insert(session_id.clone(), endpoint);
                            }

                            // Spawn sender as child process
                            match Command::new(&sender_path)
                                .args(["--session", &session_id])
                                .spawn()
                            {
                                Ok(_) => println!("Launcher: Spawned sender for {}", session_id),
                                Err(e) => eprintln!("Launcher: Failed to spawn: {}", e),
                            }
                        }

                        "claim_session" => {
                            let session_id = msg.get_string("session_id")
                                .expect("Missing session_id");

                            let endpoint = {
                                let mut sessions = sessions.lock().unwrap();
                                sessions.remove(&session_id)
                            };

                            // Send reply with endpoint
                            let reply = XpcDictionary::create_reply(&msg)
                                .expect("Failed to create reply");

                            if let Some(ep) = endpoint {
                                reply.set_endpoint("endpoint", ep);
                                println!("Launcher: Session {} claimed", session_id);
                            } else {
                                reply.set_string("error", "session not found");
                                eprintln!("Launcher: Session {} not found", session_id);
                            }

                            conn.send(&reply);
                        }

                        _ => {
                            eprintln!("Launcher: Unknown action: {}", action);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Launcher: Connection error: {}", e);
                }
            }
        });

        conn.resume();
    });

    listener.resume();

    println!("Launcher: Running...");
    run_loop();
}
```

**Info.plist** (required for XPC service registration):

```xml
<!-- xpc-service/Info.plist -->
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.termsurf.xpc-test</string>
    <key>CFBundleName</key>
    <string>termsurf-xpc-test</string>
    <key>CFBundleExecutable</key>
    <string>launcher</string>
    <key>XPCService</key>
    <dict>
        <key>ServiceType</key>
        <string>Application</string>
    </dict>
</dict>
</plist>
```

**Note on sandboxing:** XPC services are sandboxed by default. To spawn child
processes, either disable the sandbox via entitlements or restructure so the
main app spawns processes instead. For this test, we run unsigned/unsandboxed.

##### 2. Receiver (Rust)

**Location:** `ts3/termsurf-xpc/examples/receiver.rs`

Tests the "GUI side" — creates anonymous listener, asks launcher to spawn
sender:

```rust
// examples/receiver.rs
use termsurf_xpc::*;

fn main() -> Result<()> {
    println!("Receiver: Starting...");

    // 1. Connect to launcher
    let launcher = XpcConnection::connect_mach_service("com.termsurf.xpc-test")?;
    set_event_handler(&launcher, |event| {
        // Handle any errors from launcher
        if let Err(e) = event {
            eprintln!("Receiver: Launcher error: {}", e);
        }
    });
    launcher.resume();
    println!("Receiver: Connected to launcher");

    // 2. Create anonymous listener for sender to connect
    let listener = XpcListener::new_anonymous()?;
    let endpoint = listener.get_endpoint()?;
    println!("Receiver: Created anonymous listener");

    // 3. Set up handler for incoming peer connections
    set_new_connection_handler(&listener, |peer| {
        println!("Receiver: Sender connected!");

        set_event_handler(&peer, |event| {
            match event {
                Ok(msg) => {
                    let action = msg.get_string("action").unwrap_or_default();
                    if action == "send_surface" {
                        // Receive IOSurface Mach port
                        let port = msg.copy_mach_send("iosurface_port");
                        let handle = unsafe { IOSurfaceLookupFromMachPort(port) };

                        if handle.is_null() {
                            eprintln!("FAILED: IOSurfaceLookupFromMachPort returned NULL");
                            std::process::exit(1);
                        }

                        // Verify dimensions
                        let width = unsafe { IOSurfaceGetWidth(handle) };
                        let height = unsafe { IOSurfaceGetHeight(handle) };

                        if width != 100 || height != 100 {
                            eprintln!("FAILED: Expected 100x100, got {}x{}", width, height);
                            std::process::exit(1);
                        }

                        // Verify pixel color (hot pink: 0xFF69B4)
                        let pixel = read_pixel(handle, 0, 0);
                        let expected = 0xFF69B4FF; // RGBA

                        if pixel != expected {
                            eprintln!("FAILED: Expected pixel 0x{:08X}, got 0x{:08X}",
                                     expected, pixel);
                            std::process::exit(1);
                        }

                        println!("SUCCESS: Received 100x100 pink IOSurface via XPC!");
                        std::process::exit(0);
                    }
                }
                Err(e) => {
                    eprintln!("Receiver: Peer error: {}", e);
                }
            }
        });
        peer.resume();
    });
    listener.resume();

    // 4. Ask launcher to spawn sender with our endpoint
    let msg = XpcDictionary::new();
    msg.set_string("action", "spawn_sender");
    msg.set_string("session_id", "test-1");
    msg.set_endpoint("receiver_endpoint", endpoint);
    launcher.send(&msg);
    println!("Receiver: Requested sender spawn");

    // 5. Run event loop
    println!("Receiver: Waiting for sender...");
    run_loop();
}
```

##### 3. Sender (Rust)

**Location:** `ts3/termsurf-xpc/examples/sender.rs`

Tests the "profile server side" — spawned by launcher, claims endpoint, sends
IOSurface:

```rust
// examples/sender.rs
use termsurf_xpc::*;
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    session: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    println!("Sender: Starting for session '{}'", args.session);

    // 1. Connect to launcher
    let launcher = XpcConnection::connect_mach_service("com.termsurf.xpc-test")?;
    launcher.resume();
    println!("Sender: Connected to launcher");

    // 2. Claim session, get receiver endpoint
    let msg = XpcDictionary::new();
    msg.set_string("action", "claim_session");
    msg.set_string("session_id", &args.session);
    let reply = launcher.send_with_reply_sync(&msg)?;

    let receiver_endpoint = reply.get_endpoint("endpoint")
        .ok_or_else(|| XpcError::Unknown("No endpoint in reply".into()))?;
    println!("Sender: Got receiver endpoint");

    // 3. Connect directly to receiver
    let receiver = XpcConnection::from_endpoint(receiver_endpoint)?;
    set_event_handler(&receiver, |event| {
        if let Err(e) = event {
            eprintln!("Sender: Receiver error: {}", e);
        }
    });
    receiver.resume();
    println!("Sender: Connected to receiver");

    // 4. Create pink IOSurface (100x100, hot pink 0xFF69B4)
    let surface = create_iosurface(100, 100)?;
    fill_with_color(surface, 0xFF, 0x69, 0xB4, 0xFF); // Hot pink, full alpha
    println!("Sender: Created 100x100 pink IOSurface");

    // 5. Send Mach port to receiver
    let port = unsafe { IOSurfaceCreateMachPort(surface) };
    if port == 0 {
        return Err(XpcError::Unknown("IOSurfaceCreateMachPort failed".into()));
    }

    let msg = XpcDictionary::new();
    msg.set_string("action", "send_surface");
    msg.set_mach_send("iosurface_port", port);
    msg.set_int64("width", 100);
    msg.set_int64("height", 100);
    receiver.send(&msg);
    println!("Sender: Sent IOSurface Mach port");

    // 6. Keep alive briefly to ensure message delivered
    std::thread::sleep(std::time::Duration::from_secs(2));
    println!("Sender: Done");

    Ok(())
}
```

#### Directory Structure

```
ts3/termsurf-xpc/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── connection.rs
│   ├── listener.rs
│   ├── dictionary.rs
│   ├── error.rs
│   ├── ffi.rs
│   ├── iosurface.rs              # IOSurface FFI bindings
│   ├── block.rs                  # Safe block wrappers (uses block2)
│   └── runloop.rs                # CFRunLoop wrapper
├── examples/
│   ├── launcher.rs               # XPC service (spawns sender)
│   ├── receiver.rs               # Test receiver (simulates GUI)
│   └── sender.rs                 # Test sender (simulates profile server)
├── xpc-service/
│   └── Info.plist                # XPC service bundle metadata
└── scripts/
    ├── build-test.sh             # Build everything
    └── run-test.sh               # Run the test
```

#### Build Script

```bash
#!/bin/bash
# ts3/termsurf-xpc/scripts/build-test.sh

set -e
cd "$(dirname "$0")/.."

echo "=== Building XPC Test Bundle ==="

# Build all Rust binaries
cargo build --release --example launcher
cargo build --release --example receiver
cargo build --release --example sender

# Create test app bundle structure
APP="TestXPC.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS"
mkdir -p "$APP/Contents/XPCServices/com.termsurf.xpc-test.xpc/Contents/MacOS"

# Copy binaries
cp ../../target/release/examples/receiver "$APP/Contents/MacOS/"
cp ../../target/release/examples/sender "$APP/Contents/MacOS/"
cp ../../target/release/examples/launcher \
   "$APP/Contents/XPCServices/com.termsurf.xpc-test.xpc/Contents/MacOS/"

# Copy XPC service Info.plist
cp xpc-service/Info.plist \
   "$APP/Contents/XPCServices/com.termsurf.xpc-test.xpc/Contents/"

# Create app Info.plist
cat > "$APP/Contents/Info.plist" << 'EOF'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleIdentifier</key>
    <string>com.termsurf.xpc-test</string>
    <key>CFBundleExecutable</key>
    <string>receiver</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
</dict>
</plist>
EOF

# Sign the XPC service (required for launchd to load it)
codesign --force --sign - \
    "$APP/Contents/XPCServices/com.termsurf.xpc-test.xpc"

# Sign the app
codesign --force --sign - "$APP"

echo "=== Build complete: $APP ==="
```

#### Test Script

```bash
#!/bin/bash
# ts3/termsurf-xpc/scripts/run-test.sh

set -e
cd "$(dirname "$0")/.."

# Build first
./scripts/build-test.sh

echo ""
echo "=== Running XPC Test ==="
echo ""

# Run receiver (it will ask launcher to spawn sender)
# Timeout after 30 seconds
timeout 30 ./TestXPC.app/Contents/MacOS/receiver
EXIT_CODE=$?

echo ""
if [ $EXIT_CODE -eq 0 ]; then
    echo "=== TEST PASSED ==="
else
    echo "=== TEST FAILED (exit code: $EXIT_CODE) ==="
    exit 1
fi
```

#### Success Criteria

- [ ] XPC service starts when receiver connects (check Activity Monitor)
- [ ] Launcher successfully spawns sender as child process
- [ ] Sender connects to launcher and claims session
- [ ] Sender receives receiver's endpoint
- [ ] Sender connects directly to receiver via endpoint
- [ ] Sender creates IOSurface (100x100 pink)
- [ ] `IOSurfaceCreateMachPort()` succeeds
- [ ] Receiver receives Mach port
- [ ] `IOSurfaceLookupFromMachPort()` returns valid handle
- [ ] IOSurface dimensions are 100x100
- [ ] Pixel at (0,0) is hot pink (0xFF69B4)
- [ ] Test prints "SUCCESS" and exits 0

#### Failure Modes

| Symptom                               | Likely Cause                                           |
| ------------------------------------- | ------------------------------------------------------ |
| "Connection refused" on launcher      | XPC service not bundled correctly, check Console.app   |
| Sender never starts                   | Launcher can't spawn (sandbox?), check entitlements    |
| Sender starts but can't connect       | Timing issue, or XPC service crashed after spawn       |
| "No endpoint in reply"                | Session not registered, race condition                 |
| Direct connection fails               | Endpoint invalid, or anonymous listener pattern broken |
| `IOSurfaceCreateMachPort` returns 0   | IOSurface not created correctly                        |
| `IOSurfaceLookupFromMachPort` is NULL | Mach port not transferred, or wrong port type          |
| Wrong dimensions/color                | IOSurface creation bug (unrelated to XPC)              |
| Test hangs                            | Event handler not called, check block implementation   |

#### Debugging

**Console.app filters:**

- Process: `com.termsurf.xpc-test`
- Process: `receiver`
- Process: `sender`

**Check XPC service is running:**

```bash
launchctl list | grep termsurf
```

**Check code signing:**

```bash
codesign -dvv TestXPC.app
codesign -dvv TestXPC.app/Contents/XPCServices/com.termsurf.xpc-test.xpc
```

#### Dependencies

Before running this experiment, complete these in `termsurf-xpc`:

1. **Block-based event handlers** (`src/block.rs`)
   - Use `block2` crate to create Objective-C blocks from Rust closures
   - Implement `set_event_handler()` and `set_new_connection_handler()`
   - Handle block lifetime correctly (prevent use-after-free)

2. **IOSurface bindings** (`src/iosurface.rs`)
   - `IOSurfaceCreate()` — create new surface
   - `IOSurfaceGetBaseAddress()` / `IOSurfaceLock()` / `IOSurfaceUnlock()` —
     pixel access
   - `IOSurfaceCreateMachPort()` — create sendable port
   - `IOSurfaceLookupFromMachPort()` — reconstruct from port
   - `IOSurfaceGetWidth()` / `IOSurfaceGetHeight()` — dimension queries
   - Helper: `create_iosurface(w, h)`, `fill_with_color()`, `read_pixel()`

3. **Run loop** (`src/runloop.rs`)
   - `CFRunLoopRun()` wrapper
   - Or use `dispatch_main()` if using dispatch queues

4. **Verify anonymous listener pattern**
   - Test that `xpc_connection_create_mach_service(NULL, queue, LISTENER)` works
   - If not, research alternative approaches

#### After This Experiment

With all XPC primitives validated, Experiment 2 integrates with the real app:

1. Move launcher into `WezTerm.app/Contents/XPCServices/`
2. Replace receiver logic → `wezterm-gui` XPC client
3. Replace sender → profile server with CEF
4. Replace pink IOSurface → CEF's `on_accelerated_paint` surface
5. Render received texture in terminal pane
6. Replace pink IOSurface with CEF `on_accelerated_paint` surface
7. Render to actual terminal pane

---

### Experiment 2: XPC IOSurface Transfer with Test Texture

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

##### 1. Launcher XPC Service (`ts3/termsurf-launcher/`) — Swift, macOS-only

**Why macOS-only?** The launcher exists solely because XPC is required for Mach
port transfer. Other platforms don't need it:

- **Linux:** DMA-BUF file descriptors pass over Unix sockets via `SCM_RIGHTS`
- **Windows:** `DuplicateHandle()` copies DXGI handles between processes

On Linux/Windows, the GUI spawns profile servers directly and passes texture
handles over existing IPC. No launcher needed.

**Why Swift?** Swift has first-class XPC support (`NSXPCConnection`,
`NSXPCListener`). No need for Rust FFI bindings to XPC APIs. The launcher is
small (~100 lines) and macOS-specific, so using a macOS-native language makes
sense.

Minimal XPC service that relays endpoints between GUI and spawned processes:

```swift
// termsurf-launcher/main.swift
import Foundation

class LauncherDelegate: NSObject, NSXPCListenerDelegate {
    var pendingSessions: [String: NSXPCListenerEndpoint] = [:]

    func listener(_ listener: NSXPCListener,
                  shouldAcceptNewConnection conn: NSXPCConnection) -> Bool {
        conn.exportedInterface = NSXPCInterface(with: LauncherProtocol.self)
        conn.exportedObject = self
        conn.resume()
        return true
    }

    func spawnProfile(endpoint: NSXPCListenerEndpoint,
                      profile: String,
                      sessionId: String) {
        pendingSessions[sessionId] = endpoint
        let task = Process()
        task.executableURL = Bundle.main.url(forAuxiliaryExecutable: "termsurf-test-sender")
        task.arguments = ["--session-id", sessionId]
        try? task.run()
    }

    func claimSession(sessionId: String) -> NSXPCListenerEndpoint? {
        return pendingSessions.removeValue(forKey: sessionId)
    }
}

let delegate = LauncherDelegate()
let listener = NSXPCListener(machServiceName: "com.termsurf.launcher")
listener.delegate = delegate
listener.resume()
RunLoop.main.run()
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

| File                                                   | Purpose                              |
| ------------------------------------------------------ | ------------------------------------ |
| `ts3/termsurf-launcher/main.swift`                     | XPC service implementation (Swift)   |
| `ts3/termsurf-launcher/LauncherProtocol.swift`         | XPC protocol definition              |
| `ts3/termsurf-launcher/Info.plist`                     | XPC service registration             |
| `ts3/termsurf-launcher/termsurf-launcher.entitlements` | Sandbox disabled                     |
| `ts3/termsurf-test-sender/Cargo.toml`                  | Test sender crate manifest           |
| `ts3/termsurf-test-sender/src/main.rs`                 | IOSurface creation + send            |
| `ts3/wezterm-gui/src/termwindow/webview_xpc.rs`        | XPC client + listener (Rust via FFI) |

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
