+++
status = "open"
opened = "2026-06-22"
+++

# Issue 838: Deploy Next Homebrew Version

## Goal

Deploy the next TermSurf version to Homebrew so users can install the latest
shipped changes with:

```bash
brew tap termsurf/termsurf
brew trust termsurf/termsurf
brew install --cask termsurf
```

## Background

The next issue number is **838**. Issues 834 and 835 are already occupied by
upstream work, Issue 836 is the closed WebTUI top-controls change, and Issue 837
is the closed website revamp issue.

The current Homebrew cask is `homebrew/Casks/termsurf.rb` and currently points
at TermSurf `1.0.0`. The existing release pipeline is:

1. Build all release components with `scripts/build.sh all --release`.
2. Package and publish with `scripts/release.sh <version>`.
3. The release script uploads the tarball to GitHub Releases and updates the
   `termsurf/homebrew-termsurf` cask.

The WebTUI top-controls fix from Issue 836 is in the repo and should be included
in the next Homebrew deployment.

Surfari support is present in the repo and Ghostboard can resolve Surfari via
`TERMSURF_SURFARI_PATH`, but Surfari is not currently wired into the main
release packaging path:

- `scripts/build.sh all` does not build Surfari.
- `scripts/install.sh all` does not install Surfari.
- `scripts/release.sh` does not package Surfari.
- `homebrew/Casks/termsurf.rb` does not install Surfari.
- Ghostboard does not currently resolve an installed Surfari path analogous to
  the installed Roamium path.

This issue should decide whether the next Homebrew version is a
WebTUI/Ghostboard release only, or whether Surfari packaging must be completed
first in a separate issue before deployment.

## Scope

This issue covers the next Homebrew deployment. It may include:

- choosing the next version number;
- confirming the release contents;
- verifying release builds;
- running the release script;
- verifying the generated GitHub release artifact;
- verifying the Homebrew cask update;
- installing the cask from a clean or existing Homebrew install;
- documenting any deployment blockers.

If Surfari must ship in this Homebrew version, open or complete a packaging
issue first rather than silently omitting required Surfari installation work.

## Acceptance Criteria

- The issue records the intended next version number.
- The issue records whether Surfari is included in this deployment.
- Release builds complete successfully.
- The packaged tarball contains the expected installable artifacts.
- GitHub Release upload succeeds for the chosen version.
- The Homebrew cask is updated and pushed to the Homebrew tap.
- A Homebrew install or upgrade test succeeds.
- Installed TermSurf includes the WebTUI top-controls fix.
- If Surfari is included, installed Surfari can be launched through
  `web --browser surfari` without relying on a development-only
  `TERMSURF_SURFARI_PATH`.
