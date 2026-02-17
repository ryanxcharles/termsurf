---
name: issues-and-experiments
description: "Write and manage issue documents and experiments in docs/issues/. Use when creating a new issue, designing an experiment, recording experiment results, or closing an issue. Enforces the one-experiment-at-a-time workflow."
---

# Issues and Experiments

Every significant piece of work gets an issue document in `docs/issues/`. Issues
describe the problem, provide background, and propose solutions. Experiments are
the incremental steps that solve the problem.

## Issue Documents

### Location and naming

All issue documents live in `docs/issues/`. Each has a sequential number and a
short descriptive name:

```
docs/issues/514-mouse.md
docs/issues/513-ctrl-esc.md
docs/issues/512-vsync.md
```

The number is globally sequential across all generations (ts1–ts5). The name is
lowercase, hyphenated, and describes the topic — not the solution.

### Structure of a new issue

A new issue document has these sections:

1. **Title** (H1) — `# Issue {N}: {descriptive title}`
2. **Goal** — One or two sentences describing the desired outcome from the
   user's perspective.
3. **Background** — Context: what led to this issue, what prior work is
   relevant, what constraints exist.
4. **Architecture** / **Analysis** / **Proposed Solutions** — Technical details,
   diagrams, trade-offs, ideas for how to solve the problem. Use whatever
   heading name fits the content.

A new issue does **not** have an Experiments section yet. The issue is a problem
statement and analysis, not a solution plan.

### What NOT to put in a new issue

**Never list experiments upfront.** Do not write "Experiment 1: ..., Experiment
2: ..., Experiment 3: ..." when creating an issue. The outcome of each
experiment may change what comes next. Listing them in advance creates false
commitments and wastes design effort on experiments that may never happen.

Instead, the issue body may include sections like:

- "Ideas for experiments"
- "Proposed solutions"
- "Possible approaches"

These are loose, exploratory. They are not numbered experiments with
verification criteria.

## Experiments

### When to create an experiment

Only after the issue's product requirements are clear and the team is ready to
implement the next step. Each experiment is designed, implemented, and concluded
before the next one is designed.

### Adding the Experiments section

When the first experiment is ready to be designed, add an `## Experiments`
heading at the bottom of the issue document, followed by the experiment:

```markdown
## Experiments

### Experiment 1: {short descriptive title}

{design content}
```

### Experiment structure

Each experiment has:

1. **Title** (H3) — `### Experiment {N}: {descriptive title}`
2. **Description** — What this experiment will do and why. What hypothesis is
   being tested or what capability is being added.
3. **Changes** — The specific code changes required, listed by file.
4. **Verification** — How to test that the experiment worked. Include concrete
   steps and a pass/fail criterion.

### One at a time

Design and implement one experiment at a time. After Experiment 1 is concluded,
then — and only then — design Experiment 2. The result of Experiment 1 (success,
partial success, or failure) directly informs what Experiment 2 should be.

### Recording results

After implementing and testing an experiment, add a result and conclusion
directly below the experiment's verification section:

```markdown
**Result:** Pass / Partial / Fail

{description of what happened}

#### Conclusion

{what we learned, what changed, what to do next}
```

Use the appropriate result:

- **Pass** — The experiment achieved its verification criteria.
- **Partial** — Some goals were met, others were not. Describe what worked and
  what didn't.
- **Fail** — The approach did not work. Describe why and what was learned.

All three outcomes are valuable. Failed experiments eliminate dead ends and
inform better designs.

## Closing an Issue

When all experiments have satisfied the issue's product requirements (the Goal),
add a top-level conclusion:

```markdown
## Conclusion

{summary of what was accomplished, key findings, and any follow-up work}
```

This goes after the last experiment, still inside the issue document.

## Process Summary

1. **Create the issue** — Problem statement, background, analysis. No
   experiments yet.
2. **Design Experiment 1** — Add `## Experiments` and `### Experiment 1` when
   ready.
3. **Implement Experiment 1** — Write the code.
4. **Record the result** — Pass, partial, or fail with a conclusion.
5. **Repeat** — Design the next experiment based on what was learned. Continue
   until the issue's goal is met.
6. **Close the issue** — Write the issue-level conclusion.
