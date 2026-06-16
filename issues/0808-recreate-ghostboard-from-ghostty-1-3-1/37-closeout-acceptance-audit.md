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
