# Ghostty Fork

## Overview

This repo is a fork of [Ghostty](https://github.com/ghostty-org/ghostty). The
original Ghostty commit history is part of our git history — we forked, then
began modifying files in place. Later, all Ghostty files were moved into `ts1/`
to make room for other components (WezTerm forks, CEF experiments, docs, etc.).

There are now three copies of Ghostty in this repo:

| Directory | Generation     | Status     | Description                                                            |
| --------- | -------------- | ---------- | ---------------------------------------------------------------------- |
| `gui/`    | TermSurf GUI   | Active     | Ghostty fork with browser integration in Zig. Receives upstream merges.|
| `ts5/`    | TermSurf 5.0   | Superseded | Ghostty fork with browser integration in Swift. Superseded by gui/.    |
| `ts1/`    | TermSurf 1.x   | Frozen     | Ghostty + WKWebView browser panes. No longer receives upstream merges. |

**gui/ is active development.** All browser integration logic is in Zig, matching
Ghostty's architecture. gui/ receives upstream Ghostty merges.

**ts5 is superseded.** Its browser integration lived in Swift (CompositorXPC).
gui/ rewrites this in Zig. ts5 is kept for reference.

**ts1 is permanently frozen.** It contains TermSurf-specific modifications
(WKWebView integration, `web` CLI command, branding) that are specific to the
ts1 approach. It will not receive upstream Ghostty updates.

## Remote

| Remote     | URL                                        | Branch |
| ---------- | ------------------------------------------ | ------ |
| `upstream` | https://github.com/ghostty-org/ghostty.git | main   |

The `upstream` remote is shared across all Ghostty copies — they all came from
the same repo.

## How ts1 was created

The repo was forked from Ghostty. All files were then moved into `ts1/` with a
directory rename. Upstream merges used `git merge -X subtree=ts1 upstream/main`,
which tells git to map upstream's `/` to our `ts1/`. This worked because the
subtree merge strategy could follow the rename history.

ts1 is now frozen and will not be merged again.

## How ts5 was created

ts5 was imported with `git subtree add`:

```bash
git subtree add --prefix=ts5 upstream main
```

This could not use `git merge -X subtree=ts5` because git's rename detection
found the original `/ → ts1/` move and tried to merge upstream changes into
`ts1/` instead of `ts5/`. Three experiments were attempted before finding the
working approach (see Issue 418 Experiments 1–3 for details).

## How gui/ was created

gui/ was created the same way as ts5:

```bash
git subtree add --prefix=gui upstream main
```

It was originally named `ghost/` (after the working name "Ghost") and later
renamed to `gui/` in Issue 613.

## Merging upstream into gui/

To pull the latest upstream Ghostty changes into gui/:

```bash
git fetch upstream
git subtree pull --prefix=gui upstream main -m "Merge upstream Ghostty into gui"
```

### Resolving conflicts

gui/ has TermSurf modifications in several files (XPC integration, IOSurface
overlay, input forwarding). Upstream merges may conflict with these. Key files
likely to conflict:

- `gui/src/Surface.zig` — Browser state, input routing
- `gui/src/renderer/Metal.zig` — Overlay rendering
- `gui/macos/Sources/App/macOS/AppDelegate.swift` — Debug icon override

### After merging

Verify the build:

```bash
cd gui && zig build
```

If the build fails, common causes are:

- Zig version mismatch (check `gui/build.zig.zon` for the required version)
- New upstream dependencies or build system changes
