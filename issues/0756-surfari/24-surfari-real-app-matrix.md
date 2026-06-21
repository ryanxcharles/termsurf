# Experiment 24: Define Surfari real-app matrix

## Description

Issue 756 still has two broad completion items: test Surfari inside the real
TermSurf app across lifecycle/layout/focus/profile/crash behavior, then compare
Ghostboard/Surfari with the Ghostboard/Roamium feature matrix. Experiments 20-23
proved important slices, but the remaining checklist is too broad to attack
safely as one unstructured harness.

This experiment should convert the remaining real-app Surfari work into an
explicit matrix with coverage status, evidence requirements, and a tranche
order. It should not claim the matrix is complete. Its goal is to make the next
experiments mechanical: each tranche should name the scenarios, reuse or extend
the focused harnesses, record pass/fail evidence, and update the matrix.

## Changes

- Add a matrix document for Issue 756, likely
  `issues/0756-surfari/real-app-matrix.md`.
- Include every item from the remaining checklist:
  - navigation;
  - keyboard input;
  - click;
  - drag;
  - scroll;
  - resize;
  - pane resize;
  - split panes;
  - tab switching;
  - window switching;
  - focus changes;
  - shutdown;
  - restart;
  - profile isolation;
  - crash handling.
- For each row, record:
  - current coverage status: `Proven`, `Partial`, `Missing`, or `Blocked`;
  - existing evidence, if any, with experiment/log/script references;
  - required proof to mark it `Proven`;
  - proposed harness or scenario to run next.
- Map comparable Roamium matrix scenarios from
  `scripts/ghostboard-geometry-matrix.sh` where they exist, without copying the
  full Roamium harness into Surfari yet.
- Propose a tranche order that keeps tests practical:
  1. lifecycle/navigation/resize/shutdown/restart;
  2. pane/split/tab/window/focus geometry;
  3. input details not yet proven, especially click/drag and coordinate
     fidelity;
  4. profile isolation and crash handling;
  5. Ghostboard/Roamium comparison and engine-specific differences.
- Update this experiment's README entry and result after the matrix is created.

## Verification

Pass criteria:

- The matrix document exists and includes every remaining real-app checklist
  item.
- The matrix does not overclaim: already-proven keyboard/wheel/shutdown items
  cite Experiments 20-23, while click/drag/profile/crash/etc. remain partial or
  missing unless there is direct evidence.
- The matrix includes concrete evidence requirements for each row.
- The matrix includes a next-tranche recommendation detailed enough to design
  Experiment 25 without redoing the inventory.
- Run hygiene checks:

```bash
git diff --check
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/24-surfari-real-app-matrix.md \
  issues/0756-surfari/real-app-matrix.md
```

Result classification:

- `Pass` means the matrix is complete as an inventory and ready to drive the
  next experiment.
- `Partial` means the matrix exists but misses checklist items, evidence
  requirements, or tranche ordering.
- `Fail` means the issue still has no usable matrix for the remaining real-app
  work.

## Design Review

Adversarial design review returned `APPROVED` with no findings. The reviewer
confirmed that the README links Experiment 24 as `Designed`, the experiment has
Description, Changes, and Verification sections, the scope is limited to matrix
inventory/tranche planning rather than execution, all remaining real-app
checklist items are listed, the design avoids overclaiming and requires evidence
for `Proven` rows, the verification has concrete pass/fail criteria and hygiene
checks, and no plan commit had already been made.
