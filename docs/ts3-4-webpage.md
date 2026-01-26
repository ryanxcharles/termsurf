# TermSurf 3.0 Webpage Rendering

## Background

This document continues from [ts3-3-xpc.md](./ts3-3-xpc.md), which solved
cross-process GPU texture sharing on macOS.

### What We Accomplished (ts3-3-xpc)

**The Problem:** TermSurf 3.0 runs CEF in a separate process (profile server)
for browser isolation. The GUI needs to display textures rendered by CEF, but
macOS deprecated global IOSurface ID lookup in 2015. There was no obvious way to
share GPU textures between unrelated processes.

**The Solution:** XPC with Mach port transfer. After investigating and rejecting
several approaches (global IOSurface IDs, process ancestry, bootstrap
registration), we determined that XPC is the only supported mechanism for
transferring Mach port rights between processes on modern macOS.

**What We Built:**

| Component              | Purpose                                                               |
| ---------------------- | --------------------------------------------------------------------- |
| `termsurf-xpc`         | Rust bindings for XPC (connections, listeners, endpoints, Mach ports) |
| `termsurf-launcher`    | XPC service that spawns profile servers and relays endpoints          |
| `termsurf-test-sender` | Test process that creates a pink IOSurface and sends it via XPC       |
| `webview_xpc.rs`       | GUI-side XPC manager for receiving Mach ports                         |
| `webview_shader.wgsl`  | Shader for rendering webview textures                                 |

**Validation:** Running `web google.com` displayed a pink 100x100 texture
stretched to fill the terminal window. This proved the complete IPC pipeline:

```
web CLI → Unix socket → GUI → XPC → launcher → test-sender
                                                    │
                              IOSurface Mach port ──┘
                                                    │
GUI ← IOSurfaceLookupFromMachPort ← XPC ────────────┘
  │
  └── wgpu texture import → render pipeline → pink screen
```

### New Goal

Replace the pink test texture with a real webpage rendered by CEF.

**Critical requirement:** Profile isolation must work from the start. This is
the entire reason ts3 exists. Each webview must use a named profile with its own
cookies, storage, and cache directory.

**Success looks like:**

```
$ web --profile myprofile google.com
```

- Google.com renders in the terminal pane (not pink)
- `~/.config/termsurf/cef/myprofile/` directory is created
- Different `--profile` values create different directories
- Profiles are isolated (logging into Google in one profile doesn't affect
  others)

### Next Steps (After This Document)

Once basic webpage rendering with profiles works:

1. **Multiple pages** — Open multiple webviews with different profiles
   simultaneously
2. **Keyboard input** — Type in form fields, use keyboard shortcuts
3. **Mouse input** — Click links, scroll, hover states
4. **Resize handling** — CEF resizes when pane resizes, sends new IOSurface
5. **Navigation** — Back, forward, reload, URL changes
6. **Page lifecycle** — Handle page loads, errors, redirects
7. **DevTools** — Open Chrome DevTools for debugging

## Experiments

### Experiment 1: CEF Profile Server (Display Only)

**Status:** PLANNED

**Goal:** Create `termsurf-profile`, a CEF-based profile server that renders
real webpages and sends them to the GUI via XPC. Verify that profile directories
are created correctly.

**Scope:** Display only. No keyboard input, no mouse input, no scrolling, no
clicking. The page renders once and remains static. Interactivity is a separate
future experiment.

#### What the User Sees

```
$ web --profile myprofile google.com
```

- Terminal pane shows Google's homepage (not pink)
- Page is static (no scrolling, clicking, or typing — display only)
- `~/.config/termsurf/cef/myprofile/` exists with CEF data files
- Ctrl+C exits cleanly

#### Architecture

Same as ts3-3-xpc Experiment 2, but `termsurf-profile` replaces
`termsurf-test-sender`:

```
web CLI                    GUI                      Launcher              termsurf-profile
───────                    ───                      ────────              ────────────────
    │                       │                          │                         │
    │── open_webview ──────>│                          │                         │
    │   {url, profile}      │                          │                         │
    │                       │── spawn_profile ────────>│                         │
    │                       │   {session, endpoint}    │── spawn ───────────────>│
    │                       │                          │   --profile myprofile   │
    │                       │                          │   --url google.com      │
    │                       │                          │   --session-id UUID     │
    │                       │                          │                         │
    │                       │                          │<── claim_session ───────│
    │                       │                          │── endpoint ────────────>│
    │                       │                          │                         │
    │                       │<══════════ XPC (direct) ════════════════════════>│
    │                       │                          │                         │
    │                       │                          │    CEF init:            │
    │                       │                          │    cache_path =         │
    │                       │                          │    ~/.config/termsurf/  │
    │                       │                          │    cef/myprofile/       │
    │                       │                          │                         │
    │                       │                          │    Create browser       │
    │                       │                          │    Navigate to URL      │
    │                       │                          │                         │
    │                       │<── display_surface ──────────────────────────────│
    │                       │    {mach_port, w, h}     │    on_accelerated_paint │
    │                       │                          │                         │
    │                       │    Import IOSurface      │                         │
    │                       │    Render to pane        │                         │
    │                       │                          │                         │
    │<── response ─────────│                          │                         │
```

