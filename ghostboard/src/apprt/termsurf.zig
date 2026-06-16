const std = @import("std");
const internal_os = @import("../os/main.zig");

const c = @cImport({
    @cInclude("termsurf.pb-c.h");
});

const log = std.log.scoped(.termsurf);

const env_key: [:0]const u8 = "TERMSURF_SOCKET";
const max_frame_size: usize = 1024 * 1024;
const max_clients: usize = 128;

const ClientSlot = struct {
    fd: std.posix.fd_t = -1,
    thread: ?std.Thread = null,
    done: bool = false,
};

var mutex: std.Thread.Mutex = .{};
var clients_mutex: std.Thread.Mutex = .{};
var listener_fd: std.posix.fd_t = -1;
var accept_thread: ?std.Thread = null;
var stopping = std.atomic.Value(bool).init(false);
var socket_path_buf: [std.fs.max_path_bytes]u8 = undefined;
var socket_path_len: usize = 0;
var clients: [max_clients]ClientSlot = [_]ClientSlot{.{}} ** max_clients;

pub fn start() !void {
    mutex.lock();
    defer mutex.unlock();

    if (listener_fd >= 0) return error.AlreadyStarted;

    const tmpdir = std.posix.getenv("TMPDIR") orelse "/tmp";
    const sep = if (std.mem.endsWith(u8, tmpdir, "/")) "" else "/";

    var dir_buf: [std.fs.max_path_bytes]u8 = undefined;
    const dir_z = std.fmt.bufPrintZ(
        &dir_buf,
        "{s}{s}termsurf",
        .{ tmpdir, sep },
    ) catch return error.SocketPathTooLong;
    try std.fs.cwd().makePath(dir_z);

    const path_z = try socketPath(tmpdir, sep);
    std.posix.unlink(path_z) catch {};
    errdefer {
        std.posix.unlink(path_z) catch {};
        socket_path_len = 0;
        _ = internal_os.unsetenv(env_key);
    }

    const fd = try std.posix.socket(std.posix.AF.UNIX, std.posix.SOCK.STREAM, 0);
    errdefer std.posix.close(fd);

    const addr = try std.net.Address.initUnix(path_z);
    try std.posix.bind(fd, &addr.any, addr.getOsSockLen());
    try std.posix.listen(fd, 8);

    if (internal_os.setenv(env_key, path_z) != 0) return error.SetEnvFailed;

    stopping.store(false, .release);
    const thread = try std.Thread.spawn(.{}, acceptLoop, .{fd});

    listener_fd = fd;
    accept_thread = thread;

    log.info("TermSurf socket listening on {s}", .{path_z});
}

pub fn stop() void {
    mutex.lock();
    const fd = listener_fd;
    const thread = accept_thread;
    listener_fd = -1;
    accept_thread = null;
    stopping.store(true, .release);
    mutex.unlock();

    wakeAccept();

    if (fd >= 0) {
        std.posix.close(fd);
    }
    if (thread) |t| {
        t.join();
    }

    const client_threads = stopClients();
    for (client_threads) |maybe_thread| {
        if (maybe_thread) |t| t.join();
    }

    if (socket_path_len > 0) {
        const path = socket_path_buf[0..socket_path_len];
        std.posix.unlink(path) catch {};
        socket_path_len = 0;
    }

    _ = internal_os.unsetenv(env_key);
}

fn acceptLoop(fd: std.posix.fd_t) void {
    while (!stopping.load(.acquire)) {
        const client_fd = std.posix.accept(fd, null, null, 0) catch |err| {
            if (!stopping.load(.acquire)) {
                log.warn("TermSurf socket accept failed err={}", .{err});
            }
            return;
        };
        if (stopping.load(.acquire)) {
            std.posix.close(client_fd);
            return;
        }
        log.info("TermSurf client connected fd={}", .{client_fd});

        joinClientThreads(reapDoneClients());
        const slot_index = reserveClient(client_fd) orelse {
            log.warn("TermSurf client limit reached fd={}", .{client_fd});
            std.posix.close(client_fd);
            continue;
        };

        const thread = std.Thread.spawn(.{}, handleClient, .{ client_fd, slot_index }) catch |err| {
            log.warn("TermSurf client thread failed fd={} err={}", .{ client_fd, err });
            clearClient(slot_index);
            std.posix.close(client_fd);
            continue;
        };

        if (activateClient(slot_index, thread)) |done_thread| {
            log.info("TermSurf client exited before thread registration fd={}", .{client_fd});
            done_thread.join();
        }
    }
}

