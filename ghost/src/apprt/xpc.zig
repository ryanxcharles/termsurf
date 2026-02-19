// XPC communication for TermSurf Ghost (Issues 601–603).
//
// Connects to the xpc-gateway, creates an anonymous listener, registers its
// endpoint, and handles connections from `web` processes and Chromium servers.
//
// Manual extern declarations instead of @cImport("xpc/xpc.h") because the XPC
// header uses C block types in function signatures, which Zig's translate-c may
// not handle. All XPC types are opaque pointers represented as ?*anyopaque.

const std = @import("std");
const objc = @import("objc");
const CoreApp = @import("../App.zig");
const CoreSurface = @import("../Surface.zig");

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
extern "c" fn xpc_dictionary_get_string(xdict: xpc_object_t, key: [*:0]const u8) ?[*:0]const u8;
extern "c" fn xpc_dictionary_get_uint64(xdict: xpc_object_t, key: [*:0]const u8) u64;
extern "c" fn xpc_dictionary_get_bool(xdict: xpc_object_t, key: [*:0]const u8) bool;
extern "c" fn xpc_dictionary_set_uint64(xdict: xpc_object_t, key: [*:0]const u8, value: u64) void;
extern "c" fn xpc_dictionary_get_remote_connection(msg: xpc_object_t) xpc_object_t;
extern "c" fn xpc_get_type(object: xpc_object_t) xpc_object_t;
extern "c" fn xpc_retain(object: xpc_object_t) xpc_object_t;
extern "c" fn xpc_release(object: xpc_object_t) void;

// XPC type/error constants (compared by address identity).
extern const _xpc_type_connection: anyopaque;
extern const _xpc_type_error: anyopaque;
extern const _xpc_type_dictionary: anyopaque;
extern const _xpc_error_connection_invalid: anyopaque;

// -- Mach port / IOSurface C API (Issue 603) --

extern "c" fn xpc_dictionary_copy_mach_send(xdict: xpc_object_t, key: [*:0]const u8) u32;
extern "c" fn IOSurfaceLookupFromMachPort(port: u32) ?*anyopaque;
extern "c" fn mach_port_deallocate(task: u32, name: u32) i32;
extern const mach_task_self_: u32;
extern "c" fn CFRelease(cf: *anyopaque) void;

/// Cast a const extern symbol address to xpc_object_t for identity comparison.
inline fn xpcPtr(ptr: *const anyopaque) xpc_object_t {
    return @constCast(ptr);
}

// -- Pane state (one mutex per pane) --

/// Per-pane state for a single webview. All fields are protected by `mutex`.
/// Handlers must lock `mutex` before reading or writing any field.
const Pane = struct {
    mutex: std.Thread.Mutex = .{},
    web_peer: xpc_object_t = null,
    server_peer: xpc_object_t = null,
    overlay_surface: ?*CoreSurface = null,
    server_process: ?std.process.Child = null,
    pending_url_buf: [2048]u8 = undefined,
    pending_url_len: usize = 0,
    pending_pane_id: [36]u8 = undefined,
    pending_pixel_w: u64 = 0,
    pending_pixel_h: u64 = 0,
};

// -- Module state --

var app: *CoreApp = undefined;
var gateway: xpc_object_t = null;
var listener: xpc_object_t = null;

