# Experiment 1: Build, Publish, and Verify 1.0.0

## Description

Publish TermSurf `1.0.0` through the existing release workflow and verify that
the resulting Homebrew cask installs a usable production build.

The experiment has a package-only preflight before the irreversible publish
step. If the preflight finds stale paths, missing assets, a dirty tap, or a
broken tarball layout, stop and record the result instead of publishing.

## Changes

Planned release actions:

1. Confirm the repository and Homebrew tap are clean.

   ```bash
   git status --short
   git -C homebrew status --short --branch
   ```

2. Confirm GitHub release and tap preconditions.

   ```bash
   gh auth status
   gh release view v1.0.0 --repo termsurf/termsurf --json tagName,name,url,assets
   git -C homebrew fetch origin main
   git -C homebrew status --short --branch
   ```

   Expected preflight state:

   - `gh` is authenticated with permission to create releases.
   - `v1.0.0` does not already exist.
   - `homebrew/` is on `main` and not behind `origin/main`.

   If `v1.0.0` already exists, stop. Do not run `scripts/release.sh 1.0.0`,
   because the release script deletes any existing release before creating the
   replacement. Replacing an existing `1.0.0` release requires explicit user
   approval, a recorded rationale, and a recovery plan.

3. Record the design review and commit this experiment plan.

   Before running any build, package, publish, or install command:

   - record the design review findings and fixes in this file;
   - obtain reviewer approval after any fixes;
   - commit the approved experiment plan separately from the later result
     commit.

4. Build all release artifacts.

   ```bash
   scripts/build.sh all --release
   ```

5. Run a package-only release preflight.

   ```bash
   TERMSURF_RELEASE_PACKAGE_ONLY=1 scripts/release.sh 1.0.0
   ```

6. Inspect the generated tarball before publishing.

   ```bash
   tar tzf dist/termsurf-1.0.0-aarch64-apple-darwin.tar.gz | sort | sed -n '1,200p'
   shasum -a 256 dist/termsurf-1.0.0-aarch64-apple-darwin.tar.gz
   ```

   The tarball must contain:

   - `./TermSurf.app/`
   - `./web`
   - `./roamium/roamium`
   - Chromium runtime resources under `./roamium/`

   The tarball must not contain stale Wezboard app or path names.

7. Publish the release.

   ```bash
   scripts/release.sh 1.0.0
   ```

   This should create GitHub release `v1.0.0`, upload
   `termsurf-1.0.0-aarch64-apple-darwin.tar.gz`, update
   `homebrew/Casks/termsurf.rb` to `version "1.0.0"` and the matching SHA, and
   push the tap commit to `termsurf/homebrew-termsurf`.

8. Verify the published release and tap.

   ```bash
   gh release view v1.0.0 --repo termsurf/termsurf --json tagName,name,url,assets
   git -C homebrew status --short --branch
   git -C homebrew log --oneline -5
   sed -n '1,220p' homebrew/Casks/termsurf.rb
   ```

9. Verify Homebrew install behavior.

   Use the local machine unless a permission or cache issue requires a clean
   second machine.

   ```bash
   brew update
   brew uninstall --cask termsurf || true
   brew install --cask termsurf
   command -v web
   test "$(command -v web)" = "/opt/homebrew/bin/web"
   ls -ld /Applications/TermSurf.app
   ls -l /opt/homebrew/opt/termsurf-roamium/roamium
   test -f /opt/homebrew/opt/termsurf-roamium/roamium
   test -f /opt/homebrew/opt/termsurf-roamium/icudtl.dat
   test -f /opt/homebrew/opt/termsurf-roamium/gen/chrome/pdf_resources.pak
   test -f /opt/homebrew/opt/termsurf-roamium/gen/chrome/generated_resources_en-US.pak
   test -f /opt/homebrew/opt/termsurf-roamium/gen/chrome/common_resources.pak
   test -f /opt/homebrew/opt/termsurf-roamium/gen/components/components_resources.pak
   test -f /opt/homebrew/opt/termsurf-roamium/gen/components/strings/components_strings_en-US.pak
   test -f /opt/homebrew/opt/termsurf-roamium/gen/extensions/extensions_renderer_resources.pak
   find /opt/homebrew/opt/termsurf-roamium -maxdepth 1 -name '*.dylib' -type f | grep .
   find /opt/homebrew/opt/termsurf-roamium -maxdepth 1 -name '*.pak' -type f | grep .
   find /opt/homebrew/opt/termsurf-roamium -maxdepth 1 -name 'v8_context_snapshot*.bin' -type f | grep .
   find /opt/homebrew/opt/termsurf-roamium -type f | sed -n '1,160p'
   brew uninstall --cask termsurf
   brew install --cask termsurf
   ```

