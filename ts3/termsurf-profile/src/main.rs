//! CEF Profile Server for TermSurf.
//!
//! Renders webpages using CEF off-screen rendering and sends IOSurface
//! textures to the GUI via XPC Mach port transfer.
//!
//! Spawned by the launcher with: --profile, --url, --session-id, --width, --height, --scale
//!
//! Architecture:
//! 1. Load CEF framework, handle subprocess early return
//! 2. Connect to launcher, claim initial session
//! 3. Initialize CEF with profile-specific cache path
//! 4. Create command listener and register with launcher
//! 5. Create initial browser in on_context_initialized callback
//! 6. Handle create_browser commands from launcher for additional browsers
//! 7. on_accelerated_paint sends IOSurface Mach port to GUI (per-browser)
//! 8. run_message_loop() blocks until Ctrl+C

use clap::Parser;
use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicPtr, AtomicU32};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;
use termsurf_xpc::*;

/// Set NSApplication activation policy to Prohibited.
/// This prevents the process from appearing in the dock or stealing focus.
/// Must be called before CEF initializes NSApplication.
#[cfg(target_os = "macos")]
fn set_background_activation_policy() {
    use objc::{class, msg_send, sel, sel_impl};
    unsafe {
        let ns_app: *mut objc::runtime::Object = msg_send![class!(NSApplication), sharedApplication];
        // NSApplicationActivationPolicyProhibited = 2
        let _: () = msg_send![ns_app, setActivationPolicy: 2i64];
    }
}

#[derive(Parser)]
struct Args {
    #[arg(long)]
    profile: String,

    #[arg(long)]
    url: String,

    #[arg(long)]
    session_id: String,

    /// Logical width for CEF view_rect (physical pixels / scale)
    #[arg(long, default_value = "800")]
    width: u32,

    /// Logical height for CEF view_rect (physical pixels / scale)
    #[arg(long, default_value = "600")]
    height: u32,

    /// Device scale factor (e.g. 2.0 for Retina)
    #[arg(long, default_value = "2.0")]
    scale: f32,
}

fn main() {
    // Set activation policy FIRST, before CEF initializes NSApplication.
    // This prevents the process from appearing in the dock or stealing focus.
    #[cfg(target_os = "macos")]
    set_background_activation_policy();

    let args = Args::parse();
    println!(
        "Profile: Starting session='{}', url='{}', profile='{}', size={}x{}, scale={}",
        args.session_id, args.url, args.profile, args.width, args.height, args.scale
    );

    #[cfg(target_os = "macos")]
    run_profile_server(args);

    #[cfg(not(target_os = "macos"))]
    {
        let _ = args;
        eprintln!("Profile: CEF not supported on this platform");
        std::process::exit(1);
    }
}

// ============================================================================
// Profile State (multi-browser support)
// ============================================================================

/// Info for creating the initial browser (from CLI args)
struct InitialBrowserInfo {
    url: String,
    session_id: String,
    gui_endpoint: XpcEndpoint,
    width: u32,
    height: u32,
}

/// Per-browser state
struct BrowserState {
    session_id: String,
    gui: Arc<XpcConnection>,
    width: AtomicU32,
    height: AtomicU32,
    last_handle: AtomicPtr<c_void>,
    /// Browser reference for resize operations
    browser: Mutex<Option<cef::Browser>>,
}

/// Profile-wide state (shared across all browsers in this process)
struct ProfileState {
    scale: f32,
    profile: String,
    initial_browser_info: Mutex<Option<InitialBrowserInfo>>,
    browsers: Mutex<HashMap<i32, Arc<BrowserState>>>,
    command_connections: Mutex<Vec<Arc<XpcConnection>>>,
    /// Pending browser creation requests (session_id, url, gui_endpoint, width, height)
    pending_browsers: Mutex<Vec<(String, String, XpcEndpoint, u32, u32)>>,
}

/// Global profile state, set once during initialization
static PROFILE_STATE: OnceLock<Arc<ProfileState>> = OnceLock::new();

