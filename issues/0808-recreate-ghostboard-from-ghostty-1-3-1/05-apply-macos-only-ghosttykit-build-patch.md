# Experiment 5: Apply the macOS-Only GhosttyKit Build Patch

## Description

Experiment 4 proved that the pristine imported Ghostty `v1.3.1` tree does not
currently build the macOS app on this VM with the documented default build flow.
Prior work in Issue 802 found the same family of Ghostty/Zig/Xcode SDK problem
and resolved it without downgrading Xcode by applying a small build-only patch:
build only the macOS `GhosttyKit` slice and do not construct the iOS or
iOS-simulator slices for a native macOS app build. During implementation, this
experiment also found that the older Ghostty `v1.3.1` build system needed a
later upstream build-only `libtool` normalization fix to prevent newer Xcode
from dropping static archive members.

This experiment applies that known workaround to the fresh `ghostboard/` import,
then verifies that Ghostty `v1.3.1` app/runtime code builds and runs on macOS.
This is no longer a pristine-upstream baseline; it is an upstream Ghostty
app/runtime baseline with one documented build-system deviation.

## Changes

- `ghostboard/src/build/GhosttyXCFramework.zig` — gate construction of the iOS
  and iOS-simulator `GhosttyLib` values so they are only built for the universal
  xcframework target. For the native target, keep only the macOS `GhosttyKit`
  slice.
- `ghostboard/src/build/LibtoolStep.zig` — backport upstream Ghostty commit
  `a83a82b3f` (`build: normalize input archives before Darwin libtool merge`) so
  each input archive is copied and re-indexed with `ranlib` before Apple's
  `libtool -static` combines it.

These should be the only source changes under `ghostboard/`. They must not
change branding, config paths, CLI names, icons, protocol code, Swift app
behavior, runtime Zig code, `webtui`, or `roamium`.

The implementation should follow the prior successful patch:

- `scripts/ghostty-app/macos-only-xcframework.patch`
- Issue 802 Experiment 3:
  `issues/0802-libroastty-completion-and-mac-app/03-macos-only-build.md`

## Verification

1. Apply the build-only patch.
2. Run Zig formatting on the patched Zig file.
3. If the app link still fails with missing dependency symbols, apply the
   upstream Darwin `libtool` archive-normalization build fix and format that Zig
   file too.
4. Build the native macOS-only `GhosttyKit` framework:

   ```bash
   cd ghostboard
   zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false
   ```

5. Build the macOS app with Xcode:

   ```bash
   cd ghostboard/macos
   xcodebuild -target Ghostty -configuration Debug -arch arm64 \
     COMPILATION_CACHE_CAS_PATH="$HOME/Library/Developer/Xcode/DerivedData/CompilationCache.noindex" \
     COMPILATION_CACHE_KEEP_CAS_DIRECTORY=YES
   ```

6. If the app builds, launch it by absolute path, confirm a `ghostty` process is
   running from the built app bundle, then terminate only that built app
   process.
7. Confirm the `ghostboard/` diff is limited to
   `src/build/GhosttyXCFramework.zig` and `src/build/LibtoolStep.zig`.

Pass criteria:

- The only source changes under `ghostboard/` are build-only patches to
  `GhosttyXCFramework.zig` and `LibtoolStep.zig`.
- `zig fmt` accepts the patched files.
- `zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false`
  succeeds.
- `xcodebuild -target Ghostty -configuration Debug -arch arm64` succeeds.
- `ghostboard/macos/build/Debug/Ghostty.app/Contents/MacOS/ghostty` exists.
- The built app launches and produces a scoped process that can be terminated.

Fail criteria:

- More `ghostboard/` source files must be changed.
- The app still fails to build or launch.
- The workaround requires Ghostboard branding, config, protocol, Swift runtime,
  `webtui`, or `roamium` changes.
- The result does not clearly document that this is a build-only deviation from
  pristine upstream Ghostty.

## Notes