10. Smoke-test the installed production app.

    ```bash
    open -a /Applications/TermSurf.app
    ```

    In the launched TermSurf window, run:

    ```bash
    /opt/homebrew/bin/web --browser /opt/homebrew/opt/termsurf-roamium/roamium https://example.com/
    ```

    Verify that the page loads in Roamium and record concrete log evidence that
    the installed app used the Homebrew paths:

    - `/opt/homebrew/bin/web`
    - `/opt/homebrew/opt/termsurf-roamium/roamium`

    If available, also run the existing installed-release harness:

    ```bash
    TERMSURF_GHOSTBOARD_APP=/Applications/TermSurf.app \
      TERMSURF_INSTALLED_ROAMIUM_PATH=/opt/homebrew/opt/termsurf-roamium/roamium \
      scripts/ghostboard-geometry-matrix.sh installed-roamium-release-launch
    ```

11. Record the result, run completion review, and commit the result.

    After verification:

    - append `## Result` and `## Conclusion` to this file;
    - update the README experiment status to `Pass`, `Partial`, or `Fail`;
    - run the required fresh-context completion review;
    - fix any real completion-review findings;
    - commit the experiment result separately from the plan commit.

## Verification

Pass criteria:

- Release build succeeds.
- Package-only preflight creates a tarball with the expected production layout.
- The tarball and cask contain no stale Wezboard names.
- `scripts/release.sh 1.0.0` succeeds.
- GitHub release `v1.0.0` exists with the expected tarball asset.
- The Homebrew tap cask is pushed at `version "1.0.0"` with the tarball SHA.
- `brew install --cask termsurf` installs `/Applications/TermSurf.app`, `web`,
  and Roamium under `/opt/homebrew/opt/termsurf-roamium`.
- `command -v web` is exactly `/opt/homebrew/bin/web`.
- Explicit checks prove required Chromium resources exist under
  `/opt/homebrew/opt/termsurf-roamium`, including the generated resources copied
  by `scripts/roamium-resources.sh`.
- The installed app can run `web https://example.com/` and load the page.
- The runtime smoke test records evidence that the installed app used
  `/opt/homebrew/bin/web` and `/opt/homebrew/opt/termsurf-roamium/roamium`, not
  a stale debug or legacy path.

Fail criteria:

- Any release artifact is missing.
- The package layout would prevent Roamium from finding Chromium resources.
- The cask references stale Wezboard app names or removed install paths.
- `v1.0.0` already exists and explicit replacement approval has not been given.
- The GitHub release or Homebrew tap publish fails.
- The installed production app cannot run `web` and load a page.

If the publish succeeds but local Homebrew verification is blocked by local
machine state, record the publish result as **Partial** and continue with a
second verification experiment.

## Design Review

Fresh-context adversarial design review returned **CHANGES REQUIRED**.

Required findings:

- Existing-release handling was too permissive because `scripts/release.sh`
  deletes an existing `v1.0.0` before recreating it.
- Homebrew install verification did not explicitly prove `web` resolved from
  `/opt/homebrew/bin/web`.
- Roamium asset-root verification did not check deep generated Chromium resource
  paths.
- Runtime smoke testing did not record concrete evidence that the installed app
  used Homebrew `web` and Homebrew Roamium paths.
- The design did not explicitly list the required plan and result workflow
  gates.

Fixes applied:

- The publish precondition now requires `v1.0.0` to be absent, otherwise the
  experiment stops unless the user explicitly approves replacement with a
  recorded rationale and recovery plan.
- Homebrew install verification now asserts
  `test "$(command -v web)" = "/opt/homebrew/bin/web"`.