#[cfg(target_os = "macos")]
fn run_profile_server(args: Args) {
    use cef::library_loader::LibraryLoader;

    let exe = std::env::current_exe().expect("Failed to get executable path");
    println!("Profile: Executable: {:?}", exe);

    // 1. Load CEF framework
    let _loader = LibraryLoader::new(&exe, false);
    if !_loader.load() {
        eprintln!("Profile: Failed to load CEF framework");
        std::process::exit(1);
    }
    println!("Profile: CEF framework loaded");

    // Configure CEF API version (required before creating App objects)
    let _ = cef::api_hash(cef::sys::CEF_API_VERSION_LAST, 0);

    // 2. Subprocess check (early return for helper processes)
    let cef_args = cef::args::Args::new();
    let exit_code = cef::execute_process(
        Some(cef_args.as_main_args()),
        None::<&mut cef::App>,
        std::ptr::null_mut(),
    );
    if exit_code >= 0 {
        std::process::exit(exit_code);
    }
    println!("Profile: Main process (ret={})", exit_code);

    // 3. Connect to launcher
    println!("Profile: Connecting to launcher...");
    let launcher = XpcConnection::connect_mach_service("com.termsurf.launcher")
        .expect("Failed to connect to launcher");

    set_event_handler(&launcher, |event| {
        if let Err(e) = event {
            eprintln!("Profile: Launcher error: {}", e);
        }
    });
    launcher.resume();
    thread::sleep(Duration::from_millis(100));

    // 4. Claim initial session (gets gui_endpoint for first browser)
    println!("Profile: Claiming session '{}'...", args.session_id);
    let initial_gui_endpoint = claim_session_with_retry(&launcher, &args.session_id)
        .expect("Failed to claim session");
    println!("Profile: Got GUI endpoint for initial browser");

    // 5. Initialize ProfileState BEFORE CEF init
    let profile_state = Arc::new(ProfileState {
        scale: args.scale,
        profile: args.profile.clone(),
        initial_browser_info: Mutex::new(Some(InitialBrowserInfo {
            url: args.url.clone(),
            session_id: args.session_id.clone(),
            gui_endpoint: initial_gui_endpoint,
            width: args.width,
            height: args.height,
        })),
        browsers: Mutex::new(HashMap::new()),
        command_connections: Mutex::new(Vec::new()),
        pending_browsers: Mutex::new(Vec::new()),
    });
    // Store in global state (panics if already set, which shouldn't happen)
    let _ = PROFILE_STATE.set(Arc::clone(&profile_state));

    // 6. Compute paths
    let app_contents = exe.parent().unwrap().parent().unwrap();
    let helper_path = app_contents
        .join("Frameworks")
        .join("WezTerm Helper.app")
        .join("Contents/MacOS/WezTerm Helper");
    println!(
        "Profile: Helper: {:?} (exists={})",
        helper_path,
        helper_path.exists()
    );

    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
    let cache_path = std::path::PathBuf::from(home)
        .join(".config/termsurf/cef")
        .join(&args.profile);
    std::fs::create_dir_all(&cache_path).ok();
    println!("Profile: Cache: {:?}", cache_path);

    // 7. Initialize CEF
    let settings = cef::Settings {
        windowless_rendering_enabled: 1,
        no_sandbox: 1,
        root_cache_path: cef::CefString::from(cache_path.to_str().unwrap()),
        browser_subprocess_path: cef::CefString::from(helper_path.to_str().unwrap()),
        persist_session_cookies: 1,
        ..Default::default()
    };

    let mut app = cef_handlers::create_app(Arc::clone(&profile_state));

    let init_result = cef::initialize(
        Some(cef_args.as_main_args()),
        Some(&settings),
        Some(&mut app),
        std::ptr::null_mut(),
    );
    if init_result != 1 {
        eprintln!("Profile: CEF initialize failed (returned {})", init_result);
        std::process::exit(1);
    }
    println!("Profile: CEF initialized");

    // 8. Create command listener and register with launcher (AFTER CEF init, BEFORE message loop)
    let command_listener = XpcListener::new_anonymous().expect("Failed to create command listener");
    let command_endpoint = command_listener
        .get_endpoint()
        .expect("Failed to get command endpoint");

    // Set up handler for create_browser commands from launcher
    let profile_state_for_handler = Arc::clone(&profile_state);
    let launcher_for_claim = Arc::new(launcher);
    let launcher_for_handler = Arc::clone(&launcher_for_claim);

    set_new_connection_handler(&command_listener, move |conn| {
        println!("Profile: New command connection from launcher");
        let conn = Arc::new(conn);
        let state = Arc::clone(&profile_state_for_handler);
        let state_for_event = Arc::clone(&state);
        let launcher = Arc::clone(&launcher_for_handler);

        set_event_handler(&*conn, move |event| match event {
            Ok(msg) => {
                let action = msg.get_string("action").unwrap_or_default();
                println!("Profile: Received command action: {}", action);

                if action == "create_browser" {
                    handle_create_browser(&msg, &state_for_event, &launcher);
                }
            }
            Err(e) => {
                eprintln!("Profile: Command connection error: {}", e);
            }
        });
        conn.resume();

        state.command_connections.lock().unwrap().push(conn);
    });
    command_listener.resume();
    println!("Profile: Command listener ready");

    // Register with launcher so it can forward subsequent create_browser commands
    let register_msg = XpcDictionary::new();
    register_msg.set_string("action", "register_profile");
    register_msg.set_string("profile", &args.profile);
    register_msg.set_endpoint("endpoint", command_endpoint);
    launcher_for_claim.send(&register_msg);
    println!("Profile: Registered with launcher as '{}'", args.profile);

    // 9. Install Ctrl+C handler for clean shutdown
    ctrlc::set_handler(|| {
        println!("Profile: Ctrl+C, quitting...");
        cef::quit_message_loop();
    })
    .expect("Failed to set Ctrl+C handler");

    // 10. Run CEF message loop (blocks until quit_message_loop)
    // on_context_initialized fires during this loop, creating the initial browser.
    // on_accelerated_paint fires when pages render, sending IOSurface to GUI.
    println!("Profile: Running message loop...");
    cef::run_message_loop();

    // 11. Shutdown
    println!("Profile: Shutting down...");
    cef::shutdown();
    println!("Profile: Done");
    // _loader dropped here, unloading CEF framework
}

