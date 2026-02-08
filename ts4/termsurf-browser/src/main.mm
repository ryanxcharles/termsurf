#import <Metal/Metal.h>
#import <IOSurface/IOSurface.h>
#import <CoreFoundation/CoreFoundation.h>
#include <xpc/xpc.h>
#include <dispatch/dispatch.h>
#include <cstdio>
#include <mach/mach.h>

/// Create an IOSurface, render it green via Metal, and return the handle.
/// The caller is responsible for releasing the IOSurface when done.
static IOSurfaceRef render_green(id<MTLDevice> device, id<MTLCommandQueue> commandQueue,
                                  int width, int height) {
    int bytesPerElement = 4;
    // Metal requires bytesPerRow aligned to 16 bytes for IOSurface-backed textures.
    int bytesPerRow = (width * bytesPerElement + 15) & ~15;
    OSType pixelFormat = 'BGRA';

    NSDictionary *properties = @{
        (id)kIOSurfaceWidth: @(width),
        (id)kIOSurfaceHeight: @(height),
        (id)kIOSurfaceBytesPerElement: @(bytesPerElement),
        (id)kIOSurfaceBytesPerRow: @(bytesPerRow),
        (id)kIOSurfacePixelFormat: @(pixelFormat),
    };

    IOSurfaceRef surface = IOSurfaceCreate((__bridge CFDictionaryRef)properties);
    if (!surface) {
        fprintf(stderr, "[Browser] ERROR: Failed to create IOSurface %dx%d\n", width, height);
        return nullptr;
    }

    fprintf(stderr, "[Browser] IOSurface created: %dx%d\n", width, height);

    // Create Metal texture backed by the IOSurface (zero-copy)
    MTLTextureDescriptor *texDesc = [MTLTextureDescriptor
        texture2DDescriptorWithPixelFormat:MTLPixelFormatBGRA8Unorm
                                     width:width
                                    height:height
                                 mipmapped:NO];
    texDesc.usage = MTLTextureUsageRenderTarget;
    texDesc.storageMode = MTLStorageModeShared;

    id<MTLTexture> texture = [device newTextureWithDescriptor:texDesc
                                                    iosurface:surface
                                                        plane:0];
    if (!texture) {
        fprintf(stderr, "[Browser] ERROR: Failed to create Metal texture\n");
        CFRelease(surface);
        return nullptr;
    }

    // Render clear-to-green
    MTLRenderPassDescriptor *passDesc = [MTLRenderPassDescriptor renderPassDescriptor];
    passDesc.colorAttachments[0].texture = texture;
    passDesc.colorAttachments[0].loadAction = MTLLoadActionClear;
    passDesc.colorAttachments[0].storeAction = MTLStoreActionStore;
    passDesc.colorAttachments[0].clearColor = MTLClearColorMake(0.0, 1.0, 0.0, 1.0);

    id<MTLCommandBuffer> commandBuffer = [commandQueue commandBuffer];
    id<MTLRenderCommandEncoder> encoder =
        [commandBuffer renderCommandEncoderWithDescriptor:passDesc];
    [encoder endEncoding];
    [commandBuffer commit];
    [commandBuffer waitUntilCompleted];

    // Verify pixel
    IOSurfaceLock(surface, kIOSurfaceLockReadOnly, nullptr);
    auto *base = static_cast<const uint8_t *>(IOSurfaceGetBaseAddress(surface));
    uint8_t b0 = base[0], g0 = base[1], r0 = base[2], a0 = base[3];
    IOSurfaceUnlock(surface, kIOSurfaceLockReadOnly, nullptr);
    fprintf(stderr, "[Browser] Rendered green, pixel (0,0): (%u, %u, %u, %u)\n", r0, g0, b0, a0);

    return surface;
}

/// Send a frame message with the IOSurface Mach port via the given connection.
static void send_frame(xpc_connection_t peer, IOSurfaceRef surface, int width, int height) {
    mach_port_t port = IOSurfaceCreateMachPort(surface);
    fprintf(stderr, "[Browser] Created Mach port: %u\n", port);

    xpc_object_t msg = xpc_dictionary_create(NULL, NULL, 0);
    xpc_dictionary_set_string(msg, "action", "frame");
    xpc_dictionary_set_mach_send(msg, "iosurface_port", port);
    xpc_dictionary_set_uint64(msg, "width", (uint64_t)width);
    xpc_dictionary_set_uint64(msg, "height", (uint64_t)height);
    xpc_connection_send_message(peer, msg);

    fprintf(stderr, "[Browser] Frame sent: %dx%d\n", width, height);
}

int main() {
    fprintf(stderr, "[Browser] Starting...\n");

    // Step 1: Create Metal device (reused across resizes).
    id<MTLDevice> device = MTLCreateSystemDefaultDevice();
    if (!device) {
        fprintf(stderr, "[Browser] ERROR: No Metal device available\n");
        return 1;
    }

    id<MTLCommandQueue> commandQueue = [device newCommandQueue];
    fprintf(stderr, "[Browser] Metal device: %s\n", [[device name] UTF8String]);

    // Step 2: Render initial green IOSurface.
    int width = 800;
    int height = 600;
    IOSurfaceRef surface = render_green(device, commandQueue, width, height);
    if (!surface) return 1;

    // Step 3: Set up XPC listener.
    const char *service_name = "com.termsurf.ts4.browser";
    xpc_connection_t listener = xpc_connection_create_mach_service(
        service_name,
        dispatch_get_main_queue(),
        XPC_CONNECTION_MACH_SERVICE_LISTENER
    );

    if (!listener) {
        fprintf(stderr, "[Browser] Failed to create XPC listener\n");
        return 1;
    }

    xpc_connection_set_event_handler(listener, ^(xpc_object_t peer) {
        if (xpc_get_type(peer) == XPC_TYPE_ERROR) {
            fprintf(stderr, "[Browser] Listener error\n");
            return;
        }

        fprintf(stderr, "[Browser] New client connected\n");

        xpc_connection_set_event_handler(peer, ^(xpc_object_t event) {
            if (xpc_get_type(event) == XPC_TYPE_ERROR) {
                if (event == XPC_ERROR_CONNECTION_INVALID) {
                    fprintf(stderr, "[Browser] Client disconnected\n");
                }
                return;
            }

            if (xpc_get_type(event) != XPC_TYPE_DICTIONARY) return;

            const char *action = xpc_dictionary_get_string(event, "action");
            if (!action) return;

            fprintf(stderr, "[Browser] Received: %s\n", action);

            if (strcmp(action, "resize") == 0) {
                int w = (int)xpc_dictionary_get_uint64(event, "width");
                int h = (int)xpc_dictionary_get_uint64(event, "height");
                fprintf(stderr, "[Browser] Resizing to %dx%d\n", w, h);

                IOSurfaceRef new_surface = render_green(device, commandQueue, w, h);
                if (new_surface) {
                    xpc_connection_t remote = xpc_dictionary_get_remote_connection(event);
                    send_frame(remote, new_surface, w, h);
                }
            }
        });
        xpc_connection_resume(peer);

        // Send initial frame.
        send_frame(peer, surface, width, height);
    });

    xpc_connection_resume(listener);
    fprintf(stderr, "[Browser] Listening on %s\n", service_name);

    // Step 4: Block forever, processing XPC events.
    dispatch_main();
}
