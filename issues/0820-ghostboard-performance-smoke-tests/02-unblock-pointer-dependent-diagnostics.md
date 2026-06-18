# Experiment 2: Unblock Pointer-Dependent Diagnostics

## Description

Experiment 1 added a fast repeated-startup smoke, but the diagnostic profile
failed before measuring resize, mouse, scroll, or input responsiveness. Every
diagnostic row reached AppKit overlay presentation and then failed with
`FAIL: missing AppKit hit-test geometry record`.

The shared prerequisite is the geometry harness's pointer delivery path:
`click_window_center` and `click_global_point` use
`scripts/ghostty-app/inject.swift` to post CGEvent mouse moves/clicks. The Swift
injector exits successfully, but the app does not log a hit-test in this VM.
This experiment will isolate that pointer-driver dependency and, if a working
driver exists, wire the performance diagnostics to it.

## Changes

Planned script changes:

- `scripts/ghostboard-geometry-matrix.sh`
  - Add a selectable pointer driver for harness mouse actions.
  - Keep the existing CGEvent driver as the default unless verification proves a
    better default is needed.
  - Add a System Events pointer driver path for move/click actions using the
    same global point coordinates already produced by `winid.swift`.
  - Treat move-only actions as a separate capability from click actions. If
    System Events can click but cannot produce move-only hover events, record
    hover/move-only rows as still blocked rather than claiming full pointer
    coverage.
  - Route `click_window_center`, `click_global_point`, `move_global_point`, and
    `click_negative_global_point` through the selected driver.
  - Keep drag and scroll on the existing CGEvent injector unless this experiment
    proves a replacement is needed.
- `scripts/ghostboard-performance-smoke.sh`
  - If the System Events driver works, run diagnostic pointer rows with that
    driver and keep the default fast profile unchanged.
  - If no pointer driver works in this VM, leave diagnostics visible but record
    a precise blocker and the required macOS permission or VM setting to test
    next.

Planned issue-document changes:

- Add `## Result` and `## Conclusion` after verification.
- Update the Issue 820 README experiment status after verification.

Explicitly out of scope:

- Ghostboard, Roamium, webtui, protocol, or app source changes.
- Rewriting the full geometry matrix.
- Precise performance benchmarking beyond making diagnostic rows runnable.
- Adding generated logs or screenshots to git.

## Verification

Formatting actions:

```bash
prettier --write --prose-wrap always --print-width 80 \
  issues/0820-ghostboard-performance-smoke-tests/README.md \
  issues/0820-ghostboard-performance-smoke-tests/02-unblock-pointer-dependent-diagnostics.md
```

Static checks:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh scripts/ghostboard-performance-smoke.sh
git diff --check
```

Runtime checks:

1. Reproduce the current failure with the default driver:

   ```bash
   scripts/ghostboard-geometry-matrix.sh initial-open
   ```

2. Try the alternate pointer driver against the smallest hit-test row:

   ```bash
   TERMSURF_GEOMETRY_POINTER_DRIVER=system-events \
     scripts/ghostboard-geometry-matrix.sh initial-open
   ```

3. If the alternate driver passes, run the performance diagnostic profile using
   it:

   ```bash
   TERMSURF_GEOMETRY_POINTER_DRIVER=system-events \
     scripts/ghostboard-performance-smoke.sh --diagnostic
   ```

4. Re-run the fast profile to prove the existing repeated-startup smoke remains
   intact:

   ```bash
   scripts/ghostboard-performance-smoke.sh --fast
   ```

Pass criteria:

- The experiment identifies whether the current CGEvent pointer path is the
  diagnostic blocker.
- A working pointer driver is available and at least `initial-open` passes with
  an AppKit hit-test geometry record.
- The diagnostic profile either passes its pointer-dependent rows or advances
  past the generic initial hit-test gate to a more specific performance or
  scenario failure.
- The fast repeated-startup profile from Experiment 1 still passes.
- No generated logs or screenshots are staged.

Partial criteria:

- The current blocker is proven, but both pointer drivers fail in this VM.
- A driver works for `initial-open`, but later diagnostic rows expose a separate
  scenario-specific failure.

Fail criteria:

- The experiment cannot determine whether pointer injection is the blocker.
- Pointer-driver selection makes the existing fast startup smoke fail.
- The experiment does not rerun `scripts/ghostboard-performance-smoke.sh --fast`
  after pointer-driver changes.
- The harness loses the ability to report exact log paths for failures.

## Design Review

This experiment is plan-only until a fresh-context adversarial design review
approves it. Record the reviewer verdict here, fix all real findings, and commit
the approved plan before implementation begins.

Fresh-context adversarial design review by Codex subagent `Jason the 2nd`:

- **Initial verdict:** Changes required.
- **Required finding:** Verification claimed the fast repeated-startup profile
  still passes, but did not include a runtime command to run
  `scripts/ghostboard-performance-smoke.sh --fast`. Accepted; added that command
  to runtime verification and added a Fail criterion if it is not rerun after
  pointer-driver changes.
- **Optional finding:** System Events move-only behavior was underspecified.
  Accepted; clarified that move-only actions are a separate capability from
  click actions, and that hover/move-only rows remain blocked if System Events
  can click but cannot produce move-only hover events.

External Codex re-review using `skills/codex-review`:

- **Final verdict:** Approved.
- **Required findings:** None.
- **Evidence checked:** The reviewer confirmed the fast-profile runtime command
  is present, the pass/fail criteria require preserving it, the move-only
  capability clarification is adequate, and the README still lists Experiment 2
  as `Designed`.

## Completion Gate

After implementation and verification:

- add `## Result` and `## Conclusion` to this experiment file;
- update the Issue 820 README experiment status from `Designed` to `Pass`,
  `Partial`, or `Fail`;
- request a fresh-context completion review;
- fix all real completion-review findings and record the final verdict in this
  file; and
- commit the reviewed result separately before designing or implementing the
  next experiment.