/// Handle create_browser command from launcher (for additional browsers in this profile)
#[cfg(target_os = "macos")]
fn handle_create_browser(
    msg: &XpcDictionary,
    state: &Arc<ProfileState>,
    launcher: &XpcConnection,
) {
    let session_id = msg.get_string("session_id").unwrap_or_default();
    let url = msg.get_string("url").unwrap_or_default();
    let width = msg.get_i64("width") as u32;
    let height = msg.get_i64("height") as u32;

    println!(
        "Profile: create_browser session={}, url={}, size={}x{}",
        session_id, url, width, height
    );

    // Claim GUI endpoint from launcher
    let gui_endpoint = match claim_session_with_retry(launcher, &session_id) {
        Ok(ep) => ep,
        Err(e) => {
            eprintln!("Profile: Failed to claim session for browser: {}", e);
            return;
        }
    };

    // Store pending browser request and post task to process it on UI thread
    state.pending_browsers.lock().unwrap().push((
        session_id.to_string(),
        url.to_string(),
        gui_endpoint,
        width,
        height,
    ));

    // Post task to CEF UI thread to process pending browsers
    let mut task = cef_handlers::CreateBrowserTask::new(Arc::clone(state));
    cef::post_task(cef::ThreadId::UI, Some(&mut task));
}

