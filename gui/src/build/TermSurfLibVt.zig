const TermSurfLibVt = @This();

const std = @import("std");
const builtin = @import("builtin");
const assert = std.debug.assert;
const RunStep = std.Build.Step.Run;
const TermSurfZig = @import("TermSurfZig.zig");

/// The step that generates the file.
step: *std.Build.Step,

/// The artifact result
artifact: *std.Build.Step.InstallArtifact,

/// The final library file
output: std.Build.LazyPath,
dsym: ?std.Build.LazyPath,
pkg_config: ?std.Build.LazyPath,

pub fn initWasm(
    b: *std.Build,
    zig: *const TermSurfZig,
) !TermSurfLibVt {
    const target = zig.vt.resolved_target.?;
    assert(target.result.cpu.arch.isWasm());

    const exe = b.addExecutable(.{
        .name = "termsurf-vt",
        .root_module = zig.vt_c,
        .version = std.SemanticVersion{ .major = 0, .minor = 1, .patch = 0 },
    });

    // Allow exported symbols to actually be exported.
    exe.rdynamic = true;

    // There is no entrypoint for this wasm module.
    exe.entry = .disabled;

    return .{
        .step = &exe.step,
        .artifact = b.addInstallArtifact(exe, .{}),
        .output = exe.getEmittedBin(),
        .dsym = null,
        .pkg_config = null,
    };
}

pub fn initShared(
    b: *std.Build,
    zig: *const TermSurfZig,
) !TermSurfLibVt {
    const target = zig.vt.resolved_target.?;
    const lib = b.addLibrary(.{
        .name = "termsurf-vt",
        .linkage = .dynamic,
        .root_module = zig.vt_c,
        .version = std.SemanticVersion{ .major = 0, .minor = 1, .patch = 0 },
    });
    lib.installHeadersDirectory(
        b.path("include/termsurf"),
        "termsurf",
        .{ .include_extensions = &.{".h"} },
    );

    if (lib.rootModuleTarget().os.tag.isDarwin()) {
        // Self-hosted x86_64 doesn't work for darwin. It may not work
        // for other platforms too but definitely darwin.
        lib.use_llvm = true;

        // This is required for codesign and dynamic linking to work.
        lib.headerpad_max_install_names = true;

        // If we're not cross compiling then we try to find the Apple
        // SDK using standard Apple tooling.
        if (builtin.os.tag.isDarwin()) try @import("apple_sdk").addPaths(b, lib);
    }

    // Get our debug symbols
    const dsymutil: ?std.Build.LazyPath = dsymutil: {
        if (!target.result.os.tag.isDarwin()) {
            break :dsymutil null;
        }

        const dsymutil = RunStep.create(b, "dsymutil");
        dsymutil.addArgs(&.{"dsymutil"});
        dsymutil.addFileArg(lib.getEmittedBin());
        dsymutil.addArgs(&.{"-o"});
        const output = dsymutil.addOutputFileArg("libtermsurf-vt.dSYM");
        break :dsymutil output;
    };

    // pkg-config
    const pc: std.Build.LazyPath = pc: {
        const wf = b.addWriteFiles();
        break :pc wf.add("libtermsurf-vt.pc", b.fmt(
            \\prefix={s}
            \\includedir=${{prefix}}/include
            \\libdir=${{prefix}}/lib
            \\
            \\Name: libtermsurf-vt
            \\URL: https://github.com/ghostty-org/ghostty
            \\Description: TermSurf VT library
            \\Version: 0.1.0
            \\Cflags: -I${{includedir}}
            \\Libs: -L${{libdir}} -ltermsurf-vt
        , .{b.install_prefix}));
    };

    return .{
        .step = &lib.step,
        .artifact = b.addInstallArtifact(lib, .{}),
        .output = lib.getEmittedBin(),
        .dsym = dsymutil,
        .pkg_config = pc,
    };
}

pub fn install(
    self: *const TermSurfLibVt,
    step: *std.Build.Step,
) void {
    const b = step.owner;
    step.dependOn(&self.artifact.step);
    if (self.pkg_config) |pkg_config| {
        step.dependOn(&b.addInstallFileWithDir(
            pkg_config,
            .prefix,
            "share/pkgconfig/libtermsurf-vt.pc",
        ).step);
    }
}
