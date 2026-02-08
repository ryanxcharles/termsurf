#include "xipc.h"
#include <xpc/xpc.h>
#include <IOSurface/IOSurface.h>
#include <stdio.h>
#include <string.h>
#include <dispatch/dispatch.h>

xipc_connection_t xipc_connect(const char *service_name, xipc_frame_callback callback, void *context) {
    xpc_connection_t conn = xpc_connection_create_mach_service(
        service_name,
        dispatch_get_main_queue(),
        0 // Client mode (no listener flag)
    );

    if (!conn) {
        fprintf(stderr, "[XIPC] Failed to connect to %s\n", service_name);
        return NULL;
    }

    fprintf(stderr, "[XIPC] Connecting to %s\n", service_name);

    xpc_connection_set_event_handler(conn, ^(xpc_object_t event) {
        if (xpc_get_type(event) == XPC_TYPE_ERROR) {
            if (event == XPC_ERROR_CONNECTION_INVALID) {
                fprintf(stderr, "[XIPC] Connection invalid\n");
            } else if (event == XPC_ERROR_CONNECTION_INTERRUPTED) {
                fprintf(stderr, "[XIPC] Connection interrupted\n");
            }
            return;
        }

        if (xpc_get_type(event) != XPC_TYPE_DICTIONARY) {
            return;
        }

        const char *action = xpc_dictionary_get_string(event, "action");
        if (action && strcmp(action, "frame") == 0) {
            mach_port_t port = (mach_port_t)xpc_dictionary_copy_mach_send(event, "iosurface_port");
            uint64_t width = xpc_dictionary_get_uint64(event, "width");
            uint64_t height = xpc_dictionary_get_uint64(event, "height");

            fprintf(stderr, "[XIPC] Received frame: port=%u, %llux%llu\n", port, width, height);

            if (callback) {
                callback(port, (uint32_t)width, (uint32_t)height, context);
            }
        }
    });

    xpc_connection_resume(conn);

    // Send a hello message to trigger launchd to start the service.
    // XPC connections are lazy — the actual Mach port lookup happens on first send.
    xpc_object_t hello = xpc_dictionary_create(NULL, NULL, 0);
    xpc_dictionary_set_string(hello, "action", "hello");
    xpc_connection_send_message(conn, hello);
    xpc_release(hello);

    fprintf(stderr, "[XIPC] Sent hello to %s\n", service_name);

    return (xipc_connection_t)conn;
}

void xipc_send_resize(xipc_connection_t conn, uint32_t width, uint32_t height, const char *scale) {
    if (!conn) return;

    xpc_object_t msg = xpc_dictionary_create(NULL, NULL, 0);
    xpc_dictionary_set_string(msg, "action", "resize");
    xpc_dictionary_set_uint64(msg, "width", (uint64_t)width);
    xpc_dictionary_set_uint64(msg, "height", (uint64_t)height);
    xpc_dictionary_set_string(msg, "scale", scale);
    xpc_connection_send_message((xpc_connection_t)conn, msg);
    xpc_release(msg);

    fprintf(stderr, "[XIPC] Sent resize: %ux%u scale=%s\n", width, height, scale);
}

void *xipc_import_iosurface(mach_port_t port) {
    IOSurfaceRef surface = IOSurfaceLookupFromMachPort(port);
    if (!surface) {
        fprintf(stderr, "[XIPC] IOSurfaceLookupFromMachPort(%u) failed\n", port);
    }
    return (void *)surface;
}

void xipc_deallocate_port(mach_port_t port) {
    mach_port_deallocate(mach_task_self(), port);
}

size_t xipc_iosurface_width(void *surface) {
    return IOSurfaceGetWidth((IOSurfaceRef)surface);
}

size_t xipc_iosurface_height(void *surface) {
    return IOSurfaceGetHeight((IOSurfaceRef)surface);
}
