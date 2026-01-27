# TermSurf 3.0 One-Process-Per-Profile

## Background

### Progress So Far

ts3 has established a working pipeline for rendering webpages in terminal panes:

- **ts3-1 through ts3-3:** Designed the out-of-process architecture. The GUI
  (WezTerm) communicates with a launcher XPC service, which spawns profile
  server processes. Profile servers run CEF off-screen rendering and send
  IOSurface Mach ports back to the GUI for display.
- **ts3-4:** Got a webpage (google.com) rendering in a terminal pane. The full
  pipeline works: CLI → Unix socket → GUI → XPC → launcher → profile server →
  CEF → IOSurface → Mach port → GUI → wgpu → screen.
- **ts3-5:** Fixed profile path isolation. Each profile stores its CEF data at
  `~/.config/termsurf/cef/<profile>/` instead of the macOS-specific
  `~/Library/Application Support/`.
- **ts3-6:** Removed hardcoded 800x600 dimensions. The GUI now reads pane pixel
  dimensions and DPI from the Mux, computes logical size and scale factor, and
  passes them to the profile server at startup. CEF renders at the correct pane
  size on Retina displays.

### The Problem

The current code spawns a new `termsurf-profile` process for every `web`
command. This violates the foundational architectural constraint of ts3: **there
must be exactly one process per browser profile.**

CEF's `SingletonLock` file prevents two processes from opening the same
`root_cache_path`. If a user runs `web google.com` and then `web github.com`
with the same profile, the second process will crash or fail to initialize.

This is not a bug in our code -- it is how CEF and Chromium are designed. One
`root_cache_path` = one process. This constraint is the entire reason ts3 moved
CEF out-of-process: to support multiple profiles, each needs its own process.

## Goal

Implement one-process-per-profile so that multiple webviews can share a single
CEF process, like tabs in a browser.

**Product requirements:**

1. A user can open many different webviews for the same profile (e.g.,
   `web google.com` and `web github.com` both using the `default` profile). Each
   webview renders in its own pane with its own size and URL.
2. A user can open webviews across many different profiles (e.g., `default`,
   `work`, `personal`). Each profile gets its own process with its own cookies,
   storage, and cache.
3. There is always exactly one `termsurf-profile` process per profile,
   containing exactly one CEF instance. Multiple webviews within that process
   are separate CEF browser instances sharing the same CEF context.
4. All cross-process GPU texture sharing continues to use XPC Mach port
   transfer. Each webview has its own IOSurface and its own Mach port sent to
   the GUI.

**Success looks like:**

- `web google.com` opens in pane 1 -- profile process starts, page renders
- `web github.com` opens in pane 2 (same profile) -- no new process, second
  browser created in the existing profile process, page renders in pane 2
- `web --profile work gitlab.com` opens in pane 3 -- new profile process starts
  for `work`, page renders in pane 3
- All three panes display their respective pages simultaneously
- Closing a pane destroys only that browser, not the entire profile process
- Closing all panes for a profile shuts down that profile process

## Tasks

- [ ] Launcher tracks running profile processes (PID + connection per profile)
- [ ] Launcher routes `spawn_profile` to existing process if profile is running
- [ ] Profile server accepts "create browser" commands for additional webviews
- [ ] Profile server manages multiple browsers with separate sizes, URLs, and
      IOSurfaces
- [ ] Each browser's IOSurface Mach port is sent to the correct GUI pane
- [ ] GUI correctly maps incoming surfaces to the right pane when multiple
      webviews share a profile process
- [ ] Closing a pane sends a "destroy browser" command to the profile server
- [ ] Profile server shuts down when its last browser is destroyed

## Deferred Work

The following features were planned in ts3-6 but are blocked until
one-process-per-profile is implemented. They will be addressed in subsequent
documents after this architecture is in place:

- **Dynamic resize** -- Send new pane dimensions to the profile server via XPC
  when the window resizes or panes are split. Requires bidirectional XPC
  communication (GUI → profile) and calling `host.was_resized()` on the correct
  browser instance. ts2's settle delay (30ms) is a fallback if bouncing recurs.
- **Keyboard input** -- Forward keystrokes to CEF for typing in form fields and
  using keyboard shortcuts.
- **Mouse input** -- Forward clicks, scrolling, and hover events to CEF for
  interacting with page elements.
- **Navigation** -- Back, forward, reload, and URL bar changes.
- **Page lifecycle** -- Handle page loads, errors, redirects, and title updates.
- **DevTools** -- Open Chrome DevTools for debugging webview content.

## Research: Current Architecture and What Must Change

### Current Flow (Single Browser Per Process)

```
1. CLI sends "open_webview" to GUI via Unix socket
2. GUI creates anonymous XPC listener for this pane, gets endpoint
3. GUI sends "spawn_profile" to launcher (includes gui_endpoint, URL, dimensions)
4. Launcher stores gui_endpoint, spawns new termsurf-profile process
5. Profile process starts, claims session from launcher (gets gui_endpoint)
6. Profile connects to GUI via endpoint
7. Profile initializes CEF, creates ONE browser in on_context_initialized
8. on_accelerated_paint sends IOSurface Mach port to GUI
9. GUI receives surface, maps to pane via session_id
```

