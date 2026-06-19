# Experiment 1: Replace Ghostboard Icon Assets with the TermSurf Logo

## Description

Ghostboard's app identity is now `TermSurf`, but its macOS icon assets still use
the inherited terminal-style `$W` artwork. This experiment replaces the
Ghostboard source icon assets with the same TermSurf wave/prompt logo already
used by Wezboard.

The source of truth for this experiment is:

```text
wezboard/assets/macos/TermSurf Wezboard.app/Contents/Resources/wezboard.icns
```

The experiment should derive all required Ghostboard PNG sizes from that
existing `.icns` rather than introducing a new design. It should verify source
assets, build products, and install products without relying on Finder or Dock
icon caches.

## Changes

- `ghostboard/macos/Assets.xcassets/TermSurf.appiconset/`
  - Replace every `termsurf-icon-*.png` with the TermSurf wave/prompt logo at
    the existing appiconset sizes: 16, 32, 64, 128, 256, 512, and 1024 px.
  - Keep `Contents.json` structurally unchanged unless the asset filenames need
    to change.
- `ghostboard/macos/Assets.xcassets/AppIconImage.imageset/`
  - Replace the SwiftUI/about/settings icon image PNGs with the same TermSurf
    wave/prompt logo at the existing imageset sizes: 256, 512, and 1024 px.
  - Keep `Contents.json` structurally unchanged unless the asset filenames need
    to change.

Do not change app names, bundle IDs, CLI names, config paths, protocol code,
Wezboard assets, or app behavior outside icon assets.

## Verification

Before changing assets, record the current mismatch:

```bash
sips -s format png \
  "wezboard/assets/macos/TermSurf Wezboard.app/Contents/Resources/wezboard.icns" \
  --out logs/issue-0827-exp01-wezboard-source-icon.png
sips -g pixelWidth -g pixelHeight \
  logs/issue-0827-exp01-wezboard-source-icon.png \
  ghostboard/macos/Assets.xcassets/TermSurf.appiconset/termsurf-icon-512.png \
  ghostboard/macos/Assets.xcassets/AppIconImage.imageset/macOS-AppIcon-512px.png
```

Generate replacement PNGs from the Wezboard `.icns`, then verify dimensions:

```bash
sips -g pixelWidth -g pixelHeight \
  ghostboard/macos/Assets.xcassets/TermSurf.appiconset/termsurf-icon-16.png \
  ghostboard/macos/Assets.xcassets/TermSurf.appiconset/termsurf-icon-32.png \
  ghostboard/macos/Assets.xcassets/TermSurf.appiconset/termsurf-icon-64.png \
  ghostboard/macos/Assets.xcassets/TermSurf.appiconset/termsurf-icon-128.png \
  ghostboard/macos/Assets.xcassets/TermSurf.appiconset/termsurf-icon-256.png \
  ghostboard/macos/Assets.xcassets/TermSurf.appiconset/termsurf-icon-512.png \
  ghostboard/macos/Assets.xcassets/TermSurf.appiconset/termsurf-icon-1024.png \
  ghostboard/macos/Assets.xcassets/AppIconImage.imageset/macOS-AppIcon-256px-128pt@2x.png \
  ghostboard/macos/Assets.xcassets/AppIconImage.imageset/macOS-AppIcon-512px.png \
  ghostboard/macos/Assets.xcassets/AppIconImage.imageset/macOS-AppIcon-1024px.png \
  > logs/issue-0827-exp01-source-dimensions.log
```

Verify the rewritten source PNGs match the Wezboard source image content at the
same sizes by generating temporary resized reference PNGs and comparing bytes.
This check must fail on the first mismatch and must cover both the app bundle
appiconset and the SwiftUI imageset:

```bash
tmp_dir="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-issue827-icons.XXXXXX")"
sips -s format png \
  "wezboard/assets/macos/TermSurf Wezboard.app/Contents/Resources/wezboard.icns" \
  --out "$tmp_dir/source-1024.png"
for size in 16 32 64 128 256 512 1024; do
  cp "$tmp_dir/source-1024.png" "$tmp_dir/ref-$size.png"
  sips -z "$size" "$size" "$tmp_dir/ref-$size.png" >/dev/null
  cmp "$tmp_dir/ref-$size.png" \
    "ghostboard/macos/Assets.xcassets/TermSurf.appiconset/termsurf-icon-$size.png"
done
cmp "$tmp_dir/ref-256.png" \
  "ghostboard/macos/Assets.xcassets/AppIconImage.imageset/macOS-AppIcon-256px-128pt@2x.png"
cmp "$tmp_dir/ref-512.png" \
  "ghostboard/macos/Assets.xcassets/AppIconImage.imageset/macOS-AppIcon-512px.png"
cmp "$tmp_dir/ref-1024.png" \
  "ghostboard/macos/Assets.xcassets/AppIconImage.imageset/macOS-AppIcon-1024px.png"
shasum -a 256 "$tmp_dir"/ref-*.png \
  ghostboard/macos/Assets.xcassets/TermSurf.appiconset/termsurf-icon-*.png \
  ghostboard/macos/Assets.xcassets/AppIconImage.imageset/macOS-AppIcon-*.png \
  > logs/issue-0827-exp01-source-hashes.log
rm -rf "$tmp_dir"
```