/// Claim session with exponential backoff retry
fn claim_session_with_retry(
    launcher: &XpcConnection,
    session_id: &str,
) -> Result<XpcEndpoint> {
    let max_retries = 10;
    let mut delay = Duration::from_millis(100);

    for attempt in 1..=max_retries {
        let msg = XpcDictionary::new();
        msg.set_string("action", "claim_session");
        msg.set_string("session_id", session_id);

        match launcher.send_with_reply_sync(&msg) {
            Ok(reply) => {
                if let Some(err) = reply.get_string("error") {
                    println!("Profile: Attempt {}/{}: {}", attempt, max_retries, err);
                    if attempt < max_retries {
                        thread::sleep(delay);
                        delay = (delay * 2).min(Duration::from_secs(2));
                        continue;
                    }
                    return Err(XpcError::Unknown(err));
                }
                if let Some(endpoint) = reply.get_endpoint("endpoint") {
                    return Ok(endpoint);
                }
                return Err(XpcError::Unknown("No endpoint in reply".into()));
            }
            Err(e) => {
                println!("Profile: Attempt {}/{}: {:?}", attempt, max_retries, e);
                if attempt < max_retries {
                    thread::sleep(delay);
                    delay = (delay * 2).min(Duration::from_secs(2));
                    continue;
                }
                return Err(e);
            }
        }
    }

    Err(XpcError::Unknown("Max retries exceeded".into()))
}

// ============================================================================
// CEF Handlers
// ============================================================================

#[cfg(target_os = "macos")]
mod cef_handlers {
    use super::{BrowserState, ProfileState};
    use cef::rc::Rc;
    use cef::{
        wrap_app, wrap_browser_process_handler, wrap_client, wrap_context_menu_handler,
        wrap_render_handler, wrap_task, AcceleratedPaintInfo, App, Browser,
        BrowserProcessHandler, BrowserSettings, Client, ContextMenuHandler, ContextMenuParams,
        Frame, ImplApp, ImplBrowser, ImplBrowserHost, ImplBrowserProcessHandler, ImplClient,
        ImplCommandLine, ImplContextMenuHandler, ImplMenuModel, ImplRenderHandler, ImplTask,
        MenuModel, PaintElementType, Rect, RenderHandler, ScreenInfo, Task, WindowInfo, WrapApp,
        WrapBrowserProcessHandler, WrapClient, WrapContextMenuHandler, WrapRenderHandler,
        WrapTask,
    };
    use std::sync::atomic::Ordering;
    use std::sync::{Arc, Mutex};
    use termsurf_xpc::*;

    // ====== Render Handler ======
    //
    // Sends IOSurface Mach ports to the GUI via XPC when CEF paints.
    // Each browser gets its own render handler with per-browser state.
    // Deduplicates by tracking the last IOSurface handle pointer.

    #[derive(Clone)]
    struct RenderHandlerInner {
        state: Arc<BrowserState>, // Per-browser state
        scale: f32,               // From profile-wide state
    }