- Roamium verification now includes explicit `test -f` checks for the generated
  resources copied by `scripts/roamium-resources.sh`, plus checks for dylibs,
  top-level pak files, `icudtl.dat`, and V8 snapshots.
- Runtime smoke testing now runs
  `/opt/homebrew/bin/web --browser /opt/homebrew/opt/termsurf-roamium/roamium`
  and requires path evidence in logs.
- The plan now includes explicit design-review, plan-commit, completion-review,
  and result-commit gates.

Re-review result:

- Fresh-context adversarial re-review returned **APPROVED** with no remaining
  Required findings.

## Result

**Result:** Pass

TermSurf `1.0.0` was built, packaged, published to GitHub, pushed to the
Homebrew tap, installed with Homebrew, and smoke-tested with the installed app,
installed `web`, and installed Roamium path.

Release preflight:

- `git status --short` was clean before release execution.
- `git -C homebrew status --short --branch` reported `## main...origin/main`.
- `git -C homebrew rev-list --left-right --count HEAD...origin/main` reported
  `0 0`.
- `gh auth status` showed authenticated access for `ryanxcharles` with `repo`
  scope.
- `gh release view v1.0.0 --repo termsurf/termsurf` reported
  `release not found`.

Release build:

```bash
scripts/build.sh all --release
```

The release build succeeded. Chromium, `webtui`, Roamium, and Ghostboard all
built. The Xcode step emitted existing ImGui dSYM warnings from
`ghostty-internal.a(ext.o)`, but ended with `** BUILD SUCCEEDED **`.

Package-only preflight:

```bash
TERMSURF_RELEASE_PACKAGE_ONLY=1 scripts/release.sh 1.0.0
```

The package-only preflight succeeded and created
`dist/termsurf-1.0.0-aarch64-apple-darwin.tar.gz`.

Tarball verification:

- The final published tarball SHA was
  `bb6a781cc43aca779b11d2df3c68d2294e02b85c2269f37877c8cacf9ae6411d`.
- The tarball contained:
  - `./TermSurf.app/`
  - `./web`
  - `./roamium/roamium`
  - `./roamium/icudtl.dat`
  - `./roamium/gen/chrome/pdf_resources.pak`
  - `./roamium/gen/chrome/generated_resources_en-US.pak`
  - `./roamium/gen/chrome/common_resources.pak`
  - `./roamium/gen/components/components_resources.pak`
  - `./roamium/gen/components/strings/components_strings_en-US.pak`
  - `./roamium/gen/extensions/extensions_renderer_resources.pak`
- The tarball contained `492` top-level dylibs, `8` top-level pak files during
  tarball inspection, and `1` V8 snapshot.
- Stale Wezboard names were absent from the tarball.

Publish execution:

The first `scripts/release.sh 1.0.0` attempt stopped before publishing because a
local-only stale `v1.0.0` tag already existed and pointed at old commit
`4b4d4062d` (`build.zig: v1.0.0`). There was no remote `v1.0.0` release or tag.

Recovery:

```bash
git push upstream main
git tag -d v1.0.0
scripts/release.sh 1.0.0
```

This pushed the current release commits to `termsurf/termsurf`, removed only the
stale local tag, and then published successfully.

Published GitHub release:

- URL: `https://github.com/termsurf/termsurf/releases/tag/v1.0.0`
- Asset: `termsurf-1.0.0-aarch64-apple-darwin.tar.gz`
- Asset digest:
  `sha256:bb6a781cc43aca779b11d2df3c68d2294e02b85c2269f37877c8cacf9ae6411d`
- Asset size: `242693015`
- Remote tag `refs/tags/v1.0.0` points at
  `fcfeade1542e54840341b077f8587c70a41f816a`.
- `upstream/main` also points at `fcfeade1542e54840341b077f8587c70a41f816a`.

Homebrew tap:

- `homebrew/Casks/termsurf.rb` now has `version "1.0.0"`.
- The cask SHA is
  `bb6a781cc43aca779b11d2df3c68d2294e02b85c2269f37877c8cacf9ae6411d`.
- Tap commit `a59df29` (`v1.0.0`) was pushed to `termsurf/homebrew-termsurf`.