Every `web` command repeats steps 1-9, spawning a new process every time. Step 4
always spawns -- there is no check for an existing process.

### What the Launcher Must Do

The launcher must become a **router**. When `spawn_profile` arrives:

- **First request for a profile:** Spawn the process (current behavior).
- **Subsequent requests for the same profile:** Forward the request to the
  existing process as a `create_browser` command.

To do this, the launcher needs:

1. A `running_profiles` map: `HashMap<String, ProfileProcessInfo>` where
   `ProfileProcessInfo` contains the profile process's XPC endpoint.
2. A `register_profile` action: after the profile server initializes CEF, it
   creates its own anonymous XPC listener and sends the endpoint to the
   launcher.
3. Modified `spawn_profile` logic: check `running_profiles` first.

### What the Profile Server Must Do

The profile server must become **multi-browser**. Currently it creates one
browser at startup and runs forever. It must:

1. After CEF init, create an anonymous XPC listener and register it with the
   launcher via `register_profile`.
2. Listen for `create_browser` commands on that listener. Each command includes
   a GUI endpoint, URL, width, height, and scale.
3. For each browser, create a separate `Client` + `RenderHandler` pair connected
   to that browser's GUI endpoint. Each render handler sends IOSurface Mach
   ports to its own pane.
4. The initial browser (from CLI args) is created in `on_context_initialized` as
   before. Subsequent browsers are created via XPC commands.

**Thread safety:** XPC callbacks run on libdispatch queues, not the CEF UI
thread. Browser creation must be marshalled to the CEF UI thread using
`cef::post_task(ThreadId::UI, ...)`.

**Shared state refactor:** Currently `SharedState` holds a single `url`,
`width`, `height`, and `gui` connection. This must become multi-browser:

```rust
struct BrowserState {
    gui: Arc<XpcConnection>,
    width: AtomicU32,
    height: AtomicU32,
    last_handle: AtomicPtr<c_void>,
}

struct ProfileState {
    scale: f32,
    browsers: Mutex<HashMap<String, Arc<BrowserState>>>,  // keyed by session_id
}
```

Each `RenderHandlerInner` holds an `Arc<BrowserState>` instead of the global
`SharedState`.

### What the GUI Must Change

Almost nothing. The GUI already creates a separate anonymous XPC listener per
pane, each with its own `session_id` → `pane_id` mapping. Whether the profile
server is new or reused, the GUI's listener receives the `display_surface`
message and maps it to the right pane.

The only change: `request_profile_spawn` currently always sends `spawn_profile`
to the launcher. This still works -- the launcher decides whether to spawn or
forward. The GUI doesn't need to know.

## Experiments

### Experiment 1: Multi-Browser Profile Server

**Status:** PLANNED

**Goal:** Implement one-process-per-profile across all three components so that
two `web` commands with the same profile share a single process.

#### Changes

**1. Launcher: Track running profiles and route requests**

**File:** `ts3/termsurf-launcher/src/main.rs`

Add tracking state:

```rust
// Track running profile processes
// profile_name -> XPC endpoint for sending commands to that process
let running_profiles: Arc<Mutex<HashMap<String, XpcEndpoint>>> =
    Arc::new(Mutex::new(HashMap::new()));
```

Add new `register_profile` action (called by profile server after CEF init):

```rust
"register_profile" => {
    let profile = msg.get_string("profile").unwrap();
    let endpoint = msg.get_endpoint("profile_endpoint").unwrap();
    running_profiles.lock().unwrap().insert(profile.clone(), endpoint);
    println!("Launcher: Registered profile '{}'", profile);
}
```

Modify `spawn_profile` to check for existing process:

```rust
"spawn_profile" => {
    let profile = msg.get_string("profile").unwrap_or("default".into());

    // Check if profile process is already running
    let existing = running_profiles.lock().unwrap().get(&profile).cloned();

    if let Some(profile_endpoint) = existing {
        // Forward to existing process as "create_browser"
        println!("Launcher: Profile '{}' already running, forwarding", profile);
        let conn = XpcConnection::from_endpoint(profile_endpoint).unwrap();
        set_event_handler(&conn, |_| {});
        conn.resume();

        let fwd = XpcDictionary::new();
        fwd.set_string("action", "create_browser");
        fwd.set_string("session_id", &session_id);
        fwd.set_string("url", &url);
        fwd.set_i64("width", width);
        fwd.set_i64("height", height);
        fwd.set_string("scale", &scale);
        fwd.set_endpoint("gui_endpoint", endpoint);  // from the GUI
        conn.send(&fwd);
    } else {
        // Spawn new process (current behavior)
        // ... existing spawn code ...
    }
}
```

**2. Profile server: Register with launcher after CEF init**

**File:** `ts3/termsurf-profile/src/main.rs`

