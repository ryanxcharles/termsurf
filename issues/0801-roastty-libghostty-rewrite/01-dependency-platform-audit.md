# Experiment 1: Audit Dependencies and Platform Readiness

## Description

Before porting more of upstream Ghostty into Roastty, audit the macOS library,
app, toolchain, and third-party dependency surface so the rewrite does not begin
with hidden blockers.

This experiment is diagnostic only. It should not implement terminal behavior,
change build scripts, add dependencies, or modify `roastty/` source code. Its
job is to produce a concrete map of what `libroastty` will need, what can be
deferred, and what must be installed or configured before implementation work
can continue smoothly.

Roastty is macOS-only. The audit should explicitly ignore upstream Linux,
FreeBSD, Windows, GTK, Wayland, X11, OpenGL, and other non-macOS paths except
when recording that a path can be omitted from the Rust port.

## Questions

Answer these questions in the result:

1. What macOS frameworks does the remaining Ghostty library/app path depend on?
2. What third-party libraries or vendored packages does Roastty need to port,
   bind, replace with Rust crates, or defer?
3. What system tools must be present: Xcode, Command Line Tools, SDK version,
   clang, bindgen/libclang, codesign, Swift tooling, Cargo, Zig for upstream
   reference tests, or other build tools?
4. What features require user-visible macOS permissions, entitlements, prompts,
   code signing, or app bundle metadata?
5. Which dependencies are required for `libroastty` terminal parity, and which
   are app-only features that should not block the library rewrite?
6. Which upstream Ghostty tests, fixtures, or examples prove each
   dependency-heavy subsystem?
7. What runtime resources and generated artifacts must Roastty produce or
   bundle?
8. What library artifact, linkage, deployment-target, and app-integration shape
   does `libroastty` need?
9. Are there any blockers that must be solved before Experiment 2 starts?

## Changes

1. Inspect the vendored upstream Ghostty source only.
   - Use `vendor/ghostty/` as the source of truth.
   - Do not fetch upstream documentation or clone new repositories.
   - Do not modify `vendor/ghostty/`.

2. Audit macOS frameworks and system APIs.
   - Inspect at least:
     - `vendor/ghostty/build.zig`
     - `vendor/ghostty/src/`
     - `vendor/ghostty/pkg/macos/`
     - `vendor/ghostty/src/renderer/Metal.zig`
     - `vendor/ghostty/src/renderer/metal/`
     - `vendor/ghostty/src/font/`
     - `vendor/ghostty/src/termio/`
     - `vendor/ghostty/src/pty.zig`
     - `vendor/ghostty/macos/`
   - Record macOS frameworks such as AppKit, Foundation, CoreFoundation,
     CoreGraphics, CoreText, CoreVideo, CoreServices, Carbon, Metal, MetalKit,
     IOSurface, QuartzCore, UniformTypeIdentifiers, UserNotifications, and any
     others found.

3. Audit third-party and vendored packages.
   - Inspect `vendor/ghostty/pkg/`, `vendor/ghostty/vendor/`, and Ghostty build
     files.
   - Record each package's upstream role, Roastty phase, and Roastty strategy.
     Keep phase and strategy as separate table columns:
     - Phase examples: `library-now`, `app-later`, `test-only`,
       `optional/deferred`, `omit-non-macOS`.
     - Strategy examples: `Rust port`, `Rust crate`, `C binding`,
       `Swift/app integration`, `defer`, `omit`.
   - Include likely packages such as HarfBuzz, FreeType, libpng, zlib, Wuffs,
     simdutf, Oniguruma, Sentry/Breakpad, Sparkle, dcimgui, glslang,
     spirv-cross, and Nerd Fonts if present.

4. Audit toolchain and environment requirements.
   - Check the local toolchain state and record exact versions:

     ```bash
     rustc --version
     cargo --version
     xcodebuild -version
     xcrun --sdk macosx --show-sdk-path
     clang --version
     swift --version
     swiftc --version
     codesign --version
     zig version
     xcode-select -p
     xcrun --find metal
     xcrun --find swiftc
     ```

   - If a command is missing, record whether it blocks `libroastty` work now,
     blocks running upstream reference tests, blocks the Swift app later, or is
     optional.
   - Do not attempt to install or repair missing tools in this experiment.

5. Audit runtime resources and generated artifacts.
   - Inspect at least:
     - `vendor/ghostty/build.zig`
     - `vendor/ghostty/src/build/`
     - `vendor/ghostty/macos/Ghostty.xcodeproj/project.pbxproj`
     - `vendor/ghostty/zig-out` references in build or app files
     - `vendor/ghostty/terminfo/`
     - `vendor/ghostty/shell-integration/`
     - `vendor/ghostty/themes/`
     - `vendor/ghostty/dist/`
   - Record resources such as terminfo, shell integration scripts, themes,
     completions, man pages, locale data, appcast/update metadata, generated
     headers, generated Swift-accessible resources, and any other installed
     runtime assets.
   - Classify each as required for `libroastty`, required for the future macOS
     app, test-only, optional/deferred, or omit-non-macOS.

