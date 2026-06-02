+++
[implementer]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

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

## Result

**Result:** Pass

The audit found no upfront macOS permission or third-party integration blocker
that prevents implementation from continuing. The required local tools are
present. The risky work is not permission setup; it is choosing dependency
strategy slice-by-slice so Roastty does not accidentally inherit upstream
Ghostty's non-macOS build matrix or app-only package surface.

### Toolchain Readiness

| Tool                         | Observed result                                                           | Phase                        | Status                          |
| ---------------------------- | ------------------------------------------------------------------------- | ---------------------------- | ------------------------------- |
| Rust compiler                | `rustc 1.96.0 (ac68faa20 2026-05-25)`                                     | library-now                  | Ready                           |
| Cargo                        | `cargo 1.96.0 (30a34c682 2026-05-25)`                                     | library-now                  | Ready                           |
| Xcode                        | `Xcode 26.4`, build `17E192`                                              | app-later, Metal build-later | Ready                           |
| macOS SDK                    | `/Applications/Xcode.app/.../MacOSX26.4.sdk`                              | app-later, Metal build-later | Ready                           |
| Apple clang                  | `Apple clang version 21.0.0` targeting `arm64-apple-darwin25.5.0`         | C bindings/build-later       | Ready                           |
| Swift                        | `Apple Swift version 6.3`                                                 | app-later                    | Ready                           |
| Swift compiler path          | `/Applications/Xcode.app/.../usr/bin/swiftc`                              | app-later                    | Ready                           |
| Code signing tool            | `/usr/bin/codesign`; `codesign --version` is unsupported and prints usage | app-later                    | Tool present; no version output |
| Zig                          | `0.16.0`; upstream minimum is `0.15.2` in `vendor/ghostty/build.zig.zon`  | upstream-reference-tests     | Ready                           |
| Xcode selected developer dir | `/Applications/Xcode.app/Contents/Developer`                              | app-later                    | Ready                           |
| Metal compiler               | `xcrun --find metal` resolves to the installed Metal toolchain            | renderer-later               | Ready                           |
| bindgen CLI                  | `which bindgen` and `bindgen --version` fail: `bindgen not found`         | optional/C-binding-later     | Not needed before Experiment 2  |
| libclang                     | `/Applications/Xcode.app/.../usr/lib/libclang.dylib` exists               | optional/C-binding-later     | Ready for future bindgen work   |
| clang++ and libtool          | `xcrun --find clang++` and `xcrun --find libtool` resolve inside Xcode    | C++/archive-later            | Ready                           |

No tool needs user action before Experiment 2. `bindgen` is not installed, but
Experiment 2 should not need generated C bindings; if a later experiment chooses
generated bindings over handwritten shims, that experiment should install or add
a Rust bindgen dependency deliberately. `codesign --version` is not a valid
command on this system, but `codesign` itself exists and can sign/verify; that
is enough for this audit.

### macOS Frameworks and System APIs