If this passes, later experiments can begin the actual Ghostboard port on top of
a proven local macOS build, with the build-only deviation explicitly recorded.

## Design Review

Fresh-context adversarial review returned `APPROVED`.

- No required findings were reported.
- Optional finding accepted: make `-Demit-xcframework=true` explicit in the Zig
  build command rather than relying on Ghostty's default emit behavior.
- Nit accepted for implementation: do not copy the prior Roastty-specific
  comment verbatim into `ghostboard/`; use an Issue 808 comment if a comment is
  needed.

## Result

**Result:** Pass

The native macOS-only GhosttyKit patch alone built the xcframework but did not
produce a linkable app. The generated `libghostty-fat.a` was only 101 MB with
140 archive members, and the Xcode app link failed with missing dependency
symbols from ImGui, gettext, sentry, SIMD, and related libraries.

The failure matched upstream Ghostty commit `a83a82b3f`
(`build: normalize input archives before Darwin libtool merge`), which explains
that newer Apple `libtool` versions can silently drop 64-bit archive members
while merging static libraries. After backporting that build-only normalization
change to `ghostboard/src/build/LibtoolStep.zig`, the clean native framework
build succeeded and produced a 197 MB `libghostty-fat.a` with 282 archive
members.

Verification commands:

```bash
cd ghostboard
rm -rf .zig-cache zig-out macos/GhosttyKit.xcframework macos/build
zig build -Demit-xcframework=true -Dxcframework-target=native -Demit-macos-app=false

cd ghostboard/macos
xcodebuild -target Ghostty -configuration Debug -arch arm64
open -n /Users/astrohacker/dev/termsurf/ghostboard/macos/build/Debug/Ghostty.app
```

The Xcode build ended with `** BUILD SUCCEEDED **`. The launched process was:

```text
/Users/astrohacker/dev/termsurf/ghostboard/macos/build/Debug/Ghostty.app/Contents/MacOS/ghostty
```

The scoped debug app process was then terminated.

Logs:

- `logs/ghostboard-exp5-zig-native-xcframework-20260616-063914.log` — first
  native macOS-only framework build passed.
- `logs/ghostboard-exp5-xcodebuild-debug-arm64-20260616-063956.log` — first
  Xcode app build failed at link because the static archive was incomplete.
- `logs/ghostboard-exp5-zig-native-xcframework-libtool-normalized-20260616-064449.log`
  — clean framework build after the archive-normalization fix passed.
- `logs/ghostboard-exp5-zig-native-xcframework-libtool-normalized-summary-20260616-064930.log`
  — wrapper verification for the quiet Zig build, including exit status,
  `GhosttyKit.xcframework` archive path, 197 MB archive size, and 282 archive
  members.
- `logs/ghostboard-exp5-xcodebuild-debug-arm64-libtool-normalized-20260616-064531.log`
  — Xcode app build passed.
- `logs/ghostboard-exp5-open-debug-app-20260616-064554.log` — app launch
  produced the expected built `ghostty` process.

## Completion Review

Fresh-context adversarial review returned `APPROVED_WITH_NITS`.

- No required findings were reported.
- Optional nit accepted: the clean Zig build log was empty because successful
  `zig build` produced no stdout/stderr. A wrapper verification log was added to
  record the command, artifact check exit status, `GhosttyKit.xcframework`
  archive path, 197 MB archive size, and 282 archive members.

## Conclusion

The buildable baseline for this VM is not pristine Ghostty `v1.3.1`; it is
Ghostty `v1.3.1` app/runtime code with two documented build-only deviations. The
first avoids constructing iOS slices for the native macOS app baseline. The
second backports Ghostty's later archive-normalization fix so Xcode 26.6 can
link the app correctly.

This establishes that Ghostboard-specific work can begin from a locally
buildable and runnable Ghostty baseline, while keeping the build-only deviations
separate from branding, config, protocol, `webtui`, and `roamium` changes.
