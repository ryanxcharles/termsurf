+++
status = "open"
opened = "2026-06-19"
+++

# Issue 832: Publish 1.0.0 to Homebrew

## Goal

Publish TermSurf version `1.0.0` through the existing Homebrew cask workflow so
users can install or upgrade with:

```bash
brew tap termsurf/termsurf
brew install --cask termsurf
```

## Background

TermSurf is distributed through the `termsurf/homebrew-termsurf` tap, tracked in
this repository as the `homebrew/` submodule. The current release workflow is:

1. Build all release artifacts with `scripts/build.sh all --release`.
2. Run `scripts/release.sh <version>`.
3. Upload the packaged artifacts to the GitHub Release for `termsurf/termsurf`.
4. Update and push the Homebrew cask with the new version and SHA.

Issue 829 restored and verified the Homebrew installation path for recent
versions. This issue should publish the first `1.0.0` release using that same
packaging path, while verifying that the installed production app loads
Ghostboard, `web`, Roamium, and the Chromium resource bundle from the correct
locations.

## Scope

This issue is release and packaging work. It should not make feature changes
unless a release blocker is discovered and explicitly handled in an experiment.

Expected release artifacts:

- `/Applications/TermSurf.app`
- `web` installed into Homebrew's bin directory
- Roamium and Chromium assets installed under the Homebrew-managed Roamium
  prefix
- Homebrew cask version and SHA updated to `1.0.0`

## Acceptance Criteria

- `scripts/build.sh all --release` completes successfully.
- `scripts/release.sh 1.0.0` creates or updates the GitHub Release and Homebrew
  cask for version `1.0.0`.
- A clean Homebrew install or upgrade path succeeds with
  `brew install --cask termsurf`.
- The installed `web` command resolves from Homebrew's bin directory.
- The installed app launches as `/Applications/TermSurf.app`.
- The installed app can run `web` and open a page with Roamium.
- Roamium loads from the Homebrew-managed install root with its Chromium assets
  available relative to the binary.
- Any stale names from removed products, including Wezboard, do not appear in
  the cask install or uninstall paths.
- The issue records exact install, upgrade, uninstall, and smoke-test commands
  used to verify the release.
