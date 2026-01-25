//! XPC Connection wrapper.

use crate::dictionary::XpcDictionary;
use crate::error::{Result, XpcError};
use crate::ffi;
use std::ffi::CString;

/// An XPC connection to a service or peer.
pub struct XpcConnection {
    raw: ffi::xpc_connection_t,
}

// XPC connections are thread-safe
unsafe impl Send for XpcConnection {}
unsafe impl Sync for XpcConnection {}

impl XpcConnection {
    /// Connect to an XPC Mach service by name.
    ///
    /// The service must be registered with launchd (via Info.plist for XPC services,
    /// or via launchd.plist for Mach services).
    ///
    /// The connection starts suspended; call `resume()` to activate it.
    pub fn connect_mach_service(name: &str) -> Result<Self> {
        let name_c = CString::new(name).unwrap();
        let raw = unsafe {
            ffi::xpc_connection_create_mach_service(
                name_c.as_ptr(),
                ffi::dispatch_get_main_queue(),
                0, // Client mode
            )
        };
        if raw.is_null() {
            return Err(XpcError::NullPointer("xpc_connection_create_mach_service"));
        }
        Ok(Self { raw })
    }

    /// Create a connection from an endpoint received from another process.
    ///
    /// This is used after receiving an endpoint via XPC message to establish
    /// a direct connection to the original listener.
    pub fn from_endpoint(endpoint: crate::XpcEndpoint) -> Result<Self> {
        if endpoint.is_null() {
            return Err(XpcError::NullPointer("XpcConnection::from_endpoint"));
        }
        let raw = unsafe { ffi::xpc_connection_create_from_endpoint(endpoint.as_raw()) };
        if raw.is_null() {
            return Err(XpcError::NullPointer(
                "xpc_connection_create_from_endpoint",
            ));
        }
        Ok(Self { raw })
    }

    /// Wrap a raw XPC connection pointer.
    ///
    /// # Safety
    /// The pointer must be a valid xpc_connection_t.
    pub unsafe fn from_raw(raw: ffi::xpc_connection_t) -> Result<Self> {
        if raw.is_null() {
            return Err(XpcError::NullPointer("XpcConnection::from_raw"));
        }
        Ok(Self { raw })
    }

    /// Get the raw pointer.
    pub fn as_raw(&self) -> ffi::xpc_connection_t {
        self.raw
    }

    /// Set the event handler for incoming messages using a raw block pointer.
    ///
    /// # Safety
    /// The handler must be a valid Objective-C block that takes xpc_object_t.
    /// The block must remain valid for the lifetime of the connection.
    pub unsafe fn set_event_handler_raw(&self, handler: *mut std::ffi::c_void) {
        ffi::xpc_connection_set_event_handler(self.raw, handler);
    }

    /// Parse an XPC event into a Result.
    ///
    /// # Safety
    /// The event must be a valid xpc_object_t.
    pub unsafe fn parse_event(event: ffi::xpc_object_t) -> Result<XpcDictionary> {
        if event.is_null() {
            return Err(XpcError::NullPointer("event handler"));
        }

        // Check for errors
        if ffi::xpc_is_error(event) {
            if event == ffi::XPC_ERROR_CONNECTION_INTERRUPTED {
                return Err(XpcError::ConnectionInterrupted);
            } else if event == ffi::XPC_ERROR_CONNECTION_INVALID {
                return Err(XpcError::ConnectionInvalid);
            } else if event == ffi::XPC_ERROR_TERMINATION_IMMINENT {
                return Err(XpcError::TerminationImminent);
            } else {
                return Err(XpcError::Unknown("unknown XPC error".into()));
            }
        }

        // Should be a dictionary
        XpcDictionary::from_raw(event, false)
    }

    /// Resume the connection (connections start suspended).
    pub fn resume(&self) {
        unsafe {
            ffi::xpc_connection_resume(self.raw);
        }
    }

    /// Suspend the connection.
    pub fn suspend(&self) {
        unsafe {
            ffi::xpc_connection_suspend(self.raw);
        }
    }

    /// Cancel the connection.
    pub fn cancel(&self) {
        unsafe {
            ffi::xpc_connection_cancel(self.raw);
        }
    }

    /// Send a message (fire-and-forget).
    pub fn send(&self, message: &XpcDictionary) {
        unsafe {
            ffi::xpc_connection_send_message(self.raw, message.as_raw());
        }
    }

    /// Send a message and wait synchronously for a reply.
    ///
    /// This blocks the current thread until a reply is received.
    pub fn send_with_reply_sync(&self, message: &XpcDictionary) -> Result<XpcDictionary> {
        let reply = unsafe {
            ffi::xpc_connection_send_message_with_reply_sync(self.raw, message.as_raw())
        };

        if reply.is_null() {
            return Err(XpcError::NullPointer("send_with_reply_sync"));
        }

        // Check for errors
        unsafe {
            if ffi::xpc_is_error(reply) {
                let err = if reply == ffi::XPC_ERROR_CONNECTION_INTERRUPTED {
                    XpcError::ConnectionInterrupted
                } else if reply == ffi::XPC_ERROR_CONNECTION_INVALID {
                    XpcError::ConnectionInvalid
                } else {
                    XpcError::Unknown("unknown XPC error".into())
                };
                ffi::xpc_release(reply);
                return Err(err);
            }

            XpcDictionary::from_raw(reply, true)
        }
    }

    /// Create an endpoint for this connection.
    ///
    /// The endpoint can be sent to other processes, allowing them to
    /// establish a direct connection back to this connection's listener.
    pub fn create_endpoint(&self) -> Result<crate::XpcEndpoint> {
        let endpoint = unsafe { ffi::xpc_endpoint_create(self.raw) };
        if endpoint.is_null() {
            return Err(XpcError::NullPointer("xpc_endpoint_create"));
        }
        Ok(unsafe { crate::XpcEndpoint::from_raw(endpoint) })
    }
}

impl Drop for XpcConnection {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe {
                ffi::xpc_connection_cancel(self.raw);
                ffi::xpc_release(self.raw);
            }
        }
    }
}
