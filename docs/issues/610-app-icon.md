# Issue 610: Replace Ghostty icon with TermSurf Ghost icon

## Goal

The app icon in the dock, Finder, and app switcher shows the TermSurf Ghost icon
instead of the Ghostty icon, for both release and debug builds.

## Background

Ghost is a Ghostty fork. It currently ships with Ghostty's original icon — a
blue rounded-square with a ghost silhouette and `>_` prompt. Two new TermSurf
Ghost icons have been created in `assets/`:

- **`termsurf-ghost-black.png`** (1024x1024) — Release icon. A CRT monitor with
  a ghost surfing a cyan wave of binary code.
- **`termsurf-ghost-alt-black.png`** (1024x1024) — Debug icon. Same concept but
  with a green wave, visually distinguishing debug builds at a glance.

### How Ghostty handles icons

Ghostty's icon system has two layers:

**Build time (asset catalog):** The Xcode project compiles
`Assets.xcassets/AppIconImage.imageset/` into the app bundle's `Ghostty.icns`.
The imageset contains three sizes: 1024px (3x), 512px (2x), 256px (1x). The
Xcode build setting `ASSETCATALOG_COMPILER_APPICON_NAME = Ghostty` references
this asset. This is the icon Finder and Launchpad display.

**Runtime (debug override):** In `AppDelegate.swift`, `updateAppIcon(from:)`
handles icon switching. In `#if DEBUG` builds, when no custom icon is
configured, it sets `NSApplication.shared.applicationIconImage` to
`BlueprintImage` — the blueprint-style alternate icon. This changes the dock
icon without modifying the app bundle (which would corrupt code signing). The
blueprint icon lives in
`Assets.xcassets/Alternate Icons/BlueprintImage.imageset/`.

Ghostty also supports user-configurable icons via `macos-icon` config
(`Package.swift` line 338), with presets like blueprint, chalkboard, glass, etc.
These are all in the `Alternate Icons/` folder.

### What needs to change

1. **Release icon:** Replace the three PNGs in `AppIconImage.imageset/` with
   resized versions of `termsurf-ghost-black.png` (1024px, 512px, 256px).

2. **Debug icon:** Replace `BlueprintImage.imageset/macOS-AppIcon-1024px.png`
   with `termsurf-ghost-alt-black.png`. This is the only size needed — the debug
   override uses `NSImage(named:)` which handles scaling.

