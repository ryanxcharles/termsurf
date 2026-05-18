---
name: merge-upstream
description: "Merge upstream changes from Ghostty into TermSurf"
---

# Merge Upstream

Merge the latest upstream Ghostty into `ts5/`. See `docs/ghostty.md` for
background on the fork structure.

## Usage

```
/merge-upstream
```

## Steps

1. **Pre-merge checklist**
   - Ensure working tree is clean (`git status`)
   - All changes committed
   - Note current HEAD: `git rev-parse HEAD`

2. **Fetch and review upstream changes**
   ```bash
   git fetch upstream
   git log --oneline upstream/main ^$(git log --all --grep="git-subtree-dir: ts5" --format=%H | head -1) | head -30
   ```
   Review the list of new upstream commits. If there are breaking changes or
   major refactors, note them for conflict resolution.

3. **Merge upstream**
   ```bash
   git subtree pull --prefix=ts5 upstream main -m "Merge upstream Ghostty into ts5"
   ```
   This uses `git subtree pull` (not `git merge -X subtree`). The subtree merge
   strategy does not work for ts5 because git's rename detection finds the
   original `/ → ts1/` move and misroutes changes. See Issue 418 Experiments
   1–3.

4. **Resolve conflicts** — If there are conflicts, resolve them. Check
   `docs/ghostty.md` for a list of TermSurf-modified files and resolution
   strategies. ts5 currently has no TermSurf modifications, so conflicts are not
   expected until we begin adding browser pane support.

5. **Verify build**
   ```bash
   cd ts5 && zig build
   ```
   If the build fails, common causes:
   - Zig version mismatch (check `ts5/build.zig.zon`)
   - New upstream dependencies or build system changes

6. **Commit** any additional fixes needed after the merge.

## Important

- **Never merge into ts1.** ts1 is permanently frozen.
- **Always use `git subtree pull`**, not `git merge -X subtree=ts5`. The latter
  fails due to rename detection history (Issue 418).
- The `upstream` remote points to `https://github.com/ghostty-org/ghostty.git`.
