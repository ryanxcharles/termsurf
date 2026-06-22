# Experiment 7: Publish Release 1.4.0

## Description

Stage 6 proved the package-only release artifact for `1.4.0`. Stage 7 performs
the real publish step: upload GitHub Release `v1.4.0`, update the Homebrew cask
to the generated tarball SHA, and push the Homebrew tap.

This experiment should not rebuild the release artifacts from source. It uses
the existing release build outputs and reruns `scripts/release.sh 1.4.0`, which
packages, uploads, updates the cask, commits the cask version/SHA, and pushes
the tap.

## Changes

No TermSurf source-code changes are planned.

Expected generated changes:

- GitHub Release `v1.4.0` exists in `termsurf/termsurf` with asset
  `termsurf-1.4.0-aarch64-apple-darwin.tar.gz`.
- `homebrew/Casks/termsurf.rb` is updated to `version "1.4.0"` and the generated
  SHA.
- The `termsurf/homebrew-termsurf` tap receives the cask update commit.
- The parent repo records the updated `homebrew/` submodule pointer and this
  experiment result.

## Verification

Preflight:

```bash
git status --short
git -C homebrew status --short
git -C homebrew rev-parse --abbrev-ref HEAD
git -C homebrew status --branch --short
gh auth status
gh release view v1.4.0 --repo termsurf/termsurf --json tagName 2>&1 || true
```

Pass preflight only if:

- the main repo is clean before result docs are written;
- the Homebrew submodule has no uncommitted changes;
- the Homebrew submodule is on `main`;
- the Homebrew submodule is ahead of `origin/main` only by already-reviewed
  Surfari packaging commits from this issue;
- GitHub auth is active;
- `v1.4.0` does not already exist.

Publish:

```bash
scripts/release.sh 1.4.0 2>&1 | tee /tmp/termsurf-issue838-exp7-release.log
```

Verify GitHub release and asset:

```bash
gh release view v1.4.0 --repo termsurf/termsurf \
  --json tagName,name,isDraft,isPrerelease,url,assets
rg 'Released TermSurf v1.4.0' /tmp/termsurf-issue838-exp7-release.log
rg 'Uploading to GitHub' /tmp/termsurf-issue838-exp7-release.log
```

Verify cask version and SHA:

```bash
published_sha="$(shasum -a 256 dist/termsurf-1.4.0-aarch64-apple-darwin.tar.gz | awk '{print $1}')"
rg 'version "1\.4\.0"' homebrew/Casks/termsurf.rb
rg "sha256 \"${published_sha}\"" homebrew/Casks/termsurf.rb
```

Verify Homebrew tap push:

```bash
git -C homebrew status --branch --short
git -C homebrew log -1 --oneline
git -C homebrew ls-remote origin main
git -C homebrew fetch origin main
test "$(git -C homebrew rev-parse HEAD)" = "$(git -C homebrew rev-parse origin/main)"
git diff --submodule=log -- homebrew
```

Verify release asset SHA by downloading it back from GitHub:

```bash
asset_dir="$(mktemp -d)"
published_sha="$(shasum -a 256 dist/termsurf-1.4.0-aarch64-apple-darwin.tar.gz | awk '{print $1}')"
gh release download v1.4.0 \
  --repo termsurf/termsurf \
  --pattern 'termsurf-1.4.0-aarch64-apple-darwin.tar.gz' \
  --dir "$asset_dir"
shasum -a 256 "$asset_dir/termsurf-1.4.0-aarch64-apple-darwin.tar.gz"
test "$(shasum -a 256 "$asset_dir/termsurf-1.4.0-aarch64-apple-darwin.tar.gz" | awk '{print $1}')" = "$published_sha"
rm -rf "$asset_dir"
```

Final hygiene:

```bash
prettier --check issues/0838-deploy-next-homebrew-version/README.md \
  issues/0838-deploy-next-homebrew-version/07-publish-release-1-4-0.md
git diff --check
git status --short
git -C homebrew status --short
```

After the result commit, verify the parent repo recorded the pushed Homebrew
submodule pointer:

```bash
git show --submodule=log --stat HEAD -- homebrew \
  issues/0838-deploy-next-homebrew-version/README.md \
  issues/0838-deploy-next-homebrew-version/07-publish-release-1-4-0.md
```

Pass criteria:

- `scripts/release.sh 1.4.0` completes without errors.
- GitHub Release `v1.4.0` exists and is not draft or prerelease.
- The GitHub release contains `termsurf-1.4.0-aarch64-apple-darwin.tar.gz`.
- The downloaded GitHub asset SHA matches the locally generated tarball SHA.
- `homebrew/Casks/termsurf.rb` contains `version "1.4.0"` and the matching SHA.
- The Homebrew tap push succeeds, `homebrew/main` matches `origin/main`, and the
  parent repo diff points at that pushed Homebrew commit before the result
  commit.
