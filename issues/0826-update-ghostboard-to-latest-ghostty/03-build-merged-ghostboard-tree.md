# Experiment 3: Build the Merged Ghostboard Tree

## Description

Verify that the Ghostboard tree updated in Experiment 2 can build on this macOS
VM, or identify the first build/toolchain failure that must be fixed before
launch and runtime parity work can begin.

This experiment is a build gate only. It should not attempt app launch, protocol
verification, browser overlay parity, or runtime walkthroughs.

## Changes

- `issues/0826-update-ghostboard-to-latest-ghostty/README.md`
  - Link this experiment with status `Designed`, then update the status after
    the result is known.
- `issues/0826-update-ghostboard-to-latest-ghostty/03-build-merged-ghostboard-tree.md`
  - Record the build plan, verification commands, result, review, and
    conclusion.
- `ghostboard/`
  - Only modify build-related files if the build failure is clearly caused by
    the upstream merge conflict resolution or local macOS toolchain
    compatibility.

Do not modify `webtui/`, `roamium/`, or browser runtime/protocol behavior in
this experiment.

## Verification

Before implementation:

```bash
git status --short
git rev-list --parents -n 1 HEAD
git rev-list --parents -n 1 HEAD | rg '5d0a82ba337368f5632ffa6ce4d7c558fa2de9ff'
zig version
xcodebuild -version
sw_vers
```

Build commands:

```bash
cd ghostboard
zig build -Demit-macos-app=false
zig build
macos/build.nu --configuration Debug --action build
```

The `-Demit-macos-app=false` build should run first because it exercises the
merged Zig tree without immediately requiring the full macOS app bundle. The
plain `zig build` should run after that because Issue 826 ultimately requires
the updated Ghostboard app to build on macOS. The `macos/build.nu` command must
then run because `ghostboard/HACKING.md` documents it as the macOS app bundle
build command.

If any build command fails:

1. Save the full build output under the repository-root `logs/` directory. From
   inside `ghostboard/`, use paths like:
   - `../logs/issue-0826-exp03-zig-core.log`
   - `../logs/issue-0826-exp03-zig-build.log`
   - `../logs/issue-0826-exp03-macos-build.log`
2. Record the exact log path in this experiment's result.
3. Identify the first actionable failure.
4. Decide whether the failure belongs in this experiment:
   - fix it here if it is a narrow build/configuration issue caused by the merge
     or local toolchain compatibility;
   - record `Partial` and design the next experiment if the failure requires a
     broader build-system investigation.

After any edits, return to the repository root and run:

```bash
git diff --name-only -- '*.zig' | xargs -r zig fmt
(cd ghostboard && swiftlint lint --strict --fix)
prettier --write --prose-wrap always --print-width 80 \
  issues/0826-update-ghostboard-to-latest-ghostty/README.md \
  issues/0826-update-ghostboard-to-latest-ghostty/03-build-merged-ghostboard-tree.md
git diff --check
```

Run only the formatters relevant to files changed during this experiment. If no
Zig files change, `zig fmt` may be skipped and the result should say so. If no
Swift files change, SwiftLint may be skipped and the result should say so.

Result recording and review:

1. Append `## Result` and `## Conclusion` to this file.
2. Record each command, whether it passed, and the log path for each failed
   command.
3. Update the issue README status for this experiment to `Pass`, `Partial`, or
   `Fail`.
4. Run Prettier on this experiment file and the issue README.
5. Request the required result review before committing the result.
6. Record the result review in this file.
7. Commit the reviewed experiment result before designing the next experiment.

Pass criteria:

- The merged Ghostboard tree builds with `zig build -Demit-macos-app=false`.
- The merged Ghostboard tree builds with plain `zig build`.
- The merged Ghostboard tree builds with
  `macos/build.nu --configuration Debug --action build`.
- Any build-related edits are narrow, documented, and formatted.
- The result records the exact toolchain versions and commands run.

Partial criteria:

- At least one build command fails, but the first actionable failure is
  documented with logs and a clear next experiment.

Fail criteria:

- The build cannot be invoked because the merge left the tree structurally
  incoherent.
- The experiment expands into launch/runtime/protocol work.
- Build failures are summarized without preserving enough output to diagnose the
  next step.

## Design Review

An adversarial Codex subagent reviewed the initial design with fresh context.

**Verdict:** Changes required.

Required findings and fixes:

- Result review and result commit gates were not explicit. Fixed by adding a
  result recording, review, and commit checklist.
- The build commands did not prove the documented macOS app bundle build. Fixed
  by adding `macos/build.nu --configuration Debug --action build`.
- Failure log location was ambiguous after `cd ghostboard`. Fixed by requiring
  repository-root `logs/` paths such as
  `../logs/issue-0826-exp03-macos-build.log`.

The optional formatter finding was also addressed by requiring `zig fmt` on
every changed Zig file rather than only a fixed list.

The re-review approved the design with no required findings. Its only nit was
that the hygiene command working directory was implicit, so the plan now states
that those commands run from the repository root.
