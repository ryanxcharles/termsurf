+++
status = "open"
opened = "2026-06-16"
+++

# Issue 807: Restore Ghostboard Code

## Goal

Restore the archived `ghostboard/` directory to the working tree so its code is
available for inspection and future work.

This issue is only about restoring the code. It is not about making Ghostboard
build, run, pass tests, integrate with current TermSurf components, or match
current Ghostty behavior.

## Background

Ghostboard was archived in Issue 742 to reduce maintenance burden while Wezboard
carried active protocol iteration. The archive documentation records the
recovery point in `docs/early-prototypes.md`:

```text
ghostboard/ — 90b966458bd17 — 2026-03-11
```

The Issue 742 archive plan used a generic recovery form:

```bash
git checkout <commit>~1 -- ghostboard/
```

That form applies when `<commit>` is the deletion/archive commit. In this case,
the documented archive log records the Ghostboard tree state itself at
`90b966458bd17`, and the later deletion/archive commit is `2874f578f`. Therefore
the intended restore source is:

```bash
git checkout 90b966458bd17 -- ghostboard/
```

Equivalently, this is the parent of the deletion commit:

```bash
git checkout 2874f578f~1 -- ghostboard/
```

The direct `90b966458bd17` form is clearer because it matches the archive log.

## Scope

In scope:

- Restore `ghostboard/` from the documented archive point.
- Preserve the restored files as historical source code.
- Keep the issue documentation clear that this is a code restore only.
- Run lightweight verification that confirms the directory exists and came from
  the intended historical commit.

Out of scope:

- Building Ghostboard.
- Running Ghostboard.
- Updating Ghostboard dependencies.
- Fixing compile errors.
- Reintegrating Ghostboard into scripts, install paths, releases, or docs as an
  active GUI.
- Renaming or modernizing the restored code.
- Changing Roastty, Wezboard, Roamium, Chromium, WebTUI, or shared protocol code
  unless a future experiment explicitly requires a narrow documentation-only
  reference.

## Analysis

The safest restore is mechanical: check out only `ghostboard/` from the
documented recovery point and avoid any follow-on edits that could blur the
historical state. Because the user explicitly does not want to make Ghostboard
work yet, verification should avoid build attempts and focus on provenance:

- `ghostboard/` exists in the working tree.
- representative expected files exist, such as `ghostboard/build.zig`,
  `ghostboard/src/Surface.zig`, and `ghostboard/macos/`.
- `git diff --stat` shows a directory restore rather than unrelated changes.
- optional spot checks compare restored files with `90b966458bd17`.

Future issues can decide whether to build, rebase, rename, or integrate
Ghostboard. This issue should stop once the archived code is restored and
committed.

## Experiments

- [Experiment 1: Restore the archived directory](01-restore-archived-directory.md)
  — **Pass**
