//! Run loop utilities for XPC event processing.
//!
//! XPC uses dispatch queues and run loops to deliver events.
//! This module provides helpers for running the event loop.

use crate::ffi;

/// Run the current thread's run loop.
///
/// This blocks the current thread and processes XPC events until the
/// run loop is stopped or the program exits.
///
/// Call this after setting up your connections and listeners.
///
/// # Example
///
/// ```ignore
/// // Set up connections and handlers...
/// let conn = XpcConnection::connect_mach_service("com.example.service")?;
/// set_event_handler(&conn, |event| { ... });
/// conn.resume();
///
/// // Block and process events
/// run_loop();
/// ```
pub fn run_loop() {
    unsafe {
        ffi::CFRunLoopRun();
    }
    // CFRunLoopRun returns when explicitly stopped via stop_run_loop()
}

/// Stop the main run loop.
///
/// This causes `run_loop()` to return on the main thread.
/// Call this from an event handler when you want to shut down gracefully.
pub fn stop_run_loop() {
    unsafe {
        ffi::CFRunLoopStop(ffi::CFRunLoopGetMain());
    }
}

/// Run the main dispatch queue.
///
/// This is an alternative to `run_loop()` that uses Grand Central Dispatch
/// instead of CFRunLoop. It never returns.
///
/// Use this if your XPC handlers use dispatch queues rather than the main
/// run loop.
pub fn dispatch_main() -> ! {
    unsafe {
        ffi::dispatch_main();
    }
}
