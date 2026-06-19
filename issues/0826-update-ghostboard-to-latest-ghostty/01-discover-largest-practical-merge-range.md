# Experiment 1: Discover the Largest Practical Merge Range

## Description

Determine the largest practical upstream Ghostty commit range that Ghostboard
can merge first while preserving history and keeping the work reviewable.

This experiment is intentionally a dry-run discovery gate. It should not make
permanent `ghostboard/` source changes. It should use disposable branches or
worktrees to try the full upstream range first, then scale back by commit range
only if the full range produces an unmanageable conflict set.

The expected output is a documented recommendation for the first real upstream
merge experiment: either the full `v1.3.1` to latest Ghostty range, or a smaller
range justified by conflict evidence.

## Changes

- `issues/0826-update-ghostboard-to-latest-ghostty/README.md`
  - Link this experiment with status `Designed`.
- `issues/0826-update-ghostboard-to-latest-ghostty/01-discover-largest-practical-merge-range.md`
  - Define the dry-run range discovery plan and verification criteria.

No production code, build files, vendored source, `ghostboard/`, `webtui/`, or
`roamium/` files should be changed by this experiment plan.

## Verification

The implementation pass for this experiment should:

1. Confirm the working tree is clean before dry-run work begins.
2. Confirm the current Ghostboard subtree base:
   `332b2aefc6e72d363aa93ab6ecfc86eeeeb5ed28`.
3. Fetch or verify the latest Ghostty `origin/main` in `vendor/ghostty`, record
   the exact target commit, and note any fetch caveats.
4. Confirm a clean upstream Ghostty checkout at the target commit can at least
   report its build metadata and dependency expectations. If a full upstream
   build is practical within the experiment, run it; otherwise record why the
   build is deferred to a later gate.
5. Create a disposable branch or worktree for dry-run merge attempts.
6. Attempt the full upstream update range first using the same
   history-preserving mechanism intended for the real update:

```bash
git subtree pull --prefix=ghostboard ghostty <target-commit> \
  -m "Merge upstream Ghostty into ghostboard"
```

The dry run must not use `git merge -X subtree`, copy-over file replacement, or
any other non-history-preserving update mechanism.

7. If the full range is not practical, retry with smaller ranges selected from
   the upstream commit list, starting near the midpoint and adjusting based on
   observed conflict difficulty.
8. For every attempted range, record:
   - start commit;
   - end commit;
   - commit count;
   - command used;
   - whether the command completed cleanly;
   - conflicted files;
   - conflict classification: mechanical, semantic, build-system-specific,
     TermSurf-specific, or unknown;
   - unresolved conflict count from `git diff --name-only --diff-filter=U`;
   - whether a bounded inspection indicates the conflicts are likely resolvable
     within one real merge experiment;
   - whether the range is recommended for the first real merge.
9. Clean up or abandon disposable dry-run state so no dry-run `ghostboard/`
   changes remain in the main working tree.
10. Append `## Result` and `## Conclusion` to this file.
11. Update the experiment status in the issue README to `Pass`, `Partial`, or
    `Fail`.
12. Run:

```bash
prettier --write --prose-wrap always --print-width 80 \
  issues/0826-update-ghostboard-to-latest-ghostty/README.md \
  issues/0826-update-ghostboard-to-latest-ghostty/01-discover-largest-practical-merge-range.md
git diff --check
git status --short
git status --short -- ghostboard
test -z "$(git diff --name-only --diff-filter=U)"
```

Range selection rules:

- Attempt the full `v1.3.1` to latest Ghostty range first.
- Treat a range as practical if the dry run either merges cleanly or leaves a
  conflict set that is small enough to inspect file-by-file and classify during
  this experiment.
- Treat a range as not practical for the first real merge if the conflict set is
  too broad to classify file-by-file during this experiment, includes many
  unrelated conflict categories at once, or leaves no credible path to a
  buildable tree in one follow-up implementation experiment.
- If the full range is not practical, try a midpoint range. If the midpoint is
  still not practical, halve again. If the midpoint is practical, expand toward
  the latest upstream commit until the next attempted range stops being
  practical or the full range is reached.
- Select the largest attempted practical range. Do not claim a smaller range is
  largest practical unless at least one larger attempted range is documented as
  not practical.

Pass criteria:

