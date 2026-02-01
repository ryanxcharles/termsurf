//! Safe block-based event handlers for XPC connections.
//!
//! This module provides Rust closure wrappers for XPC event handlers,
//! using the `block2` crate to create Objective-C blocks.

use crate::connection::XpcConnection;
use crate::dictionary::XpcDictionary;
use crate::error::Result;
use crate::ffi;
use crate::listener::XpcListener;
use block2::RcBlock;
use std::sync::Arc;

/// Set an event handler on a connection using a Rust closure.
///
/// The closure receives parsed events: `Ok(XpcDictionary)` for messages,
/// or `Err(XpcError)` for connection errors.
///
/// # Example
///
/// ```ignore
/// set_event_handler(&conn, |event| {
///     match event {
///         Ok(msg) => println!("Received: {:?}", msg.get_string("action")),
///         Err(e) => eprintln!("Error: {}", e),
///     }
/// });
/// ```
///
/// # Memory Management
///
/// XPC copies the block via `Block_copy()` internally, so we pass a reference
/// and let our `RcBlock` drop after the call. When the connection is canceled,
/// XPC releases its copy via the block's normal ref-counting.
pub fn set_event_handler<F>(conn: &XpcConnection, handler: F)
where
    F: Fn(Result<XpcDictionary>) + Send + 'static,
{
    let handler = Arc::new(handler);

    let block = RcBlock::new(move |event: ffi::xpc_object_t| {
        let result = unsafe { XpcConnection::parse_event(event) };
        handler(result);
    });

    // XPC copies the block via Block_copy(), so we can let ours drop.
    // When the connection is canceled, XPC releases its copy.
    unsafe {
        ffi::xpc_connection_set_event_handler(
            conn.as_raw(),
            &*block as *const _ as *mut std::ffi::c_void,
        );
    }
    // block drops here, decrementing ref count (XPC still has its copy)
}

/// Set a handler for new incoming connections on a listener.
///
/// For XPC service listeners, each incoming client gets its own connection.
/// For anonymous listeners, this receives peer connections.
///
/// # CRITICAL: Peer Connection Lifetime
///
/// **You MUST store the peer connection to keep it alive.** When the handler
/// returns, the `XpcConnection` is dropped, which cancels the connection.
/// If you don't store the peer, you'll see "connection interrupted" errors
/// and messages won't be delivered.
///
/// # Example
///
/// ```ignore
/// // Storage for peer connections - REQUIRED
/// let peers: Arc<Mutex<Vec<XpcConnection>>> = Arc::new(Mutex::new(Vec::new()));
/// let peers_clone = peers.clone();
///
/// set_new_connection_handler(&listener, move |peer| {
///     println!("New connection!");
///     set_event_handler(&peer, |event| {
///         // Handle messages from this peer
///     });
///     peer.resume();
///
///     // CRITICAL: Store the peer to keep it alive!
///     peers_clone.lock().unwrap().push(peer);
/// });
/// ```
///
/// # Common Mistake
///
/// ```ignore
/// // WRONG - peer is dropped when handler returns, connection canceled!
/// set_new_connection_handler(&listener, |peer| {
///     set_event_handler(&peer, |event| { ... });
///     peer.resume();
///     // peer dropped here → connection immediately canceled
/// });
/// ```
pub fn set_new_connection_handler<F>(listener: &XpcListener, handler: F)
where
    F: Fn(XpcConnection) + Send + 'static,
{
    let handler = Arc::new(handler);

    let block = RcBlock::new(move |peer: ffi::xpc_connection_t| {
        if peer.is_null() {
            return;
        }

        // Retain the peer connection since XPC passes it without retaining
        unsafe {
            ffi::xpc_retain(peer as ffi::xpc_object_t);
        }

        // Wrap in XpcConnection (takes ownership)
        let conn = unsafe { XpcConnection::from_raw(peer) };
        if let Ok(conn) = conn {
            handler(conn);
        }
    });

    // XPC copies the block via Block_copy(), so we can let ours drop.
    // When the listener is canceled, XPC releases its copy.
    unsafe {
        ffi::xpc_connection_set_event_handler(
            listener.as_raw(),
            &*block as *const _ as *mut std::ffi::c_void,
        );
    }
    // block drops here, decrementing ref count (XPC still has its copy)
}