    wrap_render_handler! {
        pub struct ProfileRenderHandler {
            inner: RenderHandlerInner,
        }

        impl RenderHandler {
            fn view_rect(&self, _browser: Option<&mut Browser>, rect: Option<&mut Rect>) {
                if let Some(rect) = rect {
                    rect.width = self.inner.state.width.load(Ordering::Relaxed) as i32;
                    rect.height = self.inner.state.height.load(Ordering::Relaxed) as i32;
                    println!(
                        "[VIEW_RECT] session={} returning {}x{}",
                        self.inner.state.session_id,
                        rect.width,
                        rect.height
                    );
                }
            }

            fn screen_info(
                &self,
                _browser: Option<&mut Browser>,
                screen_info: Option<&mut ScreenInfo>,
            ) -> ::std::os::raw::c_int {
                if let Some(info) = screen_info {
                    info.device_scale_factor = self.inner.scale;
                    return 1;
                }
                0
            }

            fn on_accelerated_paint(
                &self,
                _browser: Option<&mut Browser>,
                type_: PaintElementType,
                _dirty_rects: Option<&[Rect]>,
                info: Option<&AcceleratedPaintInfo>,
            ) {
                let Some(info) = info else { return };

                // Only handle PET_VIEW (skip popups)
                if type_ != PaintElementType::default() {
                    return;
                }

                // Dedup: only send when IOSurface handle changes.
                // CEF calls on_accelerated_paint every frame (cursor blinks, etc.)
                // but reuses the same IOSurface buffer. We only need to send a new
                // Mach port when the buffer changes (double-buffering swap).
                let handle = info.shared_texture_io_surface as *mut std::ffi::c_void;
                if handle.is_null() {
                    return;
                }
                let prev = self.inner.state.last_handle.swap(handle, Ordering::Relaxed);
                if handle == prev {
                    return;
                }

                // Create Mach port from IOSurface handle
                let port = termsurf_xpc::iosurface::create_mach_port(handle);
                if port == 0 {
                    eprintln!("Profile: create_mach_port failed");
                    return;
                }

                let width = info.extra.coded_size.width;
                let height = info.extra.coded_size.height;
                println!(
                    "Profile: [{}] Sending IOSurface {}x{} (port={})",
                    self.inner.state.session_id, width, height, port
                );
                println!(
                    "[TEXTURE-TX] session={} iosurface={}x{} view_rect={}x{}",
                    self.inner.state.session_id,
                    width,
                    height,
                    self.inner.state.width.load(Ordering::Relaxed),
                    self.inner.state.height.load(Ordering::Relaxed)
                );

                // Send to this browser's GUI connection via XPC
                let msg = XpcDictionary::new();
                msg.set_string("action", "display_surface");
                msg.set_mach_send("iosurface_port", port);
                msg.set_i64("width", width as i64);
                msg.set_i64("height", height as i64);
                self.inner.state.gui.send(&msg);
            }
        }
    }

    // ====== Context Menu Handler ======
    //
    // Suppresses the native context menu to avoid crashes.

    #[derive(Clone)]
    struct ContextMenuInner;

    wrap_context_menu_handler! {
        pub struct ProfileContextMenuHandler {
            inner: ContextMenuInner,
        }

        impl ContextMenuHandler {
            fn on_before_context_menu(
                &self,
                _browser: Option<&mut Browser>,
                _frame: Option<&mut Frame>,
                _params: Option<&mut ContextMenuParams>,
                model: Option<&mut MenuModel>,
            ) {
                if let Some(model) = model {
                    model.clear();
                }
            }
        }
    }

    // ====== Client ======

    wrap_client! {
        pub struct ProfileClient {
            render_handler: RenderHandler,
            context_menu_handler: ContextMenuHandler,
        }

        impl Client {
            fn render_handler(&self) -> Option<RenderHandler> {
                Some(self.render_handler.clone())
            }

            fn context_menu_handler(&self) -> Option<ContextMenuHandler> {
                Some(self.context_menu_handler.clone())
            }
        }
    }

    // ====== Browser Process Handler ======
    //
    // Creates the initial browser in on_context_initialized, which fires during
    // run_message_loop() when CEF is fully ready.

    wrap_browser_process_handler! {
        pub struct ProfileBPH {
            state: Arc<ProfileState>,
        }

        impl BrowserProcessHandler {
            fn on_context_initialized(&self) {
                println!("Profile: CEF context initialized");

                // Take the initial browser info (only runs once)
                let info = self.state.initial_browser_info.lock().unwrap().take();

                if let Some(info) = info {
                    println!(
                        "Profile: Creating initial browser for session '{}', url='{}'",
                        info.session_id, info.url
                    );
                    create_browser_on_ui_thread(
                        &info.url,
                        &info.session_id,
                        info.gui_endpoint,
                        info.width,
                        info.height,
                        &self.state,
                    );
                } else {
                    println!("Profile: No initial browser info (unexpected)");
                }
            }
        }
    }

    // ====== App ======