fn handleClient(fd: std.posix.fd_t, slot_index: usize) void {
    defer {
        markClientDone(slot_index);
        std.posix.close(fd);
    }

    const allocator = std.heap.c_allocator;
    while (!stopping.load(.acquire)) {
        const frame = readFrame(fd, allocator) catch |err| {
            log.warn("TermSurf client read failed fd={} err={}", .{ fd, err });
            return;
        } orelse return;

        {
            defer allocator.free(frame);

            const msg = c.termsurf__term_surf_message__unpack(null, frame.len, frame.ptr) orelse {
                log.warn("TermSurf protobuf decode failed fd={}", .{fd});
                return;
            };
            defer c.termsurf__term_surf_message__free_unpacked(msg, null);

            log.info("TermSurf message decoded type={s}", .{msgTypeName(msg.*.msg_case)});
            switch (msg.*.msg_case) {
                c.TERMSURF__TERM_SURF_MESSAGE__MSG_HELLO_REQUEST => {
                    sendHelloReply(fd) catch |err| {
                        log.warn("TermSurf HelloReply failed fd={} err={}", .{ fd, err });
                        return;
                    };
                },
                c.TERMSURF__TERM_SURF_MESSAGE__MSG_QUERY_LAST_REQUEST => {
                    const req = msg.*.unnamed_0.query_last_request;
                    if (req) |query| {
                        log.info(
                            "TermSurf QueryLastRequest pane_id={s} profile={s}",
                            .{ query.*.pane_id, query.*.profile },
                        );
                    }
                    sendQueryLastReply(fd) catch |err| {
                        log.warn("TermSurf QueryLastReply failed fd={} err={}", .{ fd, err });
                        return;
                    };
                },
                c.TERMSURF__TERM_SURF_MESSAGE__MSG_QUERY_DEVTOOLS_REQUEST => {
                    const req = msg.*.unnamed_0.query_devtools_request;
                    if (req) |query| {
                        log.info(
                            "TermSurf QueryDevtoolsRequest pane_id={s} inspected_tab_id={} profile={s} browser={s}",
                            .{ query.*.pane_id, query.*.inspected_tab_id, query.*.profile, query.*.browser },
                        );
                        sendQueryDevtoolsReply(fd, query) catch |err| {
                            log.warn("TermSurf QueryDevtoolsReply failed fd={} err={}", .{ fd, err });
                            return;
                        };
                    } else {
                        sendQueryDevtoolsReply(fd, null) catch |err| {
                            log.warn("TermSurf QueryDevtoolsReply failed fd={} err={}", .{ fd, err });
                            return;
                        };
                    }
                },
                c.TERMSURF__TERM_SURF_MESSAGE__MSG_QUERY_TABS_REQUEST => {
                    const req = msg.*.unnamed_0.query_tabs_request;
                    if (req) |query| {
                        log.info(
                            "TermSurf QueryTabsRequest pane_id={s} profile={s}",
                            .{ query.*.pane_id, query.*.profile },
                        );
                    }
                    sendQueryTabsReply(fd) catch |err| {
                        log.warn("TermSurf QueryTabsReply failed fd={} err={}", .{ fd, err });
                        return;
                    };
                },
                else => {
                    log.info("TermSurf message ignored type={s}", .{msgTypeName(msg.*.msg_case)});
                },
            }
        }
    }
}

fn reserveClient(fd: std.posix.fd_t) ?usize {
    clients_mutex.lock();
    defer clients_mutex.unlock();

    for (&clients, 0..) |*slot, i| {
        if (slot.thread == null and slot.fd < 0) {
            slot.* = .{ .fd = fd, .thread = null, .done = false };
            return i;
        }
    }

    return null;
}

fn activateClient(index: usize, thread: std.Thread) ?std.Thread {
    clients_mutex.lock();
    defer clients_mutex.unlock();

    if (clients[index].done) {
        clients[index] = .{};
        return thread;
    }

    clients[index].thread = thread;
    return null;
}

fn clearClient(index: usize) void {
    clients_mutex.lock();
    defer clients_mutex.unlock();
    clients[index] = .{};
}

fn markClientDone(index: usize) void {
    clients_mutex.lock();
    defer clients_mutex.unlock();

    clients[index].fd = -1;
    clients[index].done = true;
}

fn reapDoneClients() [max_clients]?std.Thread {
    var threads: [max_clients]?std.Thread = [_]?std.Thread{null} ** max_clients;

    clients_mutex.lock();
    for (&clients, 0..) |*slot, i| {
        if (slot.done) {
            threads[i] = slot.thread;
            slot.* = .{};
        }
    }
    clients_mutex.unlock();

    return threads;
}