| Framework/API             | Upstream evidence                                                                                                          | Roastty phase                          | Strategy                                                                 |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- | ------------------------------------------------------------------------ |
| CoreFoundation            | `vendor/ghostty/pkg/macos/build.zig` links it; `vendor/ghostty/pkg/macos/main.zig` imports it                              | library-now once macOS wrappers begin  | Use Rust Objective-C/CoreFoundation bindings or small C/ObjC shims       |
| CoreGraphics              | `pkg/macos/build.zig`, `pkg/macos/main.zig`, and global keybind Swift code use it                                          | app-later, renderer-later              | Bind only macOS paths needed by surfaces/rendering                       |
| CoreText                  | `pkg/macos/build.zig`, `pkg/macos/text/ext.c`, `src/font/backend.zig`, and `src/font/discovery.zig`                        | library-now for font discovery/metrics | Use macOS CoreText bindings; do not port fontconfig path                 |
| CoreVideo                 | `pkg/macos/build.zig` and `pkg/macos/main.zig` import it                                                                   | renderer-later                         | Bind with Metal/IOSurface renderer slice                                 |
| QuartzCore                | `pkg/macos/build.zig`, `pkg/macos/main.zig`, and Metal layer code                                                          | renderer-later                         | Bind for layer/IOSurface presentation                                    |
| IOSurface                 | `pkg/macos/build.zig`, `pkg/macos/iosurface.zig`, `src/renderer/metal/Target.zig`, `src/renderer/metal/IOSurfaceLayer.zig` | renderer-later                         | Bind directly; macOS-only                                                |
| Metal                     | `src/renderer/Metal.zig`, `src/renderer/metal/`, `macos/Sources/Helpers/MetalView.swift`                                   | renderer-later                         | Use Rust Metal bindings or ObjC shims; omit OpenGL fallback              |
| AppKit                    | Swift frontend imports throughout `vendor/ghostty/macos/Sources/`                                                          | app-later                              | Keep in renamed Swift app, not `libroastty` core                         |
| Foundation                | Swift app and macOS helper package use it                                                                                  | app-later/library-boundary             | Swift app owns most of this; Rust uses FFI-safe ABI                      |
| Carbon                    | `pkg/macos/build.zig` links Carbon for macOS, Xcode project links `Carbon.framework`, global keybind code imports Carbon   | app-later                              | Required for keyboard/global keybind integration, not early library core |
| UniformTypeIdentifiers    | Info.plist and pasteboard/transferable helpers use UT types                                                                | app-later                              | Swift app integration                                                    |
| UserNotifications         | `SurfaceView.swift`, `SurfaceView_AppKit.swift`, and `AppDelegate.swift` import/use it                                     | app-later                              | Defer; prompts at runtime                                                |
| POSIX PTY/termios/openpty | `src/pty.zig`, `src/pty.c`, `src/termio/Exec.zig`                                                                          | library-now when PTY begins            | Port macOS POSIX path only                                               |

### Third-Party Dependencies