6. Audit build artifacts, linkage, and deployment targets.
   - Inspect upstream build outputs and app integration expectations:
     - static vs dynamic library outputs;
     - C headers;
     - xcframework generation;
     - target architectures;
     - minimum macOS deployment target;
     - rpaths/install names;
     - generated pkg-config or metadata files;
     - Swift/Xcode linkage expectations.
   - Record what `libroastty` should produce in the near term and what can wait
     until the Swift app integration issue.

7. Audit permissions, entitlements, and app metadata.
   - Inspect:
     - `vendor/ghostty/macos/Ghostty.entitlements`
     - `vendor/ghostty/macos/GhosttyDebug.entitlements`
     - `vendor/ghostty/macos/GhosttyReleaseLocal.entitlements`
     - `vendor/ghostty/macos/Ghostty-Info.plist`
     - `vendor/ghostty/macos/Ghostty.sdef`
     - Swift sources that mention AppleScript, notifications, pasteboard,
       accessibility, drag/drop, document types, services, updates, or crash
       reporting
   - Classify each item as:
     - required for `libroastty`;
     - required for the future Roastty macOS app;
     - optional/deferred;
     - not applicable after the macOS-only rewrite.

8. Audit upstream tests and examples.
   - Identify tests, fixtures, examples, and UI tests that cover
     dependency-heavy areas:
     - PTY/process handling;
     - terminal parser/screen behavior;
     - config parsing;
     - font discovery/shaping;
     - Metal/rendering;
     - clipboard/pasteboard;
     - AppleScript/app automation;
     - notifications;
     - crash reporting;
     - update integration.
   - Inspect concrete test and example roots:
     - Zig tests embedded under `vendor/ghostty/src/`
     - `vendor/ghostty/example/c-vt-*`
     - `vendor/ghostty/test/`
     - `vendor/ghostty/nix/test-src/`
     - `vendor/ghostty/.github/workflows/test.yml`
     - `vendor/ghostty/macos/Tests/`
     - `vendor/ghostty/macos/GhosttyUITests/`
     - `vendor/ghostty/macos/Ghostty.xctestplan`
   - Record whether each is portable into Rust unit tests, Rust integration
     tests, Swift app tests, or should be deferred.

9. Verify the diagnostic-only boundary.
   - Before recording the result, run:

     ```bash
     git status --short
     ```

   - Expected changed files are limited to Issue 801 documentation and
     gitignored review logs under `logs/`.
   - The experiment must not modify `roastty/`, `vendor/ghostty/`, `Cargo.toml`,
     `Cargo.lock`, scripts, build configuration, or source code.

10. Record the result inside this experiment file.
    - Append `## Result` and `## Conclusion` to this file.
    - Include these tables:
      - `Toolchain Readiness`
      - `macOS Frameworks and System APIs`
      - `Third-Party Dependencies`
      - `Runtime Resources and Generated Artifacts`
      - `Build Artifacts, Linkage, and Deployment Targets`
      - `Permissions, Entitlements, and App Metadata`
      - `Test Parity Sources`
      - `Non-macOS Paths to Omit`
      - `Blockers Before Experiment 2`
    - Update the Issue 801 README experiment index status from `Designed` to
      `Pass`, `Partial`, or `Fail` after the result is recorded.

## Verification

The experiment passes if:

- the result tables are filled in with concrete file references from
  `vendor/ghostty/`;
- every dependency has separate `Roastty phase` and `Strategy` classifications;
- all local toolchain commands were run and their versions or failures were
  recorded;
- runtime resources, generated artifacts, build outputs, linkage shape, and
  deployment-target requirements are classified;
- the result explicitly says whether any user/system action is required before
  implementation work can proceed;
- `git status --short` confirms the diagnostic-only boundary was preserved;
- the result identifies the most appropriate Experiment 2 scope.

The experiment is partial if:

- most dependencies are classified, but one or two major subsystems need deeper
  follow-up before Experiment 2 can be designed;
- a local tool is missing and prevents completing part of the audit, but the
  missing tool itself is documented clearly enough to fix.

The experiment fails if:

- it starts implementation work instead of auditing;
- it preserves non-macOS paths as live Roastty requirements without
  justification;
- it cannot identify whether implementation work is blocked.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or running the audit.

Codex reviewed the first draft and found real gaps around generated resources,
build/linkage outputs, Zig/reference-test readiness, and diagnostic-only
verification. Those gaps have been incorporated into this design.
