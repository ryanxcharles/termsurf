//! Minimal XPC bindings for TermSurf.
//!
//! This crate provides safe Rust wrappers around Apple's XPC framework,
//! specifically the functionality needed for cross-process IOSurface sharing.
//!
//! # Key Types
//!
//! - [`XpcConnection`] - Connect to XPC services or peers
//! - [`XpcListener`] - Accept incoming XPC connections
//! - [`XpcDictionary`] - XPC message type (key-value container)
//!
//! # Example: Client connecting to a service
//!
//! ```ignore
//! use termsurf_xpc::{XpcConnection, XpcDictionary, set_event_handler};
//!
//! // Connect to an XPC service
//! let conn = XpcConnection::connect_mach_service("com.termsurf.launcher")?;
//!
//! // Set up event handler for responses
//! set_event_handler(&conn, |event| {
//!     match event {
//!         Ok(dict) => println!("Received: {:?}", dict.get_string("status")),
//!         Err(e) => eprintln!("Error: {}", e),
//!     }
//! });
//!
//! // Resume connection
//! conn.resume();
//!
//! // Send a message
//! let msg = XpcDictionary::new();
//! msg.set_string("action", "ping");
//! conn.send(&msg);
//! ```
//!
//! # Example: Creating an anonymous listener and passing endpoint
//!
//! ```ignore
//! use termsurf_xpc::{XpcListener, XpcConnection, XpcDictionary};
//! use termsurf_xpc::{set_event_handler, set_new_connection_handler};
//!
//! // Create anonymous listener
//! let listener = XpcListener::new_anonymous()?;
//!
//! // Set up handler for incoming connections
//! set_new_connection_handler(&listener, |peer| {
//!     set_event_handler(&peer, |event| {
//!         if let Ok(dict) = event {
//!             // Handle incoming message, extract Mach port, etc.
//!             let port = dict.copy_mach_send("iosurface_port");
//!         }
//!     });
//!     peer.resume();
//! });
//!
//! listener.resume();
//!
//! // Get endpoint to send to another process
//! let endpoint = listener.get_endpoint()?;
//!
//! // Send endpoint via another XPC connection
//! let msg = XpcDictionary::new();
//! msg.set_endpoint("gui_endpoint", endpoint);
//! other_connection.send(&msg);
//! ```
//!
//! # Platform Support
//!
//! This crate only works on macOS. On other platforms, compilation will fail.

#![cfg_attr(not(target_os = "macos"), allow(unused))]

#[cfg(target_os = "macos")]
mod block;
#[cfg(target_os = "macos")]
mod connection;
#[cfg(target_os = "macos")]
mod dictionary;
#[cfg(target_os = "macos")]
mod error;
#[cfg(target_os = "macos")]
mod ffi;
#[cfg(target_os = "macos")]
pub mod iosurface;
#[cfg(target_os = "macos")]
mod listener;
#[cfg(target_os = "macos")]
mod runloop;

// Core XPC types
#[cfg(target_os = "macos")]
pub use connection::XpcConnection;
#[cfg(target_os = "macos")]
pub use dictionary::XpcDictionary;
#[cfg(target_os = "macos")]
pub use error::{Result, XpcError};
#[cfg(target_os = "macos")]
pub use listener::XpcListener;

// Block-based event handlers
#[cfg(target_os = "macos")]
pub use block::{set_event_handler, set_new_connection_handler};

// Run loop
#[cfg(target_os = "macos")]
pub use runloop::{dispatch_main, run_loop};

// Re-export types for convenience
#[cfg(target_os = "macos")]
pub use ffi::mach_port_t;

/// A Send-safe wrapper around XPC endpoint.
///
/// XPC endpoints are opaque handles that can be safely transferred between
/// threads and processes. This wrapper makes the raw pointer Send + Sync.
#[cfg(target_os = "macos")]
#[derive(Clone, Copy)]
pub struct XpcEndpoint(ffi::xpc_endpoint_t);

#[cfg(target_os = "macos")]
impl XpcEndpoint {
    /// Create from raw pointer.
    ///
    /// # Safety
    /// The pointer must be a valid xpc_endpoint_t.
    pub unsafe fn from_raw(ptr: ffi::xpc_endpoint_t) -> Self {
        Self(ptr)
    }

    /// Get the raw pointer.
    pub fn as_raw(&self) -> ffi::xpc_endpoint_t {
        self.0
    }

    /// Check if the endpoint is null.
    pub fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

// XPC endpoints are thread-safe handles
#[cfg(target_os = "macos")]
unsafe impl Send for XpcEndpoint {}
#[cfg(target_os = "macos")]
unsafe impl Sync for XpcEndpoint {}

// Stub implementations for non-macOS platforms
#[cfg(not(target_os = "macos"))]
compile_error!("termsurf-xpc only supports macOS");
