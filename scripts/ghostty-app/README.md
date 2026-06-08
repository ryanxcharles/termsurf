# Ghostty app build/run/automate harness (Issue 802, Exp 2 + 3)

Tooling to build, run, and UI-automate the **real, unmodified** Ghostty macOS app
(vendored at `vendor/ghostty/macos`, commit `2c62d18`, v1.3.2-dev) as the libroastty
conformance oracle. Full findings: `issues/0802-…/02-ghostty-app-baseline.md` (the
toolchain investigation) and `…/03-macos-only-build.md` (the working build).

## The toolchain situation (resolved)

- Ghostty 1.3.2-dev hard-requires **zig 0.15.2** (its `requireZig` enforces an exact
  major.minor; the system 0.16.0 fails to compile `build.zig`). `setup-zig.sh` pins
  0.15.2 under `vendor/toolchains/` (gitignored).
- zig 0.15.2 **cannot link this machine's Xcode 26.4 SDK** (`__availability_version_check`)
  — a too-new point release — but **can** link the **CommandLineTools 26.0** SDK.
- The macOS app needs only the **macOS** slice of `GhosttyKit`. The full xcframework
  also builds an **iOS** slice, which needs an iOS SDK zig 0.15.2 can't link (only
  iOS 26.4 is present; CLT has none).

## Build the app (approach 1 — macOS-only, no Xcode change)

```bash
scripts/ghostty-app/build-macos-app.sh [Debug|ReleaseLocal]
# -> vendor/ghostty/macos/build/<config>/Ghostty.app
```

What it does (see the script): pin zig 0.15.2 → apply `macos-only-xcframework.patch`
(gate the iOS slice on `.universal`) → build the macOS lib + Metal shaders under
`DEVELOPER_DIR=CommandLineTools` with Xcode's `metal` on `PATH` → `xcodebuild
-create-xcframework` under Xcode → `macos/build.nu` builds the Swift app under Xcode.
The **app is unaltered**; the only change is the build-only `build.zig` patch.

## Run + (attempt) automate

```bash
APP="$PWD/vendor/ghostty/macos/build/Debug/Ghostty.app"
open "$APP"                                  # launches a working terminal window
osascript -e "tell application \"$APP\" to activate"
screencapture -x shot.png                    # works (Screen Recording granted to Wezboard)
osascript -e "tell application \"$APP\" to quit"
```

**Known harness gap (Exp 3):** capturing _just Ghostty's window_ from the agent isn't
solved yet — a full-screen `screencapture` grabs the agent's Wezboard fullscreen
Space (not Ghostty's), and `CGWindowListCopyWindowInfo` from the agent returned no
Ghostty window (a Spaces/entitlement nuance). The window is real and working
(visually confirmed); window-isolated capture is a Phase-A/workstream-3 item.