| Dependency                                 | Upstream evidence                                                                                             | Roastty phase                          | Strategy                                                                                             |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------- | -------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| CoreText-backed font stack                 | `src/font/backend.zig` defaults Darwin to `.coretext`; `src/font/discovery.zig` implements CoreText discovery | library-now for fonts                  | Prefer native macOS bindings; omit fontconfig default path                                           |
| HarfBuzz                                   | `pkg/harfbuzz/build.zig`; optional for `coretext_harfbuzz` in `src/font/backend.zig`                          | optional/deferred                      | Defer unless CoreText shaping parity proves insufficient                                             |
| FreeType                                   | `pkg/freetype/build.zig`; `SharedDeps.zig` includes it for Dear ImGui and non-default font backends           | optional/deferred                      | Defer for terminal parity; revisit with inspector or alternate font backend                          |
| fontconfig                                 | `pkg/fontconfig/`; `src/font/backend.zig` uses it only for non-Darwin default                                 | omit-non-macOS                         | Omit                                                                                                 |
| libpng                                     | `pkg/libpng/build.zig`; used through image dependencies                                                       | library-later for Kitty graphics       | Prefer Rust crate or C binding when Kitty graphics slice starts                                      |
| zlib                                       | `pkg/zlib/build.zig`; used by libpng/frame data/build helpers                                                 | library-later                          | Prefer Rust crate or system zlib only when needed                                                    |
| Wuffs                                      | `pkg/wuffs/`; PNG decode tests in `pkg/wuffs/src/png.zig`                                                     | library-later for image decode         | Prefer Rust crate or port only required image path                                                   |
| simdutf                                    | `pkg/simdutf/build.zig`, `SharedDeps.zig` SIMD dependency hooks                                               | optional/perf-later                    | Defer; correctness first, optimize later                                                             |
| highway                                    | `pkg/highway/build.zig`, SIMD hooks in `SharedDeps.zig`                                                       | optional/perf-later                    | Defer                                                                                                |
| Oniguruma                                  | `pkg/oniguruma/build.zig`; linked in `SharedDeps.zig`                                                         | library-later if regex/search needs it | Prefer Rust regex where behavior permits; audit before binding                                       |
| libxev                                     | `build.zig.zon`; termio async IO uses xev concepts                                                            | library-now when IO loop begins        | Replace with Rust async/thread/channel design or Rust crate; do not port Zig event loop mechanically |
| vaxis                                      | `build.zig.zon`; CLI utilities such as `src/cli/list_themes.zig`, `list_keybinds.zig`, and `list_colors.zig`  | app-later/CLI-later                    | Defer; use Rust TUI tooling only if Roastty keeps equivalent CLI previews                            |
| uucode                                     | `build.zig.zon`; `SharedDeps.zig` generates Unicode tables                                                    | library-later                          | Prefer generated Rust tables or a Rust Unicode crate                                                 |
| z2d                                        | `build.zig.zon`                                                                                               | optional/deferred                      | Audit when renderer/text geometry requires it                                                        |
| zig_js                                     | `build.zig.zon`; `SharedDeps.zig` wires it for JS/web data paths                                              | app/docs-later                         | Defer; not required for `libroastty` terminal core                                                   |
| zf                                         | `build.zig.zon`; `src/cli/list_themes.zig` imports it for fuzzy ranking                                       | app-later/CLI-later                    | Defer or replace with Rust fuzzy matcher if equivalent CLI is kept                                   |
| gobject                                    | `build.zig.zon`; GTK runtime/type branches use it throughout `src/apprt/gtk/` and boxed-type helpers          | omit-non-macOS                         | Omit                                                                                                 |
| zig-objc / pkg/macos                       | `build.zig.zon`, `pkg/macos/`                                                                                 | library/app-later                      | Replace with Rust ObjC/CoreFoundation bindings or small ObjC shims                                   |
| libintl                                    | `build.zig.zon`; `SharedDeps.zig` bundles it on Apple platforms; `src/os/i18n.zig` uses libintl helpers       | app-later/i18n-later                   | Defer; decide with localization/resource work                                                        |
| apple_sdk                                  | `build.zig.zon`; `SharedDeps.zig` and `GhosttyLibVt.zig` add Apple SDK paths for Darwin builds                | build-later                            | Use Xcode SDK paths directly or a Rust build-script equivalent when C/Metal/macOS bindings need it   |
| dcimgui                                    | `pkg/dcimgui/build.zig`, inspector Metal hooks in `src/apprt/embedded.zig`                                    | optional/deferred                      | Defer inspector UI                                                                                   |
| glslang / spirv-cross                      | `pkg/glslang/`, `pkg/spirv-cross/`, shader build hooks                                                        | renderer-later                         | Prefer prebuilt Metal shaders or Rust-side shader build; defer until renderer                        |
| Sentry / Breakpad                          | `pkg/sentry/`, `pkg/breakpad/`, `src/crash/`                                                                  | optional/deferred                      | Defer crash reporting; not required for terminal correctness                                         |
| Sparkle                                    | Xcode Swift package in `macos/Ghostty.xcodeproj/.../Package.resolved` and update sources                      | app-later                              | Defer auto-update                                                                                    |
| Nerd Fonts Symbols Only                    | `build.zig.zon` font dependency                                                                               | library-later/font assets              | Decide when font fallback/glyph atlas work starts                                                    |
| JetBrains Mono                             | `build.zig.zon` font dependency                                                                               | app-later/assets                       | Defer; app packaging/default font decision                                                           |
| iterm2 themes                              | `build.zig.zon`; `GhosttyResources.zig` installs themes                                                       | app-later/resources                    | Defer until config/theme resources                                                                   |
| wayland / GTK / X11 / android-ndk / opengl | `build.zig.zon`, `SharedDeps.zig` non-macOS branches                                                          | omit-non-macOS                         | Omit                                                                                                 |

### Runtime Resources and Generated Artifacts

