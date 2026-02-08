//! XPC Service Launcher for Experiment 1.
//!
//! This XPC service spawns child processes and relays endpoints between
//! the receiver (simulating GUI) and sender (simulating profile server).
//!
//! Build with: cargo build --release --example launcher

use std::collections::HashMap;
use std::env;
use std::process::Command;
use std::sync::{Arc, Mutex};
use termsurf_xpc::*;

fn main() {
    println!("Launcher: Starting...");

    // Session storage: session_id -> receiver endpoint
    let sessions: Arc<Mutex<HashMap<String, XpcEndpoint>>> = Arc::new(Mutex::new(HashMap::new()));

    // CRITICAL: Store client connections to keep them alive!
    // Without this, connections are canceled when the handler returns.
    let clients: Arc<Mutex<Vec<Arc<XpcConnection>>>> = Arc::new(Mutex::new(Vec::new()));

    // Path to sender binary
    // The launcher is at: .app/Contents/XPCServices/com.termsurf.xpc-test.xpc/Contents/MacOS/launcher
    // The sender is at:   .app/Contents/MacOS/sender
    // So we need to go up 5 directories to get to .app/Contents/MacOS
    let exe_path = env::current_exe().expect("Failed to get exe path");
    let exe_dir = exe_path.parent().expect("Failed to get exe directory");
    let sender_path = exe_dir
        .parent() // Contents
        .and_then(|p| p.parent()) // com.termsurf.xpc-test.xpc
        .and_then(|p| p.parent()) // XPCServices
        .and_then(|p| p.parent()) // Contents
        .map(|p| p.join("MacOS").join("sender"))
        .unwrap_or_else(|| exe_dir.join("sender")); // Fallback for testing
    println!("Launcher: Sender path: {:?}", sender_path);

    // Create listener for this XPC service
    let listener = match XpcListener::new_mach_service("com.termsurf.xpc-test") {
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

    set_new_connection_handler(&listener, move |conn| {
        println!("Launcher: New connection");

        // Wrap in Arc so we can share with event handler
        let conn = Arc::new(conn);
        let conn_for_handler = conn.clone();

        let sessions = sessions_clone.clone();
        let sender_path = sender_path.clone();
        let clients_inner = clients_clone.clone();

        set_event_handler(&*conn, move |event| match event {
            Ok(msg) => {
                let action = msg.get_string("action").unwrap_or_default();
                println!("Launcher: Received action: {}", action);

                match action.as_str() {
                    "spawn_sender" => {
                        let session_id = match msg.get_string("session_id") {
                            Some(id) => id,
                            None => {
                                eprintln!("Launcher: Missing session_id");
                                return;
                            }
                        };
                        let endpoint = match msg.get_endpoint("receiver_endpoint") {
                            Some(ep) => ep,
                            None => {
                                eprintln!("Launcher: Missing receiver_endpoint");
                                return;
                            }
                        };

                        println!("Launcher: Storing endpoint for session {}", session_id);

                        // Store endpoint for sender to claim
                        {
                            let mut sessions = sessions.lock().unwrap();
                            sessions.insert(session_id.clone(), endpoint);
                        }

                        // Spawn sender as child process
                        println!("Launcher: Spawning sender...");
                        match Command::new(&sender_path)
                            .args(["--session", &session_id])
                            .spawn()
                        {
                            Ok(child) => {
                                println!(
                                    "Launcher: Spawned sender for {} (pid: {})",
                                    session_id,
                                    child.id()
                                )
                            }
                            Err(e) => eprintln!("Launcher: Failed to spawn: {}", e),
                        }
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
