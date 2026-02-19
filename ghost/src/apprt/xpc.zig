// XPC communication for TermSurf Ghost (Issue 601, Experiment 1).
//
// Connects to the xpc-gateway, creates an anonymous listener, registers its
// endpoint, and handles connections from `web` processes.
//
// Manual extern declarations instead of @cImport("xpc/xpc.h") because the XPC
// header uses C block types in function signatures, which Zig's translate-c may
// not handle. All XPC types are opaque pointers represented as ?*anyopaque.

const std = @import("std");
const objc = @import("objc");

const log = std.log.scoped(.xpc);

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
extern "c" fn xpc_get_type(object: xpc_object_t) xpc_object_t;

// XPC type/error constants (compared by address identity).
extern const _xpc_type_connection: anyopaque;
extern const _xpc_type_error: anyopaque;
extern const _xpc_error_connection_invalid: anyopaque;

/// Cast a const extern symbol address to xpc_object_t for identity comparison.
inline fn xpcPtr(ptr: *const anyopaque) xpc_object_t {
    return @constCast(ptr);
}

// -- Module state --

var gateway: xpc_object_t = null;
var listener: xpc_object_t = null;

// -- Block type --
//
// XPC event handler: fn(xpc_object_t) void.
// No captures — module-level state is used instead.
const EventBlock = objc.Block(struct {}, .{xpc_object_t}, void);

// -- Public API --

pub fn init() void {
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

        var peer_block = EventBlock.init(.{}, &peerHandler);
        xpc_connection_set_event_handler(event, @ptrCast(&peer_block));
        xpc_connection_resume(event);
    }
}

fn peerHandler(_: *const EventBlock.Context, event: xpc_object_t) callconv(.c) void {
    if (xpc_get_type(event) == xpcPtr(&_xpc_type_error)) {
        if (event == xpcPtr(&_xpc_error_connection_invalid)) {
            log.info("peer disconnected", .{});
        }
    }
}