| Resource/artifact           | Upstream evidence                                                                                                              | Roastty phase                    | Strategy                                                                     |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | -------------------------------- | ---------------------------------------------------------------------------- |
| Terminfo source/database    | `src/build/GhosttyResources.zig` runs `+terminfo`, `infotocap`, and `tic`; Xcode references `zig-out/share/terminfo`           | app-later, terminal distribution | Generate renamed Roastty terminfo later; not required for next library slice |
| Shell integration scripts   | `src/build/GhosttyResources.zig` installs `src/shell-integration`                                                              | app-later                        | Rename/adapt later                                                           |
| Themes                      | `GhosttyResources.zig` installs `iterm2_themes` into `share/ghostty/themes`                                                    | app-later                        | Defer                                                                        |
| Fish/zsh/bash completions   | `GhosttyResources.zig` generates `+fish`, `+zsh`, `+bash`; Xcode references `zig-out/share/fish`, `zsh`, and `bash-completion` | app-later                        | Defer and rename to Roastty                                                  |
| Vim/Neovim/bat syntax files | `GhosttyResources.zig` generates/install syntax files; Xcode references `vim`, `nvim`, and `bat` folders                       | app-later                        | Defer                                                                        |
| Man pages/docs              | `src/build/GhosttyDocs.zig`; Xcode expects `zig-out/share/man`                                                                 | app-later                        | Defer; issue docs should not block library work                              |
| Locale files                | `src/build/GhosttyI18n.zig`; Xcode references `zig-out/share/locale`                                                           | app-later                        | Defer unless Swift app requires localized strings early                      |
| Webdata/help data           | `src/build/GhosttyWebdata.zig`                                                                                                 | app/docs-later                   | Defer                                                                        |
| C header                    | `src/build/GhosttyLib.zig` installs `include/ghostty.h`; Issue 800 already created `roastty/include/roastty.h`                 | library-now                      | Continue expanding `roastty.h` as APIs are implemented                       |
| Metal shader metallib       | `SharedDeps.zig` creates `MetallibStep` from `src/renderer/shaders/shaders.metal`                                              | renderer-later                   | Defer until Metal renderer experiment                                        |

### Build Artifacts, Linkage, and Deployment Targets

| Artifact/constraint     | Upstream evidence                                                                                                                                     | Roastty phase     | Strategy                                                                                                    |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------- | ----------------------------------------------------------------------------------------------------------- |
| Static library          | `src/build/GhosttyLib.zig` builds a static archive and combines dependencies into one archive                                                         | library-now/later | Continue Rust `cdylib`/`staticlib` decision incrementally; do not clone Zig archive-combining unless needed |
| Dynamic library         | `GhosttyLib.zig` also builds `.dynamic` and pkg-config metadata                                                                                       | library-later     | Keep current Rust ABI build simple until Swift app requires a specific linkage                              |
| C header                | `GhosttyLib.zig` installs `ghostty.h`; `GhosttyXCFramework.zig` copies header/modulemap                                                               | library-now       | Expand `roastty.h`; add modulemap only when Swift app links                                                 |
| XCFramework             | `GhosttyXCFramework.zig` creates `GhosttyKit.xcframework` with macOS/iOS slices; `XCFrameworkStep.zig` shells out to `xcodebuild -create-xcframework` | app-later         | macOS-only Roastty should produce a native macOS framework/library first; omit iOS slices                   |
| Universal macOS archive | `GhosttyLib.initMacOSUniversal` builds `aarch64` and `x86_64` and lipo-combines                                                                       | release-later     | Defer; current development is arm64 local                                                                   |
| dSYM                    | `GhosttyLib.initShared` runs `dsymutil` for Darwin shared libs                                                                                        | release-later     | Defer until release packaging                                                                               |
| pkg-config              | `GhosttyLib.zig` writes `ghostty-internal.pc` and static variant                                                                                      | optional/deferred | Defer unless external consumers need it                                                                     |
| Minimum macOS version   | `Config.zig` computes `osVersionMin(.macos)` and generic macOS targets                                                                                | app/release-later | Record during app integration; not a blocker for library audit                                              |
| rpath/install names     | `GhosttyLibVt.zig` sets `headerpad_max_install_names` for shared libs                                                                                 | release-later     | Defer until dynamic packaging                                                                               |

