# Experiment 3: Fix Pristine macOS App Link

## Description

Investigate and fix the macOS Debug app link failure found in Experiment 2,
while keeping the imported Ghostty `v1.3.1` tree otherwise pristine. Experiment
2 proved the underlying Zig library can build, but Xcode failed while linking
`ghostty.debug.dylib` with many unresolved symbols from dependency groups such
as Spirv-Cross, glslang, libintl, Sentry, SIMD, and compiler-rt.

This experiment should make the unmodified upstream app build successfully in
this repo layout before any Ghostboard branding, config-path, icon, CLI, or
TermSurf protocol work begins.

## Changes

1. Inspect the Experiment 2 build log and generated `GhosttyKit.xcframework`:

   ```bash
   rg -n "BUILD FAILED|Undefined symbols|ghostty.debug.dylib|symbol\\(s\\) not found|spirv_cross|_sentry_|_libintl_|_ghostty_simd|___extend" \
     logs/ghostboard-exp2-macos-build.log

   nm -arch arm64 -gU \
     ghostboard/macos/GhosttyKit.xcframework/macos-arm64_x86_64/libghostty.a \
     | rg 'CompilerMSL|_sentry_init|_libintl_textdomain|_ghostty_simd_base64_decode|___extenddftf2'
   ```

2. Inspect the Xcode link command and project settings to determine whether
   `GhosttyKit.xcframework` is being linked as a static archive without forcing
   archive member loading:

   ```bash
   rg -n "OTHER_LDFLAGS|GhosttyKit|force_load|all_load|libghostty" \
     ghostboard/macos/Ghostty.xcodeproj/project.pbxproj
   ```

3. Try the smallest source-controlled fix that makes the Debug app link. The
   likely first candidate is an Xcode project linker setting that forces the
   static `GhosttyKit` archive to load the members that contain dependency
   definitions, for example `-force_load` against the generated macOS
   `libghostty.a`, or an equivalent project-supported setting.

   The fix must be narrow and justified by the evidence. Do not change `webtui`,
   `roamium`, branding, app names, config paths, icons, or TermSurf protocol
   code.

4. Rebuild:

   ```bash
   cd ghostboard
   zig build -Demit-macos-app=false
   macos/build.nu --configuration Debug --action build
   ```

5. Record the result in this experiment file.

## Verification

Pass criteria:

- The Experiment 2 unresolved-symbol groups are explained by concrete evidence:
  either the needed symbols are present in the generated static archive but not
  loaded by the Xcode link, or the needed symbols are absent from the generated
  archive and the fix corrects archive generation.
- The chosen fix is limited to the build/link path needed for the pristine macOS
  app to link.
- `cd ghostboard && zig build -Demit-macos-app=false` succeeds.
- `cd ghostboard && macos/build.nu --configuration Debug --action build`
  succeeds.
- `ghostboard/macos/build/Debug/Ghostty.app/Contents/MacOS/ghostty` exists.
- `ghostboard/macos/build/Debug/Ghostty.app/Contents/MacOS/ghostty.debug.dylib`
  exists.
- `git status --short` contains only the intentional source-controlled build fix
  and issue documentation edits; generated build outputs remain ignored.
- No branding, config-path, icon, CLI, protocol, `webtui`, or `roamium` changes
  are included.
- The issue README lists this experiment as `Pass` only after the result is
  recorded.

Fail criteria:

- The app still fails to link.
- The fix is speculative and not supported by symbol/archive/link-command
  evidence.
- The fix changes branding, config paths, icons, CLI names, protocol behavior,
  `webtui`, or `roamium`.
- The build leaves unexpected tracked generated files.
- The experiment works around the problem by weakening the acceptance criteria
  instead of making the Debug app link.

## Notes

This experiment is still a baseline-build experiment, not a Ghostboard porting
experiment. The output app may still be named Ghostty and use Ghostty config and
icons after this experiment. Those changes belong in later experiments.

## Design Review

Fresh-context adversarial review returned `APPROVED`.

- The reviewer checked the issue README, this experiment design, the Experiment
  2 result, the captured build log, and the relevant Ghostty build files.
- The reviewer confirmed the README links Experiment 3 as `Designed`, the
  experiment has the required sections, the scope stays limited to the baseline
  macOS app link failure, and the verification criteria are concrete.
- No required findings were reported.

## Result

**Result:** Fail

The experiment was stopped because its premise was wrong for this issue. A fresh
Ghostty `v1.3.1` import should build without source changes; changing
`ghostboard/` build files to compensate for the local build failure would
violate the baseline-build goal.

The attempted `ghostboard/` edits were unwound:

- `ghostboard/src/build/LibtoolStep.zig` was restored.
- `ghostboard/src/build/combine-static-libs.sh` was removed.

After the unwind, `git status --short` showed no tracked or untracked
`ghostboard/` source changes.

## Completion Review

Fresh-context adversarial completion review returned `APPROVED`.

- The reviewer confirmed this experiment has a `Result` and `Conclusion`.
- The reviewer confirmed the issue README marks Experiment 3 as `Fail`.
- The reviewer confirmed the documentation says no `ghostboard/` source changes
  should happen before the pristine Ghostty build and run baseline is proven.
- The reviewer confirmed `git diff -- ghostboard` showed no tracked
  `ghostboard/` source diffs.
- No required findings were reported.

## Conclusion

Do not fix the pristine Ghostty app build by modifying imported Ghostty source
in this experiment. Before any Ghostboard-specific modifications begin, the
imported Ghostty `v1.3.1` tree must build and run on macOS without errors as
plain upstream Ghostty.

Until that baseline is proven, make zero source changes under `ghostboard/`.
Assume failures are environment, toolchain, cache, permission, or invocation
issues to be fixed outside the imported source tree. The next experiment should
identify the missing environment or exact build/run invocation difference that
lets upstream Ghostty `v1.3.1` build and run as imported, with no source edits
to `ghostboard/`.
