# Issue 737: Wezboard app icon

## Goal

Replace Wezboard's default WezTerm icon (`terminal.icns`) with the latest
TermSurf icon (`assets/termsurf-11.png`).

## Background

Wezboard currently ships with the stock WezTerm icon at
`wezboard/assets/macos/Wezboard.app/Contents/Resources/terminal.icns`. The
`Info.plist` references it as `CFBundleIconFile = terminal.icns` (and again
under `CFBundleDocumentTypes`).

The latest TermSurf icon is `assets/termsurf-11.png`. Ghostboard already has its
own icon pipeline (`scripts/generate-icons.sh`) that uses `sips` to resize a
source PNG into all required sizes, but that script targets Ghostboard's Xcode
asset catalogs. Wezboard doesn't use Xcode — it has a bare `.app` template with
a single `.icns` file.

### What needs to change

1. Generate a `wezboard.icns` from `assets/termsurf-11.png` using `iconutil`
   (create an `.iconset` directory with all required sizes, then convert to
   `.icns`).
2. Replace `wezboard/assets/macos/Wezboard.app/Contents/Resources/terminal.icns`
   with the new `wezboard.icns`.
3. Update `Info.plist` to reference `wezboard.icns` instead of `terminal.icns`
   (both in `CFBundleIconFile` and `CFBundleDocumentTypes`).