### Permissions, Entitlements, and App Metadata

| Item                                                                 | Upstream evidence                                                                                                                                                                                     | Roastty phase                | Status                                                                          |
| -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ------------------------------------------------------------------------------- |
| Apple Events / AppleScript                                           | Entitlements include `com.apple.security.automation.apple-events`; Info.plist enables AppleScript and points at `Ghostty.sdef`; Swift AppleScript sources under `macos/Sources/Features/AppleScript/` | app-later                    | Requires renamed app metadata and may prompt; not a library blocker             |
| Notifications                                                        | `AppDelegate.swift` and `Ghostty.App.swift` use `UNUserNotificationCenter.requestAuthorization`                                                                                                       | app-later                    | Runtime prompt; defer                                                           |
| Global keybind event tap                                             | `GlobalEventTap.swift` imports CoreGraphics/Carbon; AppDelegate enables it from config                                                                                                                | app-later                    | May need Accessibility/Input Monitoring behavior testing; not a library blocker |
| Secure input                                                         | `SecureInput.swift`, config comments in `src/config/Config.zig`, and AppDelegate secure-input menu hooks                                                                                              | app-later/library-boundary   | App behavior; no upfront permission action                                      |
| Pasteboard/clipboard                                                 | `NSPasteboard+Extension.swift`, `SurfaceView_AppKit.swift`, `NSPasteboardTests.swift`                                                                                                                 | app-later with ABI callbacks | No upfront permission; implement when clipboard ABI is ported                   |
| Document types/services                                              | `Ghostty-Info.plist` declares shell scripts, folders, Unix executables, services, UTI                                                                                                                 | app-later                    | Rename/adapt in Swift app stage                                                 |
| Sparkle update metadata                                              | Info.plist has Sparkle keys; Xcode project depends on Sparkle                                                                                                                                         | app-later/optional           | Defer                                                                           |
| Camera/microphone/photos/location/address book/calendar entitlements | Entitlement files contain broad passthrough permissions                                                                                                                                               | app-later/security-audit     | Not required for `libroastty`; should be challenged before Roastty app shipping |
| Crash reporting                                                      | `src/crash/`, `pkg/sentry/`, `pkg/breakpad/`                                                                                                                                                          | optional/deferred            | Defer                                                                           |

No user needs to pre-grant permissions before implementation work continues.
Permissions become relevant only when the renamed macOS app exposes AppleScript,
notifications, global shortcuts, or app services.

### Test Parity Sources

| Area                                  | Upstream evidence                                                                                                           | Roastty test plan                                                               |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| PTY/process                           | `src/pty.zig`, `src/termio/Exec.zig` tests such as `execCommand darwin: shell command`, `src/Command.zig` process/env tests | Port as Rust unit/integration tests around macOS PTY and command spawning       |
| Shell integration                     | `src/termio/shell_integration.zig` tests for bash, zsh, nushell, missing resources                                          | Defer until resource generation and shell integration files are ported          |
| Terminal screen/grid/selection/reflow | Extensive `src/terminal/Screen.zig` tests                                                                                   | High-priority Rust unit-test port once terminal core begins                     |
| Tabstops and terminal data structures | `src/terminal/Tabstops.zig`, `src/datastruct/split_tree.zig`, `src/datastruct/*` tests                                      | Good early Rust unit-test candidates                                            |
| Config parsing/defaults               | `src/config/` embedded Zig tests and `macos/Tests/Ghostty/ConfigTests.swift`                                                | Continue from Issue 800 config defaults into parser tests                       |
| Font discovery/shaping                | `src/font/`, `src/font/discovery.zig`, `src/font/shaper/` tests                                                             | Port when CoreText font subsystem begins; may need macOS-only integration tests |
| Image/Kitty graphics                  | `pkg/wuffs/src/png.zig` tests and `example/c-vt-kitty-graphics`                                                             | Defer until Kitty graphics slice                                                |
| C VT ABI examples                     | `example/c-vt-*`, especially encode-key, encode-mouse, render, stream, paste, selection, SGR, modes                         | Use as black-box ABI examples after Roastty implements those APIs               |
| Swift app behavior                    | `macos/Tests/`, `macos/GhosttyUITests/`, `macos/Ghostty.xctestplan`                                                         | Defer until renamed Swift app integration                                       |
| Fuzzing                               | `test/fuzz-libghostty/`                                                                                                     | Defer until parser/VT core exists                                               |
| Nix/lib C smoke                       | `nix/test-src/test_libghostty_vt.c`                                                                                         | Later C ABI smoke test, renamed to Roastty                                      |

