//! XPC Service Launcher for TermSurf.
//!
//! This XPC service spawns profile server processes and relays XPC endpoints
//! between the GUI and profile servers, enabling Mach port transfer for
//! cross-process IOSurface sharing.
//!
//! Service name: com.termsurf.launcher

use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::process::Command;
use std::sync::{Arc, Mutex};
use termsurf_xpc::*;

extern "C" {
    fn dup2(oldfd: i32, newfd: i32) -> i32;
}

fn redirect_output() {
    use std::os::unix::io::AsRawFd;
    let file = match File::create("/tmp/termsurf-launcher.log") {
        Ok(f) => f,
        Err(_) => return,
    };
    let fd = file.as_raw_fd();
    unsafe {
        dup2(fd, 1); // stdout
        dup2(fd, 2); // stderr
    }
    // Leak the file so the fd stays open for the process lifetime
    std::mem::forget(file);
}

fn main() {
    redirect_output();
    println!("Launcher: Starting...");

    // Session storage: session_id -> GUI endpoint
    let sessions: Arc<Mutex<HashMap<String, XpcEndpoint>>> = Arc::new(Mutex::new(HashMap::new()));

    // Running profile processes: profile_name -> XpcConnection to that profile's command listener
    let running_profiles: Arc<Mutex<HashMap<String, Arc<XpcConnection>>>> =
        Arc::new(Mutex::new(HashMap::new()));

    // CRITICAL: Store client connections to keep them alive!
    let clients: Arc<Mutex<Vec<Arc<XpcConnection>>>> = Arc::new(Mutex::new(Vec::new()));

    // Path to profile server binary
    // Launcher is at: .app/Contents/XPCServices/com.termsurf.launcher.xpc/Contents/MacOS/termsurf-launcher
    // Profile is at:  .app/Contents/MacOS/termsurf-profile
    let exe_path = env::current_exe().expect("Failed to get exe path");
    let profile_bin_path = exe_path
        .parent() // MacOS
        .and_then(|p| p.parent()) // Contents
        .and_then(|p| p.parent()) // com.termsurf.launcher.xpc
        .and_then(|p| p.parent()) // XPCServices
        .and_then(|p| p.parent()) // Contents
        .map(|p| p.join("MacOS").join("termsurf-profile"))
        .unwrap_or_else(|| {
            // Fallback for testing outside app bundle
            exe_path
                .parent()
                .map(|p| p.join("termsurf-profile"))
                .unwrap_or_default()
        });
    println!("Launcher: Profile binary path: {:?}", profile_bin_path);

    // Create listener for this XPC service
    let listener = match XpcListener::new_mach_service("com.termsurf.launcher") {
        Ok(l) => {
            println!("Launcher: Created Mach service listener");
            l
        }
        Err(e) => {
            eprintln!("Launcher: Failed to create listener: {}", e);
            std::process::exit(1);
        }
    };

    // Handle incoming connections
    let sessions_clone = sessions.clone();
    let clients_clone = clients.clone();
    let running_profiles_clone = running_profiles.clone();

    set_new_connection_handler(&listener, move |conn| {
        println!("Launcher: New connection");

        // Wrap in Arc so we can share with event handler
        let conn = Arc::new(conn);
        let conn_for_handler = conn.clone();

        let sessions = sessions_clone.clone();
        let profile_bin_path = profile_bin_path.clone();
        let clients_inner = clients_clone.clone();
        let running_profiles = running_profiles_clone.clone();

        set_event_handler(&*conn, move |event| match event {
            Ok(msg) => {
                let action = msg.get_string("action").unwrap_or_default();
                println!("Launcher: Received action: {}", action);

                match action.as_str() {
                    "spawn_profile" => {
                        let session_id = match msg.get_string("session_id") {
                            Some(id) => id,
                            None => {
                                eprintln!("Launcher: Missing session_id");
                                return;
                            }
                        };
                        let gui_endpoint = match msg.get_endpoint("gui_endpoint") {
                            Some(ep) => ep,
                            None => {
                                eprintln!("Launcher: Missing gui_endpoint");
                                return;
                            }
                        };

                        // Extract URL, profile, and dimensions from message
                        let url = msg
                            .get_string("url")
                            .unwrap_or_else(|| "about:blank".to_string());
                        let profile = msg
                            .get_string("profile")
                            .unwrap_or_else(|| "default".to_string());
                        let width = msg.get_i64("width");
                        let height = msg.get_i64("height");
                        let scale = msg
                            .get_string("scale")
                            .unwrap_or_else(|| "2.0".to_string());

                        // Always store GUI endpoint for claiming (by profile process)
                        println!("Launcher: Storing endpoint for session {}", session_id);
                        sessions
                            .lock()
                            .unwrap()
                            .insert(session_id.clone(), gui_endpoint);

                        // Check if profile process already running
                        let existing_conn = running_profiles.lock().unwrap().get(&profile).cloned();

                        if let Some(profile_conn) = existing_conn {
                            // Forward to existing profile process
                            println!(
                                "Launcher: Forwarding to existing profile '{}' (session={}, url={})",
                                profile, session_id, url
                            );

                            let create_msg = XpcDictionary::new();
                            create_msg.set_string("action", "create_browser");
                            create_msg.set_string("session_id", &session_id);
                            create_msg.set_string("url", &url);
                            create_msg.set_i64("width", width);
                            create_msg.set_i64("height", height);
                            create_msg.set_string("scale", &scale);

                            profile_conn.send(&create_msg);
                        } else {
                            // Spawn new profile process
                            println!(
                                "Launcher: Spawning new profile '{}' (session={}, url={}, size={}x{}, scale={})...",
                                profile, session_id, url, width, height, scale
                            );
                            let log_path = format!("/tmp/termsurf-profile-{}.log", profile);
                            let mut cmd = Command::new(&profile_bin_path);
                            cmd.args(["--session-id", &session_id])
                                .args(["--url", &url])
                                .args(["--profile", &profile])
                                .args(["--width", &width.to_string()])
                                .args(["--height", &height.to_string()])
                                .args(["--scale", &scale]);
                            if let Ok(log_file) = File::create(&log_path) {
                                if let Ok(log_file2) = log_file.try_clone() {
                                    cmd.stdout(log_file).stderr(log_file2);
                                }
                            }
                            match cmd.spawn() {
                                Ok(child) => {
                                    println!(
                                        "Launcher: Spawned profile '{}' (pid: {}, log: {})",
                                        profile,
                                        child.id(),
                                        log_path
                                    )
                                }
                                Err(e) => eprintln!("Launcher: Failed to spawn: {}", e),
                            }
                        }
                    }

                    "register_profile" => {
                        let profile = match msg.get_string("profile") {
                            Some(p) => p,
                            None => {
                                eprintln!("Launcher: register_profile missing profile");
                                return;
                            }
                        };
                        let endpoint = match msg.get_endpoint("endpoint") {
                            Some(ep) => ep,
                            None => {
                                eprintln!("Launcher: register_profile missing endpoint");
                                return;
                            }
                        };

                        // Create persistent connection from endpoint
                        let profile_conn = match XpcConnection::from_endpoint(endpoint) {
                            Ok(c) => Arc::new(c),
                            Err(e) => {
                                eprintln!("Launcher: Failed to connect to profile: {}", e);
                                return;
                            }
                        };

                        let profile_name = profile.to_string();
                        set_event_handler(&*profile_conn, move |event| {
                            if let Err(e) = event {
                                eprintln!(
                                    "Launcher: Profile '{}' connection error: {}",
                                    profile_name, e
                                );
                            }
                        });
                        profile_conn.resume();

                        running_profiles
                            .lock()
                            .unwrap()
                            .insert(profile.to_string(), profile_conn);

                        println!("Launcher: Profile '{}' registered", profile);
                    }

                    "claim_session" => {
                        let session_id = match msg.get_string("session_id") {
                            Some(id) => id,
                            None => {
                                eprintln!("Launcher: Missing session_id in claim_session");
                                return;
                            }
                        };

                        println!("Launcher: Claim request for session {}", session_id);

                        let endpoint = {
                            let mut sessions = sessions.lock().unwrap();
                            sessions.remove(&session_id)
                        };

                        // Create and send reply
                        let reply = match XpcDictionary::create_reply(&msg) {
                            Ok(r) => r,
                            Err(e) => {
                                eprintln!("Launcher: Failed to create reply: {}", e);
                                return;
                            }
                        };

                        if let Some(ep) = endpoint {
                            reply.set_endpoint("endpoint", ep);
                            println!("Launcher: Session {} claimed successfully", session_id);
                        } else {
                            reply.set_string("error", "session not found");
                            println!("Launcher: Session {} not found", session_id);
                        }

                        // Send reply on the connection
                        conn_for_handler.send(&reply);
                    }

                    _ => {
                        eprintln!("Launcher: Unknown action: {}", action);
                    }
                }
            }
            Err(e) => {
                eprintln!("Launcher: Connection error: {}", e);
            }
        });

        conn.resume();

        // CRITICAL: Store the connection to keep it alive!
        clients_inner.lock().unwrap().push(conn);
    });

    listener.resume();

    println!("Launcher: Running...");
    run_loop();
}
