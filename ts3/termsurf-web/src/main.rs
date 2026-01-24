use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

// ============================================================================
// Protocol Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct Request {
    id: String,
    action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Response {
    id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl Response {
    fn ok(id: &str, data: Option<serde_json::Value>) -> Self {
        Self {
            id: id.to_string(),
            status: "ok".to_string(),
            data,
            error: None,
        }
    }

    fn error(id: &str, error: &str) -> Self {
        Self {
            id: id.to_string(),
            status: "error".to_string(),
            data: None,
            error: Some(error.to_string()),
        }
    }
}

// ============================================================================
// Profile Mode
// ============================================================================

#[derive(Debug, Clone)]
enum ProfileMode {
    Named(String),
    Incognito(String), // UUID for unique socket path
}

impl ProfileMode {
    fn cache_path(&self) -> Option<PathBuf> {
        match self {
            ProfileMode::Named(name) => {
                let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
                Some(PathBuf::from(format!(
                    "{}/.config/termsurf/cef/{}",
                    home, name
                )))
            }
            ProfileMode::Incognito(_) => None,
        }
    }

    fn socket_path(&self) -> PathBuf {
        let home = env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let sockets_dir = PathBuf::from(format!("{}/.config/termsurf/sockets", home));

        // Ensure sockets directory exists
        let _ = fs::create_dir_all(&sockets_dir);

        match self {
            ProfileMode::Named(name) => sockets_dir.join(format!("{}.sock", name)),
            ProfileMode::Incognito(uuid) => sockets_dir.join(format!("incognito-{}.sock", uuid)),
        }
    }

    fn display_name(&self) -> &str {
        match self {
            ProfileMode::Named(name) => name.as_str(),
            ProfileMode::Incognito(_) => "incognito",
        }
    }
}

// ============================================================================
// Argument Parsing
// ============================================================================

fn validate_profile_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Profile name cannot be empty".into());
    }

    let first_char = name.chars().next().unwrap();
    if !first_char.is_ascii_lowercase() {
        return Err("Profile name must start with a lowercase letter".into());
    }

    for c in name.chars() {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() {
            return Err(format!(
                "Profile name must be lowercase alphanumeric, found '{}'",
                c
            ));
        }
    }

    Ok(())
}

fn parse_args() -> Result<(bool, ProfileMode), String> {
    let args: Vec<String> = env::args().collect();

    let is_subprocess = args.iter().any(|a| a == "--browser-subprocess");
    let has_incognito = args.iter().any(|a| a == "--incognito");

    // Find --profile value
    let profile_value = args
        .iter()
        .position(|a| a == "--profile")
        .and_then(|i| args.get(i + 1).cloned());

    // Find --incognito-id value (for subprocess)
    let incognito_id = args
        .iter()
        .position(|a| a == "--incognito-id")
        .and_then(|i| args.get(i + 1).cloned());

    // Check for mutual exclusivity
    if has_incognito && profile_value.is_some() {
        return Err("Cannot specify both --incognito and --profile".into());
    }

    let profile_mode = if has_incognito || incognito_id.is_some() {
        // Use provided ID or generate new one
        let uuid = incognito_id.unwrap_or_else(|| Uuid::new_v4().to_string());
        ProfileMode::Incognito(uuid)
    } else if let Some(name) = profile_value {
        validate_profile_name(&name)?;
        ProfileMode::Named(name)
    } else {
        ProfileMode::Named("default".to_string())
    };

    Ok((is_subprocess, profile_mode))
}

// ============================================================================
// CEF Loading
// ============================================================================

