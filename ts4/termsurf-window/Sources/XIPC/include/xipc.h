#ifndef XIPC_H
#define XIPC_H

#include <stdint.h>
#include <mach/mach.h>

/// Opaque handle to an XPC connection.
typedef void *xipc_connection_t;

/// Callback invoked when a "frame" message is received via XPC.
/// The callback receives the IOSurface Mach port, width, height, and user context.
typedef void (*xipc_frame_callback)(mach_port_t port, uint32_t width, uint32_t height, void *context);

/// Connect to a named XPC Mach service (client mode).
///
/// The service must be registered with launchd. When the service sends a
/// message with action "frame", the callback is invoked with the IOSurface
/// Mach port and dimensions.
///
/// Returns a connection handle that can be used to send messages (e.g., resize).
/// This function returns immediately. Events are dispatched on the main queue.
xipc_connection_t xipc_connect(const char *service_name, xipc_frame_callback callback, void *context);

/// Send a resize message to a child process.
///
/// The child process should create a new IOSurface at the given dimensions,
/// render to it, and send back a new "frame" message with the updated Mach port.
void xipc_send_resize(xipc_connection_t conn, uint32_t width, uint32_t height, const char *scale);

/// Import an IOSurface from a Mach port received from another process.
/// Returns the IOSurfaceRef as void*, or NULL on failure.
void *xipc_import_iosurface(mach_port_t port);

/// Deallocate a Mach port send right after importing the IOSurface.
/// Call this after xipc_import_iosurface to avoid leaking kernel resources.
void xipc_deallocate_port(mach_port_t port);

/// Get the width of an IOSurface.
size_t xipc_iosurface_width(void *surface);

/// Get the height of an IOSurface.
size_t xipc_iosurface_height(void *surface);

#endif
