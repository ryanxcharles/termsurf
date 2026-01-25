//! XPC Dictionary wrapper.

use crate::error::{Result, XpcError};
use crate::ffi;
use std::ffi::{CStr, CString};
use std::ptr;

/// An XPC dictionary object.
///
/// Dictionaries are the primary message type in XPC communication.
/// They can contain strings, integers, Mach ports, endpoints, and nested dictionaries.
pub struct XpcDictionary {
    raw: ffi::xpc_object_t,
    /// If true, we own the object and must release it on drop.
    owned: bool,
}

// XPC objects are thread-safe
unsafe impl Send for XpcDictionary {}
unsafe impl Sync for XpcDictionary {}

impl XpcDictionary {
    /// Create a new empty dictionary.
    pub fn new() -> Self {
        let raw = unsafe { ffi::xpc_dictionary_create(ptr::null(), ptr::null(), 0) };
        Self { raw, owned: true }
    }

    /// Create a reply dictionary for a received message.
    ///
    /// The reply will automatically be routed back to the sender.
    pub fn create_reply(original: &XpcDictionary) -> Result<Self> {
        let raw = unsafe { ffi::xpc_dictionary_create_reply(original.raw) };
        if raw.is_null() {
            return Err(XpcError::NullPointer("xpc_dictionary_create_reply"));
        }
        Ok(Self { raw, owned: true })
    }

    /// Wrap a raw XPC dictionary pointer.
    ///
    /// # Safety
    /// The pointer must be a valid xpc_object_t of dictionary type.
    /// If `owned` is true, the dictionary will be released on drop.
    pub unsafe fn from_raw(raw: ffi::xpc_object_t, owned: bool) -> Result<Self> {
        if raw.is_null() {
            return Err(XpcError::NullPointer("XpcDictionary::from_raw"));
        }
        if !ffi::xpc_is_dictionary(raw) {
            return Err(XpcError::TypeMismatch {
                expected: "dictionary",
                context: "from_raw",
            });
        }
        Ok(Self { raw, owned })
    }

    /// Get the raw pointer (for passing to FFI functions).
    pub fn as_raw(&self) -> ffi::xpc_object_t {
        self.raw
    }

    // === Setters ===

    /// Set a string value.
    pub fn set_string(&self, key: &str, value: &str) {
        let key_c = CString::new(key).unwrap();
        let value_c = CString::new(value).unwrap();
        unsafe {
            ffi::xpc_dictionary_set_string(self.raw, key_c.as_ptr(), value_c.as_ptr());
        }
    }

    /// Set an i64 value.
    pub fn set_i64(&self, key: &str, value: i64) {
        let key_c = CString::new(key).unwrap();
        unsafe {
            ffi::xpc_dictionary_set_int64(self.raw, key_c.as_ptr(), value);
        }
    }

    /// Set a u64 value.
    pub fn set_u64(&self, key: &str, value: u64) {
        let key_c = CString::new(key).unwrap();
        unsafe {
            ffi::xpc_dictionary_set_uint64(self.raw, key_c.as_ptr(), value);
        }
    }

    /// Set a boolean value.
    pub fn set_bool(&self, key: &str, value: bool) {
        let key_c = CString::new(key).unwrap();
        unsafe {
            ffi::xpc_dictionary_set_bool(self.raw, key_c.as_ptr(), value);
        }
    }

    /// Set a Mach send right.
    ///
    /// The port is moved into the dictionary. After this call, the caller
    /// should not use or deallocate the port.
    pub fn set_mach_send(&self, key: &str, port: ffi::mach_port_t) {
        let key_c = CString::new(key).unwrap();
        unsafe {
            ffi::xpc_dictionary_set_mach_send(self.raw, key_c.as_ptr(), port);
        }
    }

    /// Set an XPC endpoint.
    pub fn set_endpoint(&self, key: &str, endpoint: ffi::xpc_endpoint_t) {
        let key_c = CString::new(key).unwrap();
        unsafe {
            ffi::xpc_dictionary_set_value(self.raw, key_c.as_ptr(), endpoint);
        }
    }

    /// Set a nested dictionary.
    pub fn set_dictionary(&self, key: &str, dict: &XpcDictionary) {
        let key_c = CString::new(key).unwrap();
        unsafe {
            ffi::xpc_dictionary_set_value(self.raw, key_c.as_ptr(), dict.raw);
        }
    }

    // === Getters ===

    /// Get a string value.
    pub fn get_string(&self, key: &str) -> Option<String> {
        let key_c = CString::new(key).unwrap();
        let ptr = unsafe { ffi::xpc_dictionary_get_string(self.raw, key_c.as_ptr()) };
        if ptr.is_null() {
            return None;
        }
        let cstr = unsafe { CStr::from_ptr(ptr) };
        Some(cstr.to_string_lossy().into_owned())
    }

    /// Get an i64 value.
    pub fn get_i64(&self, key: &str) -> i64 {
        let key_c = CString::new(key).unwrap();
        unsafe { ffi::xpc_dictionary_get_int64(self.raw, key_c.as_ptr()) }
    }

    /// Get a u64 value.
    pub fn get_u64(&self, key: &str) -> u64 {
        let key_c = CString::new(key).unwrap();
        unsafe { ffi::xpc_dictionary_get_uint64(self.raw, key_c.as_ptr()) }
    }

    /// Get a boolean value.
    pub fn get_bool(&self, key: &str) -> bool {
        let key_c = CString::new(key).unwrap();
        unsafe { ffi::xpc_dictionary_get_bool(self.raw, key_c.as_ptr()) }
    }

    /// Get a Mach send right (copies the right, caller must deallocate).
    pub fn copy_mach_send(&self, key: &str) -> ffi::mach_port_t {
        let key_c = CString::new(key).unwrap();
        unsafe { ffi::xpc_dictionary_copy_mach_send(self.raw, key_c.as_ptr()) }
    }

    /// Get an XPC endpoint.
    pub fn get_endpoint(&self, key: &str) -> Option<ffi::xpc_endpoint_t> {
        let key_c = CString::new(key).unwrap();
        let value = unsafe { ffi::xpc_dictionary_get_value(self.raw, key_c.as_ptr()) };
        if value.is_null() {
            return None;
        }
        if !unsafe { ffi::xpc_is_endpoint(value) } {
            return None;
        }
        // Retain since get_value doesn't transfer ownership
        Some(unsafe { ffi::xpc_retain(value) })
    }
}

impl Default for XpcDictionary {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for XpcDictionary {
    fn drop(&mut self) {
        if self.owned && !self.raw.is_null() {
            unsafe {
                ffi::xpc_release(self.raw);
            }
        }
    }
}