3. **Alternate icons:** The existing Ghostty alternate icons (chalkboard, glass,
   holographic, etc.) depict the Ghostty ghost. They could be left as-is for now
   (they're only used when explicitly configured by the user) or removed to
   avoid shipping Ghostty branding. Not critical for this issue.

### Sizing

The asset catalog expects three sizes for the release icon:

| Scale | Filename                           | Pixels    |
| ----- | ---------------------------------- | --------- |
| 1x    | `macOS-AppIcon-256px-128pt@2x.png` | 256x256   |
| 2x    | `macOS-AppIcon-512px.png`          | 512x512   |
| 3x    | `macOS-AppIcon-1024px.png`         | 1024x1024 |

The source image is 1024x1024, so 512px and 256px versions need to be generated
by downscaling. macOS `sips` can do this:

```bash
sips -z 512 512 input.png --out output-512.png
sips -z 256 256 input.png --out output-256.png
```

### Key files

- `assets/termsurf-ghost-black.png` — New release icon (1024x1024)
- `assets/termsurf-ghost-alt-black.png` — New debug icon (1024x1024)
- `ghost/macos/Assets.xcassets/AppIconImage.imageset/` — Release icon imageset
  (3 PNGs + Contents.json)
- `ghost/macos/Assets.xcassets/Alternate Icons/BlueprintImage.imageset/` — Debug
  icon imageset (1 PNG + Contents.json)
- `ghost/macos/Sources/App/macOS/AppDelegate.swift` — Debug icon override (line
  1003: `NSImage(named: "BlueprintImage")`)

## Experiments

### Experiment 1: Replace both icons

#### Goal

The dock shows the TermSurf Ghost surfing icon for both release and debug
builds. Release shows the cyan wave, debug shows the green wave.

#### Description

This is a straightforward asset replacement. No code changes — only image files
are swapped. The asset catalog's `Contents.json` files and the Swift code that
references `BlueprintImage` by name remain unchanged.

#### Changes

**Release icon — `ghost/macos/Assets.xcassets/AppIconImage.imageset/`:**

Generate the three required sizes from `assets/termsurf-ghost-black.png`:

```bash
cp assets/termsurf-ghost-black.png \
   ghost/macos/Assets.xcassets/AppIconImage.imageset/macOS-AppIcon-1024px.png

sips -z 512 512 assets/termsurf-ghost-black.png --out \
   ghost/macos/Assets.xcassets/AppIconImage.imageset/macOS-AppIcon-512px.png

sips -z 256 256 assets/termsurf-ghost-black.png --out \
   ghost/macos/Assets.xcassets/AppIconImage.imageset/macOS-AppIcon-256px-128pt@2x.png
```

No changes to `Contents.json` — the filenames are preserved.

**Debug icon —
`ghost/macos/Assets.xcassets/Alternate Icons/BlueprintImage.imageset/`:**

```bash
cp assets/termsurf-ghost-alt-black.png \
   "ghost/macos/Assets.xcassets/Alternate Icons/BlueprintImage.imageset/macOS-AppIcon-1024px.png"
```

No changes to `Contents.json` or `AppDelegate.swift` — the asset name
`BlueprintImage` is preserved, only the underlying PNG changes.

#### Verification

```bash
cd ghost && zig build
open ghost/zig-out/Ghostty.app
```

1. **Dock icon:** The dock shows the CRT-with-surfing-ghost icon. In a debug
   build, the wave is green. In a release build, the wave is cyan.
2. **App switcher (Cmd+Tab):** Shows the same icon.
3. **Finder:** `ghost/zig-out/Ghostty.app` shows the new icon in Finder. (May
   require `touch ghost/zig-out/Ghostty.app` to bust the icon cache.)
4. **No Ghostty icon visible:** The old blue rounded-square Ghostty icon does
   not appear anywhere.

**Result:** Partial

The debug icon works — the green wave surfer appears in the dock. The release
icon still shows the old Ghostty icon, which flashes briefly before the debug
override replaces it.

#### Conclusion

The debug icon succeeded because it's loaded at runtime via
`NSImage(named: "BlueprintImage")` from the asset catalog imageset we replaced.

The release icon failed because `AppIconImage.imageset` is NOT what Xcode uses
for the app icon. The actual app icon comes from `ghost/images/Ghostty.icon/` —
an **Icon Composer** document (macOS/Xcode 16+ format). This is a layered bundle
containing multiple PNGs (`Ghostty.png`, `Screen.png`, `gloss.png`,
`Inner Bevel 6px.png`, `Screen Effects.png`) composited together with blend
modes, gradients, and glass effects. Xcode compiles this `.icon` bundle into
`Ghostty.icns` at build time.

The Xcode project includes `Ghostty.icon` as a resource (`project.pbxproj` line
57: `path = ../images/Ghostty.icon`), and
`ASSETCATALOG_COMPILER_APPICON_NAME = Ghostty` references it. The
`AppIconImage.imageset` is just a regular imageset used by the runtime icon-
switching system (`macos-icon` config) — not the Finder/Launchpad icon.

The Background section's description of how Ghostty handles icons was wrong. It
assumed `AppIconImage.imageset` was the source of the app icon. The correct
source is `Ghostty.icon`.

### Experiment 2: Replace the Icon Composer document

#### Goal

The release icon in Finder, Launchpad, and the initial dock display shows the
TermSurf Ghost surfing icon (cyan wave) instead of the Ghostty icon.

#### Description

The app icon is compiled from `ghost/images/Ghostty.icon/`, an Icon Composer
bundle. This layered format composites multiple PNGs with blend modes,
gradients, and glass effects. Our new icon is a single flat PNG — it doesn't use
Icon Composer's layering system.

The simplest approach: replace the `Ghostty.icon` bundle with a minimal Icon
Composer document that has a single layer containing our 1024px PNG. This
preserves the `.icon` format that Xcode expects while using our flat image.

An Icon Composer document is a folder with:

- `icon.json` — Layer definitions, blend modes, effects
- `Assets/` — PNG files referenced by `icon.json`

The minimal `icon.json` needs one group with one layer pointing to our PNG. No
gradients, no glass, no blend modes.

#### Changes

**`ghost/images/Ghostty.icon/Assets/`** — Remove all existing PNGs and copy in
the new icon:

```bash
rm ghost/images/Ghostty.icon/Assets/*.png
cp assets/termsurf-ghost-black.png ghost/images/Ghostty.icon/Assets/termsurf-ghost.png
```

**`ghost/images/Ghostty.icon/icon.json`** — Replace with a minimal single-layer
document:

```json
{
  "groups" : [
    {
      "layers" : [
        {
          "hidden" : false,
          "image-name" : "termsurf-ghost.png",
          "name" : "TermSurf Ghost"
        }
      ],
      "name" : "Icon"
    }
  ],
  "supported-platforms" : {
    "squares" : "shared"
  }
}
```

No changes to the Xcode project — it already references `Ghostty.icon` by path.

No changes to `AppIconImage.imageset` — Experiment 1 already replaced it (used
by the runtime icon-switching system).

No changes to `BlueprintImage.imageset` — Experiment 1 already replaced it
(debug override).

#### Verification

```bash
cd ghost && zig build
open ghost/zig-out/Ghostty.app
```

1. **Dock icon (before runtime override):** The TermSurf Ghost surfing icon
   (cyan wave) appears immediately when the app launches, before any runtime
   icon switching occurs.
2. **Finder:** `ghost/zig-out/Ghostty.app` shows the new icon. (May require
   `touch ghost/zig-out/Ghostty.app` to bust the icon cache.)
3. **Debug build dock icon:** The green wave debug icon still appears (runtime
   override from Experiment 1).
4. **App switcher (Cmd+Tab):** Shows the correct icon.

**Result:** Fail

The release icon does not load. The built `Ghostty.icns` is only 85KB / 256x256
— the Icon Composer compilation produced a degraded result. The minimal
`icon.json` with a single layer and no additional properties is likely missing
required fields that Icon Composer needs to properly render the icon at all
sizes.

#### Conclusion

The minimal `icon.json` format doesn't work. Icon Composer's `.icon` format
likely requires additional properties (lighting, shadow, fill, or platform
definitions) even for a single flat layer. Without documentation on the exact
required schema, guessing at the format is unreliable. A different approach is
needed — either reverse-engineering a working minimal `.icon` document or
bypassing Icon Composer entirely by generating the `.icns` file directly.
