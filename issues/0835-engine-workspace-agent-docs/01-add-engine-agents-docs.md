# Experiment 1: Add engine AGENTS docs

## Description

Add root `AGENTS.md` files to the Chromium and WebKit workspaces so agents can
find complete build, branch, and patch workflow instructions in the workspace
they are about to modify.

This experiment is documentation-only. It should not modify `chromium/src`,
`webkit/src`, build outputs, engine patches, Rust code, Zig code, Swift code, or
protocol files.

The experiment should also remove or redirect stale separate skill guidance so
the new source of truth is clear: basic Chromium and WebKit engine workflow
instructions live in `chromium/AGENTS.md` and `webkit/AGENTS.md`.

## Changes

- Add `chromium/AGENTS.md`.
  - Include purpose, tracked/ignored layout, current state, build prerequisites,
    setup, safe build commands, cache cautions, branch workflow, patch
    generation/application, branch table maintenance, and verification commands.
  - Preserve the critical Chromium-specific rules from the current docs,
    especially using `autoninja` instead of `ninja`, keeping
    `chromium/src/out/Default` intact, tracking Electron's Chromium version, and
    using issue-specific branches plus patch archives.
- Add `webkit/AGENTS.md`.
  - Use the same section shape where practical.
  - Include purpose, tracked/ignored layout, current state, prerequisites,
    setup, build commands, shallow-checkout cautions, branch workflow, patch
    generation/application, branch table maintenance, and verification commands.
  - Preserve WebKit-specific facts: the source checkout is `webkit/src`, the
    current base is an upstream commit, builds use
    `webkit/src/Tools/Scripts/build-webkit --debug`, and WebKit source patches
    are archived under `webkit/patches/issue-{N}/`.
- Update `chromium/README.md` and `webkit/README.md` only as needed to avoid
  contradictory duplicated instructions and point agents at the new `AGENTS.md`
  files.
- Update `skills/chromium/SKILL.md` so it no longer acts as a separate build
  source of truth. It may remain as a short redirect to `chromium/AGENTS.md`, or
  be otherwise reduced so it cannot drift from the workspace-local agent docs.
- Do not create a WebKit skill.
- Do not modify engine source checkouts or generate new engine patches.

## Verification

Pass criteria:

- `chromium/AGENTS.md` exists and contains complete Chromium build, branch, and
  patch workflow instructions.
- `webkit/AGENTS.md` exists and contains complete analogous WebKit build,
  branch, and patch workflow instructions.
- Both files use a visibly analogous section structure while preserving genuine
  engine-specific differences.
- `chromium/README.md`, `webkit/README.md`, and `skills/chromium/SKILL.md` do
  not contradict the new `AGENTS.md` files.
- `skills/chromium/SKILL.md` no longer presents separate build instructions as
  canonical.
- No `skills/webkit/` or WebKit skill is created.
- No files under `chromium/src` or `webkit/src` are changed.
- The issue README links this experiment with status `Designed` before
  implementation, then `Pass`, `Partial`, or `Fail` after result recording.

Run:

```bash
prettier --write --prose-wrap always --print-width 80 \
  issues/0835-engine-workspace-agent-docs/README.md \
  issues/0835-engine-workspace-agent-docs/01-add-engine-agents-docs.md \
  chromium/AGENTS.md \
  webkit/AGENTS.md \
  chromium/README.md \
  webkit/README.md \
  skills/chromium/SKILL.md

prettier --check --prose-wrap always --print-width 80 \
  issues/0835-engine-workspace-agent-docs/README.md \
  issues/0835-engine-workspace-agent-docs/01-add-engine-agents-docs.md \
  chromium/AGENTS.md \
  webkit/AGENTS.md \
  chromium/README.md \
  webkit/README.md \
  skills/chromium/SKILL.md

test -f chromium/AGENTS.md
test -f webkit/AGENTS.md
test ! -e skills/webkit
rg -n "AGENTS.md" chromium/README.md webkit/README.md skills/chromium/SKILL.md
test "$(rg -c "autoninja|gn gen|gclient sync|build-webkit|format-patch|git am" skills/chromium/SKILL.md)" -le 1
test "$(rg -c "AGENTS.md" skills/chromium/SKILL.md)" -ge 1
git diff --check
git status --short -- chromium/src webkit/src
```

Also manually inspect the three legacy documentation files after implementation:

- `chromium/README.md` must either be a workspace overview that points agents to
  `chromium/AGENTS.md`, or have summaries that are consistent with
  `chromium/AGENTS.md`.
- `webkit/README.md` must either be a workspace overview that points agents to
  `webkit/AGENTS.md`, or have summaries that are consistent with
  `webkit/AGENTS.md`.
- `skills/chromium/SKILL.md` must be reduced to a redirect or brief locator. It
  must not contain a separate detailed Chromium build workflow, branch workflow,
  or patch workflow.

The `rg` checks prove those files mention the new workspace-local instructions.
The `skills/chromium/SKILL.md` command-count check is intentionally mechanical:
it fails if the Chromium skill still contains multiple detailed workflow command
references instead of a short redirect. The final `git status` command must show
no engine source checkout changes.

Result classification:

- `Pass` means both workspace-local `AGENTS.md` files are complete and
  analogous, stale separate Chromium skill guidance is redirected or reduced,
  and verification passes without engine source changes.
- `Partial` means the `AGENTS.md` files exist but one documentation source still
  conflicts, or one workflow is not complete enough for safe engine work.
- `Fail` means the experiment changes engine source, creates a WebKit skill, or
  leaves agents without clear workspace-local build/branch/patch instructions.

## Design Review

Adversarial design review initially returned `CHANGES REQUIRED` with one
Required finding: the verification plan said legacy docs and
`skills/chromium/SKILL.md` must not contradict the new `AGENTS.md` files, but
the runnable checks did not concretely prove that.

The design was updated to add explicit checks for `AGENTS.md` references in the
legacy docs, a mechanical command-count check that fails if
`skills/chromium/SKILL.md` still contains multiple detailed workflow command
references, and manual inspection criteria for `chromium/README.md`,
`webkit/README.md`, and `skills/chromium/SKILL.md`.

Focused re-review returned `APPROVED` with no Required findings. The reviewer
confirmed the verification plan is now concrete enough for the design gate.

## Result

**Result:** Pass

Implemented the documentation alignment:

- Added `chromium/AGENTS.md` as the agent-facing source of truth for Chromium
  workspace layout, prerequisites, setup, build rules, branch workflow, patch
  archives, verification, and cautions.
- Added `webkit/AGENTS.md` with the analogous WebKit workspace layout,
  prerequisites, setup, build rules, branch workflow, patch archives,
  verification, and cautions.
- Updated `chromium/README.md` and `webkit/README.md` with `Agent Instructions`
  sections pointing agents to the new workspace-local `AGENTS.md` files.
- Updated `webkit/README.md` so the current branch summary matches the actual
  local WebKit branch: `webkit-1452a439-issue-756-exp12`.
- Reduced `skills/chromium/SKILL.md` to a short redirect to `chromium/AGENTS.md`
  so it no longer contains a separate detailed build, branch, or patch workflow.
- Updated `.gitignore` to allow `chromium/AGENTS.md` and `webkit/AGENTS.md` to
  be tracked while keeping the rest of the engine workspaces ignored.

Verification:

```bash
prettier --check --prose-wrap always --print-width 80 \
  issues/0835-engine-workspace-agent-docs/README.md \
  issues/0835-engine-workspace-agent-docs/01-add-engine-agents-docs.md \
  chromium/AGENTS.md \
  webkit/AGENTS.md \
  chromium/README.md \
  webkit/README.md \
  skills/chromium/SKILL.md

test -f chromium/AGENTS.md
test -f webkit/AGENTS.md
test ! -e skills/webkit
rg -n "AGENTS.md" chromium/README.md webkit/README.md skills/chromium/SKILL.md
test "$(rg -c "autoninja|gn gen|gclient sync|build-webkit|format-patch|git am" skills/chromium/SKILL.md)" -le 1
test "$(rg -c "AGENTS.md" skills/chromium/SKILL.md)" -ge 1
git diff --check
git status --short -- chromium/src webkit/src
git check-ignore -v chromium/AGENTS.md webkit/AGENTS.md || true
```

All checks passed. The engine source checkout status check produced no output,
confirming no `chromium/src` or `webkit/src` changes. `git check-ignore`
reported the explicit `.gitignore` negation rules for both new `AGENTS.md`
files, confirming they are intentionally trackable.

Manual consistency inspection:

- `chromium/README.md` now points agents to `chromium/AGENTS.md`; its current
  branch summary still matches `chromium/AGENTS.md`.
- `webkit/README.md` now points agents to `webkit/AGENTS.md`; its current branch
  summary now matches `webkit/AGENTS.md`.
- `skills/chromium/SKILL.md` is now only a redirect/locator and contains no
  independent detailed Chromium build, branch, or patch workflow.
- No `skills/webkit/` directory exists.

## Conclusion

Chromium and WebKit now have analogous root `AGENTS.md` files that make the
engine workspace instructions local to each engine folder. The separate Chromium
skill no longer acts as a competing workflow source, and no WebKit skill was
created.

## Completion Review

Adversarial completion review initially returned `CHANGES REQUIRED` with one
Required finding: `chromium/AGENTS.md` documented an invalid fresh setup path by
checking out `148.0.7778.97-issue-816` from the vanilla tag and applying only
`chromium/patches/issue-816/*.patch`, even though that patch archive contains
one incremental patch.

The fix aligned `chromium/AGENTS.md` and `chromium/README.md` around the actual
full-stack reconstruction path:

- `148.0.7778.97-issue-794-exp19` is now documented as the current fully
  archived build baseline;
- `148.0.7778.97-issue-816` remains documented as the latest documented branch;
- the fresh setup path applies `chromium/patches/issue-794-exp19/*.patch`;
- both docs warn that later patch directories may be incremental and must not be
  treated as fresh setup recipes unless regenerated and verified as cumulative.

Focused re-review returned `APPROVED` with no Required findings. The reviewer
confirmed the reconstruction-path issue is fixed and noted the verified patch
counts: `issue-794-exp19` has 60 patches and `issue-816` has 1 patch.
