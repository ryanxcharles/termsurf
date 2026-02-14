---
name: build-chromium
description: "Build the Chromium fork for TermSurf. Use when building, rebuilding, or troubleshooting the Chromium build."
---

# Build Chromium

Build the Chromium fork for TermSurf. Everything stays inside the repo.

## Paths

| What | Path |
|------|------|
| depot_tools | `ts4/termsurf-chromium/depot_tools` |
| Source root | `ts4/termsurf-chromium/src` |
| Build output | `ts4/termsurf-chromium/src/out/Default/` |
| Built app | `ts4/termsurf-chromium/src/out/Default/One Profile.app/` |
| GN args | `ts4/termsurf-chromium/src/out/Default/args.gn` |

All paths are relative to `~/dev/termsurf`.

## PATH Setup

depot_tools must be on PATH before running any build tool (`autoninja`, `gn`,
`gclient`, etc.):

```bash
export PATH="$HOME/dev/termsurf/ts4/termsurf-chromium/depot_tools:$PATH"
```

## Build Commands

```bash
# Navigate to source root
cd ~/dev/termsurf/ts4/termsurf-chromium/src

# Add depot_tools to PATH
export PATH="$HOME/dev/termsurf/ts4/termsurf-chromium/depot_tools:$PATH"

# Generate build files (only needed once, or after changing args.gn)
gn gen out/Default

# Build
autoninja -C out/Default one_profile
```

The current build target is `one_profile`. This builds the `One Profile.app`
bundle in `out/Default/`.

## GN Args

Current `out/Default/args.gn`:

```
is_debug = false
symbol_level = 0
is_component_build = true
```

Regenerate with `gn gen out/Default` after editing `args.gn`.

## Cache Preservation

The `out/Default/` directory IS the build cache. Never delete it. `autoninja`
does incremental builds automatically — only recompiling changed files. A full
build is ~42,000 steps; an incremental build after small changes is typically
50-200 steps.

## The Rule

All binaries, `.app` bundles, and build artifacts stay inside `~/dev/termsurf`.
Nothing gets installed to `/usr/local`, `/usr/bin`, `~/Library`, or anywhere
outside the repo without explicit user approval.

## Submodule Branches

The Chromium fork at `ts4/termsurf-chromium/src` uses branches named
`{version}-termsurf` or `{version}-issue-{N}` (e.g.,
`146.0.7650.0-issue-414`). These are built as commits on top of the vanilla
Chromium version tag.
