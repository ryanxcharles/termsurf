# Experiment 37: Closeout acceptance audit

## Description

Experiment 36 fixed the remaining CLI-command blocker from Experiment 33 by
producing a runnable `zig-out/bin/termsurf` helper command and proving
`emit-exe=false` suppresses it.

This experiment will perform the final Issue 808 closeout audit. It will compare
the current state against every acceptance criterion in the issue README,
confirm that no known blocker remains, and close Issue 808 if the evidence
supports closure.

## Changes

Expected files:

- `issues/0808-recreate-ghostboard-from-ghostty-1-3-1/37-closeout-acceptance-audit.md`
  - record the closeout audit, evidence, result, and conclusion.
- `issues/0808-recreate-ghostboard-from-ghostty-1-3-1/README.md`
  - add Experiment 37 to the experiment index;
  - if the audit passes, update frontmatter to `status = "closed"` with
    `closed = "2026-06-16"`;
  - add the issue-level `## Conclusion`.
- `issues/README.md`
  - regenerate with `scripts/build-issues-index.sh` after closing the issue.

No product code changes are planned.

## Verification

Pass criteria:

- The audit matrix covers every Issue 808 acceptance criterion.
- Each acceptance criterion is marked **Pass** or, if not pass, the issue is not
  closed.
- Evidence includes:
  - Ghostty `v1.3.1` subtree import and history preservation from Experiment 1
    and Experiment 33;
  - pristine/imported build baseline and documented build-only deviations from
    Experiments 2-5;
  - app build and launch evidence from Experiments 31-34;
  - CLI command evidence from Experiment 36;
  - config path, app identity, menu/about branding, and icon evidence from
    Experiments 6 and 33;
  - `webtui`, Roamium, overlay, input, and ordinary browsing evidence from
    Experiments 30-33;
  - current protocol implementation evidence from Experiments 7-32;
  - current `git status --short --untracked-files=all`;
  - `git diff --check`;
  - current `ghostboard/zig-out/bin/termsurf` execution evidence or a fresh
    rebuild/run of the helper command.
- If the audit passes, `scripts/build-issues-index.sh` is run and
  `issues/README.md` reflects Issue 808 as closed.
- A completion review approves the closeout before the result commit.

Fail criteria:

- Any acceptance criterion remains **Fail**, **Partial**, or **Not tested**.
- The issue is closed without concrete evidence for every criterion.
- The experiment makes product code changes.

## Design Review

A fresh-context adversarial reviewer returned **APPROVED** with no required
findings. The reviewer confirmed that the closeout scope is documentation/index
only, the verification requires every Issue 808 acceptance criterion to have
concrete evidence and pass before closure, the design includes issue-level
conclusion and index regeneration, and completion review is required before the
result commit.

## Result

**Result:** Pass

The closeout audit found concrete passing evidence for every Issue 808
acceptance criterion. Issue 808 was closed on 2026-06-16, an issue-level
conclusion was added to the README, and the issue index was regenerated.

### Acceptance Matrix

| Criterion                                                              | Status | Evidence                                                                                                                            |
| ---------------------------------------------------------------------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------- |
| `ghostboard/` exists as Ghostty `v1.3.1` subtree import                | Pass   | Experiment 1; `logs/ghostboard-exp37-history-20260616.log` finds `493817fd9 Import Ghostty v1.3.1 into ghostboard`                  |
| Upstream Ghostty history is preserved                                  | Pass   | Experiment 33; `git merge-base --is-ancestor 22efb0be2bbea73e5339f5426fa3b20edabcaa11 HEAD` returned `merge_base_exit=0`            |
| Imported Ghostty built before port changes, with deviations documented | Pass   | Experiments 2-5 document pristine failures and the macOS-only GhosttyKit/libtool build patch                                        |
| App builds locally                                                     | Pass   | Experiments 31-34 and 36; `logs/ghostboard-exp36-app-bundle-regression-20260616.log` has `build_exit=0`                             |
| App can be launched as `TermSurf.app`                                  | Pass   | Experiment 33 launched `ghostboard/macos/build/Debug/TermSurf.app`; Experiment 36 verifies `zig-out/TermSurf.app` is still built    |
| CLI command is `termsurf`                                              | Pass   | Experiment 36; `logs/ghostboard-exp36-positive-cli-build-20260616.log` and `logs/ghostboard-exp36-positive-cli-run-20260616.log`    |
| App uses `~/.config/termsurf/config`                                   | Pass   | Experiments 6 and 33 source/config audits                                                                                           |
| Dock/menu/about branding says `TermSurf`                               | Pass   | Experiments 6 and 33 bundle/source audits; `MainMenu.xib`, `AboutView.swift`, and bundle metadata                                   |
| App icon matches the current Wezboard icon                             | Pass   | Experiment 6 icon setup and pixel comparison; Experiment 33 confirms current `TermSurf.icns` bundle metadata                        |
| `webtui` runs inside Ghostboard without changes                        | Pass   | Experiments 28 and 30-33 use real `target/debug/web`                                                                                |
| Roamium launches and is controlled without changes                     | Pass   | Experiments 30-33 use Chromium-output Roamium and Roamium-side logs                                                                 |
| Current TermSurf protocol supports ordinary browsing workflows         | Pass   | Experiments 7-32 implement/query/control protocol pieces; Experiments 31-33 prove overlay presentation and browser input forwarding |
| Experiments are recorded one at a time                                 | Pass   | Issue README links Experiments 1-37; each has a result/conclusion before the next implementation step                               |

### Current Evidence

- `logs/ghostboard-exp37-git-hygiene-20260616.log`
  - `git status --short --untracked-files=all` was clean before closeout docs;
  - `git diff --check` returned `diff_check_exit=0`;
  - HEAD was `8bb04d183 Plan issue 808 closeout`.
- `logs/ghostboard-exp37-history-20260616.log`
  - found the Ghostty subtree import commit;
  - proved exact upstream `v1.3.1` commit reachability with `merge_base_exit=0`;
  - confirmed the `ghostty` remote points to
    `https://github.com/ghostty-org/ghostty.git`.
- `logs/ghostboard-exp37-current-artifacts-20260616.log`
  - current `ghostboard/zig-out/bin/termsurf` exists and runs;
  - current `ghostboard/zig-out/TermSurf.app` exists;
  - current app bundle metadata reports `TermSurf` and `termsurf`.
- `logs/ghostboard-exp37-build-issues-index-20260616.log`
  - `scripts/build-issues-index.sh` exited with `exit=0`;
  - regenerated `issues/README.md` with 5 open issues and 314 closed issues.

## Completion Review

A fresh-context adversarial reviewer returned **APPROVED** with no required
findings. The reviewer confirmed that Issue 808 frontmatter is closed, the issue
README has an issue-level conclusion, Experiment 37 is marked **Pass**, the
acceptance matrix covers all 13 issue acceptance criteria, the issue index moved
Issue 808 from open to closed, `git diff --check` was clean, and the closeout
diff touches only issue documentation and the generated issue index.

## Conclusion

Issue 808 is complete. The fresh Ghostty `v1.3.1` subtree has been imported,
renamed minimally to the TermSurf user-facing identity, wired to the current
TermSurf protocol, verified with real `webtui` and Roamium workflows, given a
working `termsurf` helper command, and closed with all acceptance criteria
passing.