fn joinClientThreads(threads: [max_clients]?std.Thread) void {
    for (threads) |maybe_thread| {
        if (maybe_thread) |thread| {
            thread.join();
        }
    }
}

fn stopClients() [max_clients]?std.Thread {
    var threads: [max_clients]?std.Thread = [_]?std.Thread{null} ** max_clients;

    clients_mutex.lock();
    for (&clients, 0..) |*slot, i| {
        if (slot.fd >= 0) {
            std.posix.shutdown(slot.fd, .both) catch {};
            slot.fd = -1;
        }
        threads[i] = slot.thread;
        slot.* = .{};
    }
    clients_mutex.unlock();

    return threads;
}

fn readFrame(fd: std.posix.fd_t, allocator: std.mem.Allocator) !?[]u8 {
    var len_buf: [4]u8 = undefined;
    if (!try readExactOrEof(fd, &len_buf)) return null;

    const frame_len = std.mem.readInt(u32, &len_buf, .little);
    if (frame_len > max_frame_size) {
        log.warn("TermSurf frame rejected len={} max={}", .{ frame_len, max_frame_size });
        return error.FrameTooLarge;
    }

    const frame = try allocator.alloc(u8, frame_len);
    errdefer allocator.free(frame);
    if (!try readExactOrEof(fd, frame)) return error.UnexpectedEof;
    return frame;
}

fn readExactOrEof(fd: std.posix.fd_t, buf: []u8) !bool {
    var offset: usize = 0;
    while (offset < buf.len) {
        const n = try std.posix.read(fd, buf[offset..]);
        if (n == 0) {
            if (offset == 0) return false;
            return error.UnexpectedEof;
        }
        offset += n;
    }
    return true;
}

fn sendHelloReply(fd: std.posix.fd_t) !void {
    var reply: c.Termsurf__HelloReply = undefined;
    c.termsurf__hello_reply__init(&reply);

    var wrapper: c.Termsurf__TermSurfMessage = undefined;
    c.termsurf__term_surf_message__init(&wrapper);
    wrapper.msg_case = c.TERMSURF__TERM_SURF_MESSAGE__MSG_HELLO_REPLY;
    wrapper.unnamed_0.hello_reply = &reply;

    try sendProtobuf(fd, &wrapper);
    log.info("TermSurf HelloReply sent", .{});
}

fn sendQueryLastReply(fd: std.posix.fd_t) !void {
    var reply: c.Termsurf__QueryLastReply = undefined;
    c.termsurf__query_last_reply__init(&reply);
    reply.@"error" = @constCast("No browser pane yet");

    var wrapper: c.Termsurf__TermSurfMessage = undefined;
    c.termsurf__term_surf_message__init(&wrapper);
    wrapper.msg_case = c.TERMSURF__TERM_SURF_MESSAGE__MSG_QUERY_LAST_REPLY;
    wrapper.unnamed_0.query_last_reply = &reply;

    try sendProtobuf(fd, &wrapper);
    log.info("TermSurf QueryLastReply sent", .{});
}

fn sendQueryDevtoolsReply(fd: std.posix.fd_t, req: ?*c.Termsurf__QueryDevtoolsRequest) !void {
    var reply: c.Termsurf__QueryDevtoolsReply = undefined;
    c.termsurf__query_devtools_reply__init(&reply);

    const allocator = std.heap.c_allocator;
    var allocated_error: ?[]u8 = null;
    defer if (allocated_error) |err| allocator.free(err);

    const error_msg: [:0]const u8 = if (req) |query| blk: {
        if (std.mem.len(query.*.browser) == 0) {
            break :blk "DevTools target browser is required";
        }
        if (std.mem.len(query.*.profile) == 0) {
            break :blk "DevTools target profile is required";
        }
        if (query.*.inspected_tab_id == 0) {
            break :blk "DevTools target tab id is required";
        }
        const error_len = std.fmt.count(
            "Inspected tab {} not found in {s}/{s}",
            .{ query.*.inspected_tab_id, query.*.browser, query.*.profile },
        );
        allocated_error = try allocator.alloc(u8, error_len + 1);
        break :blk std.fmt.bufPrintZ(
            allocated_error.?,
            "Inspected tab {} not found in {s}/{s}",
            .{ query.*.inspected_tab_id, query.*.browser, query.*.profile },
        ) catch unreachable;
    } else "DevTools target browser is required";
    reply.@"error" = @constCast(error_msg.ptr);

    var wrapper: c.Termsurf__TermSurfMessage = undefined;
    c.termsurf__term_surf_message__init(&wrapper);
    wrapper.msg_case = c.TERMSURF__TERM_SURF_MESSAGE__MSG_QUERY_DEVTOOLS_REPLY;
    wrapper.unnamed_0.query_devtools_reply = &reply;

    try sendProtobuf(fd, &wrapper);
    log.info("TermSurf QueryDevtoolsReply sent", .{});
}

