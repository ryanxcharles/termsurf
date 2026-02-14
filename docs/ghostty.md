# Ghostty Fork

## Overview

This repo is a fork of [Ghostty](https://github.com/ghostty-org/ghostty). The
original Ghostty commit history is part of our git history — we forked, then
began modifying files in place. Later, all Ghostty files were moved into `ts1/`
to make room for other components (WezTerm forks, CEF experiments, docs, etc.).

There are now two copies of Ghostty in this repo:

| Directory | Generation   | Status | Description                                                              |
| --------- | ------------ | ------ | ------------------------------------------------------------------------ |
| `ts1/`    | TermSurf 1.x | Frozen | Ghostty + WKWebView browser panes. No longer receives upstream merges.   |
| `ts5/`    | TermSurf 5.0 | Active | Clean upstream Ghostty. Will receive Chromium Content API browser panes. |

**ts1 is permanently frozen.** It contains TermSurf-specific modifications
(WKWebView integration, `web` CLI command, branding) that are specific to the
ts1 approach. It will not receive upstream Ghostty updates.

**ts5 is active development.** It starts as unmodified upstream Ghostty. Browser
pane integration (in-process Chromium via the Content API) will be added
incrementally. ts5 receives upstream Ghostty merges.

## Remote

| Remote     | URL                                        | Branch |
| ---------- | ------------------------------------------ | ------ |
| `upstream` | https://github.com/ghostty-org/ghostty.git | main   |

The `upstream` remote is shared between ts1 and ts5 — both came from the same
Ghostty repo.

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

## Merging upstream into ts5

To pull the latest upstream Ghostty changes into ts5:

```bash
git fetch upstream
git subtree pull --prefix=ts5 upstream main -m "Merge upstream Ghostty into ts5"
```

This uses `git subtree pull`, which finds changes since the last subtree
operation and applies them under `ts5/`. It does not use the three-way merge
against the original fork point, so the `/ → ts1/` rename history is irrelevant.

### Resolving conflicts

ts5 currently has no TermSurf modifications, so upstream merges should be
conflict-free. As we add browser pane support, conflicts will arise in modified
files. Document those files and resolution strategies here as they develop.

### After merging

Verify the build:

```bash
cd ts5 && zig build
```

If the build fails, common causes are:

- Zig version mismatch (check `ts5/build.zig.zon` for the required version)
- New upstream dependencies or build system changes
