# Experiment 3: Wire Surfari Into Packaging Paths

## Description

Stages 1 through 3 proved that this machine can build patched WebKit,
`libtermsurf_webkit`, pass the strict Surfari smoke test, and build the Rust
`surfari` binary. Stage 4 now needs to make Surfari a first-class packaged
engine instead of a development-only binary resolved through
`TERMSURF_SURFARI_PATH`.

Roamium already has complete packaging and installed discovery:

- `scripts/build.sh all --release` builds the Roamium Rust binary.
- `scripts/install.sh all` installs Roamium to
  `/opt/homebrew/opt/termsurf-roamium/`.
- `scripts/release.sh` stages `roamium/` in the release tarball.
- `homebrew/Casks/termsurf.rb` installs the `roamium` artifact into
  `/opt/homebrew/opt/termsurf-roamium`.
- Ghostboard resolves named `roamium` from `TERMSURF_ROAMIUM_PATH` in debug and
  from `/opt/homebrew/opt/termsurf-roamium/roamium` in release.

Surfari currently has only the development resolver:

- Ghostboard resolves named `surfari` only through `TERMSURF_SURFARI_PATH`.
- `scripts/build.sh all` does not build `libtermsurf_webkit` or `surfari`.
- `scripts/install.sh all` does not install Surfari or
  `libtermsurf_webkit.dylib`.
- `scripts/release.sh` does not package Surfari.
- `homebrew/Casks/termsurf.rb` does not install or codesign Surfari.

This experiment wires Surfari into the local build/install/release packaging
paths and installed Ghostboard discovery. It should not publish a GitHub release
or push the Homebrew tap; publishing belongs to a later stage after full release
build and package validation.

## Changes

- `scripts/build.sh`:
  - Add a `surfari` component.
  - Build `surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib` before
    `cargo build -p surfari`.
  - Include Surfari in `all`.
  - Update usage text.
- `scripts/install.sh`:
  - Add a `surfari` component.
  - Install `target/release/surfari` and
    `surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib` into
    `/opt/homebrew/opt/termsurf-surfari/`.
  - Support `TERMSURF_SURFARI_INSTALL_DIR` for non-root local verification,
    analogous to `TERMSURF_ROAMIUM_INSTALL_DIR`.
  - Include Surfari in `all`.
  - Codesign installed Surfari artifacts best-effort, matching the existing
    Roamium install style.
- `scripts/uninstall.sh`:
  - Add a `surfari` component and remove `/opt/homebrew/opt/termsurf-surfari/`
    plus any override directory.
  - Include Surfari in `all`.
- `scripts/release.sh`:
  - Require `target/release/surfari` and
    `surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib`.
  - Stage a `surfari/` directory containing the binary and dylib.
- `homebrew/Casks/termsurf.rb`:
  - Install the staged `surfari` directory into
    `/opt/homebrew/opt/termsurf-surfari`.
  - Codesign and clear quarantine for the installed Surfari binary and dylib.
- `ghostboard/src/apprt/termsurf.zig`:
  - Add installed Surfari discovery for release builds at
    `/opt/homebrew/opt/termsurf-surfari/surfari`.
  - Add an installed-override env var for release tests, analogous to
    `TERMSURF_INSTALLED_ROAMIUM_PATH`.
  - Preserve the debug contract: named `surfari` should still require
    `TERMSURF_SURFARI_PATH` in debug builds and should not silently fall through
    to an installed binary.
  - Extend existing resolver tests for named Surfari.
- `scripts/ghostboard-geometry-matrix.sh` or Ghostboard's test build wiring:
  - Add concrete release-mode coverage for named `surfari` resolving through the
    installed override path, so the release-only fallback is actually exercised.
- `docs/ghostboard-launch-discovery.md`:
  - Update the browser selection table and text to document named Surfari,
    `TERMSURF_SURFARI_PATH`, the installed Surfari path, and
    `TERMSURF_INSTALLED_SURFARI_PATH`.

## Verification

Script and source checks:

