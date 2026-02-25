# Chromium Fork

TermSurf's Chromium fork. The full source tree (`src/`) and build tools
(`depot_tools/`) are gitignored — only this README and the `patches/` directory
are tracked.

For build instructions and branch tracking, see [docs/chromium.md](../docs/chromium.md).

## Patches

`patches/` contains `git format-patch` output for every TermSurf branch. Each
subdirectory holds the complete patch set needed to reconstruct that branch from
the vanilla `146.0.7650.0` tag.

```
patches/
├── termsurf/          — Base TermSurf modifications (5 patches)
├── issue-411/         — Two profiles experiment 3
├── issue-412/         — One profile
├── ...
└── issue-639/         — Open new-tab links in same tab
```

Each patch set is cumulative — it contains all commits from the base tag to the
branch tip, including inherited commits from parent branches.

### Applying patches

To reconstruct a branch from a fresh Chromium checkout:

```bash
cd chromium/src
git checkout 146.0.7650.0
git checkout -b 146.0.7650.0-issue-{N}
git am ../../chromium/patches/issue-{N}/*.patch
```

### Generating patches

After committing to a Chromium branch, regenerate its patch set:

```bash
cd chromium/src
rm -rf ../../chromium/patches/issue-{N}/
git format-patch 146.0.7650.0..HEAD -o ../../chromium/patches/issue-{N}/
```

Then commit the updated patches in the main repo.

## Directory Layout

| Path               | Tracked | Description                             |
| ------------------ | ------- | --------------------------------------- |
| `README.md`        | Yes     | This file                               |
| `patches/`         | Yes     | Patch archive for all TermSurf branches |
| `src/`             | No      | Chromium source tree (~100 GB)          |
| `depot_tools/`     | No      | Chromium build tools (647 MB)           |
| `.gclient`         | No      | gclient configuration                   |
| `.gclient_entries` | No      | gclient dependency map                  |
| `_bad_scm/`        | No      | Quarantined gclient artifacts           |
| `.cipd/`           | No      | CIPD package cache                      |
