# Issue 652: Rename CLI Binary to termsurf

## Goal

The CLI binary inside the app bundle is still named `ghostty`. It should be
`termsurf`. When you look inside `TermSurf.app/Contents/MacOS/`, you should see
`termsurf`, not `ghostty`.

## Current state

The Zig build system already names the binary `termsurf` (`GhosttyExe.zig:15`,
changed in Issue 650). However, the **Xcode build** — which produces the actual
`.app` bundle — still uses `EXECUTABLE_NAME = ghostty` in the project file. The
Xcode-built binary wins because `xcodebuild` is what creates the final app
bundle.

Current binary names:

- `gui/macos/build/Debug/TermSurf Debug.app/Contents/MacOS/ghostty` — wrong
- `gui/macos/build/ReleaseLocal/TermSurf.app/Contents/MacOS/ghostty` — wrong
- `/Applications/TermSurf.app/Contents/MacOS/ghostty` — wrong

## What needs to change

### 1. Xcode project (`gui/macos/Ghostty.xcodeproj/project.pbxproj`)

Three `EXECUTABLE_NAME` settings control the binary name inside the app bundle:

- Line 603: `EXECUTABLE_NAME = ghostty;` (Release config)
- Line 913: `EXECUTABLE_NAME = ghostty;` (Debug config)
- Line 967: `EXECUTABLE_NAME = ghostty;` (iOS config)

All three need to change to `EXECUTABLE_NAME = termsurf;`.

### 2. Bridging header references

The Xcode project references a bridging header file by its current name:

- Line 637:
  `SWIFT_OBJC_BRIDGING_HEADER = "Sources/App/macOS/ghostty-bridging-header.h";`
- Line 945:
  `SWIFT_OBJC_BRIDGING_HEADER = "Sources/App/macOS/ghostty-bridging-header.h";`
- Line 1000:
  `SWIFT_OBJC_BRIDGING_HEADER = "Sources/App/macOS/ghostty-bridging-header.h";`

The file itself at `gui/macos/Sources/App/macOS/ghostty-bridging-header.h` needs
renaming to `termsurf-bridging-header.h`, and all three references updated.

### 3. Test host paths

Test targets reference the binary by name in `TEST_HOST`:

- Line 729:
  `TEST_HOST = "$(BUILT_PRODUCTS_DIR)/Ghostty.app/$(BUNDLE_EXECUTABLE_FOLDER_PATH)/ghostty";`
- Line 752: same
- Line 775: same

These need `ghostty` changed to `termsurf`. (The `Ghostty.app` part is the Xcode
target name which is a separate, larger rename — not in scope here.)

### 4. Resource folder reference

The Zig build produces resources at `zig-out/share/ghostty/`. The Xcode project
references this folder:

- Line 48: `path = "../zig-out/share/ghostty";`

This is the Ghostty resource directory (terminfo, themes, etc.) managed by
upstream Ghostty's build system. Renaming it would require changes throughout
the Zig build. This is a cosmetic reference — the resources inside are correct
regardless of the folder name. Not in scope for this issue.

### What NOT to change

These use "ghostty" as an infrastructure/branding identifier, not as the binary
name. They are inherited from upstream Ghostty and changing them would break
functionality or create unnecessary divergence:

- Environment variables: `GHOSTTY_LOG`, `GHOSTTY_MAC_LAUNCH_SOURCE`,
  `GHOSTTY_RESOURCES_DIR`, `GHOSTTY_CONFIG_PATH`
- Info.plist keys: `GhosttyBuild`, `GhosttyCommit`
- UTType identifier: `com.mitchellh.ghosttySurfaceId`
- Library name: `libghostty` (the C library linked by Swift)
- Resource folder: `zig-out/share/ghostty/`