```bash
bash -n scripts/build.sh scripts/install.sh scripts/uninstall.sh scripts/release.sh
prettier --check issues/0838-deploy-next-homebrew-version/README.md \
  issues/0838-deploy-next-homebrew-version/03-wire-surfari-packaging-paths.md \
  docs/ghostboard-launch-discovery.md
zig fmt --check ghostboard/src/apprt/termsurf.zig
git diff --check
```

Build Surfari through the script:

```bash
./scripts/build.sh surfari --release
```

Verify direct artifacts:

```bash
test -x target/release/surfari
test -f surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib
otool -L target/release/surfari
```

Run Ghostboard resolver tests:

```bash
cd ghostboard
zig build test
```

Run release-mode installed Surfari discovery coverage. The implementation may
use a dedicated release resolver test step or a harness scenario, but it must
compile or run with `build_config.is_debug == false`:

```bash
TERMSURF_INSTALLED_SURFARI_PATH="$PWD/target/release/surfari" \
  scripts/ghostboard-geometry-matrix.sh installed-surfari-release-launch
```

Verify non-root install paths:

```bash
tmpdir="$(mktemp -d)"
TERMSURF_SURFARI_INSTALL_DIR="$tmpdir/surfari" ./scripts/install.sh surfari
test -x "$tmpdir/surfari/surfari"
test -f "$tmpdir/surfari/libtermsurf_webkit.dylib"
TERMSURF_SURFARI_INSTALL_DIR="$tmpdir/surfari" ./scripts/uninstall.sh surfari
test ! -e "$tmpdir/surfari"
rm -rf "$tmpdir"
```

Verify package-only release staging without publishing:

```bash
TERMSURF_RELEASE_PACKAGE_ONLY=1 scripts/release.sh 1.4.0
test -x dist/release/surfari/surfari
test -f dist/release/surfari/libtermsurf_webkit.dylib
tar tzf dist/termsurf-1.4.0-aarch64-apple-darwin.tar.gz |
  rg '^\\./surfari/(surfari|libtermsurf_webkit\\.dylib)$'
rg 'artifact "surfari".*/opt/homebrew/opt/termsurf-surfari' \
  homebrew/Casks/termsurf.rb
rg 'termsurf-surfari/(surfari|libtermsurf_webkit\\.dylib)' \
  homebrew/Casks/termsurf.rb
```

Pass criteria:

- `scripts/build.sh surfari --release` builds `libtermsurf_webkit.dylib` and
  `target/release/surfari`.
- Script syntax checks pass.
- Markdown checks pass for the edited issue and docs files.
- Ghostboard resolver tests pass.
- Debug Ghostboard still requires `TERMSURF_SURFARI_PATH` for named `surfari`.
- Release Ghostboard coverage proves named `surfari` resolves through the
  installed path or the installed override env var with
  `build_config.is_debug == false`.
- Non-root `scripts/install.sh surfari` installs both Surfari artifacts into the
  override directory.
- Non-root `scripts/uninstall.sh surfari` removes the override directory.
- `TERMSURF_RELEASE_PACKAGE_ONLY=1 scripts/release.sh 1.4.0` stages and packages
  a `surfari/` directory containing the Surfari binary and
  `libtermsurf_webkit.dylib`, but the experiment does not publish a release or
  push Homebrew.
- The Homebrew cask has a Surfari artifact plus codesign and quarantine-clearing
  coverage for the Surfari binary and dylib.
- `docs/ghostboard-launch-discovery.md` documents the new Surfari installed
  discovery semantics.
- `git diff --check` reports no whitespace errors.

Fail criteria:

- Surfari is still absent from `scripts/build.sh all --release`.
- Surfari cannot be resolved by installed release Ghostboard without
  `TERMSURF_SURFARI_PATH`.
- The cask does not install Surfari.
- Install/release packaging omits `libtermsurf_webkit.dylib`.
- The implementation changes default Roamium behavior or debug installed
  fallback rules.
- Any required build, syntax, or resolver test fails.

## Design Review

Initial fresh-context adversarial review returned **Changes Required** with four
required findings:

- Release installed Surfari discovery was not actually proven because
  `zig build test` runs Debug tests.
- Release and cask packaging were not concretely verified.
- Launch discovery docs were optional even though resolver semantics change.
- Markdown hygiene checks were missing.

