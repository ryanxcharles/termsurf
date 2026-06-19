# Experiment 1: Align Ghostboard cask packaging

## Description

Bring the existing `termsurf/homebrew-termsurf` tap and release packaging into
line with the current Ghostboard product shape.

The tap already exists as the `homebrew/` submodule and already contains
`Casks/termsurf.rb`, but that cask still describes the archived Wezboard
package:

- `app "TermSurf Wezboard.app"`
- `binary "wezboard"`
- postflight signing and quarantine clearing for `wezboard`
- postflight signing and quarantine clearing for
  `/Applications/TermSurf Wezboard.app`

The current supported frontend is Ghostboard, built and packaged as
`TermSurf.app`. This experiment updates the package/cask contract so a release
tarball produced by `scripts/release.sh` is installable by Homebrew without
building Chromium and without installing stale Wezboard artifacts.

This experiment does not publish a real public release until local package-only
verification proves the artifact layout and cask agree. If publishing is needed
to satisfy the public tap acceptance criterion, record the exact release version
and public tap verification in the result.

## Changes

1. Update `homebrew/Casks/termsurf.rb`.

   - install `TermSurf.app`;
   - keep installing the `web` binary;
   - remove the archived `wezboard` binary;
   - keep installing `roamium` to `/opt/homebrew/opt/termsurf-roamium`;
   - update postflight signing and quarantine clearing to reference
     `TermSurf.app`, `web`, and Roamium only.

2. Update `scripts/release.sh` if needed.

   - ensure package-only mode stages exactly the cask layout: `TermSurf.app`,
     `web`, and `roamium/`;
   - ensure the cask update path edits the existing tap cask without
     reintroducing Wezboard;
   - keep the release artifact name
     `termsurf-${VERSION}-aarch64-apple-darwin.tar.gz` unless verification
     proves a platform-tag change is required.

3. Update public documentation if stale.

   - document the tap install path for the Ghostboard package;
   - mention `brew trust termsurf/termsurf` if the local Homebrew version
     requires tap trust for third-party casks;
   - keep source-build instructions separate from the Homebrew prebuilt install
     path.

4. Do not modify app code unless packaging verification proves that app code is
   the blocker.

## Verification

1. Static checks:

   ```bash
   bash -n \
     scripts/release.sh \
     scripts/install.sh \
     scripts/build.sh \
     scripts/roamium-resources.sh
   brew style homebrew/Casks/termsurf.rb
   brew audit --cask --strict homebrew/Casks/termsurf.rb
   git diff --check
   ```

   Pass criteria:

   - shell syntax passes;
   - Homebrew style/audit either pass or any warnings are recorded and justified
     if they are expected for an unreleased local artifact;
   - no whitespace errors.

2. Build or confirm the required release artifacts:

   ```bash
   ./scripts/build.sh all --release
   ```

   Pass criteria:

   - `target/release/web` exists;
   - `target/release/roamium` exists;
   - `ghostboard/macos/build/Release/TermSurf.app/Contents/MacOS/termsurf`
     exists;
   - Chromium output still contains the Roamium runtime resources required by
     `scripts/roamium-resources.sh`.

3. Package without publishing:

   ```bash
   TERMSURF_RELEASE_PACKAGE_ONLY=1 ./scripts/release.sh 0.1.6-issue829
   ```

   Pass criteria:

   - `dist/termsurf-0.1.6-issue829-aarch64-apple-darwin.tar.gz` is created;
   - the tarball contains `TermSurf.app/`, `web`, and `roamium/`;
   - the tarball does not contain `TermSurf Wezboard.app` or `wezboard`;
   - `roamium/` contains the required generated resource packs from
     `scripts/roamium-resources.sh`.

4. Local cask contract check.

   Exercise the cask against the package layout without publishing by creating a
   temporary cask that points at the local package-only tarball:

   ```bash
   TMP_CASK_DIR="$(mktemp -d /tmp/termsurf-issue829-cask.XXXXXX)"
   TMP_CASK="$TMP_CASK_DIR/termsurf.rb"
   cp homebrew/Casks/termsurf.rb "$TMP_CASK"
   SHA="$(shasum -a 256 dist/termsurf-0.1.6-issue829-aarch64-apple-darwin.tar.gz | awk '{print $1}')"
   perl -0pi -e 's/version ".*"/version "0.1.6-issue829"/' "$TMP_CASK"
   perl -0pi -e "s/sha256 \".*\"/sha256 \"$SHA\"/" "$TMP_CASK"
   perl -0pi -e 's#url ".*"#url "file://'"$PWD"'/dist/termsurf-0.1.6-issue829-aarch64-apple-darwin.tar.gz"#' "$TMP_CASK"

   if brew list --cask termsurf >/dev/null 2>&1; then
     echo "termsurf cask is already installed; use a clean VM or get explicit approval before replacing it" >&2
     exit 77
   fi

   brew install --cask --appdir=/tmp/termsurf-issue829-apps "$TMP_CASK"
   ```

   Pass criteria:

   - Homebrew installs `TermSurf.app` into `/tmp/termsurf-issue829-apps`;
   - Homebrew links `web` into its bin path;
   - Homebrew installs Roamium and Chromium runtime resources into
     `/opt/homebrew/opt/termsurf-roamium`;
   - the installed layout has no `wezboard` binary and no
     `TermSurf Wezboard.app`;
   - the temporary cask pins the package-only tarball's sha256;
   - if `termsurf` is already installed as a cask, stop and use a clean VM or
     get explicit approval before replacing it;
   - after the runtime smoke, uninstall only the local cask installed by this
     experiment and remove `/tmp/termsurf-issue829-apps`.