Build the release app and verify the generated icon resource is no longer the
small stale icon:

```bash
./scripts/build.sh ghostboard --release \
  > logs/issue-0827-exp01-build-release.log 2>&1
test -x ghostboard/macos/build/Release/TermSurf.app/Contents/MacOS/termsurf
ls -lh ghostboard/macos/build/Release/TermSurf.app/Contents/Resources/TermSurf.icns \
  > logs/issue-0827-exp01-built-icon-resource.log
sips -s format png \
  ghostboard/macos/build/Release/TermSurf.app/Contents/Resources/TermSurf.icns \
  --out logs/issue-0827-exp01-built-icon.png
sips -g pixelWidth -g pixelHeight logs/issue-0827-exp01-built-icon.png \
  > logs/issue-0827-exp01-built-icon-dimensions.log
cmp logs/issue-0827-exp01-wezboard-source-icon.png \
  logs/issue-0827-exp01-built-icon.png
shasum -a 256 logs/issue-0827-exp01-wezboard-source-icon.png \
  logs/issue-0827-exp01-built-icon.png \
  > logs/issue-0827-exp01-built-icon-hashes.log
```

Install into a temporary Applications directory so the real `/Applications`
install path is not required for verification:

```bash
tmp_app_dir="$(mktemp -d "${TMPDIR:-/tmp}/termsurf-issue827-install.XXXXXX")"
TERMSURF_APPLICATIONS_DIR="$tmp_app_dir" ./scripts/install.sh ghostboard \
  > logs/issue-0827-exp01-install-temp.log 2>&1
test -x "$tmp_app_dir/TermSurf.app/Contents/MacOS/termsurf"
ls -lh "$tmp_app_dir/TermSurf.app/Contents/Resources/TermSurf.icns" \
  > logs/issue-0827-exp01-installed-icon-resource.log
sips -s format png "$tmp_app_dir/TermSurf.app/Contents/Resources/TermSurf.icns" \
  --out logs/issue-0827-exp01-installed-icon.png
cmp logs/issue-0827-exp01-wezboard-source-icon.png \
  logs/issue-0827-exp01-installed-icon.png
shasum -a 256 logs/issue-0827-exp01-wezboard-source-icon.png \
  logs/issue-0827-exp01-installed-icon.png \
  > logs/issue-0827-exp01-installed-icon-hashes.log
rm -rf "$tmp_app_dir"
```

Run hygiene checks:

```bash
bash -n scripts/build.sh scripts/install.sh scripts/uninstall.sh
prettier --write --prose-wrap always --print-width 80 \
  issues/0827-ghostboard-termsurf-icon/README.md \
  issues/0827-ghostboard-termsurf-icon/01-replace-ghostboard-icon-assets.md
git diff --check
```

Pass criteria:

- The Ghostboard source appiconset and SwiftUI imageset visually use the
  TermSurf wave/prompt logo, not the `$W` icon.
- The source PNG dimensions match their `Contents.json` declarations.
- Failing byte comparisons prove every Ghostboard source PNG was derived from
  the Wezboard TermSurf icon at the matching size.
- The release `TermSurf.app` builds successfully.
- The built `TermSurf.icns` converts back to a 1024 px PNG that byte-matches the
  Wezboard source icon conversion.
- Temporary install of Ghostboard succeeds and installs
  `TermSurf.app/Contents/MacOS/termsurf` plus the corrected icon resource.
- The installed `TermSurf.icns` converts back to a 1024 px PNG that byte-matches
  the Wezboard source icon conversion.
- Verification inspects bundle resources directly rather than relying on
  LaunchServices, Finder, or Dock cached icons.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

**Initial verdict:** Changes required.

Required findings and fixes:

- Source asset verification logged hashes without failing and did not cover
  `AppIconImage.imageset`. Fixed by requiring `cmp` checks for all seven
  `TermSurf.appiconset` PNGs and all three `AppIconImage.imageset` PNGs.
- Built and installed app verification only proved icon resource existence and
  dimensions. Fixed by requiring `cmp` checks between the converted Wezboard
  source icon and the converted built and installed `TermSurf.icns` resources.

The re-review approved the design with no remaining findings.
