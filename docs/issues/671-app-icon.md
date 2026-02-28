# Issue 671: Update App Icon

Update the app icon to the new source image and add a clean-zig script to
simplify cache clearing.

## Background

Issue 612 established the icon pipeline: a source image in `assets/`, a
`generate-icons.sh` script, and pre-rendered PNGs in the asset catalog. Changing
the release icon is a one-command operation: `gui/scripts/generate-icons.sh`.

The source image has been updated (`assets/termsurf-2-black-2.png`), but the
generated icon PNGs haven't been regenerated yet. Additionally, rebuilding the
icon requires clearing the zig build cache (stale `zig-out/` retains old icons
via `cp -R`), and there's no script for that. You have to remember three
separate `rm -rf` commands and be careful not to nuke the Chromium build in
`chromium/src/out/`.

## Goals

1. Create `gui/scripts/clean-zig.sh` — clears zig build artifacts without
   touching the Chromium cache.
2. Run `gui/scripts/generate-icons.sh` to regenerate all icon sizes from the
   updated source image.
3. Rebuild with a clean zig cache and verify the new icon appears.

## Experiment 1: Clean-zig script and icon regeneration

### Hypothesis

A `clean-zig.sh` script that removes `gui/zig-out/`, `gui/.zig-cache/`,
`gui/macos/build/`, and the Xcode DerivedData cache will ensure clean builds
without affecting the Chromium build at `chromium/src/out/`. Running
`generate-icons.sh` followed by a clean build will produce the updated icon.

### Changes

#### 1. Create `gui/scripts/clean-zig.sh`

```bash
#!/bin/bash
# Clean zig build artifacts without touching the Chromium cache.
# Usage: ./scripts/clean-zig.sh

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GUI_DIR="$(dirname "$SCRIPT_DIR")"

echo "Cleaning zig build artifacts..."
rm -rf "$GUI_DIR/zig-out/"
rm -rf "$GUI_DIR/.zig-cache/"
rm -rf "$GUI_DIR/macos/build/"
rm -rf ~/Library/Developer/Xcode/DerivedData/Ghostty-*

echo "Done. Chromium cache at chromium/src/out/ is untouched."
```

#### 2. Run `gui/scripts/generate-icons.sh`

Regenerate all icon sizes from the updated `assets/termsurf-2-black-2.png`.

#### 3. Clean and rebuild

```bash
gui/scripts/clean-zig.sh
cd gui && zig build
```

### Test

1. `gui/scripts/clean-zig.sh` runs without errors.
2. `gui/zig-out/`, `gui/.zig-cache/`, `gui/macos/build/` are gone.
3. `chromium/src/out/` is untouched.
4. `gui/scripts/generate-icons.sh` regenerates all 7 icon sizes + 3 AppIconImage
   sizes.
5. `cd gui && zig build` compiles without errors.
6. `open gui/zig-out/TermSurf.app` — new icon appears in Finder and dock.
