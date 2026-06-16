//! Build logic for TermSurf. A single "build.zig" file became far too complex
//! and spaghetti, so this package extracts the build logic into smaller,
//! more manageable pieces.

pub const gtk = @import("gtk.zig");
pub const Config = @import("Config.zig");
pub const GitVersion = @import("GitVersion.zig");

// Artifacts
pub const TermSurfBench = @import("TermSurfBench.zig");
pub const TermSurfDist = @import("TermSurfDist.zig");
pub const TermSurfDocs = @import("TermSurfDocs.zig");
pub const TermSurfExe = @import("TermSurfExe.zig");
pub const TermSurfFrameData = @import("TermSurfFrameData.zig");
pub const TermSurfLib = @import("TermSurfLib.zig");
pub const TermSurfLibVt = @import("TermSurfLibVt.zig");
pub const TermSurfResources = @import("TermSurfResources.zig");
pub const TermSurfI18n = @import("TermSurfI18n.zig");
pub const TermSurfXcodebuild = @import("TermSurfXcodebuild.zig");
pub const TermSurfXCFramework = @import("TermSurfXCFramework.zig");
pub const TermSurfWebdata = @import("TermSurfWebdata.zig");
pub const TermSurfZig = @import("TermSurfZig.zig");
pub const HelpStrings = @import("HelpStrings.zig");
pub const SharedDeps = @import("SharedDeps.zig");
pub const UnicodeTables = @import("UnicodeTables.zig");

// Steps
pub const LibtoolStep = @import("LibtoolStep.zig");
pub const LipoStep = @import("LipoStep.zig");
pub const MetallibStep = @import("MetallibStep.zig");
pub const XCFrameworkStep = @import("XCFrameworkStep.zig");

// Helpers
pub const requireZig = @import("zig.zig").requireZig;
