//! Test Sender for Experiment 2.
//!
//! Spawned by launcher, claims session to get GUI endpoint, creates
//! IOSurface and sends Mach port to GUI for rendering.
//!
//! Service name: com.termsurf.launcher

use clap::Parser;
use std::thread;
use std::time::Duration;
use termsurf_xpc::*;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    session_id: String,
}

fn main() {
    let args = Args::parse();
    println!("Sender: Starting for session '{}'", args.session_id);

    // 1. Connect to launcher
    println!("Sender: Connecting to launcher...");
    let launcher = match XpcConnection::connect_mach_service("com.termsurf.launcher") {
        Ok(c) => {
            println!("Sender: Connected to launcher");
            c
        }
        Err(e) => {
            eprintln!("Sender: Failed to connect to launcher: {}", e);
            std::process::exit(1);
        }
    };

    set_event_handler(&launcher, |event| {
        if let Err(e) = event {
            eprintln!("Sender: Launcher error: {}", e);
        }
    });
    launcher.resume();

    // Give the connection a moment to establish
    thread::sleep(Duration::from_millis(100));

    // 2. Claim session with retry (session may not be registered yet)
    println!("Sender: Claiming session...");
    let gui_endpoint = match claim_session_with_retry(&launcher, &args.session_id) {
        Ok(ep) => {
            println!("Sender: Got GUI endpoint");
            ep
        }
        Err(e) => {
            eprintln!("Sender: Failed to claim session: {}", e);
            std::process::exit(1);
        }
    };

    // 3. Connect directly to GUI
    println!("Sender: Connecting to GUI...");
    let gui = match XpcConnection::from_endpoint(gui_endpoint) {
        Ok(c) => {
            println!("Sender: Connected to GUI");
            c
        }
        Err(e) => {
            eprintln!("Sender: Failed to connect to GUI: {}", e);
            std::process::exit(1);
        }
    };

    set_event_handler(&gui, |event| {
        if let Err(e) = event {
            eprintln!("Sender: GUI error: {}", e);
        }
    });
    gui.resume();

    // Give connection time to establish
    thread::sleep(Duration::from_millis(100));

    // 4. Create pink IOSurface (100x100, hot pink 0xFF69B4)
    println!("Sender: Creating IOSurface...");
    let surface = match iosurface::create_iosurface(100, 100) {
        Ok(s) => {
            println!("Sender: Created 100x100 IOSurface");
            s
        }
        Err(e) => {
            eprintln!("Sender: Failed to create IOSurface: {}", e);
            std::process::exit(1);
        }
    };

    // Fill with hot pink (255, 105, 180)
    iosurface::fill_with_color(surface, 255, 105, 180, 255);
    println!("Sender: Filled with hot pink (255, 105, 180, 255)");

    // 5. Create Mach port and send to GUI
    let port = iosurface::create_mach_port(surface);
    if port == 0 {
        eprintln!("Sender: create_mach_port failed");
        std::process::exit(1);
    }
    println!("Sender: Created Mach port: {}", port);

    let msg = XpcDictionary::new();
    msg.set_string("action", "display_surface");
    msg.set_mach_send("iosurface_port", port);
    msg.set_i64("width", 100);
    msg.set_i64("height", 100);
    gui.send(&msg);
    println!("Sender: Sent IOSurface Mach port to GUI");

    // 6. Keep alive briefly to ensure message delivered
    thread::sleep(Duration::from_secs(2));
    println!("Sender: Done");
}

/// Claim session with exponential backoff retry.
/// The session may not be registered yet if we start before the launcher
/// finishes processing the spawn request.
fn claim_session_with_retry(launcher: &XpcConnection, session_id: &str) -> Result<XpcEndpoint> {
    let max_retries = 10;
    let mut delay = Duration::from_millis(100);

    for attempt in 1..=max_retries {
        let msg = XpcDictionary::new();
        msg.set_string("action", "claim_session");
        msg.set_string("session_id", session_id);

        match launcher.send_with_reply_sync(&msg) {
            Ok(reply) => {
                // Check for error in reply
                if let Some(err) = reply.get_string("error") {
                    println!(
                        "Sender: Attempt {}/{}: {}",
                        attempt, max_retries, err
                    );
                    if attempt < max_retries {
                        thread::sleep(delay);
                        delay = (delay * 2).min(Duration::from_secs(2)); // Cap at 2s
                        continue;
                    }
                    return Err(XpcError::Unknown(err));
                }

                // Success - get endpoint
                if let Some(endpoint) = reply.get_endpoint("endpoint") {
                    return Ok(endpoint);
                }
                return Err(XpcError::Unknown("No endpoint in reply".into()));
            }
            Err(e) => {
                println!(
                    "Sender: Attempt {}/{}: {:?}",
                    attempt, max_retries, e
                );
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