#[cfg(target_os = "macos")]
fn load_cef(profile: &ProfileMode) -> Result<(), String> {
    use cef::args::Args;
    use cef::library_loader::LibraryLoader;
    use cef::{api_hash, execute_process, initialize, sys, CefString, Settings};

    let exe = env::current_exe().map_err(|e| format!("current_exe: {e}"))?;

    let loader = LibraryLoader::new(&exe, false);
    if !loader.load() {
        return Err("Failed to load CEF framework".into());
    }

    let _ = api_hash(sys::CEF_API_VERSION_LAST, 0);

    let args = Args::new();

    let ret = execute_process(
        Some(args.as_main_args()),
        None::<&mut cef::App>,
        std::ptr::null_mut(),
    );
    if ret >= 0 {
        std::process::exit(ret);
    }

    let cache_path_str = match profile.cache_path() {
        Some(path) => {
            let _ = fs::create_dir_all(&path);
            path.to_string_lossy().to_string()
        }
        None => String::new(),
    };

    let helper_path = exe
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("Frameworks")
        .join("WezTerm Helper.app")
        .join("Contents")
        .join("MacOS")
        .join("WezTerm Helper");
    let helper_path_str = helper_path.to_string_lossy().to_string();

    let settings = Settings {
        windowless_rendering_enabled: 1,
        external_message_pump: 1,
        no_sandbox: 1,
        root_cache_path: CefString::from(cache_path_str.as_str()),
        browser_subprocess_path: CefString::from(helper_path_str.as_str()),
        ..Default::default()
    };

    if initialize(
        Some(args.as_main_args()),
        Some(&settings),
        None::<&mut cef::App>,
        std::ptr::null_mut(),
    ) != 1
    {
        return Err("CEF initialize failed".into());
    }

    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn load_cef(_profile: &ProfileMode) -> Result<(), String> {
    Err("CEF loading not yet implemented for this platform".into())
}

// ============================================================================
// Socket Server (Browser Subprocess)
// ============================================================================

fn handle_request(request: &Request) -> Response {
    match request.action.as_str() {
        "ping" => Response::ok(&request.id, Some(serde_json::json!({"pong": true}))),
        _ => Response::error(&request.id, &format!("Unknown action: {}", request.action)),
    }
}

fn handle_connection(mut stream: UnixStream) {
    let reader = BufReader::new(stream.try_clone().expect("Failed to clone stream"));

    for line in reader.lines() {
        match line {
            Ok(line) if line.is_empty() => continue,
            Ok(line) => {
                let response = match serde_json::from_str::<Request>(&line) {
                    Ok(request) => {
                        println!("Received request: {:?}", request);
                        handle_request(&request)
                    }
                    Err(e) => Response::error("unknown", &format!("Invalid JSON: {}", e)),
                };

                let response_json = serde_json::to_string(&response).unwrap();
                println!("Sending response: {}", response_json);

                if let Err(e) = writeln!(stream, "{}", response_json) {
                    eprintln!("Failed to write response: {}", e);
                    break;
                }
                let _ = stream.flush();
            }
            Err(e) => {
                eprintln!("Error reading from socket: {}", e);
                break;
            }
        }
    }
    println!("Connection closed");
}

fn run_socket_server(socket_path: &PathBuf) -> Result<(), String> {
    // Remove stale socket if it exists
    if socket_path.exists() {
        fs::remove_file(socket_path).map_err(|e| format!("Failed to remove stale socket: {}", e))?;
    }

    let listener =
        UnixListener::bind(socket_path).map_err(|e| format!("Failed to bind socket: {}", e))?;

    println!("Socket server listening at {:?}", socket_path);

    // Accept one connection for this test
    match listener.accept() {
        Ok((stream, _addr)) => {
            println!("Client connected");
            handle_connection(stream);
        }
        Err(e) => {
            return Err(format!("Failed to accept connection: {}", e));
        }
    }

    Ok(())
}

fn run_browser_subprocess(profile: ProfileMode) {
    let socket_path = profile.socket_path();

    println!(
        "Browser subprocess starting with profile={}",
        profile.display_name()
    );

    match load_cef(&profile) {
        Ok(()) => {
            println!("loaded CEF with profile={}", profile.display_name());
        }
        Err(e) => {
            eprintln!("Failed to load CEF: {}", e);
            std::process::exit(1);
        }
    }

    // Start socket server
    println!("Starting socket server...");
    if let Err(e) = run_socket_server(&socket_path) {
        eprintln!("Socket server error: {}", e);
    }

    // Cleanup
    let _ = fs::remove_file(&socket_path);
    println!("Socket cleaned up");

    #[cfg(target_os = "macos")]
    cef::shutdown();
}

// ============================================================================
// Socket Client (Coordinator)
// ============================================================================

fn wait_for_socket(socket_path: &PathBuf, timeout: Duration) -> Result<(), String> {
    let start = std::time::Instant::now();
    while !socket_path.exists() {
        if start.elapsed() > timeout {
            return Err(format!("Timeout waiting for socket at {:?}", socket_path));
        }
        thread::sleep(Duration::from_millis(50));
    }
    // Small delay to ensure socket is ready to accept
    thread::sleep(Duration::from_millis(100));
    Ok(())
}

fn send_ping(socket_path: &PathBuf) -> Result<Response, String> {
    let mut stream =
        UnixStream::connect(socket_path).map_err(|e| format!("Failed to connect: {}", e))?;

    let request = Request {
        id: Uuid::new_v4().to_string(),
        action: "ping".to_string(),
        data: None,
    };

    let request_json = serde_json::to_string(&request).unwrap();
    println!("Sending: {}", request_json);

    writeln!(stream, "{}", request_json).map_err(|e| format!("Failed to write: {}", e))?;
    stream.flush().map_err(|e| format!("Failed to flush: {}", e))?;

    let reader = BufReader::new(stream);
    let response_line = reader
        .lines()
        .next()
        .ok_or("No response received")?
        .map_err(|e| format!("Failed to read response: {}", e))?;

    println!("Received: {}", response_line);

    serde_json::from_str(&response_line).map_err(|e| format!("Invalid response JSON: {}", e))
}

fn spawn_subprocess(profile: &ProfileMode) -> Child {
    let exe = env::current_exe().expect("Failed to get current executable path");

    let mut cmd = Command::new(&exe);
    cmd.arg("--browser-subprocess");

    match profile {
        ProfileMode::Named(name) => {
            cmd.arg("--profile").arg(name);
        }
        ProfileMode::Incognito(uuid) => {
            cmd.arg("--incognito").arg("--incognito-id").arg(uuid);
        }
    }

    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn browser subprocess")
}

fn run_coordinator(profile: ProfileMode) {
    let socket_path = profile.socket_path();

    println!(
        "Coordinator: spawning browser subprocess with profile={}...",
        profile.display_name()
    );
    println!("Socket path: {:?}", socket_path);

    let mut child = spawn_subprocess(&profile);

    // Wait for socket to be created
    println!("Waiting for socket...");
    match wait_for_socket(&socket_path, Duration::from_secs(10)) {
        Ok(()) => println!("Socket ready"),
        Err(e) => {
            eprintln!("Error: {}", e);
            let _ = child.kill();
            std::process::exit(1);
        }
    }

    // Send ping
    println!("Sending ping...");
    match send_ping(&socket_path) {
        Ok(response) => {
            if response.status == "ok" {
                println!("SUCCESS: Received pong response!");
                if let Some(data) = response.data {
                    println!("Response data: {}", data);
                }
            } else {
                eprintln!("ERROR: {}", response.error.unwrap_or_default());
            }
        }
        Err(e) => {
            eprintln!("Failed to send ping: {}", e);
        }
    }

    // Wait for subprocess to finish
    println!("Waiting for subprocess to exit...");
    let output = child.wait_with_output().expect("Failed to wait for subprocess");

    if !output.stdout.is_empty() {
        println!(
            "Subprocess stdout:\n{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    if !output.stderr.is_empty() {
        eprintln!(
            "Subprocess stderr:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    println!("Subprocess exited with: {}", output.status);
}

// ============================================================================
// Main
// ============================================================================

fn main() {
    match parse_args() {
        Ok((is_subprocess, profile)) => {
            if is_subprocess {
                run_browser_subprocess(profile);
            } else {
                run_coordinator(profile);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            eprintln!();
            eprintln!("Usage: web [--profile <name>] [--incognito]");
            eprintln!();
            eprintln!("Options:");
            eprintln!("  --profile <name>  Use named profile (default: 'default')");
            eprintln!("  --incognito       Use incognito mode (no persistent storage)");
            eprintln!();
            eprintln!("Profile names must be lowercase alphanumeric and start with a letter.");
            std::process::exit(1);
        }
    }
}
