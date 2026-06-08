+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"
+++

# Experiment 3: macOS-only build — the real Ghostty app builds and runs on this machine

## Description

Experiment 2 found a real toolchain blocker: the vendored Ghostty (commit
`2c62d18`, v1.3.2-dev) hard-requires **zig 0.15.2**, which cannot link this
machine's **Xcode 26.4** SDK, and the full `GhosttyKit.xcframework`'s **iOS**
slice needs an iOS SDK zig 0.15.2 also can't link. My initial Exp-2
recommendation ("install Xcode 16") was **wrong** — the official docs require
**Xcode 26** (which this machine has), and zig can't be bumped (Ghostty's
`requireZig` enforces an exact major.minor and its source targets 0.15.x; even
Ghostty's current `main` still pins 0.15.2).

This experiment implements **approach 1**: build only the **macOS** slice of
`GhosttyKit` (the app doesn't need iOS), linking the zig code against the
**CommandLineTools 26.0** SDK (which zig 0.15.2 _can_ link), then package and
build the Swift app under Xcode 26.4 — with **no Xcode change and the app
unaltered**.

## Why this works

- zig 0.15.2 fails to link Xcode 26.4's SDK
  (`undefined symbol: __availability_version_check`), but **links the
  CommandLineTools 26.0 SDK fine** (a swept test confirmed: all Xcode-26.4 SDKs
  fail, all CLT SDKs pass).
- The macOS app needs only the **macOS** xcframework slice. The iOS slice is the
  only thing that requires an SDK zig 0.15.2 can't link (only iOS 26.4 is
  installed; CLT has no iOS SDK), so we gate it out.
- The Metal toolchain lives in Xcode, not CLT, so the `metal` compiler is
  supplied via `PATH` for the CLT-context build.
- The final `xcodebuild -create-xcframework` and the Swift app build run under
  Xcode 26.4; the app link resolves `__availability_version_check` against
  **libSystem 26.4** (which provides it), so the SDK-version mismatch is
  confined to the zig compile.

## Changes / Deliverables

- **`scripts/ghostty-app/macos-only-xcframework.patch`** — a **build-only**
  patch to `vendor/ghostty/src/build/GhosttyXCFramework.zig` that gates the iOS
  / iOS-sim `GhosttyLib.initStatic` calls on `target == .universal`, so a
  `-Dxcframework-target=native` build never _constructs_ the iOS lib (whose
  `findNative` would fail). **The Ghostty app itself is unaltered** — this
  touches only Ghostty's build script. (A deviation, documented and reversible;
  approach 2 — an earlier Xcode 26.x — would avoid even this.)
- **`scripts/ghostty-app/build-macos-app.sh`** — the full reproducible recipe:
  pin zig 0.15.2 → apply the patch → build the macOS lib + Metal shaders under
  `DEVELOPER_DIR=CommandLineTools` with Xcode's `metal` on `PATH`
  (`zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false`)
  → `xcodebuild -create-xcframework` under Xcode → `macos/build.nu` builds the
  app.
- **`scripts/ghostty-app/README.md`** — updated with the resolved recipe.

## Verification (what actually happened)

1. **macOS lib + Metal shaders built under CommandLineTools** — `180/182` build
   steps succeeded, producing `libghostty-internal-fat.a` (195 MB), **no**
   `DarwinSdkNotFound` (iOS gated), **no** `__availability_version_check` (CLT
   SDK). The one failing step was the final `create-xcframework` (xcodebuild
   needs Xcode).
2. **`xcodebuild -create-xcframework` under Xcode** →
   `macos/GhosttyKit.xcframework` (`macos-arm64` slice + headers), `rc=0`.
3. **`macos/build.nu --configuration Debug` under Xcode 26.4** →
   `** BUILD SUCCEEDED **` with `0` errors, producing
   `macos/build/Debug/Ghostty.app` (the `ghostty` executable + a 73 MB
   `ghostty.debug.dylib`). The app-link **resolved
   `__availability_version_check`** against libSystem 26.4 — confirming the SDK
   mismatch is confined to the zig stage.
4. **The app launches and shows a working terminal window** — confirmed visually
   by the user (the Ghostty window opened and rendered a live terminal).
5. `screencapture` itself works (Screen Recording is granted to the host
   terminal).

## Result

**Result:** Pass.

The real, unmodified Ghostty 1.3.2-dev macOS app **builds and runs on this
machine** with no Xcode change and no downgrade — the Exp-2 toolchain blocker is
resolved via the macOS-only build. The conformance host that Phase A needs now
exists.

**Known harness gap (not a blocker for this result):** capturing _just Ghostty's
window_ programmatically from the agent is unsolved — a full-screen
`screencapture` grabs the agent's Wezboard fullscreen Space, and
`CGWindowListCopyWindowInfo` from the agent returned no Ghostty window (a
Spaces/entitlement nuance; `pyobjc`/Quartz is absent, and the JXA bridge
enumerated none). The window is real and working; isolating it for a golden
screenshot is a Phase-A / workstream-3 task (likely: a tiny helper with
Screen-Recording + window-list entitlements, or driving capture from within the
app's own test target / XCUITest).

## Conclusion

Approach 1 works and is the no-new-install path: **the macOS-only build produces
a runnable real Ghostty app.** Experiment 2's blocker is closed (its "install
Xcode 16" recommendation was wrong and is corrected here). Phase A's build/run
is done; the remaining Phase-A items are (a) solving agent-side
**window-isolated screenshot capture**, then (b) the golden baseline, after
which Phase B (copy + rename the app against `libroastty`) begins.

**Next:** solve window-isolated capture (so the automated UI harness and the
golden baseline can be produced), or — if you'd rather move faster — proceed to
Phase B (copy the now-buildable app, rename `ghostty→roastty`, point it at
`libroastty`) and return to the screenshot harness alongside it.