/// Single active pane. Multi-pane will replace this with a HashMap.
var pane: Pane = .{};

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
    pane.mutex.lock();
    cleanupPaneLocked(&pane);
    pane.mutex.unlock();

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

        // Don't assign to web_peer or server_peer yet —
        // we identify the peer type by its first message.
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
            handleDisconnect();
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
    } else if (std.mem.eql(u8, action_str, "server_register")) {
        handleServerRegister(msg);
    } else if (std.mem.eql(u8, action_str, "display_surface")) {
        handleDisplaySurface(msg);
    } else if (std.mem.eql(u8, action_str, "tab_ready")) {
        handleTabReady(msg);
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

    log.info("set_overlay pane={s} col={} row={} w={} h={} url={s} profile={s} browsing={}", .{
        pane_id, col, row, width, height, url, profile, browsing,
    });

    // Look up the surface by pane ID and set the overlay.
    if (app.findSurfaceByPaneId(pane_id)) |surface| {
        pane.mutex.lock();
        defer pane.mutex.unlock();

        // Retain web peer on first message.
        if (pane.web_peer == null) {
            const conn = xpc_dictionary_get_remote_connection(msg);
            if (conn != null) pane.web_peer = xpc_retain(conn);
        }

        pane.overlay_surface = surface.core();
        surface.core().setOverlay(
            @intCast(col),
            @intCast(row),
            @intCast(width),
            @intCast(height),
        );

        // Store pending state for create_tab (sent after server_register).
        if (pane_id.len <= 36) {
            @memcpy(pane.pending_pane_id[0..pane_id.len], pane_id);
        }
        if (url.len <= pane.pending_url_buf.len) {
            @memcpy(pane.pending_url_buf[0..url.len], url);
            pane.pending_url_len = url.len;
        }

        // Compute pixel dimensions from grid cells × cell size.
        const cell = surface.core().getCellSize();
        const new_pixel_w = width * @as(u64, cell.width);
        const new_pixel_h = height * @as(u64, cell.height);
        const resized = pane.server_peer != null and
            (new_pixel_w != pane.pending_pixel_w or new_pixel_h != pane.pending_pixel_h);
        pane.pending_pixel_w = new_pixel_w;
        pane.pending_pixel_h = new_pixel_h;

        log.info("overlay set pane={s} pixel={d}x{d}", .{
            pane_id, pane.pending_pixel_w, pane.pending_pixel_h,
        });

        // Spawn the Chromium Profile Server (if not already running).
        if (pane.server_process == null) {
            spawnServer(&pane, profile);
        } else if (resized) {
            sendResize(&pane);
        }
    } else {
        log.warn("no surface found for pane={s}", .{pane_id});
    }
}

fn handleServerRegister(msg: xpc_object_t) void {
    const profile = str(xpc_dictionary_get_string(msg, "profile"));
    log.info("server_register profile={s}", .{profile});

    pane.mutex.lock();
    defer pane.mutex.unlock();

    // Retain server peer on first message.
    if (pane.server_peer == null) {
        const conn = xpc_dictionary_get_remote_connection(msg);
        if (conn != null) pane.server_peer = xpc_retain(conn);
    }

    if (pane.server_peer == null) {
        log.warn("server_register but no server_peer", .{});
        return;
    }

    // Send create_tab with the pending URL and pixel dimensions.
    const reply = xpc_dictionary_create(null, null, 0);
    xpc_dictionary_set_string(reply, "action", "create_tab");

    // URL must be null-terminated for xpc_dictionary_set_string.
    var url_z: [2049]u8 = undefined;
    if (pane.pending_url_len > 0 and pane.pending_url_len < url_z.len) {
        @memcpy(url_z[0..pane.pending_url_len], pane.pending_url_buf[0..pane.pending_url_len]);
        url_z[pane.pending_url_len] = 0;
        xpc_dictionary_set_string(reply, "url", @ptrCast(&url_z));
    }

    // Pane ID is already a [36]u8 — add null terminator.
    var pane_z: [37]u8 = undefined;
    @memcpy(pane_z[0..36], &pane.pending_pane_id);
    pane_z[36] = 0;
    xpc_dictionary_set_string(reply, "pane_id", @ptrCast(&pane_z));

    xpc_dictionary_set_uint64(reply, "pixel_width", pane.pending_pixel_w);
    xpc_dictionary_set_uint64(reply, "pixel_height", pane.pending_pixel_h);

    xpc_connection_send_message(pane.server_peer, reply);
    log.info("sent create_tab url_len={d} pixel={d}x{d}", .{
        pane.pending_url_len, pane.pending_pixel_w, pane.pending_pixel_h,
    });
}

fn handleDisplaySurface(msg: xpc_object_t) void {
    const port = xpc_dictionary_copy_mach_send(msg, "iosurface_port");
    if (port == 0) return;

    const iosurface = IOSurfaceLookupFromMachPort(port) orelse {
        _ = mach_port_deallocate(mach_task_self_, port);
        return;
    };
    _ = mach_port_deallocate(mach_task_self_, port);

    pane.mutex.lock();
    const surface = pane.overlay_surface;
    pane.mutex.unlock();

    if (surface) |s| {
        s.setOverlayIOSurface(iosurface);
    }

    // IOSurfaceLookupFromMachPort returns +1 ref; setOverlayIOSurface
    // CFRetains, so we CFRelease our lookup reference.
    CFRelease(iosurface);
}

fn handleTabReady(msg: xpc_object_t) void {
    const tab_id = str(xpc_dictionary_get_string(msg, "tab_id"));
    log.info("tab_ready tab_id={s}", .{tab_id});
}