    wrap_app! {
        pub struct ProfileCefApp {
            handler: BrowserProcessHandler,
        }

        impl App {
            fn on_before_command_line_processing(
                &self,
                _process_type: Option<&cef::CefStringUtf16>,
                command_line: Option<&mut cef::CommandLine>,
            ) {
                if let Some(command_line) = command_line {
                    command_line.append_switch(Some(&"no-startup-window".into()));
                }
            }

            fn browser_process_handler(&self) -> Option<BrowserProcessHandler> {
                Some(self.handler.clone())
            }
        }
    }

    pub fn create_app(state: Arc<ProfileState>) -> App {
        let handler = ProfileBPH::new(state);
        ProfileCefApp::new(handler)
    }

    // ====== Create Browser Task ======
    //
    // Task for creating browsers on the UI thread when requested via XPC.

    wrap_task! {
        pub struct CreateBrowserTask {
            state: Arc<ProfileState>,
        }

        impl Task {
            fn execute(&self) {
                // Process all pending browser creation requests
                let pending: Vec<_> = self.state.pending_browsers.lock().unwrap().drain(..).collect();

                for (session_id, url, gui_endpoint, width, height) in pending {
                    println!(
                        "Profile: Processing pending browser: session='{}', url='{}'",
                        session_id, url
                    );
                    create_browser_on_ui_thread(
                        &url,
                        &session_id,
                        gui_endpoint,
                        width,
                        height,
                        &self.state,
                    );
                }
            }
        }
    }

