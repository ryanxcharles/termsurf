// XPC communication for TermSurf Ghost (Issue 601).
//
// Connects to the xpc-gateway, creates an anonymous listener, registers its
// endpoint, and handles connections from `web` processes.
//
// Manual extern declarations instead of @cImport("xpc/xpc.h") because the XPC
// header uses C block types in function signatures, which Zig's translate-c may
// not handle. All XPC types are opaque pointers represented as ?*anyopaque.

const std = @import("std");
const objc = @import("objc");
const CoreApp = @import("../App.zig");
const CoreSurface = @import("../Surface.zig");

const log = std.log.scoped(.xpc);

// -- IOSurface / CoreFoundation C API (test IOSurface, Issue 603) --

extern "c" fn IOSurfaceCreate(properties: *anyopaque) ?*anyopaque;
extern "c" fn IOSurfaceLock(surface: *anyopaque, options: u32, seed: ?*u32) i32;
extern "c" fn IOSurfaceUnlock(surface: *anyopaque, options: u32, seed: ?*u32) i32;
extern "c" fn IOSurfaceGetBaseAddress(surface: *anyopaque) ?[*]u8;
extern "c" fn IOSurfaceGetBytesPerRow(surface: *anyopaque) usize;

extern "c" fn CFDictionaryCreateMutable(allocator: ?*anyopaque, capacity: isize, key_cbs: ?*const anyopaque, val_cbs: ?*const anyopaque) ?*anyopaque;
extern "c" fn CFDictionarySetValue(dict: *anyopaque, key: *const anyopaque, value: *const anyopaque) void;
extern "c" fn CFNumberCreate(allocator: ?*anyopaque, the_type: i32, value_ptr: *const anyopaque) ?*anyopaque;
extern "c" fn CFRelease(cf: *anyopaque) void;

extern const kCFTypeDictionaryKeyCallBacks: anyopaque;
extern const kCFTypeDictionaryValueCallBacks: anyopaque;
// These are CFStringRef values (pointers), not structs.
extern const kIOSurfaceWidth: *const anyopaque;
extern const kIOSurfaceHeight: *const anyopaque;
extern const kIOSurfaceBytesPerElement: *const anyopaque;
extern const kIOSurfacePixelFormat: *const anyopaque;

// -- XPC C API --

const xpc_object_t = ?*anyopaque;

extern "c" fn xpc_connection_create_mach_service(name: [*:0]const u8, targetq: xpc_object_t, flags: u64) xpc_object_t;
extern "c" fn xpc_connection_set_event_handler(connection: xpc_object_t, handler: xpc_object_t) void;
extern "c" fn xpc_connection_resume(connection: xpc_object_t) void;
extern "c" fn xpc_connection_cancel(connection: xpc_object_t) void;
extern "c" fn xpc_connection_send_message(connection: xpc_object_t, message: xpc_object_t) void;
extern "c" fn xpc_connection_create(name: ?[*:0]const u8, targetq: xpc_object_t) xpc_object_t;
extern "c" fn xpc_endpoint_create(connection: xpc_object_t) xpc_object_t;
extern "c" fn xpc_dictionary_create(keys: xpc_object_t, values: xpc_object_t, count: usize) xpc_object_t;
extern "c" fn xpc_dictionary_set_string(xdict: xpc_object_t, key: [*:0]const u8, string: [*:0]const u8) void;
extern "c" fn xpc_dictionary_set_value(xdict: xpc_object_t, key: [*:0]const u8, value: xpc_object_t) void;
extern "c" fn xpc_dictionary_get_string(xdict: xpc_object_t, key: [*:0]const u8) ?[*:0]const u8;
extern "c" fn xpc_dictionary_get_uint64(xdict: xpc_object_t, key: [*:0]const u8) u64;
extern "c" fn xpc_dictionary_get_bool(xdict: xpc_object_t, key: [*:0]const u8) bool;
extern "c" fn xpc_get_type(object: xpc_object_t) xpc_object_t;
extern "c" fn xpc_retain(object: xpc_object_t) xpc_object_t;
extern "c" fn xpc_release(object: xpc_object_t) void;

// XPC type/error constants (compared by address identity).
extern const _xpc_type_connection: anyopaque;
extern const _xpc_type_error: anyopaque;
extern const _xpc_type_dictionary: anyopaque;
extern const _xpc_error_connection_invalid: anyopaque;