#### CEF Multi-Process Architecture

CEF inherits Chromium's multi-process design. When `termsurf-profile` calls
`cef_initialize()`, CEF spawns several child subprocesses:

| Subprocess | Purpose |
| ---------- | ------- |
| GPU | Hardware-accelerated rendering |
| Renderer | V8 JavaScript, DOM, layout |
| Utility | Network, audio, etc. |
| Alerts | System dialogs (macOS only) |

These subprocesses run a **helper binary** — a minimal executable that calls
`execute_process()` and lets CEF determine the subprocess role from command-line
arguments. The helper binary already exists in cef-rs as `cefsimple_helper`.

All profile processes share the same helper binary from the app bundle:

```
TermSurf.app/Contents/
├── MacOS/
│   ├── wezterm-gui               (main GUI)
│   ├── termsurf-profile          (profile server)
│   └── termsurf-test-sender      (test, will be deleted)
├── Frameworks/
│   ├── Chromium Embedded Framework.framework/
│   ├── TermSurf Helper.app/              ◄── shared by all profiles
│   ├── TermSurf Helper (GPU).app/
│   ├── TermSurf Helper (Renderer).app/
│   ├── TermSurf Helper (Plugin).app/
│   └── TermSurf Helper (Alerts).app/
└── XPCServices/
    └── com.termsurf.launcher.xpc/
```

#### Profile Isolation Strategy

**Why ts3 succeeds where ts2 couldn't:** ts2 discovered that CEF's per-browser
request contexts with custom `cache_path` don't work reliably. ts2 was forced
to use a single global context for all browsers.

ts3 solves this through **process separation**. Each profile runs in its own
`termsurf-profile` process with its own `cef_initialize()` call and its own
`root_cache_path`. CEF is fully isolated at the OS process level — no shared
state between profiles.

```
termsurf-profile --profile myprofile
    └── CEF instance (root_cache_path = ~/.config/termsurf/cef/myprofile/)
        ├── GPU subprocess
        ├── Renderer subprocess
        └── Utility subprocess

termsurf-profile --profile work
    └── CEF instance (root_cache_path = ~/.config/termsurf/cef/work/)
        ├── GPU subprocess
        ├── Renderer subprocess
        └── Utility subprocess
```

#### Components

##### 1. termsurf-profile (New Package)

**Location:** `ts3/termsurf-profile/`

CEF-based profile server. Combines:

- XPC session claiming from `termsurf-test-sender`
- CEF initialization with profile-specific cache path
- `on_accelerated_paint` handler that sends IOSurface via XPC