- The full-range dry run was attempted first, or the experiment explains why it
  could not be attempted.
- The experiment identifies the largest practical first merge range from
  observed conflict data.
- The main working tree has no dry-run source changes.
- `git status --short -- ghostboard` is empty after cleanup.
- No unmerged paths remain in the main working tree after cleanup.
- The next experiment has enough evidence to perform the selected real merge
  range with history preserved.

Fail criteria:

- Dry-run state contaminates the main working tree.
- The experiment cannot identify any actionable next merge range.
- The recorded data is too vague to distinguish an easy merge range from an
  unreviewable one.
- The dry run uses `git merge -X subtree`, copy-over replacement, or another
  non-history-preserving update mechanism.

## Design Review

An adversarial Codex subagent reviewed the initial design with fresh context.

**Verdict:** Changes required.

Required findings and fixes:

- The subtree update mechanism was underspecified. Fixed by naming
  `git subtree pull --prefix=ghostboard ghostty <target-commit>` as the command
  shape and explicitly banning `git merge -X subtree`, copy-over replacement,
  and other non-history-preserving mechanisms.
- The pass criteria did not objectively prove "largest practical." Fixed by
  adding range selection rules, conflict counts, conflict classification, and a
  requirement to document at least one larger failed range before selecting a
  smaller range.
- Cleanup checks were too broad. Fixed by adding
  `git status --short -- ghostboard` and an unmerged-path check.

The optional note that the experiment file was untracked will be addressed at
the plan commit gate by staging this file with the issue README.

Re-review after those fixes:

**Verdict:** Approved.

The reviewer found no remaining findings. It confirmed the command shape,
non-history-preserving mechanism ban, objective range selection rules, focused
cleanup checks, issue README link, experiment structure, scope, and verification
criteria.

## Result

**Result:** Pass

The full upstream range from Ghostty `v1.3.1` to latest fetched Ghostty
`origin/main` was attempted first in a disposable worktree, as required.

Baseline and target:

```text
TermSurf HEAD before dry run: ab61b94fab49c9613eeb8c586ee2fac0ef8e2723
Ghostboard subtree base:      332b2aefc6e72d363aa93ab6ecfc86eeeeb5ed28
Latest Ghostty target:        5d0a82ba337368f5632ffa6ce4d7c558fa2de9ff
Upstream commit count:        1159
```

Fetch caveat:

```text
git fetch ghostty main --tags --prune
```

updated `ghostty/main` to `5d0a82ba337368f5632ffa6ce4d7c558fa2de9ff`, but exited
nonzero because the moving upstream `tip` tag would clobber the local `tip` tag.
The branch target was updated and was usable for the dry run.

Upstream build metadata at the target commit:

```text
build.zig.zon version:             1.3.2-dev
build.zig.zon minimum_zig_version: 0.15.2
local zig version:                 0.15.2
local Xcode version:               26.6
local macOS version:               26.5.1
```

A full clean upstream Ghostty build was deferred to the later build gate. This
experiment's purpose was range discovery; the target build metadata and local
toolchain compatibility were enough to proceed with the merge dry run.

Dry-run worktree:

```text
/tmp/termsurf-issue826-dryrun-full
```

Dry-run command:

```bash
git subtree pull --prefix=ghostboard ghostty \
  5d0a82ba337368f5632ffa6ce4d7c558fa2de9ff \
  -m "Merge upstream Ghostty into ghostboard"
```

The command exited with conflicts. It used the required history-preserving
subtree mechanism. It did not use `git merge -X subtree`, copy-over replacement,
or another non-history-preserving mechanism.

Unresolved conflict count:

```text
17
```

Unmerged files:

```text
ghostboard/.agents/commands/gh-issue
ghostboard/.github/VOUCHED.td
ghostboard/.github/workflows/test.yml
ghostboard/.github/workflows/vouch-check-issue.yml
ghostboard/.github/workflows/vouch-check-pr.yml
ghostboard/.github/workflows/vouch-manage-by-discussion.yml
ghostboard/.github/workflows/vouch-manage-by-issue.yml
ghostboard/.github/workflows/vouch-sync-codeowners.yml
ghostboard/CONTRIBUTING.md
ghostboard/HACKING.md
ghostboard/README.md
ghostboard/build.zig
ghostboard/include/ghostty.h
ghostboard/macos/Sources/Features/Terminal/TerminalController.swift
ghostboard/macos/Sources/Ghostty/Surface View/SurfaceView_AppKit.swift
ghostboard/src/build/SharedDeps.zig
ghostboard/src/main_c.zig
```