/// Cast a const extern symbol address to xpc_object_t for identity comparison.
inline fn xpcPtr(ptr: *const anyopaque) xpc_object_t {
    return @constCast(ptr);
}

// -- Module state --

var app: *CoreApp = undefined;
var gateway: xpc_object_t = null;
var listener: xpc_object_t = null;
var web_peer: xpc_object_t = null;
var overlay_surface: ?*CoreSurface = null;

// -- Block type --
//
// XPC event handler: fn(xpc_object_t) void.
// No captures — module-level state is used instead.
const EventBlock = objc.Block(struct {}, .{xpc_object_t}, void);

// -- Public API --

pub fn init(core_app: *CoreApp) void {
    app = core_app;
    log.info("connecting to xpc-gateway", .{});

    // Connect to the xpc-gateway Mach service.
    gateway = xpc_connection_create_mach_service("com.termsurf.xpc-gateway", null, 0);
    var gw_block = EventBlock.init(.{}, &gatewayHandler);
    xpc_connection_set_event_handler(gateway, @ptrCast(&gw_block));
    xpc_connection_resume(gateway);

    // Create anonymous listener for direct peer connections.
    listener = xpc_connection_create(null, null);
    var listener_block = EventBlock.init(.{}, &listenerHandler);
    xpc_connection_set_event_handler(listener, @ptrCast(&listener_block));
    xpc_connection_resume(listener);

    // Register endpoint with the gateway.
    const endpoint = xpc_endpoint_create(listener);
    const msg = xpc_dictionary_create(null, null, 0);
    xpc_dictionary_set_string(msg, "action", "register_app");
    xpc_dictionary_set_value(msg, "endpoint", endpoint);
    xpc_connection_send_message(gateway, msg);

    log.info("registered endpoint with xpc-gateway", .{});
}

pub fn deinit() void {
    if (web_peer) |peer| {
        xpc_release(peer);
        web_peer = null;
    }
    if (listener != null) {
        xpc_connection_cancel(listener);
        listener = null;
    }
    if (gateway != null) {
        xpc_connection_cancel(gateway);
        gateway = null;
    }
    log.info("xpc connections closed", .{});
}

// -- Event handlers --

fn gatewayHandler(_: *const EventBlock.Context, event: xpc_object_t) callconv(.c) void {
    if (xpc_get_type(event) == xpcPtr(&_xpc_type_error)) {
        log.err("gateway connection error", .{});
    }
}

fn listenerHandler(_: *const EventBlock.Context, event: xpc_object_t) callconv(.c) void {
    if (xpc_get_type(event) == xpcPtr(&_xpc_type_connection)) {
        log.info("peer connected", .{});

        web_peer = xpc_retain(event);

        var peer_block = EventBlock.init(.{}, &peerHandler);
        xpc_connection_set_event_handler(event, @ptrCast(&peer_block));
        xpc_connection_resume(event);
    }
}

fn peerHandler(_: *const EventBlock.Context, event: xpc_object_t) callconv(.c) void {
    if (xpc_get_type(event) == xpcPtr(&_xpc_type_dictionary)) {
        handleMessage(event);
    } else if (xpc_get_type(event) == xpcPtr(&_xpc_type_error)) {
        if (event == xpcPtr(&_xpc_error_connection_invalid)) {
            if (overlay_surface) |surface| {
                surface.clearOverlay();
                overlay_surface = null;
            }
            if (web_peer) |peer| {
                xpc_release(peer);
                web_peer = null;
            }
            log.info("peer disconnected", .{});
        }
    }
}

fn handleMessage(msg: xpc_object_t) void {
    const action = xpc_dictionary_get_string(msg, "action") orelse {
        log.warn("message missing 'action'", .{});
        return;
    };
    const action_str = std.mem.span(action);

    if (std.mem.eql(u8, action_str, "set_overlay")) {
        handleSetOverlay(msg);
    } else if (std.mem.eql(u8, action_str, "mode_changed")) {
        handleModeChanged(msg);
    } else {
        log.warn("unknown action: {s}", .{action_str});
    }
}