- The result commit records the updated `homebrew/` submodule pointer.
- Final hygiene checks pass with only the expected parent-repo issue
  documentation and submodule pointer changes before the result commit.

Fail criteria:

- GitHub auth is missing.
- `v1.4.0` already exists before publishing.
- The release script fails.
- The GitHub asset is missing or its SHA does not match the cask SHA.
- The Homebrew cask version/SHA are wrong.
- The Homebrew tap push fails or leaves uncommitted submodule changes.

## Design Review

An adversarial subagent reviewed the initial design with fresh context.

**Verdict:** Changes required.

The reviewer found that the design did not adequately prove the parent repo
records the updated `homebrew/` submodule pointer. The plan now verifies
`homebrew` HEAD matches `origin/main`, includes
`git diff --submodule=log -- homebrew` before the result commit, and requires
post-result-commit evidence with `git show --submodule=log --stat HEAD`.

The reviewer also noted that the downloaded asset SHA block depended on a shell
variable from an earlier fenced block. The plan now recomputes `published_sha`
inside that block.

The reviewer re-reviewed those fixes and returned `VERDICT: APPROVED` with no
findings.

## Result

**Result:** Pass

Release publication succeeded.

Preflight confirmed:

- the main repo was clean;
- the Homebrew submodule was clean on `main`;
- the Homebrew submodule was ahead of `origin/main` only by the three reviewed
  Surfari packaging commits from this issue;
- GitHub auth was active for account `ryanxcharles`;
- GitHub Release `v1.4.0` did not exist before publishing.

The real release command completed successfully:

```bash
scripts/release.sh 1.4.0 2>&1 | tee /tmp/termsurf-issue838-exp7-release.log
```

The release script created and uploaded the tarball, updated the Homebrew cask,
committed the tap update, and pushed `termsurf/homebrew-termsurf`:

```text
==> SHA256: efb72712b962c77605df9ee2b67cfda2e116fd39cb863588b62df1b1857ea260
==> Uploading to GitHub...
https://github.com/termsurf/termsurf/releases/tag/v1.4.0
==> Updating Homebrew cask...
[main 0c52904] v1.4.0
To github.com:termsurf/homebrew-termsurf.git
   a59df29..0c52904  main -> main
==> Released TermSurf v1.4.0
```

GitHub release verification passed:

```json
{
  "isDraft": false,
  "isPrerelease": false,
  "name": "v1.4.0",
  "tagName": "v1.4.0",
  "url": "https://github.com/termsurf/termsurf/releases/tag/v1.4.0"
}
```

The GitHub release contains asset `termsurf-1.4.0-aarch64-apple-darwin.tar.gz`
with digest
`sha256:efb72712b962c77605df9ee2b67cfda2e116fd39cb863588b62df1b1857ea260` and
size `443907861` bytes.

The Homebrew cask now contains:

```ruby
version "1.4.0"
sha256 "efb72712b962c77605df9ee2b67cfda2e116fd39cb863588b62df1b1857ea260"
```

The tap is pushed and aligned:

```text
## main...origin/main
0c52904 v1.4.0
0c52904d57d118f878abff564c6037300a2fb88b refs/heads/main
```

The parent repo submodule diff points at the pushed tap commit:

```text
Submodule homebrew d91e075a8..0c52904d5:
  > v1.4.0
```

Downloading the release asset back from GitHub produced the same SHA:

```text
efb72712b962c77605df9ee2b67cfda2e116fd39cb863588b62df1b1857ea260  termsurf-1.4.0-aarch64-apple-darwin.tar.gz
```

Final hygiene passed:

```bash
prettier --check issues/0838-deploy-next-homebrew-version/README.md \
  issues/0838-deploy-next-homebrew-version/07-publish-release-1-4-0.md
git diff --check
git -C homebrew status --short
```

Before result documentation, `git status --short` showed only the expected
parent submodule pointer update:

```text
 M homebrew
```

The completion reviewer returned `VERDICT: APPROVED` with no findings. The
reviewer independently verified the GitHub release, asset digest, local tarball
SHA, Homebrew cask SHA, pushed tap commit, parent submodule diff, README status,
and that the result commit had not yet been made.

## Conclusion

Stage 7 is complete. GitHub Release `v1.4.0` exists, its uploaded asset SHA
matches the local tarball and cask SHA, and the Homebrew tap is pushed to commit
`0c52904`.

The next experiment should verify installing or upgrading TermSurf through
Homebrew and confirm the installed WebTUI top controls plus installed Surfari
launch via `web --browser surfari` without `TERMSURF_SURFARI_PATH`.