```rust
// ts3/termsurf-profile/src/main.rs (sketch)
use cef::*;
use clap::Parser;
use termsurf_xpc::*;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    profile: String,

    #[arg(long)]
    url: String,

    #[arg(long)]
    session_id: String,
}

fn main() {
    let args = Args::parse();
    let cef_args = cef::args::Args::new();

    // 0. Load CEF framework (macOS only)
    #[cfg(target_os = "macos")]
    let _loader = {
        let loader = cef::library_loader::LibraryLoader::new(
            &std::env::current_exe().unwrap(),
            false, // not a helper
        );
        assert!(loader.load());
        loader
    };

    // 1. Handle CEF subprocess (returns early if this is a subprocess)
    let exit_code = cef::execute_process(
        Some(cef_args.as_main_args()),
        None::<&mut App>,
        std::ptr::null_mut(),
    );
    if exit_code >= 0 {
        std::process::exit(exit_code);
    }

    // 2. Claim session and connect to GUI via XPC (same as test-sender)
    let gui = claim_and_connect(&args.session_id);

    // 3. Compute paths
    let exe = std::env::current_exe().unwrap();
    let app_contents = exe.parent().unwrap().parent().unwrap();

    let helper_path = app_contents
        .join("Frameworks")
        .join("TermSurf Helper.app")
        .join("Contents/MacOS/TermSurf Helper");

    let cache_path = dirs::config_dir()
        .unwrap()
        .join("termsurf/cef")
        .join(&args.profile);

    // 4. Initialize CEF with profile-specific settings
    let settings = Settings {
        windowless_rendering_enabled: 1,
        external_message_pump: 1,       // Integrate with CFRunLoop
        no_sandbox: 1,                  // Required for development
        root_cache_path: CefString::from(cache_path.to_str().unwrap()),
        browser_subprocess_path: CefString::from(helper_path.to_str().unwrap()),
        persist_session_cookies: 1,
        ..Default::default()
    };

    let mut app = create_app(); // BrowserProcessHandler for message pump
    cef::initialize(
        Some(cef_args.as_main_args()),
        Some(&settings),
        Some(&mut app),
        std::ptr::null_mut(),
    ).expect("CEF initialization failed");

    // 5. Create render handler that sends IOSurface via XPC
    let render_handler = ProfileRenderHandler::new(gui.clone());

    // 6. Create browser with off-screen rendering
    let window_info = WindowInfo {
        windowless_rendering_enabled: 1,
        shared_texture_enabled: 1, // Critical: enables IOSurface on macOS
        ..Default::default()
    };

    let browser_settings = BrowserSettings {
        windowless_frame_rate: 60,
        ..Default::default()
    };

    let browser = cef::browser_host_create_browser_sync(
        Some(&window_info),
        Some(&mut build_client(render_handler)),
        Some(&args.url.into()),
        Some(&browser_settings),
        None, // extra_info
        None, // request_context (uses global with our root_cache_path)
    );

    // 7. Run CEF message loop
    cef::run_message_loop();
    cef::shutdown();
}

struct ProfileRenderHandler {
    gui: Arc<XpcConnection>,
}

impl RenderHandler for ProfileRenderHandler {
    fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
        // Return initial size (resize not supported in this experiment)
        if let Some(rect) = rect {
            rect.width = 800;
            rect.height = 600;
        }
    }

    fn screen_info(
        &self,
        _browser: Option<&mut Browser>,
        screen_info: Option<&mut ScreenInfo>,
    ) -> i32 {
        if let Some(info) = screen_info {
            // macOS base DPI is 72; Retina is typically 144 (2x)
            info.device_scale_factor = 2.0;
            return 1;
        }
        0
    }

    fn on_accelerated_paint(
        &self,
        _browser: Option<&mut Browser>,
        _type: PaintElementType,
        _dirty_rects: Option<&[Rect]>,
        info: Option<&AcceleratedPaintInfo>,
    ) {
        let Some(info) = info else { return };

        // Create Mach port from IOSurface
        let port = unsafe { IOSurfaceCreateMachPort(info.shared_texture_io_surface) };
        if port == 0 { return; }

        // Send to GUI via XPC
        let msg = XpcDictionary::new();
        msg.set_string("action", "display_surface");
        msg.set_mach_send("iosurface_port", port);
        msg.set_i64("width", info.extra.coded_size.width);
        msg.set_i64("height", info.extra.coded_size.height);
        self.gui.send(&msg);
    }
}
```

**Key differences from the earlier sketch:**

- Loads CEF framework via `LibraryLoader` (macOS requirement)
- Calls `execute_process()` first for subprocess handling
- Sets `browser_subprocess_path` to shared helper binary
- Sets `external_message_pump: 1` for CFRunLoop integration
- Implements `view_rect()` and `screen_info()` for proper DPI handling
- Uses `shared_texture_enabled: 1` in WindowInfo for IOSurface
- Sets `windowless_frame_rate: 60`

##### 2. CEF Helper Binary

The helper binary already exists in cef-rs as `cefsimple_helper`. It is bundled
into the app by the build scripts. All profile processes point to the same
helper via `browser_subprocess_path`.

The helper is minimal:

```rust
fn main() {
    let args = Args::new();
    let _loader = LibraryLoader::new(&std::env::current_exe().unwrap(), true);
    execute_process(Some(args.as_main_args()), None::<&mut App>, std::ptr::null_mut());
}
```

##### 3. Launcher Modification

Update `termsurf-launcher` to spawn `termsurf-profile` instead of
`termsurf-test-sender`. Pass `--profile`, `--url`, and `--session-id` arguments.

The profile and URL must be passed from the GUI to the launcher in the
`spawn_profile` message.

##### 4. GUI Modification

Update `webview_socket.rs` to extract the profile name from `open_webview` and
pass it to the XPC manager.

Update `webview_xpc.rs` to include profile and URL in the `spawn_profile`
message to the launcher.

##### 5. Web CLI Modification

Add `--profile` flag to the `web` command. Include profile in the `open_webview`
message sent to the GUI.

```
$ web --profile myprofile google.com
$ web google.com  # Uses "default" profile
```

