# Experiment 3: Parity Matrix Schema

## Description

Issue 805 cannot close from ad hoc notes. It requires durable matrices for
features, config, source audit findings, app walkthrough scenarios, and accepted
divergences. This experiment creates those proof artifacts and gives each one a
row schema that future experiments must fill in.

This experiment does not attempt the full source audit, config inventory, or app
walkthrough. It creates the tracking surface that makes those later experiments
reviewable and automatable.

## Changes

Planned issue-doc changes:

- `issues/0805-roastty-ghostty-parity/03-parity-matrix-schema.md`
  - Record the plan, review, commands, result, and conclusion.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add Experiment 3 to the issue index with status `Designed`.
  - Update learnings if the schema reveals a reusable rule for future parity
    rows.
- `issues/0805-roastty-ghostty-parity/feature-matrix.md`
  - Create the feature/workflow parity matrix and row schema.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Create the config parity matrix and row schema.
- `issues/0805-roastty-ghostty-parity/source-audit.md`
  - Create the source subsystem audit matrix and row schema.
- `issues/0805-roastty-ghostty-parity/walkthrough-matrix.md`
  - Create the app walkthrough scenario matrix and row schema.
- `issues/0805-roastty-ghostty-parity/divergences.md`
  - Create the accepted divergence / not-applicable matrix and row schema.

No product code, harness code, Ghostty source, or Roastty source should change
in this experiment.

## Design Review

Reviewed by a fresh-context Codex adversarial subagent.

Initial verdict: **Changes required**.

Required finding and fix:

- The original `divergences.md` schema could support evidence-free accepted
  divergences or not-applicable claims. Fixed by requiring status/outcome,
  evidence artifact, regression guard tier, guard command or manual checklist,
  run cadence, guard sufficiency, and owner experiment for divergence rows. Rows
  without automated guards must still name a manual walkthrough checklist,
  evidence artifact, cadence, and acceptance owner.

Re-review verdict: **Approved**. The reviewer confirmed the required finding was
resolved and no new required finding was introduced.

## Verification

Run from the repo root. Write transcripts to `logs/` with the prefix
`issue805-exp3-` if any command output is worth preserving.

### 1. Create Required Artifacts

Commands:

```bash
ls issues/0805-roastty-ghostty-parity/
```

Pass criteria:

- The issue folder contains all required artifacts:
  - `feature-matrix.md`
  - `config-matrix.md`
  - `source-audit.md`
  - `walkthrough-matrix.md`
  - `divergences.md`

### 2. Schema Completeness Check

Commands:

```bash
rg -n 'Status|Upstream|Roastty|Verification|Evidence|Guard|Owner experiment' \
  issues/0805-roastty-ghostty-parity/{feature-matrix.md,config-matrix.md,source-audit.md,walkthrough-matrix.md}

rg -n 'Upstream behavior|Roastty behavior|Reason|User impact|Acceptance' \
  issues/0805-roastty-ghostty-parity/divergences.md

rg -n 'Evidence|Guard tier|Guard command|Run cadence|Guard sufficiency' \
  issues/0805-roastty-ghostty-parity/divergences.md
```

Pass criteria:

- Each non-divergence matrix documents columns for:
  - upstream behavior or source;
  - Roastty implementation or behavior;
  - status;
  - verification method;
  - evidence;
  - regression guard tier and command/checklist;
  - owner experiment.
- `divergences.md` documents columns for:
  - upstream behavior;
  - Roastty behavior;
  - status / outcome (`Intentional divergence` or `Not applicable`);
  - reason;
  - user impact;
  - acceptance rationale;
  - evidence artifact;
  - regression guard tier;
  - guard command or manual checklist;
  - run cadence;
  - why the guard is sufficient;
  - owner experiment.
- Divergence rows that cannot have an automated guard must still name a manual
  walkthrough checklist, evidence artifact, run cadence, and acceptance owner.
- Status vocabulary is limited to the issue-approved values:
  - `Pass`
  - `Gap`
  - `Intentional divergence`
  - `Not applicable`

### 3. Seed Only Known Rows

Seed rows should be conservative and limited to behavior already proven by
Experiments 1 and 2. Do not invent coverage. Do not mark untested behavior as
passing.

Expected seed rows:

- clean Ghostty build baseline;
- matched config-file baseline;
- live A/B smoke baseline;
- PID-guarded keyboard delivery baseline;
- Roastty focus-click requirement for keyboard input.

Pass criteria:

- Seed rows cite Experiment 1 or Experiment 2 as owner.
- Seed rows cite concrete evidence logs or screenshots.
- Any unresolved item is `Gap`, not `Pass`.

### 4. Hygiene

Commands:

```bash
prettier --write --prose-wrap always --print-width 80 \
  issues/0805-roastty-ghostty-parity/*.md

git diff --check
git status --short
```

Pass criteria:

- Markdown is formatted.
- `git diff --check` passes.
- Only the planned issue documentation files changed.

Overall result:

- **Pass** if all required matrix files exist, each schema supports the closure
  requirements in the issue README, and seeded rows are backed by existing
  evidence only.
- **Partial** if the files exist but a later experiment must refine schema
  fields before audits can start.
- **Fail** if the matrix files are missing or the schema would allow parity to
  be claimed without evidence and guards.

## Result

**Result:** Pass

All required matrix artifacts were created:

- `feature-matrix.md`
- `config-matrix.md`
- `source-audit.md`
- `walkthrough-matrix.md`
- `divergences.md`

Each matrix defines the required row schema. Non-divergence rows include
upstream behavior/source, Roastty behavior/path, status, verification method,
evidence, regression guard tier, guard command/checklist, run cadence, guard
sufficiency, owner experiment, and notes. `divergences.md` also requires
status/outcome, reason, user impact, acceptance rationale, evidence, guard
fields, and owner experiment.

Seed rows were intentionally limited to behavior already proven by Experiments 1
and 2:

- live A/B smoke render baseline;
- PID-guarded keyboard delivery;
- matched config-file baseline;
- clean pinned Ghostty source build;
- side-by-side debug app launch and cleanup;
- keyboard marker command delivery.

Verification:

- `prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/*.md`
- `rg -n 'Status|Upstream|Roastty|Verification|Evidence|Guard|Owner experiment' issues/0805-roastty-ghostty-parity/{feature-matrix.md,config-matrix.md,source-audit.md,walkthrough-matrix.md}`
- `rg -n 'Upstream behavior|Roastty behavior|Reason|User impact|Acceptance' issues/0805-roastty-ghostty-parity/divergences.md`
- `rg -n 'Evidence|Guard tier|Guard command|Run cadence|Guard sufficiency' issues/0805-roastty-ghostty-parity/divergences.md`
- `git diff --check`

## Completion Review

Reviewed by a fresh-context Codex adversarial subagent.

Verdict: **Approved**.

The reviewer reported no required, optional, or nit findings. It independently
verified that the result remained docs-only, all five matrix artifacts exist,
schemas require evidence and guard fields, seed rows are limited to Experiment 1
and 2 evidence, `divergences.md` does not allow evidence-free accepted rows, the
README marks Experiment 3 as `Pass`, the experiment has Result and Conclusion,
the result commit had not been made before review, and `git diff --check`
passes.

## Conclusion

Issue 805 now has the proof surface needed for the real parity work. Future
experiments should add or update matrix rows as they audit source, config, and
app behavior. A row cannot count toward final parity certification unless it has
an accepted status, concrete evidence, and a durable guard or documented manual
walkthrough guard.
