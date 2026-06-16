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

## Result

**Result:** Pass

Imported Ghostty `v1.3.1` into `ghostboard/` with:

```bash
git subtree add --prefix=ghostboard ghostty v1.3.1 \
  -m "Import Ghostty v1.3.1 into ghostboard"
```

The import created merge commit:

```text
493817fd94ee3bc6bdefb24274132e7862378226 Import Ghostty v1.3.1 into ghostboard
```

Verification completed:

- Confirmed the working tree was clean before import.
- Confirmed `ghostboard/` did not exist before import.
- Confirmed the `ghostty` remote points to
  `https://github.com/ghostty-org/ghostty.git`.
- Confirmed the remote `v1.3.1` annotated tag object:

  ```text
  22efb0be2bbea73e5339f5426fa3b20edabcaa11 refs/tags/v1.3.1
  ```

- Confirmed the remote `v1.3.1` peeled commit:

  ```text
  332b2aefc6e72d363aa93ab6ecfc86eeeeb5ed28 refs/tags/v1.3.1^{}
  ```

- Confirmed the local `v1.3.1` tag resolves to
  `22efb0be2bbea73e5339f5426fa3b20edabcaa11`.
- Confirmed local `v1.3.1^{commit}` resolves to
  `332b2aefc6e72d363aa93ab6ecfc86eeeeb5ed28`.
- Confirmed `ghostboard/` exists after import.
- Confirmed representative upstream files exist under the new prefix:
  - `ghostboard/build.zig`;
  - `ghostboard/build.zig.zon`;
  - `ghostboard/src/Surface.zig`;
  - `ghostboard/macos/`;
  - `ghostboard/README.md`.
- Confirmed representative imported files match upstream `v1.3.1` content by
  SHA-256:
  - `build.zig`:
    `2a100b316cffd1eb9bb80e99093542baec12d70d7721b95c4c94ad88a2b7a1fa`;
  - `src/Surface.zig`:
    `320f36fba48cdcd9ed3add5c0e2e294137732f5d779ff064c0c7b62f7b129f5c`;
  - `README.md`:
    `aac5ca0698f31b78df5c11fe581a23a0ca276e0050b89d857be94817955739e2`.
- Confirmed the import commit is a merge commit with parents:

  ```text
  493817fd94ee3bc6bdefb24274132e7862378226 248e06fde20e6259b3001e04c7396b65a178fe3e 332b2aefc6e72d363aa93ab6ecfc86eeeeb5ed28
  ```

- Confirmed the import commit's second parent equals `v1.3.1^{commit}`.
- Confirmed `find ghostboard -type f | wc -l` returns `5544`.
- Confirmed `git status --short` was clean immediately after the import
  verification.

No branding, config-path, icon, CLI, build-system, protocol, `webtui`, or
`roamium` changes were made in this experiment.

## Completion Review

Fresh-context adversarial completion review returned `APPROVED`.

- The reviewer confirmed the import commit is a merge commit with parents
  `248e06fde20e6259b3001e04c7396b65a178fe3e` and
  `332b2aefc6e72d363aa93ab6ecfc86eeeeb5ed28`.
- The reviewer confirmed `v1.3.1^{commit}` resolves to
  `332b2aefc6e72d363aa93ab6ecfc86eeeeb5ed28`, matching the import commit's
  second parent.
- The reviewer confirmed the `ghostty` remote, remote tag object, and remote
  peeled commit match the documented values.
- The reviewer confirmed `ghostboard/build.zig`, `ghostboard/src/Surface.zig`,
  and `ghostboard/README.md` match upstream `v1.3.1` exactly.
- The reviewer confirmed the first-parent import diff contains `5544` files, all
  under `ghostboard/`.
- The reviewer confirmed the working tree had only the two issue documentation
  result updates.
- No findings were reported.

## Conclusion

Experiment 1 established a clean upstream Ghostty `v1.3.1` base at `ghostboard/`
with upstream history preserved. The next experiment can start from this
imported tree and make the first minimal Ghostboard-specific change.