    /// Create a browser on the CEF UI thread.
    /// Used by both initial browser creation (on_context_initialized) and
    /// subsequent browser creation (create_browser command from launcher).
    pub fn create_browser_on_ui_thread(
        url: &str,
        session_id: &str,
        gui_endpoint: XpcEndpoint,
        width: u32,
        height: u32,
        state: &Arc<ProfileState>,
    ) {
        use std::sync::atomic::AtomicPtr;

        // 1. Connect to GUI (don't resume yet)
        let gui = match XpcConnection::from_endpoint(gui_endpoint) {
            Ok(c) => Arc::new(c),
            Err(e) => {
                eprintln!("Profile: Failed to connect to GUI: {}", e);
                return;
            }
        };

        // 2. Create deferred state wrapper (will be populated after browser creation)
        let deferred_state: Arc<Mutex<Option<Arc<BrowserState>>>> =
            Arc::new(Mutex::new(None));

        // 3. Set event handler BEFORE resume
        let deferred_for_handler = Arc::clone(&deferred_state);
        let scale_for_handler = state.scale;
        set_event_handler(&*gui, move |event| {
            match event {
                Ok(msg) => {
                    let action = msg.get_string("action").unwrap_or_default();
                    match action.as_str() {
                        "resize_browser" => {
                            // Get state from deferred wrapper
                            let state_guard = deferred_for_handler.lock().unwrap();
                            let Some(bs) = state_guard.as_ref() else {
                                println!("Profile: resize_browser ignored (state not ready)");
                                return;
                            };

                            // Check for physical dimensions first (new protocol)
                            let (width, height) = if msg.get_i64("physical_width") != 0 {
                                let physical_w = msg.get_i64("physical_width") as u32;
                                let physical_h = msg.get_i64("physical_height") as u32;
                                let scale_str = msg.get_string("scale").unwrap_or_default();
                                let scale: f32 = scale_str.parse().unwrap_or(scale_for_handler);
                                // Convert to logical, rounding up to ensure texture >= viewport
                                let logical_w = (physical_w as f32 / scale).ceil() as u32;
                                let logical_h = (physical_h as f32 / scale).ceil() as u32;
                                println!(
                                    "[RESIZE-RECV] physical={}x{} scale={:.2} -> logical={}x{} (ceil) (prev={}x{})",
                                    physical_w, physical_h, scale,
                                    logical_w, logical_h,
                                    bs.width.load(Ordering::Relaxed),
                                    bs.height.load(Ordering::Relaxed)
                                );
                                (logical_w, logical_h)
                            } else {
                                // Fallback to legacy logical dimensions
                                let width = msg.get_i64("width") as u32;
                                let height = msg.get_i64("height") as u32;
                                println!(
                                    "[RESIZE-RECV] logical={}x{} scale={:.2} -> physical={}x{} (prev={}x{})",
                                    width, height, scale_for_handler,
                                    (width as f32 * scale_for_handler) as u32,
                                    (height as f32 * scale_for_handler) as u32,
                                    bs.width.load(Ordering::Relaxed),
                                    bs.height.load(Ordering::Relaxed)
                                );
                                (width, height)
                            };

                            let bs = Arc::clone(bs);
                            drop(state_guard); // Release lock before post_task

                            let mut task = ResizeBrowserTask::new(bs, width, height);
                            cef::post_task(cef::ThreadId::UI, Some(&mut task));
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    eprintln!("Profile: GUI connection error: {}", e);
                }
            }
        });

        // 4. NOW resume the connection (handler is ready)
        gui.resume();

        // 5. Create per-browser state
        let browser_state = Arc::new(BrowserState {
            session_id: session_id.to_string(),
            gui: Arc::clone(&gui),
            width: std::sync::atomic::AtomicU32::new(width),
            height: std::sync::atomic::AtomicU32::new(height),
            last_handle: AtomicPtr::new(std::ptr::null_mut()),
            browser: Mutex::new(None),
        });

        // Create render handler with browser-specific state
        let inner = RenderHandlerInner {
            state: Arc::clone(&browser_state),
            scale: state.scale,
        };

        let render_handler = ProfileRenderHandler::new(inner);
        let context_menu_handler = ProfileContextMenuHandler::new(ContextMenuInner);
        let mut client = ProfileClient::new(render_handler, context_menu_handler);

        let window_info = WindowInfo {
            windowless_rendering_enabled: 1,
            shared_texture_enabled: 1,
            ..Default::default()
        };

        let browser_settings = BrowserSettings {
            windowless_frame_rate: 60,
            ..Default::default()
        };

        let url_cef: cef::CefString = url.into();

        let browser = cef::browser_host_create_browser_sync(
            Some(&window_info),
            Some(&mut client),
            Some(&url_cef),
            Some(&browser_settings),
            None, // extra_info
            None, // request_context (uses global with our root_cache_path)
        );

        match browser {
            Some(b) => {
                let browser_id = b.identifier();
                println!(
                    "Profile: Browser {} created for '{}' (session='{}')",
                    browser_id, url, session_id
                );

                // Store browser reference for resize operations
                *browser_state.browser.lock().unwrap() = Some(b);

                // 6. Populate deferred state (handler can now access it)
                *deferred_state.lock().unwrap() = Some(Arc::clone(&browser_state));

                // Store browser state by ID
                state
                    .browsers
                    .lock()
                    .unwrap()
                    .insert(browser_id, browser_state);
            }
            None => eprintln!("Profile: Failed to create browser for '{}'", url),
        }
    }

    // ====== Resize Browser Task ======
    //
    // Task for resizing browsers on the UI thread when resize commands arrive via XPC.

    wrap_task! {
        pub struct ResizeBrowserTask {
            state: Arc<BrowserState>,
            width: u32,
            height: u32,
        }

        impl Task {
            fn execute(&self) {
                resize_browser_on_ui_thread(&self.state, self.width, self.height);
            }
        }
    }

    /// Resize a browser (called on CEF UI thread via post_task)
    fn resize_browser_on_ui_thread(state: &BrowserState, width: u32, height: u32) {
        // Update stored dimensions (used by get_view_rect)
        state.width.store(width, Ordering::Relaxed);
        state.height.store(height, Ordering::Relaxed);

        // Notify CEF of size change
        if let Some(ref browser) = *state.browser.lock().unwrap() {
            if let Some(host) = browser.host() {
                println!("Profile: was_resized {}x{}", width, height);
                host.was_resized();
                // PaintElementType::default() is PET_VIEW (0)
                host.invalidate(PaintElementType::default());
            }
        }
    }
}
