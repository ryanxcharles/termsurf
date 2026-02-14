# Issue 500: Rename Ghostty to TermSurf in ts5

## Goal

Rename high-impact uses of "Ghostty" to "TermSurf" in ts5, and replace the
Ghostty logo with the TermSurf logo. The ts5 codebase was imported as unmodified
upstream Ghostty (Issue 418). This issue tracks making it ours.

## Approach

ts1 already solved this problem. We created a parallel `termsurf-macos/`
directory alongside the upstream `macos/`, with its own Xcode project, icons,
and Info.plist. We renamed user-facing strings in the shared Zig CLI code and
updated config paths. Internal identifiers (`com.mitchellh.ghostty.*`
notification names, `GhosttyKit` framework name, `Ghostty.*` Swift namespace, C
API function names) were left unchanged — they're internal plumbing and renaming
them creates unnecessary merge conflicts with upstream.

We replicate the same strategy in ts5.

## What ts1 Changed

### 1. Parallel macOS App Directory

Created `ts1/termsurf-macos/` alongside `ts1/macos/`:

```
ts1/termsurf-macos/
├── TermSurf.xcodeproj/       # New Xcode project
├── TermSurf-Info.plist        # Custom Info.plist
├── TermSurf.entitlements      # Entitlements (release)
├── TermSurfDebug.entitlements # Entitlements (debug)
├── TermSurfReleaseLocal.entitlements
├── Assets.xcassets/           # TermSurf icons and assets
├── icon-source/               # Source icon files
│   ├── termsurf-icon.png
│   └── termsurf-debug-icon.png
├── Sources/                   # Swift source (copied from macos/Sources)
└── Tests/
```

The key is that `macos/Sources/` was copied to `termsurf-macos/Sources/`, then
modified. The original `macos/` stays untouched for clean upstream merges.

### 2. Xcode Project Configuration

In `TermSurf.xcodeproj/project.pbxproj`:

| Setting                             | Ghostty value            | TermSurf value     |
| ----------------------------------- | ------------------------ | ------------------ |
| Product Name                        | `Ghostty`                | `TermSurf`         |
| `PRODUCT_BUNDLE_IDENTIFIER`         | `com.mitchellh.ghostty`  | `com.termsurf`     |
| `PRODUCT_BUNDLE_IDENTIFIER` (debug) | `com.mitchellh.ghostty.debug` | `com.termsurf.debug` |
| `INFOPLIST_KEY_CFBundleDisplayName` | `Ghostty`                | `TermSurf`         |
| `INFOPLIST_KEY_CFBundleDisplayName` (debug) | `Ghostty[DEBUG]` | `TermSurf[DEBUG]`  |
| Executable name                     | `ghostty`                | (unchanged)        |

### 3. Info.plist

`TermSurf-Info.plist` changes from `Ghostty-Info.plist`:

- Added `TermSurfBuild` and `TermSurfCommit` keys (parallel to Ghostty's)
- Changed UTType description: `"Ghostty Surface Identifier"` →
  `"TermSurf Surface Identifier"`
- Menu items use `$(INFOPLIST_KEY_CFBundleDisplayName)` so they automatically
  read "New TermSurf Tab Here" / "New TermSurf Window Here"
- Kept `GHOSTTY_MAC_LAUNCH_SOURCE` and `com.mitchellh.ghosttySurfaceId` as-is
  (internal compatibility)

### 4. CLI Text (Shared Zig Code)

These changes are in `ts5/src/` (shared between macOS and other platforms):

**`src/cli/help.zig`:**
```
"Usage: ghostty"     → "Usage: termsurf"
"Run the Ghostty"    → "Run the TermSurf"
"ghostty -e top"     → "termsurf -e top"
"open -na Ghostty.app" → "open -na TermSurf.app"
```

**`src/cli/version.zig`:**
```
"Ghostty {version}"  → "TermSurf {version}"
```

**`src/cli/list_themes.zig`:**
```
"👻 Ghostty Theme Preview 👻" → "🏄 TermSurf Theme Preview 🏄"
```

### 5. Config Paths

In `termsurf-macos/Sources/Ghostty/Ghostty.Config.swift`:
```swift
// Was: ghostty_config_load_default_files(cfg)
ghostty_config_load_files(cfg, "termsurf", "com.termsurf")
```

This makes the app read config from `~/.config/termsurf/` with fallback to
`~/Library/Application Support/com.termsurf/`, instead of Ghostty's default
`~/.config/ghostty/`.

In `termsurf-macos/Sources/Ghostty/Ghostty.Config.swift` (icon path):
```swift
// Was: "~/.config/ghostty/Ghostty.icns"
"~/.config/termsurf/TermSurf.icns"
```

In `termsurf-macos/Sources/Features/Settings/SettingsView.swift`:
```swift
// Was: "$HOME/.config/ghostty/config and restart Ghostty"
"$HOME/.config/termsurf/config and restart TermSurf"
```

### 6. About View

In `termsurf-macos/Sources/Features/About/AboutView.swift`:
```swift
Text("TermSurf")
  .bold()
  .font(.title)
Text("Terminal emulator with integrated browser,\nbuilt on Ghostty.")
```

GitHub URL changed to `https://github.com/termsurf/termsurf`.

### 7. Icons

Custom TermSurf icon set in `termsurf-macos/Assets.xcassets/`:
- `AppIcon.appiconset/` — Multiple sizes (16–1024px)
- `TermSurfDebugIcon.imageset/` — Debug build icon
- `AppIconImage.imageset/` — Icon variants
- Source files in `termsurf-macos/icon-source/`

### 8. Build System

In `build.zig`, added a second xcframework target:
```zig
// TermSurf xcframework (for termsurf-macos/)
const xcframework_termsurf = try buildpkg.GhosttyXCFramework.initWithPath(
    b, &deps, config.xcframework_target,
    "termsurf-macos/GhosttyKit.xcframework",
);
```

Both `xcframework.install()` and `xcframework_termsurf.install()` are called
when building.

### 9. TermSurf-Specific Swift Code

New files added for browser integration (not rename-related, but present in
`termsurf-macos/` and not in `macos/`):
- `Sources/Features/Socket/TermsurfEnvironment.swift` — Injects `TERMSURF_SOCKET` and `TERMSURF_PANE_ID` env vars
- `Sources/Features/Socket/TermsurfProtocol.swift` — Socket protocol
- `Sources/Features/WebView/` — WebView integration files

### 10. What Was NOT Renamed

These internal identifiers stayed as-is in ts1 to minimize merge conflicts:

- `com.mitchellh.ghostty.*` notification names (dozens in `Package.swift`)
- `com.mitchellh.ghosttySurfaceId` UTType identifier
- `GhosttyKit.xcframework` framework name
- `Ghostty.*` Swift namespace (`Ghostty.Config`, `Ghostty.App`, etc.)
- `ghostty_config_*` C API function names
- `GHOSTTY_MAC_LAUNCH_SOURCE` environment variable
- Swift file names like `AppDelegate+Ghostty.swift`, `Ghostty.Config.swift`

## Changes for ts5

### Change 1: Create `ts5/termsurf-macos/`

Copy `ts5/macos/` to `ts5/termsurf-macos/`. Then modify:

1. Create `TermSurf.xcodeproj` with updated bundle identifier (`com.termsurf`),
   display name (`TermSurf`), and build settings
2. Create `TermSurf-Info.plist` with `TermSurfBuild`/`TermSurfCommit` keys
3. Create `TermSurf.entitlements`, `TermSurfDebug.entitlements`,
   `TermSurfReleaseLocal.entitlements`
4. Replace icons in `Assets.xcassets/` with TermSurf icons (copy from
   `ts1/termsurf-macos/Assets.xcassets/`)

### Change 2: Update CLI Text

In `ts5/src/`:

1. `src/cli/help.zig` — Replace "ghostty"/"Ghostty" with "termsurf"/"TermSurf"
2. `src/cli/version.zig` — `"Ghostty {version}"` → `"TermSurf {version}"`
3. `src/cli/list_themes.zig` — Theme preview title

### Change 3: Update Config Paths

In `ts5/termsurf-macos/Sources/`:

1. `Ghostty/Ghostty.Config.swift` — Use `ghostty_config_load_files(cfg,
   "termsurf", "com.termsurf")` instead of `ghostty_config_load_default_files`
2. `Ghostty/Ghostty.Config.swift` — Icon path →
   `~/.config/termsurf/TermSurf.icns`
3. `Features/Settings/SettingsView.swift` — Config path and app name in
   instructions
4. `Features/About/AboutView.swift` — Display name, description, GitHub URL

### Change 4: Update Build System

In `ts5/build.zig`:

1. Add second xcframework target pointing to
   `termsurf-macos/GhosttyKit.xcframework`
2. Install both xcframeworks

### Change 5: Replace Icons

Copy TermSurf icon assets from ts1:
- `ts1/termsurf-macos/Assets.xcassets/AppIcon.appiconset/` →
  `ts5/termsurf-macos/Assets.xcassets/AppIcon.appiconset/`
- `ts1/termsurf-macos/Assets.xcassets/TermSurfDebugIcon.imageset/` →
  `ts5/termsurf-macos/Assets.xcassets/TermSurfDebugIcon.imageset/`
- `ts1/termsurf-macos/Assets.xcassets/AppIconImage.imageset/` →
  `ts5/termsurf-macos/Assets.xcassets/AppIconImage.imageset/`
- `ts1/termsurf-macos/icon-source/` →
  `ts5/termsurf-macos/icon-source/`

## Scope

This issue covers renaming and icon replacement only. Browser integration (the
`web` command, socket server, WebView panes) is separate future work that builds
on top of the renamed app.

## Merge Conflict Expectations

Changes to `ts5/src/cli/help.zig`, `version.zig`, and `list_themes.zig` will
create merge conflicts on future `git subtree pull` from upstream Ghostty. These
are small, predictable conflicts — easy to resolve by keeping our version.

The `ts5/termsurf-macos/` directory is entirely new and will never conflict with
upstream (upstream only has `macos/`). The `ts5/build.zig` change (adding the
second xcframework target) may conflict if upstream modifies that section.
