# Experiment 4: Establish WebKit branch workflow

## Description

Experiments 1-3 proved that this VM can build WebKit and that the macOS
compositor path is viable. Before introducing `libtermsurf_webkit` or any WebKit
source patches, TermSurf needs a WebKit branch and patch-management workflow
analogous to Chromium's.

Chromium has a documented local source workspace, issue-specific branches,
branch table, patch archive layout, and commands for generating/applying patch
sets. WebKit needs the same kind of traceability, adapted for its current
shallow checkout and upstream commit-based workflow.

This experiment should establish the workflow only. It should not modify WebKit
source code, create `libtermsurf_webkit`, create Surfari, modify Ghostboard, or
modify the protocol.

## Changes

- Update `webkit/README.md` with:
  - repository/remotes;
  - current local state;
  - branch naming convention;
  - branch table with the current Issue 756 branch;
  - patch archive layout;
  - commands to create an issue branch;
  - commands to generate patches after WebKit commits exist;
  - commands to apply patches from a fresh checkout;
  - rules for when to deepen the shallow checkout.
- Create tracked `webkit/patches/` documentation/placeholder files so future
  WebKit patch sets have a durable home in the TermSurf repo.
- Create a local WebKit issue branch in `webkit/src` from the current verified
  upstream commit `1452a43959523449099b2616793fd2c5b6a6487e`.
- Do not commit anything inside `webkit/src` in this experiment.
- Do not generate non-empty WebKit patches in this experiment because no WebKit
  source change is being made.

## Verification

Start from a clean TermSurf repo root:

```bash
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

Then create or verify the local Issue 756 branch:

```bash
git -C webkit/src switch -C webkit-1452a439-issue-756 1452a43959523449099b2616793fd2c5b6a6487e
```

Verify the branch and patch workflow docs:

```bash
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src status --short
find webkit/patches -maxdepth 3 -type f | sort
git diff --check
```

**Pass** = `webkit/README.md` documents the WebKit branch and patch workflow,
`webkit/patches/` has tracked documentation/placeholder files, `webkit/src` is
on `webkit-1452a439-issue-756` at commit
`1452a43959523449099b2616793fd2c5b6a6487e`, `webkit/src` is clean, no WebKit
source patches were created, and the TermSurf worktree contains only the
intended documentation/placeholder changes.

**Partial** = the documentation is written but the local branch cannot be
created or verified because of shallow-checkout, detached-head, or local WebKit
workspace state. The result must record the exact blocker and the next
experiment needed.

**Fail** = the WebKit checkout is missing/unusable, or the workflow cannot be
specified clearly enough to guide future WebKit patches.

Before recording the result, capture:

```bash
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

**Verdict:** Approved.

Findings: none.

## Result

**Result:** Pass

`webkit/README.md` now documents the local WebKit workspace, upstream remote,
current verified base commit, branch naming convention, branch table, patch
archive layout, patch generation commands, patch application commands, and rules
for deepening the shallow checkout.

`webkit/patches/README.md` now exists as the tracked placeholder and operating
notes for future WebKit patch archives. No non-empty WebKit patches were created
because this experiment did not modify WebKit source.

`.gitignore` now keeps `webkit/src` and WebKit build output ignored while
allowing `webkit/README.md` and `webkit/patches/` to be tracked by the main
TermSurf repo.

The local WebKit issue branch was created from the verified upstream commit:

```bash
git -C webkit/src switch -c webkit-1452a439-issue-756 1452a43959523449099b2616793fd2c5b6a6487e
```

Output:

```text
Switched to a new branch 'webkit-1452a439-issue-756'
1452a43959523449099b2616793fd2c5b6a6487e
webkit-1452a439-issue-756
true
```

Final verification:

```bash
git status --short
```

```text
 M .gitignore
 M webkit/README.md
?? webkit/patches/
```

```bash
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

```text
1452a43959523449099b2616793fd2c5b6a6487e
webkit-1452a439-issue-756
true
```

The empty first line from `git -C webkit/src status --short` confirms the
ignored WebKit checkout is clean.

```bash
find webkit/patches -maxdepth 3 -type f | sort
```

```text
webkit/patches/README.md
```

```bash
git diff --check
```

```text

```

`git diff --check` produced no output.

## Conclusion

TermSurf now has a documented WebKit branch and patch workflow analogous to the
Chromium workflow, adapted for a shallow WebKit checkout and patch archives
tracked outside the ignored `webkit/src` tree. The next experiment can start
introducing the first `libtermsurf_webkit` surface without also inventing source
control mechanics.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

**Initial verdict:** Changes required.

Required finding:

- `webkit/README.md` documented the WebKit remote as `upstream`, but the actual
  `webkit/src` checkout uses `origin`, and the documented apply workflow also
  uses `origin`.

Fix:

- Updated `webkit/README.md` to document the remote as `origin`.

**Re-review verdict:** Approved.

The reviewer confirmed that `webkit/README.md` now documents `origin`, matching
`git -C webkit/src remote -v`, and that no new required findings were introduced
by the fix.