5. Runtime smoke on this arm64 Tahoe VM.

   After installing through the temporary local cask path, explicitly launch the
   cask-installed app and run the installed `web` from inside that Ghostboard
   session:

   ```bash
   open -na /tmp/termsurf-issue829-apps/TermSurf.app
   ```

   Then use the existing GUI automation helpers or AppleScript/System Events to
   run this command inside the launched Ghostboard terminal pane:

   ```bash
   /opt/homebrew/bin/web \
     --browser /opt/homebrew/opt/termsurf-roamium/roamium \
     https://example.com
   ```

   Pass criteria:

   - the installed `/tmp/termsurf-issue829-apps/TermSurf.app` launches;
   - the launched terminal session has a `TERMSURF_SOCKET` value owned by that
     app run;
   - `/opt/homebrew/bin/web` is the binary executed inside that session;
   - Roamium starts from `/opt/homebrew/opt/termsurf-roamium/roamium`;
   - the browser leaves the startup waiting state and loads a page;
   - logs or screenshots prove the installed runtime is Ghostboard/TermSurf, not
     Wezboard.

6. Public tap verification, when publishing is performed.

   ```bash
   brew tap termsurf/termsurf
   brew trust termsurf/termsurf
   brew install --cask termsurf
   ```

   Pass criteria:

   - installation downloads a prebuilt GitHub Release artifact;
   - installation does not build Chromium;
   - the installed layout matches the documented locations;
   - the installed app and CLI pass the runtime smoke.

## Design Review

Fresh-context adversarial review returned **CHANGES REQUIRED**.

Required findings:

- The local cask contract check was too vague to prove the Homebrew install path
  before publishing.
- The runtime smoke launched `web` directly and did not prove that the
  cask-installed `TermSurf.app` session was the GUI receiving the TUI
  connection.

Optional finding:

- The shell syntax check omitted `scripts/roamium-resources.sh`, even though the
  release and install scripts source it.

Fixes applied:

- Added a concrete temporary local cask flow that rewrites version, sha256, and
  URL to the package-only tarball, then installs it with Homebrew.
- Tightened the runtime smoke so it launches the cask-installed `TermSurf.app`
  and runs `/opt/homebrew/bin/web` from inside that app session, with
  `TERMSURF_SOCKET` evidence required.
- Added `scripts/roamium-resources.sh` to the `bash -n` hygiene check.

Re-review returned **CHANGES REQUIRED** after confirming the original findings
were resolved.

Required finding:

- The temporary cask flow forcibly uninstalled any existing `termsurf` cask,
  which could overwrite the user's stable Homebrew/app install without explicit
  approval.

Fix applied:

- Replaced the forced uninstall with a guard: if the `termsurf` cask is already
  installed, local verification stops and requires a clean VM or explicit
  approval before replacing it.

Second re-review verdict: **APPROVED**.

The reviewer confirmed that the forced uninstall was replaced by a guard that
exits if `termsurf` is already installed, and that the pass criteria require a
clean VM or explicit approval before replacing an existing cask. No Required
findings remain.

## Result

**Result:** Pass

Implementation changed the packaging and release path only:

- `homebrew/Casks/termsurf.rb` now installs `TermSurf.app`, `web`, and
  `roamium`, with no archived `TermSurf Wezboard.app` or `wezboard` artifact.
- `scripts/release.sh` now checks out the Homebrew tap's `main` branch before
  editing, committing, and pushing the cask update, so a detached submodule
  checkout does not silently break release publication.
- `README.md` now documents the public tap install flow with
  `brew trust termsurf/termsurf`, which Homebrew 6 requires for this third-party
  cask.

Static verification passed:

```bash
bash -n scripts/release.sh scripts/install.sh scripts/build.sh scripts/roamium-resources.sh
brew style homebrew/Casks/termsurf.rb
HOMEBREW_NO_AUTO_UPDATE=1 brew audit --cask --strict termsurf/termsurf/termsurf
git diff --check
```

Homebrew 6 no longer allows `brew audit` directly on a cask file path, so the
final strict audit was run against the public tap cask name after pushing the
tap metadata fix.

The required release inputs already existed and were confirmed:

- `target/release/web`
- `target/release/roamium`
- `ghostboard/macos/build/Release/TermSurf.app/Contents/MacOS/termsurf`
- the six generated Roamium resource packs under `chromium/src/out/Default`.

Package-only verification created:

```text
dist/termsurf-0.1.6-issue829-aarch64-apple-darwin.tar.gz
sha256: 3bac668afab56c9124db7e50438b782e51111100f2d0836ff8b8d80b63d4bb2f
```

The tarball contained the expected top-level layout:

- `TermSurf.app/`
- `web`
- `roamium/`

It did not contain `TermSurf Wezboard.app` or `wezboard`, and `roamium/`
included the required generated resource packs.

Local Homebrew contract verification passed through a temporary local tap. A
direct cask-file install was not accepted by Homebrew 6 because Homebrew now
requires casks to live inside taps. The temporary local tap installed the
package-only tarball into:

- `/tmp/termsurf-issue829-apps/TermSurf.app`
- `/opt/homebrew/bin/web`
- `/opt/homebrew/opt/termsurf-roamium`

The local install had no Wezboard artifacts. Runtime smoke launched the
temporary cask-installed `TermSurf.app`, ran:

```bash
/opt/homebrew/bin/web \
  --browser /opt/homebrew/opt/termsurf-roamium/roamium \
  https://example.com
```

from inside that app session, spawned Roamium from the installed opt path, and
loaded `https://example.com`. Screenshot evidence is in
`logs/issue829-installed-runtime-screen-2.png`.

Public release verification passed with release `v0.1.6`:

- URL: `https://github.com/termsurf/termsurf/releases/tag/v0.1.6`
- artifact: `termsurf-0.1.6-aarch64-apple-darwin.tar.gz`
- artifact sha256:
  `2cecf0a518b087b6feb59f8d37203cde6b3d411b59e430234b21e7bad74e2016`
- Homebrew tap commit: `09e61f8 Restrict TermSurf cask to Apple silicon`

The public cask install command succeeded:

```bash
brew tap termsurf/termsurf
brew trust termsurf/termsurf
brew install --cask termsurf
```

The installed public layout was:

- `/Applications/TermSurf.app`
- `/opt/homebrew/bin/web -> /opt/homebrew/Caskroom/termsurf/0.1.6/web`
- `/opt/homebrew/opt/termsurf-roamium`

The public install had no `TermSurf Wezboard.app` or `wezboard` artifact.
Runtime smoke launched `/Applications/TermSurf.app`, ran the installed
`/opt/homebrew/bin/web`, spawned `/opt/homebrew/opt/termsurf-roamium/roamium`,
and loaded `https://example.com`. Screenshot evidence is in
`logs/issue829-public-runtime-screen-2.png`. Roamium's log at
`~/.local/state/termsurf/chromium-server.log` showed all six generated resource
packs as `found=1 loaded=1`.

One final strict public cask audit finding appeared after publication: Homebrew
required `verified: "github.com/termsurf/termsurf/"` because the release URL
domain differs from the homepage domain. That metadata fix was committed and
pushed to `termsurf/homebrew-termsurf` as `e2d2c40`, and the strict public cask
audit then passed.

## Completion Review

Fresh-context adversarial completion review returned **CHANGES REQUIRED**.

Required finding:

- The public cask URL installs an `aarch64` artifact, but the cask did not
  declare an Apple silicon architecture requirement. Intel macOS users could
  therefore attempt to install an unsupported arm64 package.

Fixes applied:

- Added `depends_on arch: :arm64` to `homebrew/Casks/termsurf.rb`.
- Updated `README.md` to state that the Homebrew cask currently supports Apple
  silicon macOS.
- Committed and pushed the tap metadata fix as
  `09e61f8 Restrict TermSurf cask to Apple silicon`.

Verification after the fix:

```bash
git -C "$(brew --repo termsurf/termsurf)" pull --ff-only
HOMEBREW_NO_AUTO_UPDATE=1 brew audit --cask --strict termsurf/termsurf/termsurf
brew info --cask termsurf/termsurf/termsurf
```

The public audit passed. `brew info` now reports:

```text
Required: arm64 architecture, macOS >= 15
Artifacts: TermSurf.app, roamium -> /opt/homebrew/opt/termsurf-roamium, web
```

Re-review verdict: **APPROVED**.

The reviewer confirmed that the prior Required finding is resolved:
`homebrew/Casks/termsurf.rb` declares `depends_on arch: :arm64`, the README and
issue conclusion explicitly state Apple silicon macOS support, the public tap
HEAD is `09e61f8 Restrict TermSurf cask to Apple silicon`, the strict public
cask audit exits successfully, and `brew info` reports
`Required: arm64 architecture, macOS >= 15`. No new Required findings remain.

## Conclusion

Experiment 1 solved the issue. TermSurf can be installed on Apple silicon macOS
from the public Homebrew cask using a prebuilt release tarball, without building
Chromium, and the installed Ghostboard/Web/Roamium runtime works from the
expected production locations on the arm64 Tahoe VM.
