---
name: research-open-source
description: "Use the local copies of each repo when doing research. Use this when doing research on any open source repo."
---

# Research Open Source

When researching open source code, use the local clones in `vendor/` and
`chromium/src/` instead of searching the internet. The full source code is
already on disk — there is no reason to fetch it remotely.

## Check what's available

Before starting any research, check what repos are already cloned:

```bash
ls ~/dev/termsurf/vendor/
```

Also check `docs/vendor.md` for the full list including Chromium.

### Currently available repos

| Repo | Path |
|------|------|
| Ghostty | `vendor/ghostty/` |
| WezTerm | `vendor/wezterm/` |
| Electron | `vendor/electron/` |
| Alacritty | `vendor/alacritty/` |
| Chromium | `chromium/src/` |

## Research workflow

1. **Identify which repo has the code you need.** Check the vendor directory and
   `docs/vendor.md`.
2. **Read the source directly.** Use Grep, Glob, and Read tools on the local
   clone. No web fetching needed.
3. **If the repo is not cloned yet**, ask the user before cloning it. Do not
   clone repos without confirmation.

## Cloning a new repo

If research requires a repo that is not yet in `vendor/`:

1. **Ask the user** if it is OK to clone the repo into `vendor/`.
2. Clone it: `git clone <url> vendor/<name>/`
3. Add the directory to `vendor/.gitignore`.
4. Update `docs/vendor.md` with the new repo, its URL, and why it was cloned.

Never clone without user approval. Never skip updating the gitignore or the
vendor doc.

## Chromium is a special case

Chromium lives at `chromium/src/`, not in `vendor/`. The repo is too large to
have two copies. When researching Chromium internals, read from `chromium/src/`
directly. Do not attempt to clone Chromium into `vendor/`.
