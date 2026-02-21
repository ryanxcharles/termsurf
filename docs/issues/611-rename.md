# Issue 611: Rename Ghostty to TermSurf Ghost

## Goal

Rename the app from "Ghostty" to "TermSurf Ghost". The bundle identifier,
display name, product name, CLI text, config paths, and About view all reflect
the new name. Internal identifiers (`GhosttyKit`, `Ghostty.*` Swift namespaces,
`ghostty_*` C API, `Ghostty.xcodeproj`) stay unchanged for upstream merge
safety.

## Background

Ghost is a Ghostty fork. It currently ships with all of Ghostty's branding —
same name, same bundle identifier (`com.mitchellh.ghostty`), same config paths.
This causes real problems:

1. **Icon collision (Issue 610).** macOS Launch Services caches app icons by
   bundle identifier. With `/Applications/Ghostty.app` installed, our app
   inherits Ghostty's icon regardless of what's in our app bundle.
2. **Config collision.** Both apps read from `~/.config/ghostty/`. Changes to
   one affect the other.
3. **Identity.** Users and developers need to distinguish Ghost from upstream
   Ghostty at a glance — in the dock, menu bar, Finder, and CLI.

### Prior art

This rename was done twice before:

- **ts1** created a parallel `termsurf-macos/` directory alongside the upstream
  `macos/`, keeping the originals untouched for clean merges. See
  `docs/issues/500-rename.md` for the full inventory.
- **ts5 (Issue 500)** modified `ts5/macos/` directly (no parallel directory).
  Renamed to "TermSurf" with bundle identifier `com.termsurf`. Internal
  identifiers stayed unchanged. The icon was left as Ghostty's — the same
  problem we're solving now in Issue 610.

Ghost follows ts5's approach: modify `ghost/macos/` directly. The name is
"TermSurf Ghost" (not just "TermSurf") to give this generation its own identity
within the TermSurf family.

### Naming convention

| Context                   | Value                                               |
| ------------------------- | --------------------------------------------------- |
| App name                  | TermSurf Ghost                                      |
| Bundle identifier         | com.termsurf.ghost                                  |
| Bundle identifier (debug) | com.termsurf.ghost.debug                            |
| Config directory          | `~/.config/termsurf/`                               |
| Config fallback (macOS)   | `~/Library/Application Support/com.termsurf.ghost/` |
| CLI binary name           | `ghostty` (unchanged)                               |
| CLI usage text            | `termsurf-ghost`                                    |
| CLI version output        | `TermSurf Ghost {version}`                          |
| Custom icon path          | `~/.config/termsurf/Ghost.icns`                     |

The CLI binary stays `ghostty` because renaming it requires changes to the Zig
build system, shell completions, and the `ghostty` symlink in the app bundle. A
future issue can tackle that.

## What to change

### 1. Xcode project configuration

In `ghost/macos/Ghostty.xcodeproj/project.pbxproj`:

| Setting                                     | Old                                      | New                        |
| ------------------------------------------- | ---------------------------------------- | -------------------------- |
| `PRODUCT_BUNDLE_IDENTIFIER`                 | `com.mitchellh.ghostty`                  | `com.termsurf.ghost`       |
| `PRODUCT_BUNDLE_IDENTIFIER` (debug)         | `com.mitchellh.ghostty.debug`            | `com.termsurf.ghost.debug` |
| `INFOPLIST_KEY_CFBundleDisplayName`         | `Ghostty`                                | `TermSurf Ghost`           |
| `INFOPLIST_KEY_CFBundleDisplayName` (debug) | `Ghostty[DEBUG]`                         | `TermSurf Ghost[DEBUG]`    |
| `PRODUCT_NAME`                              | `$(TARGET_NAME)` → resolves to `Ghostty` | `TermSurf Ghost`           |
| Permission dialog strings                   | `within Ghostty`                         | `within TermSurf Ghost`    |

Rename files in `ghost/macos/`:

- `Ghostty-Info.plist` → `Ghost-Info.plist`
- `Ghostty.entitlements` → `Ghost.entitlements`
- `GhosttyDebug.entitlements` → `GhosttyDebug.entitlements` (unchanged —
  internal)
- `GhosttyReleaseLocal.entitlements` → `GhosttyReleaseLocal.entitlements`
  (unchanged — internal)

Update file references in `project.pbxproj` for renamed files.

Do NOT rename:

- `Ghostty.xcodeproj/` — internal
- `GhosttyKit.xcframework` — internal
- `Ghostty.icon` — internal (references in `icon.json` stay as-is)
- Any `Ghostty.*` Swift namespaces
- Entitlements files that are only referenced by build settings (internal)

### 2. Info.plist

In `ghost/macos/Ghost-Info.plist` (after rename):

- Change UTType description: `"Ghostty Surface Identifier"` →
  `"TermSurf Ghost Surface Identifier"`
- Menu items already use `$(INFOPLIST_KEY_CFBundleDisplayName)` — they'll
  automatically read "New TermSurf Ghost Tab Here" etc.
- Keep `GHOSTTY_MAC_LAUNCH_SOURCE` and `com.mitchellh.ghosttySurfaceId` as-is
  (internal compatibility)

### 3. CLI text

In `ghost/src/`:

- `src/cli/help.zig` — `"ghostty"` → `"termsurf-ghost"`, `"Ghostty"` →
  `"TermSurf Ghost"`, example commands updated
- `src/cli/version.zig` — `"Ghostty {version}"` → `"TermSurf Ghost {version}"`
- `src/cli/list_themes.zig` — `"👻 Ghostty Theme Preview 👻"` →
  `"🏄 TermSurf Ghost Theme Preview 🏄"`

### 4. Config paths

Change the hardcoded directory names in Zig source code. No new code needed —
`ghostty_config_load_default_files` will automatically load from the correct
paths after these changes.

In `ghost/src/`:

- `build_config.zig:58` — `"com.mitchellh.ghostty"` → `"com.termsurf.ghost"`
  (controls macOS App Support and cache paths)
- `config/file_load.zig:14` — `"ghostty/config.ghostty"` →
  `"termsurf/config.ghostty"` (XDG config path)
- `config/file_load.zig:23` — `"ghostty/config"` → `"termsurf/config"` (legacy
  XDG config path)
- `config/theme.zig:30` — `"ghostty"` → `"termsurf"` (theme directory)

In `ghost/macos/Sources/`:

- `Ghostty/Ghostty.Config.swift:335` — Custom icon path →
  `~/.config/termsurf/Ghost.icns`
- `Features/Settings/SettingsView.swift:17` — Config path and app name in
  instructions: `$HOME/.config/termsurf/config.ghostty` and
  `restart TermSurf Ghost`

### 5. About view

In `ghost/macos/Sources/Features/About/AboutView.swift`:

- Title: `"Ghostty"` → `"TermSurf Ghost"`
- Subtitle: `"Terminal emulator with integrated browser,\nbuilt on Ghostty."`
- GitHub URL → `https://github.com/termsurf/termsurf`

### 6. Build system

In `ghost/src/build/GhosttyXcodebuild.zig`:

- App path: `Ghostty.app` → `TermSurf Ghost.app`

### 7. Icon (from Issue 610)

The `Ghostty.icon` modification from Issue 610 is already in place
(uncommitted). Once the bundle identifier changes to `com.termsurf.ghost`, macOS
Launch Services will treat this as a new app with no cached icon, and the
surfing ghost should display correctly.

No additional icon work needed beyond what Issue 610 already did.

## What NOT to change

These internal identifiers stay as-is to minimize upstream merge conflicts:

- `Ghostty.xcodeproj/` directory name
- `GhosttyKit.xcframework` framework name
- `Ghostty.*` Swift namespaces (`Ghostty.Config`, `Ghostty.App`, etc.)
- `ghostty_*` C API function names
- `GHOSTTY_MAC_LAUNCH_SOURCE` environment variable
- `com.mitchellh.ghostty.*` notification names in `Package.swift`
- `com.mitchellh.ghosttySurfaceId` UTType identifier
- Swift file names (`AppDelegate+Ghostty.swift`, `Ghostty.Config.swift`, etc.)
- `ghostty` CLI binary name
- `GhosttyDebug.entitlements`, `GhosttyReleaseLocal.entitlements` filenames

## Merge conflict expectations

All changes are in files that upstream Ghostty also modifies. Future
`git subtree pull` may produce conflicts. The conflicts will be small and
predictable — keep our version of the renamed strings, resolve Xcode project
changes manually if upstream restructures build settings.

## Experiments

### Experiment 1: Implement all name changes

#### Goal

`cd ghost && zig build` produces `TermSurf Ghost.app`. The menu bar reads
"TermSurf Ghost", the About view shows "TermSurf Ghost", the CLI prints
"TermSurf Ghost {version}", config loads from `~/.config/termsurf/`, and the
bundle identifier is `com.termsurf.ghost`. The surfing ghost icon (from Issue
610) displays correctly.

#### Approach

Change hardcoded values in the existing source code. No new functions, no new
files. `ghostty_config_load_default_files` continues to work — it just loads
from `~/.config/termsurf/` instead of `~/.config/ghostty/` after the underlying
constants change.

#### Steps

##### Step 1: Change config paths in Zig source

These four changes redirect all config, cache, and App Support paths:

**`ghost/src/build_config.zig:58`:**

```
"com.mitchellh.ghostty" → "com.termsurf.ghost"
```

This controls macOS App Support
(`~/Library/Application Support/com.termsurf.ghost/`) and cache paths.

**`ghost/src/config/file_load.zig:14`:**

```
"ghostty/config.ghostty" → "termsurf/config.ghostty"
```

XDG config path becomes `~/.config/termsurf/config.ghostty`.

**`ghost/src/config/file_load.zig:23`:**

```
"ghostty/config" → "termsurf/config"
```

Legacy config path becomes `~/.config/termsurf/config`.

**`ghost/src/config/theme.zig:30`:**

```
"ghostty" → "termsurf"
```

Theme directory becomes `~/.config/termsurf/themes/`.

##### Step 2: Rename plist and entitlements files

```bash
cd ghost/macos
git mv Ghostty-Info.plist Ghost-Info.plist
git mv Ghostty.entitlements Ghost.entitlements
```

`GhosttyDebug.entitlements` and `GhosttyReleaseLocal.entitlements` stay as-is
(internal).

##### Step 3: Update `project.pbxproj`

In `ghost/macos/Ghostty.xcodeproj/project.pbxproj`:

**File references** (update paths for renamed files):

- `Ghostty-Info.plist` → `Ghost-Info.plist`
- `Ghostty.entitlements` → `Ghost.entitlements`

**Bundle identifiers:**

- `com.mitchellh.ghostty` → `com.termsurf.ghost` (release, all 3 configs)
- `com.mitchellh.ghostty.debug` → `com.termsurf.ghost.debug`

**Display names:**

- `INFOPLIST_KEY_CFBundleDisplayName = Ghostty` → `"TermSurf Ghost"` (3 release
  configs)
- `INFOPLIST_KEY_CFBundleDisplayName = "Ghostty[DEBUG]"` →
  `"TermSurf Ghost[DEBUG]"`

**Product name:**

- `PRODUCT_NAME = "$(TARGET_NAME)"` → `PRODUCT_NAME = "TermSurf Ghost"` (all 3
  main app configs: Debug, Release, ReleaseLocal)

**Permission dialog strings** (all `within Ghostty` → `within TermSurf Ghost`):

There are ~14 permission strings per config (3 configs: Debug, Release,
ReleaseLocal), all following the pattern
`"A program running within Ghostty
would like to..."`. Replace `within Ghostty`
with `within TermSurf Ghost` in all of them.

**Do NOT change:**

