# Experiment 1: Import Ghostty 1.3.1 Subtree

## Description

Import upstream Ghostty `v1.3.1` into a new `ghostboard/` directory using
`git subtree add`, preserving upstream Ghostty history in the TermSurf
repository. This experiment creates the clean upstream base for the new
Ghostboard and deliberately does not perform branding, configuration-path, icon,
CLI, build, or TermSurf protocol changes.

The previous history-preserving Ghostty imports used `git subtree add`; Issue
418 records why `git merge -X subtree` is not reliable for this repo's Ghostty
history.

## Changes

1. Verify the working tree is clean.
2. Verify no active `ghostboard/` directory exists before the import.
3. Configure a distinct Ghostty remote if needed:

   ```bash
   if git remote get-url ghostty >/dev/null 2>&1; then
     test "$(git remote get-url ghostty)" = "https://github.com/ghostty-org/ghostty.git"
   else
     git remote add ghostty https://github.com/ghostty-org/ghostty.git
   fi
   ```

4. Fetch the exact upstream release tag:

   ```bash
   git fetch ghostty --tags
   git ls-remote --tags ghostty refs/tags/v1.3.1 'refs/tags/v1.3.1^{}'
   git rev-parse v1.3.1
   git rev-parse 'v1.3.1^{commit}'
   ```

   Expected tag object:

   ```text
   22efb0be2bbea73e5339f5426fa3b20edabcaa11
   ```

   Expected peeled commit:

   ```text
   332b2aefc6e72d363aa93ab6ecfc86eeeeb5ed28
   ```

5. Import the release into `ghostboard/`:

   ```bash
   git subtree add --prefix=ghostboard ghostty v1.3.1 \
     -m "Import Ghostty v1.3.1 into ghostboard"
   ```

6. Record the import result in this experiment file.

7. Verify the subtree import commit preserves upstream history by checking that
   the import commit is a merge commit and its second parent is the peeled
   Ghostty `v1.3.1` commit:

   ```bash
   import_commit="$(git log -1 --format=%H --grep='Import Ghostty v1.3.1 into ghostboard')"
   test "$(git rev-list --parents -n 1 "$import_commit" | wc -w | tr -d ' ')" = "3"
   test "$(git rev-parse "$import_commit^2")" = "$(git rev-parse 'v1.3.1^{commit}')"
   ```

Do not edit files under `ghostboard/` after the subtree import in this
experiment. The imported tree should remain a clean upstream Ghostty `v1.3.1`
base.

## Verification

Pass criteria:

- `git status --short` is clean before the import begins.
- `ghostboard/` does not exist before the import.
- The `ghostty` remote exists and points to
  `https://github.com/ghostty-org/ghostty.git`.
- `git ls-remote --tags ghostty refs/tags/v1.3.1 'refs/tags/v1.3.1^{}'` reports
  the expected upstream tag object `22efb0be2bbea73e5339f5426fa3b20edabcaa11`.
- `git ls-remote --tags ghostty refs/tags/v1.3.1 'refs/tags/v1.3.1^{}'` reports
  the expected peeled release commit `332b2aefc6e72d363aa93ab6ecfc86eeeeb5ed28`.
- Local `v1.3.1` resolves to the expected tag object
  `22efb0be2bbea73e5339f5426fa3b20edabcaa11`.
- Local `v1.3.1^{commit}` resolves to the expected peeled commit
  `332b2aefc6e72d363aa93ab6ecfc86eeeeb5ed28`.
- `git subtree add --prefix=ghostboard ghostty v1.3.1` succeeds.
- `ghostboard/` exists after the import.
- Representative upstream files exist under the new prefix:
  - `ghostboard/build.zig`;
  - `ghostboard/build.zig.zon`;
  - `ghostboard/src/Surface.zig`;
  - `ghostboard/macos/`;
  - `ghostboard/README.md`.
- A representative content spot check confirms imported files match `v1.3.1`,
  for example:

  ```bash
  cmp <(git show v1.3.1:build.zig) ghostboard/build.zig
  cmp <(git show v1.3.1:src/Surface.zig) ghostboard/src/Surface.zig
  cmp <(git show v1.3.1:README.md) ghostboard/README.md
  ```

- Git history contains a subtree import merge commit for `ghostboard/`, and the
  import commit's second parent equals `v1.3.1^{commit}`:

  ```bash
  import_commit="$(git log -1 --format=%H --grep='Import Ghostty v1.3.1 into ghostboard')"
  test "$(git rev-list --parents -n 1 "$import_commit" | wc -w | tr -d ' ')" = "3"
  test "$(git rev-parse "$import_commit^2")" = "$(git rev-parse 'v1.3.1^{commit}')"
  ```

- The issue README lists this experiment as `Pass` only after the import result
  is recorded.

Fail criteria:

- The import uses `main`, `HEAD`, or any floating ref instead of `v1.3.1`.
- The import uses `git merge -X subtree`.
- Any branding, config, CLI, icon, protocol, build-system, `webtui`, or
  `roamium` changes are included in this experiment.
- Any file under `ghostboard/` is edited after the subtree import.
- The imported content does not match upstream Ghostty `v1.3.1` for
  representative spot checks.
- The import commit is not a merge commit whose second parent is the peeled
  Ghostty `v1.3.1` commit.

## Design Review

Fresh-context adversarial review returned `CHANGES REQUIRED`.

- Required: history preservation was not concretely verified. The design only
  required a subtree import commit, which would not prove the import commit's
  second parent is the Ghostty `v1.3.1` release commit. Fixed by adding explicit
  merge-parent verification against `v1.3.1^{commit}`.
- Optional: tag verification could be tied directly to the Ghostty remote. Fixed
  by adding
  `git ls-remote --tags ghostty refs/tags/v1.3.1 'refs/tags/v1.3.1^{}'` and
  checking both the annotated tag object and the peeled release commit.

Fresh-context adversarial re-review returned `APPROVED`.

- The reviewer confirmed the design now verifies the subtree import commit is a
  merge commit and that its second parent equals `v1.3.1^{commit}`.
- The reviewer confirmed the design now checks the remote annotated tag object
  `22efb0be2bbea73e5339f5426fa3b20edabcaa11` and peeled commit
  `332b2aefc6e72d363aa93ab6ecfc86eeeeb5ed28`.
- No new required findings were reported.