Conflict classification:

| Files                                             | Classification                         | Notes                                                                                                   |
| ------------------------------------------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `.agents/commands/gh-issue`                       | TermSurf-specific / cleanup            | TermSurf modified a file upstream deleted.                                                              |
| `.github/VOUCHED.td`, `.github/workflows/vouch-*` | poisoned-file / upstream workflow      | TermSurf deleted poisoned/vouch files that upstream later changed.                                      |
| `.github/workflows/test.yml`                      | build/CI workflow                      | Single conflict between local workflow state and upstream path filter update.                           |
| `CONTRIBUTING.md`, `HACKING.md`, `README.md`      | documentation / poisoned-file cleanup  | Documentation conflicts are expected and should be resolved according to TermSurf documentation policy. |
| `build.zig`                                       | build-system-specific                  | Local emit/install behavior conflicts with upstream `emit_lib_vt` build changes.                        |
| `include/ghostty.h`                               | semantic TermSurf API + upstream C API | TermSurf protocol exports conflict with upstream `GHOSTTY_API` and config API additions.                |
| `TerminalController.swift`                        | semantic TermSurf lifecycle            | TermSurf pane cleanup conflicts with upstream pending-initial-presentation cleanup.                     |
| `SurfaceView_AppKit.swift`                        | semantic TermSurf UI/input             | TermSurf copy-current-URL feedback and published state conflict with upstream copy/action changes.      |
| `src/build/SharedDeps.zig`                        | build-system-specific                  | TermSurf protobuf C sources conflict with upstream MSVC sanitizer flags for `stb.c`.                    |
| `src/main_c.zig`                                  | semantic TermSurf API + upstream C API | TermSurf exported protocol functions conflict with upstream Windows `DllMain` addition.                 |

The full range is practical for the first real merge experiment. Although the
update touches hundreds of files, the unresolved conflict set is bounded and
inspectable file-by-file. The 17 conflicts are concentrated in predictable
areas: poisoned/vouch cleanup, docs, build glue, C API exports, and macOS
TermSurf lifecycle/UI integration. There is a credible path to a buildable tree
in one follow-up implementation experiment if the merge resolves these files
deliberately and keeps upstream behavior except where TermSurf-specific behavior
is required.

No smaller ranges were attempted because the full range met the experiment's
practicality rule. There is therefore no larger attempted range to document as
not practical; the full range is already the largest possible target for this
issue's current upstream head.

Cleanup verification after removing the disposable worktree and branch:

```text
git worktree remove --force /tmp/termsurf-issue826-dryrun-full
git branch -D issue826-dryrun-full
git status --short -- ghostboard
test -z "$(git diff --name-only --diff-filter=U)"
```

`git status --short -- ghostboard` produced no output, and the unmerged-path
check passed.

## Conclusion

Experiment 1 recommends using the full upstream Ghostty range as the first real
merge range:

```text
332b2aefc6e72d363aa93ab6ecfc86eeeeb5ed28..5d0a82ba337368f5632ffa6ce4d7c558fa2de9ff
```

The next experiment should perform the actual history-preserving subtree pull
for that full range, resolve the 17 known conflicts, preserve TermSurf-specific
behavior where required, and then move into build verification.

## Result Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

**Verdict:** Approved.

The reviewer found no required issues. It independently checked that:

- the diff from the plan commit only updated the issue README status and
  appended this experiment's result/conclusion;
- `git status --short -- ghostboard` produced no output;
- `git diff --name-only --diff-filter=U` produced no output;
- `ghostty/main` resolved to `5d0a82ba337368f5632ffa6ce4d7c558fa2de9ff`;
- the upstream range contained 1159 commits;
- the README recorded Experiment 1 as `Pass`;
- the result and conclusion were present;
- the result commit had not yet been made;
- `git diff --check` passed.

It agreed that the full range recommendation is supported by the 17-file
conflict set, file-by-file classification, clean main `ghostboard/` state after
cleanup, and the fact that the full range is the largest possible range for the
current upstream target.