fn sendQueryTabsReply(fd: std.posix.fd_t) !void {
    var reply: c.Termsurf__QueryTabsReply = undefined;
    c.termsurf__query_tabs_reply__init(&reply);

    var wrapper: c.Termsurf__TermSurfMessage = undefined;
    c.termsurf__term_surf_message__init(&wrapper);
    wrapper.msg_case = c.TERMSURF__TERM_SURF_MESSAGE__MSG_QUERY_TABS_REPLY;
    wrapper.unnamed_0.query_tabs_reply = &reply;

    try sendProtobuf(fd, &wrapper);
    log.info("TermSurf QueryTabsReply sent", .{});
}

fn sendProtobuf(fd: std.posix.fd_t, wrapper: *c.Termsurf__TermSurfMessage) !void {
    const size = c.termsurf__term_surf_message__get_packed_size(wrapper);
    if (size > max_frame_size) return error.FrameTooLarge;

    const allocator = std.heap.c_allocator;
    const payload = try allocator.alloc(u8, size);
    defer allocator.free(payload);

    const packed_size = c.termsurf__term_surf_message__pack(wrapper, payload.ptr);
    if (packed_size != size) return error.ProtobufPackFailed;

    var len_buf: [4]u8 = undefined;
    std.mem.writeInt(u32, &len_buf, @intCast(size), .little);
    try writeAll(fd, &len_buf);
    try writeAll(fd, payload);
}

fn writeAll(fd: std.posix.fd_t, bytes: []const u8) !void {
    var offset: usize = 0;
    while (offset < bytes.len) {
        const n = try std.posix.write(fd, bytes[offset..]);
        if (n == 0) return error.WriteZero;
        offset += n;
    }
}

fn msgTypeName(msg_case: c.Termsurf__TermSurfMessage__MsgCase) []const u8 {
    return switch (msg_case) {
        c.TERMSURF__TERM_SURF_MESSAGE__MSG_HELLO_REQUEST => "HelloRequest",
        c.TERMSURF__TERM_SURF_MESSAGE__MSG_HELLO_REPLY => "HelloReply",
        c.TERMSURF__TERM_SURF_MESSAGE__MSG_QUERY_LAST_REQUEST => "QueryLastRequest",
        c.TERMSURF__TERM_SURF_MESSAGE__MSG_QUERY_LAST_REPLY => "QueryLastReply",
        c.TERMSURF__TERM_SURF_MESSAGE__MSG_QUERY_DEVTOOLS_REQUEST => "QueryDevtoolsRequest",
        c.TERMSURF__TERM_SURF_MESSAGE__MSG_QUERY_DEVTOOLS_REPLY => "QueryDevtoolsReply",
        c.TERMSURF__TERM_SURF_MESSAGE__MSG_QUERY_TABS_REQUEST => "QueryTabsRequest",
        c.TERMSURF__TERM_SURF_MESSAGE__MSG_QUERY_TABS_REPLY => "QueryTabsReply",
        c.TERMSURF__TERM_SURF_MESSAGE__MSG_SERVER_REGISTER => "ServerRegister",
        c.TERMSURF__TERM_SURF_MESSAGE__MSG_SET_OVERLAY => "SetOverlay",
        else => "Other",
    };
}

fn wakeAccept() void {
    if (socket_path_len == 0) return;

    const fd = std.posix.socket(std.posix.AF.UNIX, std.posix.SOCK.STREAM, 0) catch return;
    defer std.posix.close(fd);

    const path = socket_path_buf[0..socket_path_len];
    const addr = std.net.Address.initUnix(path) catch return;
    std.posix.connect(fd, &addr.any, addr.getOsSockLen()) catch {};
}

fn socketPath(tmpdir: []const u8, sep: []const u8) ![:0]u8 {
    const pid = std.c.getpid();
    const path_z = std.fmt.bufPrintZ(
        &socket_path_buf,
        "{s}{s}termsurf/termsurf-ghostboard-{d}.sock",
        .{ tmpdir, sep, pid },
    ) catch return error.SocketPathTooLong;
    socket_path_len = path_z.len;
    return path_z;
}
