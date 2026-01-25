//! Test Receiver for Experiment 1.
//!
//! Simulates the GUI side - creates anonymous listener, asks launcher to spawn
//! sender, receives IOSurface Mach port.
//!
//! Build with: cargo build --release --example receiver

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use termsurf_xpc::*;

fn main() {
    println!("Receiver: Starting...");

    let success = Arc::new(AtomicBool::new(false));
    let success_clone = success.clone();

    // CRITICAL: Storage for peer connections - must keep them alive!
    let peers: Arc<Mutex<Vec<XpcConnection>>> = Arc::new(Mutex::new(Vec::new()));
    let peers_clone = peers.clone();

    // 1. Connect to launcher
    println!("Receiver: Connecting to launcher...");
    let launcher = match XpcConnection::connect_mach_service("com.termsurf.xpc-test") {
        Ok(c) => {
            println!("Receiver: Connected to launcher");
            c
        }
        Err(e) => {
            eprintln!("Receiver: Failed to connect to launcher: {}", e);
            eprintln!("Receiver: Make sure the XPC service bundle is built and signed.");
            std::process::exit(1);
        }
    };

    set_event_handler(&launcher, |event| {
        // Handle any errors from launcher
        if let Err(e) = event {
            eprintln!("Receiver: Launcher error: {}", e);
        }
    });
    launcher.resume();

    // 2. Create anonymous listener for sender to connect
    println!("Receiver: Creating anonymous listener...");
    let listener = match XpcListener::new_anonymous() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Receiver: Failed to create anonymous listener: {}", e);
            std::process::exit(1);
        }
    };

    let endpoint = match listener.get_endpoint() {
        Ok(ep) => {
            println!("Receiver: Got endpoint from listener");
            ep
        }
        Err(e) => {
            eprintln!("Receiver: Failed to get endpoint: {}", e);
            std::process::exit(1);
        }
    };

    // 3. Set up handler for incoming peer connections
    set_new_connection_handler(&listener, move |peer| {
        println!("Receiver: Sender connected!");

        let success_inner = success_clone.clone();

        set_event_handler(&peer, move |event| {
            match event {
                Ok(msg) => {
                    let action = msg.get_string("action").unwrap_or_default();
                    println!("Receiver: Received action: {}", action);

                    if action == "send_surface" {
                        // Receive IOSurface Mach port
                        let port = msg.copy_mach_send("iosurface_port");
                        println!("Receiver: Got Mach port: {}", port);

                        if port == 0 {
                            eprintln!("FAILED: Received null Mach port");
                            std::process::exit(1);
                        }

                        let handle = iosurface::lookup_from_mach_port(port);

                        let handle = match handle {
                            Some(h) => {
                                println!("Receiver: Reconstructed IOSurface from Mach port");
                                h
                            }
                            None => {
                                eprintln!("FAILED: lookup_from_mach_port returned None");
                                std::process::exit(1);
                            }
                        };

                        // Verify dimensions
                        let width = iosurface::get_width(handle);
                        let height = iosurface::get_height(handle);
                        println!("Receiver: IOSurface dimensions: {}x{}", width, height);

                        if width != 100 || height != 100 {
                            eprintln!("FAILED: Expected 100x100, got {}x{}", width, height);
                            std::process::exit(1);
                        }

                        // Verify pixel color (hot pink: 0xFF69B4)
                        let pixel = iosurface::read_pixel(handle, 0, 0);
                        let r = ((pixel >> 24) & 0xFF) as u8;
                        let g = ((pixel >> 16) & 0xFF) as u8;
                        let b = ((pixel >> 8) & 0xFF) as u8;
                        let a = (pixel & 0xFF) as u8;
                        println!("Receiver: Pixel at (0,0): RGBA({}, {}, {}, {})", r, g, b, a);

                        // Expected: hot pink (255, 105, 180, 255)
                        if r == 255 && g == 105 && b == 180 && a == 255 {
                            println!("\n=== SUCCESS ===");
                            println!("Receiver: IOSurface transferred correctly via XPC!");
                            println!("  - Dimensions: 100x100");
                            println!("  - Pixel color: hot pink (255, 105, 180, 255)");
                            success_inner.store(true, Ordering::SeqCst);
                            std::process::exit(0);
                        } else {
                            eprintln!(
                                "FAILED: Expected hot pink (255, 105, 180, 255), got ({}, {}, {}, {})",
                                r, g, b, a
                            );
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Receiver: Peer error: {}", e);
                }
            }
        });
        peer.resume();

        // CRITICAL: Store the peer to keep connection alive!
        peers_clone.lock().unwrap().push(peer);
    });
    listener.resume();
    println!("Receiver: Anonymous listener ready");

    // 4. Ask launcher to spawn sender with our endpoint
    println!("Receiver: Requesting launcher to spawn sender...");
    let msg = XpcDictionary::new();
    msg.set_string("action", "spawn_sender");
    msg.set_string("session_id", "test-1");
    msg.set_endpoint("receiver_endpoint", endpoint);
    launcher.send(&msg);
    println!("Receiver: Spawn request sent");

    // 5. Set up timeout
    let success_timeout = success.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(30));
        if !success_timeout.load(Ordering::SeqCst) {
            eprintln!("\n=== TIMEOUT ===");
            eprintln!("Receiver: Test timed out after 30 seconds");
            std::process::exit(1);
        }
    });

    // 6. Run event loop (keep peers in scope!)
    println!("Receiver: Waiting for sender...\n");
    let _keep_alive = peers;
    run_loop();
}
