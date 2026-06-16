# Experiment 1: Restore the Archived Directory

## Description

Restore the archived `ghostboard/` directory to the working tree from the
documented recovery point. This experiment deliberately does not try to build,
run, modernize, rename, or integrate Ghostboard. The goal is only to make the
historical code available again.

## Changes

- Restore `ghostboard/` with:

  ```bash
  git checkout 90b966458bd17 -- ghostboard/
  ```

- Leave restored files mechanically as they were at that historical point.
- Do not edit restored Ghostboard source, scripts, project files, assets, or
  dependency metadata.
- Do not update active build/install/release scripts to include Ghostboard.
- Update this experiment file with the result after verification.

## Verification

Pass criteria:

- `ghostboard/` exists in the working tree.
- Representative expected files exist:
  - `ghostboard/build.zig`;
  - `ghostboard/src/Surface.zig`;
  - `ghostboard/src/apprt/embedded.zig`;
  - `ghostboard/macos/`;
  - `ghostboard/include/termsurf.h`.
- `git diff --stat -- ghostboard/` shows a restore of the archived directory and
  no unrelated product-code changes.
- Provenance spot checks compare representative restored files against
  `90b966458bd17`.
- Markdown documentation changes pass `git diff --check`; full
  `git diff --check` over `ghostboard/` is not a pass criterion because the
  historical tree contains existing whitespace warnings that must not be
  repaired during a mechanical restore.
- The issue README lists this experiment as `Pass` when complete.

Fail criteria:

- Any file outside `ghostboard/` and issue documentation is changed for product
  behavior.
- The restore source is not the documented recovery commit.
- The experiment attempts to build or fix Ghostboard.
- Representative restored files differ from `90b966458bd17` without an explicit
  reason.
- Restored historical whitespace is modified only to satisfy lint output.

## Design Review

Fresh-context adversarial review returned `CHANGES REQUIRED`.

- Required: the original plan used
  `git checkout 90b966458bd17~1 -- ghostboard/`, which would restore a tree one
  commit older than the documented Ghostboard state. Fixed by using
  `git checkout 90b966458bd17 -- ghostboard/`.
- Required: the issue README repeated the incorrect parent-commit explanation.
  Fixed by distinguishing the documented Ghostboard tree commit `90b966458bd17`
  from the later deletion/archive commit `2874f578f`.
- Required on re-review: two stale `90b966458bd17~1` provenance references
  remained. Fixed by changing both to `90b966458bd17`.

Fresh-context adversarial re-review returned `APPROVED`.

- The reviewer confirmed the restore command now uses the documented tree state.
- The reviewer confirmed `90b966458bd17:ghostboard` and `2874f578f~1:ghostboard`
  resolve to the same tree.
- No new required findings were reported.

## Result

**Result:** Pass

Restored `ghostboard/` with:

```bash
git checkout 90b966458bd17 -- ghostboard/
```

Verification completed:

- Confirmed representative files exist:
  - `ghostboard/build.zig`;
  - `ghostboard/src/Surface.zig`;
  - `ghostboard/src/apprt/embedded.zig`;
  - `ghostboard/macos/`;
  - `ghostboard/include/termsurf.h`.
- Confirmed the restored working-tree file count matches the source commit:
  - `find ghostboard -type f | wc -l` returned `1536`.
  - `git ls-tree -r --name-only 90b966458bd17 ghostboard | wc -l` returned
    `1536`.
- Confirmed the staged restore summary is limited to `ghostboard/`:
  - `git diff --cached --shortstat -- ghostboard/` returned
    `1536 files changed, 437576 insertions(+)`.
- Confirmed representative staged files match `90b966458bd17`:

  ```bash
  git diff --cached --quiet 90b966458bd17 -- \
    ghostboard/build.zig \
    ghostboard/src/Surface.zig \
    ghostboard/src/apprt/embedded.zig \
    ghostboard/include/termsurf.h
  ```

- Confirmed the full staged `ghostboard/` tree matches `90b966458bd17`:

  ```bash
  git diff --cached --quiet 90b966458bd17 -- ghostboard/
  ```

- Confirmed Markdown documentation changes pass `git diff --check`:

  ```bash
  git diff --check -- \
    issues/0807-restore-ghostboard-code/README.md \
    issues/0807-restore-ghostboard-code/01-restore-archived-directory.md
  ```

- Ran full `git diff --check` and found expected historical whitespace warnings
  inside restored `ghostboard/` files. Those warnings were not fixed because
  modifying them would break the mechanical restore requirement.

No build or run attempt was made, by design.

## Completion Review

Fresh-context adversarial completion review returned `APPROVED`.

- The reviewer confirmed staged `ghostboard/` matches `90b966458bd17`.
- The reviewer confirmed the restored and source file counts both equal `1536`.
- The reviewer confirmed representative staged files match the source commit.
- The reviewer confirmed staged changes are limited to `ghostboard/` plus this
  issue's documentation.
- The reviewer initially confirmed `git diff --check` passes. A later local
  re-check found this was too broad: full `git diff --check` reports historical
  whitespace warnings in the restored `ghostboard/` tree. The experiment treats
  documentation-only `git diff --check` as the relevant formatting gate and
  preserves the historical tree unchanged.
- No build, run, modernization, or fix attempt was found.

Fresh-context adversarial completion re-review returned `APPROVED`.

- The reviewer confirmed the correction is acceptable because fixing whitespace
  inside `ghostboard/` would violate the stronger requirement that the staged
  tree match `90b966458bd17`.
- The reviewer confirmed documentation-only `git diff --check` passes.
- The reviewer confirmed full staged `ghostboard/` still matches
  `90b966458bd17`.
- The reviewer confirmed staged changes remain limited to `ghostboard/` plus the
  two Issue 807 documentation files.

## Conclusion

The archived Ghostboard code is restored mechanically from the documented
archive point, and the restore matches the expected historical tree in both file
count and representative file content. The next issue can decide whether to
build, modernize, or integrate Ghostboard; this issue only restores the source.