Homebrew install verification:

```bash
brew update
brew uninstall --cask termsurf || true
brew install --cask termsurf
command -v web
test "$(command -v web)" = "/opt/homebrew/bin/web"
```

Homebrew installed cask `1.0.0` successfully:

- `/Applications/TermSurf.app`
- `/opt/homebrew/bin/web`
- `/opt/homebrew/opt/termsurf-roamium/roamium`

Explicit installed resource checks passed for:

- `/opt/homebrew/opt/termsurf-roamium/roamium`
- `/opt/homebrew/opt/termsurf-roamium/icudtl.dat`
- `/opt/homebrew/opt/termsurf-roamium/gen/chrome/pdf_resources.pak`
- `/opt/homebrew/opt/termsurf-roamium/gen/chrome/generated_resources_en-US.pak`
- `/opt/homebrew/opt/termsurf-roamium/gen/chrome/common_resources.pak`
- `/opt/homebrew/opt/termsurf-roamium/gen/components/components_resources.pak`
- `/opt/homebrew/opt/termsurf-roamium/gen/components/strings/components_strings_en-US.pak`
- `/opt/homebrew/opt/termsurf-roamium/gen/extensions/extensions_renderer_resources.pak`
- at least one top-level dylib, pak file, and V8 snapshot.

Installed-file stale-name check:

- No `Wezboard` or `TermSurf Wezboard` references were found in:
  - `/opt/homebrew/Caskroom/termsurf/1.0.0`
  - `/Applications/TermSurf.app`
  - `/opt/homebrew/opt/termsurf-roamium`
  - `homebrew/Casks/termsurf.rb`

Second clean install cycle:

```bash
brew uninstall --cask termsurf
brew install --cask termsurf
```

The second uninstall/install cycle also passed for cask `1.0.0`.

Installed runtime smoke test:

```bash
TERMSURF_GHOSTBOARD_APP=/Applications/TermSurf.app \
  TERMSURF_WEB=/opt/homebrew/bin/web \
  TERMSURF_INSTALLED_ROAMIUM=/opt/homebrew/opt/termsurf-roamium/roamium \
  scripts/ghostboard-geometry-matrix.sh installed-roamium-release-launch
```

The first corrected installed-path run spawned the installed Roamium path but
timed out before AppKit overlay presentation. The app log proved the installed
path was used:

```text
SetOverlay: named browser resolved browser=roamium env=TERMSURF_INSTALLED_ROAMIUM_PATH path=/opt/homebrew/opt/termsurf-roamium/roamium
spawned browser path=/opt/homebrew/opt/termsurf-roamium/roamium
```

A second corrected installed-path run passed:

- Harness log:
  `logs/ghostboard-geometry-installed-roamium-release-launch-harness-20260619-191053.log`
- App log:
  `logs/ghostboard-geometry-installed-roamium-release-launch-app-20260619-191053.log`
- Roamium trace:
  `logs/ghostboard-geometry-installed-roamium-release-launch-roamium-20260619-191053.log`
- Screenshot:
  `logs/ghostboard-geometry-installed-roamium-release-launch-screenshot-20260619-191053.png`

The passing run recorded:

```text
web=/opt/homebrew/bin/web
SetOverlay: named browser resolved browser=roamium env=TERMSURF_INSTALLED_ROAMIUM_PATH path=/opt/homebrew/opt/termsurf-roamium/roamium
spawned browser path=/opt/homebrew/opt/termsurf-roamium/roamium
PASS: scenario installed-roamium-release-launch
```

Manual screenshot inspection confirmed `https://example.com/` loaded in the
installed TermSurf app.

## Conclusion

TermSurf `1.0.0` is published to GitHub and Homebrew. The final Homebrew cask
installs the expected app, `web` binary, and Roamium resource root, and the
installed release can launch a browser page through the installed app and
installed Roamium path.

The only notable issue was a stale local-only `v1.0.0` tag from older history.
Removing that local tag after confirming no remote `v1.0.0` tag or release
existed allowed the release script to create the correct tag at current
`upstream/main`.

## Completion Review

Fresh-context adversarial completion review returned **APPROVED** with no
findings.
