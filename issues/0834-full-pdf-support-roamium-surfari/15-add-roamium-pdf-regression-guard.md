# Experiment 15: Add Roamium PDF Regression Guard

## Description

Experiments 2 through 14 proved many Roamium PDF workflows, but the evidence is
spread across one-off harness commands and issue logs. Issue 834 requires
durable Roamium regression guards before the Surfari phase begins. This
experiment should consolidate the proven Roamium PDF workflows into one
repeatable regression entry point with explicit tiers.

This experiment should not attempt new PDF feature work. It should protect what
is already proven, classify still-partial rows honestly, and make future
breakage obvious.

The guard must distinguish:

- fast Roamium PDF smoke suitable for frequent development;
- focused Roamium PDF feature checks suitable before changing PDF/input code;
- unsafe or OS-contained workflows, especially native print, that must not run
  accidentally in unattended automation.

## Changes

1. Add a Roamium PDF regression runner.

   Add a script, expected path
   `scripts/test-issue-834-roamium-pdf-regression.py`, that can run selected
   groups of existing Roamium PDF probes and write a machine-readable summary.

   Suggested CLI:

   ```bash
   python3 scripts/test-issue-834-roamium-pdf-regression.py \
     --log-dir logs/issue-834-exp15-roamium-pdf-regression \
     --tier smoke
   ```

   Required tiers:

   - `smoke`: only fast, high-signal checks that should stay cheap enough to run
     frequently;
   - `focused`: all safe proven Roamium PDF feature probes from Issue 834;
   - `forms`: the Experiment 14 form comparison guard;
   - `unsafe-manual`: dry/list-only by default, for native print or any workflow
     that may open OS UI.

   The runner must exit nonzero if any selected safe guard fails. It must not
   silently pass on missing child summaries.

2. Reuse existing probes instead of duplicating feature logic.

   The runner should call the existing focused scripts wherever possible,
   including the harnesses used by:

   - Roamium baseline rendering / toolbar / local parity from Experiment 2;
   - keyboard and page navigation from Experiment 3;
   - internal/external links from Experiment 4;
   - find/search from Experiment 5;
   - document restrictions from Experiment 6, but only rows that are proven or
     explicitly accepted;
   - password PDFs from Experiments 7 and 8;
   - malformed/error PDFs from Experiment 9;
   - forms from Experiment 14.

   Do not copy large probe implementations into the regression runner. The
   runner should orchestrate, collect summaries, and classify results.

3. Keep native print safe.

   Native print is still Partial from Experiment 10 because the VM could not
   prove a safe watcher preflight. The regression runner may include a
   descriptive `unsafe-manual` entry for native print, but it must not click a
   production native print control by default.

   The `unsafe-manual` tier should emit skipped/manual entries and exit
   successfully when it only lists unsafe checks. It must not execute native
   print production clicks unless a future experiment designs and verifies a
   separate explicit opt-in safety gate.

   The contained print bridge check from Experiment 10 may be included in
   `focused` if it does not open native print UI and still has a deterministic
   intercept.

4. Define a stable summary schema.

   The top-level summary should include at least:

   - `tier`;
   - `first_failing_hop`;
   - `overall_result`;
   - `checks`;
   - for each check: `name`, `command`, `returncode`, `summary_path`,
     `first_failing_hop`, `result`, and any `accepted_limitation`;
   - `skipped_unsafe_checks`;
   - `duration_seconds`.

   Use classifications that future automation can consume:

   - `pass`;
   - `fail`;
   - `accepted-limitation`;
   - `skipped-unsafe`;
   - `automation-gap`.

5. Update Issue 834 documentation.

   Update this experiment's result with the final tier list and command output.
   If the runner reveals that an earlier "proven" workflow no longer passes on
   the current tree, record this experiment as Partial and make the next
   experiment target the failing row.

   Do not mark the Issue 834 Roamium regression checklist item complete unless
   the focused tier protects all safe proven Roamium PDF rows and documents
   every accepted limitation.

## Verification

Verification for the completed result is:

```bash
PYTHONDONTWRITEBYTECODE=1 python3 -m py_compile \
  scripts/test-issue-834-roamium-pdf-regression.py

python3 scripts/test-issue-834-roamium-pdf-regression.py \
  --log-dir logs/issue-834-exp15-roamium-pdf-regression-smoke \
  --tier smoke

python3 scripts/test-issue-834-roamium-pdf-regression.py \
  --log-dir logs/issue-834-exp15-roamium-pdf-regression-focused \
  --tier focused

python3 scripts/test-issue-834-roamium-pdf-regression.py \
  --log-dir logs/issue-834-exp15-roamium-pdf-regression-forms \
  --tier forms

python3 scripts/test-issue-834-roamium-pdf-regression.py \
  --log-dir logs/issue-834-exp15-roamium-pdf-regression-unsafe-manual \
  --tier unsafe-manual

git diff --check
```

If the focused tier is too slow or exposes an expensive command, split that
command into a separate explicit tier rather than hiding it. The result must
record the duration of each run and justify the tiering.

Required evidence:

- the runner fails if a selected safe child check fails or omits its summary;
- `smoke` passes or records a concrete failing row;
- `focused` passes or records a concrete failing row;
- `forms` passes or records a concrete failing row from the Experiment 14
  compare guard;
- `unsafe-manual` lists native print as skipped/manual without running a
  production print click;
- accepted limitations are explicitly named rather than treated as passes;
- Python bytecode cache is removed after compilation;
- markdown is formatted with Prettier;
- design review is recorded, all real design-review findings are fixed, the
  design is approved, and the plan commit exists before implementation begins;
- completion review is recorded before the result commit.

## Pass Criteria

This experiment passes if Roamium has a repeatable PDF regression runner whose
safe selected tiers pass on the current tree, whose summary identifies each
covered workflow, and whose skipped or accepted limitations are explicit.

## Partial Criteria

This experiment is partial if the runner works but one or more previously proven
safe Roamium PDF workflows now fails, or if focused coverage has to omit a
required safe row because the existing probe is not automation-ready.

## Failure Criteria

This experiment fails if it clicks unsafe native print UI by default, duplicates
large probe logic instead of orchestrating existing checks, returns success when
selected child checks fail, or overstates accepted limitations as passing
coverage.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

Initial verdict: **Changes Required**.

Required findings:

- verification omitted the required design-review and plan-commit gate evidence;
- verification defined `forms` as a required tier but did not run
  `--tier forms`.

Optional finding:

- verify `unsafe-manual` as a dry/list-only tier so native print is listed
  without running a production print click.

Fixes:

- required evidence now includes design review recorded, real findings fixed,
  design approval recorded, and the plan commit before implementation;
- verification now runs `--tier forms`;
- `unsafe-manual` is dry/list-only by default and its verification command must
  prove native print is listed as skipped/manual without production print
  clicks.

Re-review verdict: **Approved**.

The reviewer confirmed that no Required findings remain.