fn handleModeChanged(msg: xpc_object_t) void {
    const pane_id = str(xpc_dictionary_get_string(msg, "pane_id"));
    const browsing = xpc_dictionary_get_bool(msg, "browsing");

    log.info("mode_changed pane={s} browsing={}", .{ pane_id, browsing });
}

/// Send a resize message to the Chromium server. Caller must hold `p.mutex`.
fn sendResize(p: *Pane) void {
    const msg = xpc_dictionary_create(null, null, 0);
    xpc_dictionary_set_string(msg, "action", "resize");

    var pane_z: [37]u8 = undefined;
    @memcpy(pane_z[0..36], &p.pending_pane_id);
    pane_z[36] = 0;
    xpc_dictionary_set_string(msg, "pane_id", @ptrCast(&pane_z));

    xpc_dictionary_set_uint64(msg, "pixel_width", p.pending_pixel_w);
    xpc_dictionary_set_uint64(msg, "pixel_height", p.pending_pixel_h);

    xpc_connection_send_message(p.server_peer, msg);
    log.info("sent resize pixel={d}x{d}", .{ p.pending_pixel_w, p.pending_pixel_h });
}

// -- Server lifecycle --

/// Spawn the Chromium Profile Server. Caller must hold `p.mutex`.
fn spawnServer(p: *Pane, profile: []const u8) void {
    const home = std.posix.getenv("HOME") orelse {
        log.err("HOME not set, cannot spawn server", .{});
        return;
    };

    // Build null-terminated path strings.
    var path_buf: [512]u8 = undefined;
    const server_path = std.fmt.bufPrintZ(
        &path_buf,
        "{s}/dev/termsurf/chromium/src/out/Default/Chromium Profile Server.app/Contents/MacOS/Chromium Profile Server",
        .{home},
    ) catch {
        log.err("server path too long", .{});
        return;
    };

    var xpc_arg_buf: [128]u8 = undefined;
    const xpc_arg = std.fmt.bufPrintZ(
        &xpc_arg_buf,
        "--xpc-service=com.termsurf.xpc-gateway",
        .{},
    ) catch return;

    var data_arg_buf: [512]u8 = undefined;
    const data_arg = std.fmt.bufPrintZ(
        &data_arg_buf,
        "--user-data-dir={s}/.config/termsurf/chromium-profiles/{s}",
        .{ home, profile },
    ) catch {
        log.err("data dir path too long", .{});
        return;
    };

    var hidden_buf: [16]u8 = undefined;
    const hidden_arg = std.fmt.bufPrintZ(&hidden_buf, "--hidden", .{}) catch return;

    log.info("spawning server: {s}", .{server_path});

    var child = std.process.Child.init(
        &.{ server_path, xpc_arg, data_arg, hidden_arg },
        std.heap.page_allocator,
    );
    child.spawn() catch |err| {
        log.err("failed to spawn server: {}", .{err});
        return;
    };

    p.server_process = child;
    log.info("server spawned pid={d}", .{child.id});
}

/// Kill the server process and wait for it to exit. Caller must hold `p.mutex`.
fn killServer(p: *Pane) void {
    if (p.server_process) |*proc| {
        _ = proc.kill() catch {};
        _ = proc.wait() catch {};
        log.info("server killed", .{});
    }
    p.server_process = null;
}

/// Full cleanup of a pane's state. Caller must hold `p.mutex`.
fn cleanupPaneLocked(p: *Pane) void {
    if (p.overlay_surface) |surface| {
        surface.clearOverlay();
        p.overlay_surface = null;
    }

    killServer(p);

    if (p.web_peer) |peer| {
        xpc_release(peer);
        p.web_peer = null;
    }
    if (p.server_peer) |peer| {
        xpc_release(peer);
        p.server_peer = null;
    }

    p.pending_url_len = 0;
    p.pending_pixel_w = 0;
    p.pending_pixel_h = 0;
}

fn handleDisconnect() void {
    pane.mutex.lock();
    defer pane.mutex.unlock();

    // Idempotent: if already cleaned up, the second disconnect is a no-op.
    if (pane.web_peer == null and pane.server_peer == null) {
        log.info("peer disconnected (already cleaned up)", .{});
        return;
    }

    cleanupPaneLocked(&pane);
    log.info("peer disconnected, cleaned up", .{});
}

/// Convert a nullable C string to a Zig slice, defaulting to "(null)".
fn str(ptr: ?[*:0]const u8) []const u8 {
    return if (ptr) |p| std.mem.span(p) else "(null)";
}