fn handleSetOverlay(msg: xpc_object_t) void {
    const pane_id = str(xpc_dictionary_get_string(msg, "pane_id"));
    const url = str(xpc_dictionary_get_string(msg, "url"));
    const profile = str(xpc_dictionary_get_string(msg, "profile"));
    const col = xpc_dictionary_get_uint64(msg, "col");
    const row = xpc_dictionary_get_uint64(msg, "row");
    const width = xpc_dictionary_get_uint64(msg, "width");
    const height = xpc_dictionary_get_uint64(msg, "height");
    const browsing = xpc_dictionary_get_bool(msg, "browsing");

    log.info("set_overlay pane={s} col={} row={} width={} height={} url={s} profile={s} browsing={}", .{
        pane_id, col, row, width, height, url, profile, browsing,
    });

    // Look up the surface by pane ID and set the overlay.
    if (app.findSurfaceByPaneId(pane_id)) |surface| {
        overlay_surface = surface.core();
        surface.core().setOverlay(
            @intCast(col),
            @intCast(row),
            @intCast(width),
            @intCast(height),
        );

        // Test IOSurface: blue checkerboard (Issue 603 Experiment 1).
        if (createTestIOSurface()) |iosurface| {
            surface.core().setOverlayIOSurface(iosurface);
            log.info("test IOSurface set for pane={s}", .{pane_id});
        }

        log.info("overlay set for pane={s}", .{pane_id});
    } else {
        log.warn("no surface found for pane={s}", .{pane_id});
    }
}

fn handleModeChanged(msg: xpc_object_t) void {
    const pane_id = str(xpc_dictionary_get_string(msg, "pane_id"));
    const browsing = xpc_dictionary_get_bool(msg, "browsing");

    log.info("mode_changed pane={s} browsing={}", .{ pane_id, browsing });
}

/// Create a 200×200 blue checkerboard IOSurface for testing (Issue 603).
/// Returns an IOSurfaceRef (caller does NOT own — retained by module).
fn createTestIOSurface() ?*anyopaque {
    const size: i32 = 200;
    const bpe: i32 = 4;
    // 'BGRA' as a 32-bit integer (little-endian: 0x41524742).
    const pixel_format: i32 = 0x42475241;
    const cf_number_sint32: i32 = 3; // kCFNumberSInt32Type

    // Build properties dictionary.
    const dict = CFDictionaryCreateMutable(null, 4, &kCFTypeDictionaryKeyCallBacks, &kCFTypeDictionaryValueCallBacks) orelse return null;
    defer CFRelease(dict);

    const w_num = CFNumberCreate(null, cf_number_sint32, &size) orelse return null;
    defer CFRelease(w_num);
    const h_num = CFNumberCreate(null, cf_number_sint32, &size) orelse return null;
    defer CFRelease(h_num);
    const bpe_num = CFNumberCreate(null, cf_number_sint32, &bpe) orelse return null;
    defer CFRelease(bpe_num);
    const pf_num = CFNumberCreate(null, cf_number_sint32, &pixel_format) orelse return null;
    defer CFRelease(pf_num);

    CFDictionarySetValue(dict, kIOSurfaceWidth, w_num);
    CFDictionarySetValue(dict, kIOSurfaceHeight, h_num);
    CFDictionarySetValue(dict, kIOSurfaceBytesPerElement, bpe_num);
    CFDictionarySetValue(dict, kIOSurfacePixelFormat, pf_num);

    const surface = IOSurfaceCreate(dict) orelse return null;

    // Lock, fill with blue checkerboard, unlock.
    _ = IOSurfaceLock(surface, 0, null);
    const base = IOSurfaceGetBaseAddress(surface) orelse {
        _ = IOSurfaceUnlock(surface, 0, null);
        return surface;
    };
    const bpr = IOSurfaceGetBytesPerRow(surface);
    const sz: usize = @intCast(size);

    for (0..sz) |y| {
        const row = base + y * bpr;
        for (0..sz) |x| {
            const px = row + x * 4;
            const checker = ((x / 20) + (y / 20)) % 2 == 0;
            if (checker) {
                // Blue (BGRA: B=255, G=100, R=50, A=255)
                px[0] = 255;
                px[1] = 100;
                px[2] = 50;
                px[3] = 255;
            } else {
                // Dark blue (BGRA: B=180, G=40, R=20, A=255)
                px[0] = 180;
                px[1] = 40;
                px[2] = 20;
                px[3] = 255;
            }
        }
    }
    _ = IOSurfaceUnlock(surface, 0, null);

    log.info("created test IOSurface {d}x{d}", .{ sz, sz });
    return surface;
}

/// Convert a nullable C string to a Zig slice, defaulting to "(null)".
fn str(ptr: ?[*:0]const u8) []const u8 {
    return if (ptr) |p| std.mem.span(p) else "(null)";
}
