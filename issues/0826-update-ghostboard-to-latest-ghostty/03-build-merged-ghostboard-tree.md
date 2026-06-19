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

## Result

**Result:** Pass

Preflight:

```text
git status --short                                  # clean
merge commit: 2c883d4b6c6c3aa275c01efc75033178a0549c8b
merge parent: 5d0a82ba337368f5632ffa6ce4d7c558fa2de9ff
zig version: 0.15.2
Xcode: 26.6 build 17F109
macOS: 26.5.1 build 25F80
```

The direct `git rev-list --parents -n 1 HEAD | rg <upstream>` command from the
initial plan failed because `HEAD` was the Experiment 3 plan commit, not the
Experiment 2 merge commit. The intended invariant was verified against the
actual merge commit:

```text
2c883d4b6c6c3aa275c01efc75033178a0549c8b \
  593bb21084e502449275f5746d47457c54d9c903 \
  5d0a82ba337368f5632ffa6ce4d7c558fa2de9ff
```

Build results:

| Command                                               | Result | Log path                                |
| ----------------------------------------------------- | ------ | --------------------------------------- |
| `zig build -Demit-macos-app=false`                    | Pass   | `logs/issue-0826-exp03-zig-core.log`    |
| `zig build`                                           | Pass   | `logs/issue-0826-exp03-zig-build.log`   |
| `macos/build.nu --configuration Debug --action build` | Pass   | `logs/issue-0826-exp03-macos-build.log` |

The two successful Zig commands produced no output, so their log files are
empty. The macOS build log contains the full Xcode output and ends with:

```text
** BUILD SUCCEEDED **
```

The app bundle exists at:

```text
ghostboard/macos/build/Debug/TermSurf Ghostboard.app
```

Narrow build fix:

The first plain `zig build` attempt failed inside Xcode with ambiguous Swift
property lookup errors:

```text
ScriptTerminal.swift:55:29: error: ambiguous use of 'pwd'
TerminalCommandPalette.swift:155:35: error: ambiguous use of 'pwd'
```

The conflict resolution from Experiment 2 had preserved `pwd`, `cellSize`,
`healthy`, `error`, and `hoverUrl` declarations in `SurfaceView_AppKit.swift`,
but upstream moved those shared properties into `OSSurfaceView`. The fix removed
the duplicate subclass declarations while keeping the TermSurf-only
`termsurfCopyUrlFeedback` property.

After that narrow Swift fix:

- `swiftlint lint --strict --fix` passed from `ghostboard/`;
- plain `zig build` passed;
- `macos/build.nu --configuration Debug --action build` passed.

SwiftLint again printed:

```text
warning: The option --strict has no effect together with --fix.
```

This is the same option-combination warning observed in Experiment 2, and the
command still completed successfully.

Final verification:

```text
git diff --check                                      # pass
ghostboard/macos/build/Debug/TermSurf Ghostboard.app  # exists
```

## Conclusion

The merged Ghostboard tree builds successfully on this macOS VM. Experiment 3
also fixed one narrow Swift merge issue caused by duplicate properties after the
upstream `OSSurfaceView` split.

The next experiment should move to the launch gate: run the built app, confirm
it starts, and capture the first launch/runtime failure if one remains.

## Result Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

**Verdict:** Approved.

The reviewer found no findings. It independently checked that:

- the scope was limited to the allowed Swift build fix plus issue docs;
- `SurfaceView_AppKit.swift` subclasses `OSSurfaceView`;
- `OSSurfaceView` owns `pwd`, `cellSize`, `healthy`, `error`, and `hoverUrl`, so
  removing duplicate subclass declarations was correct;
- this experiment and the issue README both recorded `Pass`;
- `git diff --check` passed;
- the app bundle existed at
  `ghostboard/macos/build/Debug/TermSurf Ghostboard.app`;
- `logs/issue-0826-exp03-macos-build.log` ended with `** BUILD SUCCEEDED **`;
- the Zig logs were empty as documented;
- the Experiment 2 merge commit had the upstream Ghostty target as a parent;
- the result commit had not yet been made.

The reviewer did not rerun the build commands because they would mutate build
outputs; it verified the recorded logs and artifacts instead.