#### CEF Initialization Details

**Profile directory structure:**

```
~/.config/termsurf/cef/
├── myprofile/
│   ├── Cache/
│   ├── Cookies
│   ├── Local Storage/
│   └── ...
├── otherprofile/
│   └── ...
└── default/
    └── ...
```

**CefSettings (complete):**

```rust
Settings {
    // Profile-specific storage
    root_cache_path: "~/.config/termsurf/cef/{profile}/",

    // Enable off-screen rendering (no visible window)
    windowless_rendering_enabled: 1,

    // Use external message pump (CFRunLoop on macOS)
    // Required for proper event loop integration
    external_message_pump: 1,

    // Path to shared helper binary
    browser_subprocess_path: ".../TermSurf Helper.app/Contents/MacOS/TermSurf Helper",

    // Disable sandbox for development
    no_sandbox: 1,

    // Persist cookies across sessions
    persist_session_cookies: 1,
}
```

**Browser creation:**

```rust
let window_info = WindowInfo {
    windowless_rendering_enabled: 1,
    shared_texture_enabled: 1,      // Critical: enables IOSurface on macOS
    external_begin_frame_enabled: 0, // Let CEF control frame timing
    ..Default::default()
};

let browser_settings = BrowserSettings {
    windowless_frame_rate: 60,
    ..Default::default()
};

cef::browser_host_create_browser_sync(
    Some(&window_info),
    Some(&mut client),  // Has our RenderHandler
    Some(&url.into()),
    Some(&browser_settings),
    None,  // extra_info
    None,  // request_context (uses global with our root_cache_path)
);
```

#### Device Scale Factor

CEF operates in logical pixels (DIP — device-independent pixels). macOS has a
base DPI of 72, so Retina displays have a scale factor of 2.0.

The `screen_info()` handler must report the correct scale factor so CEF renders
at the right resolution. For this experiment, we hardcode 2.0 (Retina). A future
experiment will read the actual DPI from the GUI.

#### Message Pump

ts2 uses `external_message_pump: 1`, which means CEF doesn't run its own event
loop. Instead, it calls `on_schedule_message_pump_work(delay_ms)` on the
BrowserProcessHandler, and we schedule a CFRunLoop timer to call
`cef::do_message_loop_work()` at the right time.

For this experiment, we can use `cef::run_message_loop()` as a simpler
alternative (CEF manages its own loop). If this causes issues with XPC event
delivery, we switch to external message pump with CFRunLoop integration.

#### Files to Create

| File                               | Purpose            |
| ---------------------------------- | ------------------ |
| `ts3/termsurf-profile/Cargo.toml`  | Package manifest   |
| `ts3/termsurf-profile/src/main.rs` | CEF profile server |

#### Files to Modify

| File                                               | Changes                                         |
| -------------------------------------------------- | ----------------------------------------------- |
| `ts3/termsurf-launcher/src/main.rs`                | Spawn `termsurf-profile`, pass profile/URL args |
| `ts3/termsurf-web/src/main.rs`                     | Add `--profile` flag, include in open_webview   |
| `ts3/wezterm-gui/src/termwindow/webview_socket.rs` | Extract profile from request                    |
| `ts3/wezterm-gui/src/termwindow/webview_xpc.rs`    | Pass profile/URL to launcher                    |
| `ts3/Cargo.toml`                                   | Add termsurf-profile to workspace               |
| Build scripts                                      | Bundle termsurf-profile and helper in app       |

#### Success Criteria

- [ ] `web --profile myprofile google.com` renders Google homepage in pane
- [ ] `~/.config/termsurf/cef/myprofile/` directory exists after running
- [ ] `web --profile other google.com` creates `~/.config/termsurf/cef/other/`
- [ ] Page content is visible (not pink, not black, not error screen)
- [ ] Ctrl+C exits cleanly
- [ ] No CEF crashes or GPU errors in logs

**Out of scope for this experiment:**

- Keyboard input (typing in search box)
- Mouse input (clicking links, scrolling)
- Page resize (window resize updates texture)
- Navigation (back, forward, URL changes)

#### What This Proves

1. **CEF initialization works** in the profile server process
2. **Profile isolation works** — each profile gets its own directory
3. **on_accelerated_paint works** — CEF sends IOSurface to our handler
4. **End-to-end rendering works** — real webpage pixels reach the screen

This experiment validates rendering only. Interactivity (keyboard, mouse) will
be proven in subsequent experiments.

#### After This Experiment

With webpage rendering working:

1. Delete `termsurf-test-sender` (no longer needed)
2. Proceed to keyboard/mouse input handling
3. Add resize support (CEF resize → new IOSurface → GUI update)