### Non-macOS Paths to Omit

| Path/dependency                  | Upstream evidence                                                                              | Decision                      |
| -------------------------------- | ---------------------------------------------------------------------------------------------- | ----------------------------- |
| GTK/X11/Wayland                  | `build.zig.zon` dependencies and `SharedDeps.zig` GTK/X11/Wayland linking branches             | Omit                          |
| OpenGL renderer                  | `pkg/opengl/`, `src/renderer/OpenGL.zig`, `SharedDeps.zig` OpenGL link branch                  | Omit; Roastty uses Metal only |
| fontconfig default path          | `src/font/backend.zig` uses fontconfig for non-Darwin default                                  | Omit                          |
| Windows PTY and command paths    | `src/pty.zig`, `src/termio/Exec.zig`, `src/build/GhosttyLib.zig` Windows branches              | Omit                          |
| Linux Flatpak/Snap/app resources | `Config.zig`, `GhosttyResources.zig`, `termio/Exec.zig` Flatpak handling                       | Omit                          |
| iOS slices                       | `GhosttyXCFramework.zig` builds iOS/iOS simulator slices; Metal renderer contains iOS branches | Omit for Issue 801            |
| Android NDK                      | `build.zig.zon` dependency                                                                     | Omit                          |
| Wasm examples/runtime            | `build.zig.zon`, `example/wasm-*`, Wasm target config                                          | Omit                          |

### Blockers Before Experiment 2

No blocker requires user action before Experiment 2.

The audit recommends that Experiment 2 should not begin with Metal, PTY process
spawning, Swift app integration, or font shaping. Those slices involve external
frameworks, app lifecycle, or larger behavior surfaces. The best next slice is a
self-contained library subsystem with heavy upstream tests and no special
permissions: terminal data structures and screen/grid behavior, starting with a
small Rust port of a focused upstream test group such as `Tabstops` or a narrow
`Screen` subset.

That choice exercises the real terminal core, honors the test-parity rule, and
avoids spending the next experiment on packaging, entitlements, or app-only
machinery.

Diagnostic-only boundary check:

```text
$ git status --short
 M issues/0801-roastty-libghostty-rewrite/README.md
?? issues/0801-roastty-libghostty-rewrite/01-dependency-platform-audit.md
```

The only non-gitignored files changed during the audit are Issue 801
documentation files. No `roastty/`, `vendor/ghostty/`, Cargo files, scripts,
build configuration, or source code were modified.

## Conclusion

Experiment 1 passes. Roastty can proceed without installing new tools or asking
the user for macOS permissions. The dependency strategy is clear enough for the
next experiment: start with a macOS-neutral terminal-core library slice and
ported upstream tests, while deferring app-only integrations, generated runtime
resources, Metal rendering, crash reporting, updates, AppleScript, and
permissions until the corresponding subsystem requires them.

Codex reviewed the completed result, found two audit-completeness gaps, and
those gaps were fixed. A follow-up Codex review found no remaining blocking
result issues and approved the result for commit.