After `cef::initialize()` succeeds and before `cef::run_message_loop()`, create
an anonymous XPC listener and register its endpoint with the launcher:

```rust
// Create listener for incoming commands from launcher
let cmd_listener = XpcListener::new_anonymous().unwrap();
let cmd_endpoint = cmd_listener.get_endpoint().unwrap();

// Set up handler for "create_browser" commands
let profile_state = Arc::clone(&shared);
set_new_connection_handler(&cmd_listener, move |conn| {
    let conn = Arc::new(conn);
    let state = Arc::clone(&profile_state);
    set_event_handler(&*conn, move |event| {
        if let Ok(msg) = event {
            let action = msg.get_string("action").unwrap_or_default();
            if action == "create_browser" {
                // Extract parameters and create browser on CEF UI thread
                // ... (see below)
            }
        }
    });
    conn.resume();
});
cmd_listener.resume();

// Register with launcher
let reg = XpcDictionary::new();
reg.set_string("action", "register_profile");
reg.set_string("profile", &args.profile);
reg.set_endpoint("profile_endpoint", cmd_endpoint);
launcher.send(&reg);
```

**3. Profile server: Handle `create_browser` commands**

**File:** `ts3/termsurf-profile/src/main.rs`

When a `create_browser` message arrives on the command listener:

1. Extract GUI endpoint, URL, width, height from the message.
2. Connect to the new GUI pane's endpoint.
3. Create a new `BrowserState` for this browser.
4. Post browser creation to the CEF UI thread via `cef::post_task`.

```rust
"create_browser" => {
    let gui_endpoint = msg.get_endpoint("gui_endpoint").unwrap();
    let url = msg.get_string("url").unwrap_or("about:blank".into());
    let width = msg.get_i64("width") as u32;
    let height = msg.get_i64("height") as u32;

    // Connect to GUI pane
    let gui = XpcConnection::from_endpoint(gui_endpoint).unwrap();
    set_event_handler(&gui, |event| {
        if let Err(e) = event { eprintln!("Profile: GUI error: {}", e); }
    });
    gui.resume();
    let gui = Arc::new(gui);

    // Create browser state
    let browser_state = Arc::new(BrowserState {
        gui,
        width: AtomicU32::new(width),
        height: AtomicU32::new(height),
        last_handle: AtomicPtr::new(std::ptr::null_mut()),
    });

    // Post browser creation to CEF UI thread
    // (XPC callbacks run on dispatch queues, not the CEF thread)
    let scale = state.scale;
    // Use cef::post_task(ThreadId::UI, task) to create browser
}
```

**4. Profile server: Refactor SharedState to support multiple browsers**

**File:** `ts3/termsurf-profile/src/main.rs`

Replace the current single-browser `SharedState` with a multi-browser
`ProfileState`:

```rust
struct BrowserState {
    gui: Arc<XpcConnection>,
    width: AtomicU32,
    height: AtomicU32,
    last_handle: AtomicPtr<c_void>,
}

struct ProfileState {
    scale: f32,
    profile: String,
    launcher: XpcConnection,  // keep alive for register_profile
}
```

Each `RenderHandlerInner` holds an `Arc<BrowserState>` for its specific browser.
The `view_rect()` and `screen_info()` methods read from `BrowserState` instead
of a global shared state.

The initial browser (from CLI args) creates its own `BrowserState` in
`on_context_initialized`, just like subsequent browsers created via
`create_browser`.

#### Files to Modify

| File                                | Changes                                                                        |
| ----------------------------------- | ------------------------------------------------------------------------------ |
| `ts3/termsurf-launcher/src/main.rs` | Track running profiles, route spawn vs create_browser                          |
| `ts3/termsurf-profile/src/main.rs`  | Register with launcher, handle create_browser, refactor to multi-browser state |

The GUI files (`webview_socket.rs`, `webview_xpc.rs`) should require no changes.

#### Verification

```bash
cd ts3
./scripts/build-debug.sh --open

# First webview -- should spawn profile process
web google.com

# Second webview, same profile -- should NOT spawn new process
web github.com

# Check logs
cat /tmp/termsurf-launcher.log
# Should show: "Registered profile 'default'" then "Profile 'default' already running, forwarding"

cat /tmp/termsurf-profile-*.log
# Should show: browser created for google.com, then browser created for github.com
# Only ONE log file (one process)

# Different profile -- should spawn new process
web --profile work gitlab.com

cat /tmp/termsurf-launcher.log
# Should show: new spawn for 'work' profile
```

#### Success Criteria

- [ ] First `web` command spawns a profile process and renders the page
- [ ] Second `web` command with the same profile does NOT spawn a new process
- [ ] Second `web` command creates a second browser in the existing process
- [ ] Both panes display their respective pages simultaneously
- [ ] A `web` command with a different `--profile` spawns a separate process
- [ ] Only one `/tmp/termsurf-profile-*.log` file per profile (not per webview)
- [ ] No crashes when opening multiple webviews