The design was updated to add release-mode installed Surfari coverage,
package-only tarball and cask assertions, required launch-discovery docs, and
explicit markdown checks.

Re-review returned **Approved** with no required findings. The reviewer
confirmed that the revised design now proves release-mode Surfari discovery,
concretely checks package-only release and cask staging, makes launch-discovery
docs required, and includes markdown hygiene checks.

## Result

**Result:** Partial

Implemented the Surfari build, install, uninstall, release-staging, Homebrew
cask, installed Ghostboard resolver, release resolver harness, and launch
discovery documentation wiring.

Verification passed:

- `bash -n scripts/build.sh scripts/install.sh scripts/uninstall.sh scripts/release.sh scripts/ghostboard-geometry-matrix.sh`
- `prettier --check issues/0838-deploy-next-homebrew-version/README.md issues/0838-deploy-next-homebrew-version/03-wire-surfari-packaging-paths.md docs/ghostboard-launch-discovery.md`
- `zig fmt --check ghostboard/src/apprt/termsurf.zig`
- `git diff --check`
- `./scripts/build.sh surfari --release`
- direct artifact checks for `target/release/surfari` and
  `surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib`
- `otool -L target/release/surfari`, which confirms Surfari loads
  `@rpath/libtermsurf_webkit.dylib`
- `cd ghostboard && zig build test`
- non-root `scripts/install.sh surfari` / `scripts/uninstall.sh surfari` with
  `TERMSURF_SURFARI_INSTALL_DIR`
- `TERMSURF_RELEASE_PACKAGE_ONLY=1 scripts/release.sh 1.4.0`
- tarball inspection for `./surfari/surfari` and
  `./surfari/libtermsurf_webkit.dylib`
- static Homebrew cask assertions for Surfari artifact, codesign, and xattr
  handling
- `TERMSURF_INSTALLED_SURFARI_PATH="$PWD/target/release/surfari" scripts/ghostboard-geometry-matrix.sh installed-surfari-release-launch`

The release resolver harness proves that Release Ghostboard resolves named
`surfari` through `TERMSURF_INSTALLED_SURFARI_PATH`, does not use
`TERMSURF_SURFARI_PATH`, and spawns the installed Surfari binary.

During verification, the first release harness implementation waited for full
overlay presentation. That exposed a larger runtime dependency issue: Surfari
spawned, but dyld failed inside the repo-built custom WebKit framework because
it loaded WebKit from `webkit/src/WebKitBuild/Debug` while resolving
JavaScriptCore from the system framework:

```text
dyld: Symbol not found: __Z20WTFCrashWithInfoImpliPKcS0_y
Referenced from: webkit/src/WebKitBuild/Debug/WebKit.framework/Versions/A/WebKit
Expected in: /System/Library/Frameworks/JavaScriptCore.framework/Versions/A/JavaScriptCore
```

Inspection confirmed that `libtermsurf_webkit.dylib` carries an rpath to
`webkit/src/WebKitBuild/Debug` and depends on
`@rpath/WebKit.framework/Versions/A/WebKit`. Packaging only the Surfari binary
and `libtermsurf_webkit.dylib` is therefore not enough to make installed Surfari
render pages with the patched WebKit build. The next experiment needs to decide
and verify the WebKit runtime packaging strategy.

## Conclusion

Experiment 3 successfully made Surfari first-class in the local build,
install/uninstall, release staging, Homebrew cask, Ghostboard installed
resolver, release resolver harness, and launch discovery docs. It did not finish
the complete Surfari deployment path because a real installed-style launch
showed that the package also needs a correct custom WebKit runtime closure, or a
different deliberate runtime-linking strategy.

The next experiment should focus specifically on Surfari's packaged WebKit
runtime: which frameworks and dylibs must ship, how their install names/rpaths
must be arranged, and how to prove `web --browser surfari` can reach
`BrowserReady` and overlay presentation without development-only paths.

## Completion Review

Fresh-context adversarial completion review returned **Approved** with no
findings. The reviewer accepted the Partial result, including the Surfari
packaging/resolver progress and the recorded WebKit runtime closure blocker.
