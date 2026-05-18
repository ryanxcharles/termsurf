+++
status = "open"
opened = "2026-05-18"
+++

# Issue 781: Chromium Upgrade Path

## Goal

Update Roamium to use the latest viable Chromium version while preserving the
ability to build and run Chromium, Roamium, and Wezboard together.

The issue is successful when Roamium builds and works against the latest
Chromium target selected by this issue, with a clear record of any upstream
versions skipped or rejected.

## Background

TermSurf currently tracks Chromium `146.0.7650.0`. The documented active branch
is `146.0.7650.0-issue-762`, based on `146.0.7650.0`.

The Chromium fork is expensive to rebuild, and TermSurf carries custom browser
embedding changes for Roamium. A blind jump to the newest Chromium tag could
waste time if several upstream changes break the build or invalidate the current
embedding API. At the same time, upgrading through every obsolete intermediate
version would also waste time.

As of 2026-05-18, Electron's release feed shows these relevant Chromium
versions:

- Stable Electron 42.1.0: Chromium `148.0.7778.97`
- Prerelease/nightly Electron 43/44: Chromium `150.0.7834.0`

TermSurf should use these as initial landmarks, then verify the actual Chromium
tags available in `chromium/src` before doing any migration work.

## Plan

Start by proving the current tree still builds. The baseline must include:

- `scripts/build.sh chromium`
- `scripts/build.sh roamium`
- `scripts/build.sh wezboard`

After the baseline is known, inspect the Chromium versions between
`146.0.7650.0` and the latest available target. For each serious candidate,
assess:

- whether the tag exists and syncs cleanly
- whether TermSurf's patch set applies cleanly
- whether Chromium's build files, Content API, embedding hooks, or macOS
  framework layout changed in relevant ways
- whether Roamium's FFI boundary still matches the Chromium library
- whether Wezboard still launches and talks to Roamium over the TermSurf socket
  protocol

Prefer the fewest migration steps that still preserve debuggability. If the
latest Chromium target is close enough to the current base, attempt a direct
upgrade. If the jump is too large, use major-version checkpoints rather than
every patch release.

## Candidate Upgrade Path

The likely path is:

1. Verify current `146.0.7650.0` baseline.
2. Assess a direct move to the latest stable Electron Chromium, currently
   `148.0.7778.97`.
3. Assess whether moving beyond stable to the latest prerelease/nightly Chromium
   is worth the extra risk.
4. If direct migration fails, bisect by Chromium major version or Electron major
   version until the first breaking point is isolated.
5. Land the newest working Chromium version with patches, documentation, and
   build instructions updated.

## Constraints

Every Chromium source change must happen on a new Chromium branch for this
issue. The branch should be named `{version}-issue-781`, based on the selected
Chromium target, and recorded in `chromium/README.md`.

Use `autoninja` for Chromium builds. Do not run `ninja` directly.

Do not delete or clean Chromium build outputs unless the issue explicitly
requires it. The build cache is valuable.

## Open Questions

- Should TermSurf track Electron stable Chromium, Chromium stable, or Chromium
  tip-of-tree for this upgrade?
- Is Roamium's current C ABI small enough to forward-port directly, or should
  the Chromium patch set be reorganized first?
- Does the current packaging layout still match newer Chromium app bundle and
  framework output?

## Experiments

### Experiment 1: Map Chromium releases since 146

#### Description

Before changing code or creating a Chromium branch, map the Chromium versions
released since TermSurf last updated Roamium's Chromium base. This experiment is
read-only discovery work. It should establish the available version landscape
and recommend the smallest sensible set of migration targets.

The current TermSurf base is `146.0.7650.0`. The mapping should start there and
cover every newer Chromium milestone relevant to this issue.

#### Changes

No code changes.

Collect version data from:

- Chromium release channels and milestone data
- Electron stable, beta, alpha, and nightly release metadata
- Chromium tags available to `chromium/src`
- TermSurf's current `chromium/README.md` branch and patch documentation

Record the findings in this issue as a table with:

- Chromium milestone
- representative Chromium version or tag
- release channel/source
- Electron version, if applicable
- whether the tag exists and is fetchable
- notes about why it is or is not a useful upgrade checkpoint

#### Verification

This experiment passes when the issue contains:

1. A table of Chromium versions newer than `146.0.7650.0`.
2. A shortlist of candidate upgrade targets.
3. A recommendation for Experiment 2.

Experiment 1 should not modify Chromium source, create a Chromium branch, or run
large builds. Its job is to choose where the migration should aim before we
spend build time.
