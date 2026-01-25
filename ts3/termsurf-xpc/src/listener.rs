//! XPC Listener wrapper.
//!
//! Listeners accept incoming connections from other processes.

use crate::error::{Result, XpcError};
use crate::ffi;
use std::ffi::CString;

/// An XPC listener that accepts incoming connections.
///
/// There are two types of listeners:
/// 1. Named service listeners (for XPC services registered with launchd)
/// 2. Anonymous listeners (for dynamic peer-to-peer connections)
pub struct XpcListener {
    raw: ffi::xpc_connection_t,
}

unsafe impl Send for XpcListener {}
unsafe impl Sync for XpcListener {}

impl XpcListener {
    /// Create a listener for a named Mach service.
    ///
    /// This is used by XPC services to accept incoming connections.
    /// The service name must be registered in the XPC service's Info.plist.
    pub fn new_mach_service(name: &str) -> Result<Self> {
        let name_c = CString::new(name).unwrap();
        let raw = unsafe {
            ffi::xpc_connection_create_mach_service(
                name_c.as_ptr(),
                ffi::dispatch_get_main_queue(),
                ffi::XPC_CONNECTION_MACH_SERVICE_LISTENER,
            )
        };
        if raw.is_null() {
            return Err(XpcError::NullPointer(
                "xpc_connection_create_mach_service (listener)",
            ));
        }
        Ok(Self { raw })
    }

    /// Create an anonymous listener.
    ///
    /// Anonymous listeners don't have a registered name. Instead, you get an
    /// endpoint that can be sent to other processes, allowing them to connect.
    ///
    /// This is the key mechanism for dynamic peer-to-peer XPC connections.
    pub fn new_anonymous() -> Result<Self> {
        // Create a connection with a NULL name to get an anonymous listener
        // Note: This uses a private but stable pattern
        let raw = unsafe {
            ffi::xpc_connection_create_mach_service(
                std::ptr::null(),
                ffi::dispatch_get_main_queue(),
                ffi::XPC_CONNECTION_MACH_SERVICE_LISTENER,
            )
        };
        if raw.is_null() {
            return Err(XpcError::NullPointer("anonymous listener creation"));
        }
        Ok(Self { raw })
    }

    /// Get the raw pointer.
    pub fn as_raw(&self) -> ffi::xpc_connection_t {
        self.raw
    }

    /// Set the handler for new incoming connections using a raw block pointer.
    ///
    /// # Safety
    /// The handler must be a valid Objective-C block that takes xpc_connection_t.
    /// The block must remain valid for the lifetime of the listener.
    pub unsafe fn set_new_connection_handler_raw(&self, handler: *mut std::ffi::c_void) {
        ffi::xpc_connection_set_event_handler(self.raw, handler);
    }

    /// Resume the listener (listeners start suspended).
    pub fn resume(&self) {
        unsafe {
            ffi::xpc_connection_resume(self.raw);
        }
    }

    /// Get an endpoint for this listener.
    ///
    /// The endpoint can be sent to other processes via XPC message.
    /// They can then use `XpcConnection::from_endpoint()` to connect.
    pub fn get_endpoint(&self) -> Result<ffi::xpc_endpoint_t> {
        let endpoint = unsafe { ffi::xpc_endpoint_create(self.raw) };
        if endpoint.is_null() {
            return Err(XpcError::NullPointer("xpc_endpoint_create"));
        }
        Ok(endpoint)
    }
}

impl Drop for XpcListener {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                ffi::xpc_connection_cancel(self.raw);
                ffi::xpc_release(self.raw);
            }
        }
    }
}