- `PRODUCT_BUNDLE_IDENTIFIER` for test targets (`com.mitchellh.GhosttyTests`,
  `com.mitchellh.GhosttyUITests`) — internal
- `PRODUCT_NAME = "$(TARGET_NAME)"` for test targets — internal
- `-target "Ghostty"` in xcodebuild — that's the Xcode target name, internal
- iOS bundle identifiers (`com.mitchellh.ghostty-ios`) — not our platform

##### Step 4: Update `Ghost-Info.plist`

In `ghost/macos/Ghost-Info.plist` (after rename in step 2):

- `"Ghostty Surface Identifier"` → `"TermSurf Ghost Surface Identifier"`

Keep `GHOSTTY_MAC_LAUNCH_SOURCE` and `com.mitchellh.ghosttySurfaceId` as-is
(internal).

##### Step 5: Update CLI text

**`ghost/src/cli/help.zig`:**

- Line 37: `"ghostty"` → `"termsurf-ghost"` in usage line
- Line 39: `"Ghostty"` → `"TermSurf Ghost"` in description
- Line 41: `"Ghostty"` → `"TermSurf Ghost"`
- Line 53: `"ghostty"` → `"termsurf-ghost"` in example command
- Line 56: `"Ghostty.app"` → `"TermSurf Ghost.app"`
- Line 57: `"ghostty.app"` → `"termsurf ghost.app"` (lowercase for `open`)

**`ghost/src/cli/version.zig`:**

- Line 31: `"Ghostty {s}"` → `"TermSurf Ghost {s}"`

**`ghost/src/cli/list_themes.zig`:**

- Line 303: `"👻 Ghostty Theme Preview 👻"` →
  `"🏄 TermSurf Ghost Theme Preview 🏄"`

Leave doc comments and path references in `list_themes.zig` that refer to
`ghostty` config directories and resource paths — those are upstream paths that
still exist in the binary. Only change user-visible output strings.

##### Step 6: Update Swift strings

**`ghost/macos/Sources/Ghostty/Ghostty.Config.swift:335`:**

- `"~/.config/ghostty/Ghostty.icns"` → `"~/.config/termsurf/Ghost.icns"`

No change needed on line 70 — `ghostty_config_load_default_files(cfg)` already
loads from the correct paths after step 1.

**`ghost/macos/Sources/Features/Settings/SettingsView.swift:17`:**

- `"$HOME/.config/ghostty/config.ghostty and restart Ghostty"` →
  `"$HOME/.config/termsurf/config.ghostty and restart TermSurf Ghost"`

##### Step 7: Update About view

**`ghost/macos/Sources/Features/About/AboutView.swift`:**

- Line 6: GitHub URL → `"https://github.com/termsurf/termsurf"`
- Line 51: `"Ghostty"` → `"TermSurf Ghost"`

##### Step 8: Update build system

**`ghost/src/build/GhosttyXcodebuild.zig`:**

- Line 52: `"Ghostty.app"` → `"TermSurf Ghost.app"`

##### Step 9: Build and verify

```bash
rm -rf ~/Library/Developer/Xcode/DerivedData/Ghostty-*
rm -rf ghost/macos/build/
cd ghost && zig build
```

#### Verification

1. **App name:** `ls ghost/zig-out/` shows `TermSurf Ghost.app`
2. **Menu bar:** Launch the app — menu bar reads "TermSurf Ghost"
3. **About view:** Help → About shows "TermSurf Ghost" and links to the TermSurf
   GitHub repo
4. **Bundle identifier:**
   `defaults read ghost/zig-out/TermSurf\ Ghost.app/Contents/Info.plist CFBundleIdentifier`
   returns `com.termsurf.ghost`
5. **Icon:** The surfing ghost icon appears in the dock (no cached old icon,
   since the bundle identifier is new)
6. **CLI version:**
   `ghost/zig-out/TermSurf\ Ghost.app/Contents/MacOS/ghostty +version` prints
   `TermSurf Ghost {version}`

**Result:** (pending)
