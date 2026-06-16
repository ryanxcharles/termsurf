const std = @import("std");
const internal_os = @import("../os/main.zig");

const log = std.log.scoped(.termsurf);

const env_key: [:0]const u8 = "TERMSURF_SOCKET";

var mutex: std.Thread.Mutex = .{};
var listener_fd: std.posix.fd_t = -1;
var accept_thread: ?std.Thread = null;
var stopping = std.atomic.Value(bool).init(false);
var socket_path_buf: [std.fs.max_path_bytes]u8 = undefined;
var socket_path_len: usize = 0;

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
        std.posix.close(client_fd);
    }
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
