# Walkthrough Matrix

This matrix tracks real macOS app walkthrough scenarios for Ghostty versus
Roastty.

## Row Schema

| Column                     | Meaning                                                       |
| -------------------------- | ------------------------------------------------------------- |
| ID                         | Stable row ID, prefixed `WALK-`.                              |
| Scenario                   | User workflow or app behavior under walkthrough.              |
| Upstream behavior / source | Ghostty expected behavior and app path.                       |
| Roastty behavior / path    | Roastty expected behavior and app path.                       |
| Status                     | `Pass`, `Gap`, `Intentional divergence`, or `Not applicable`. |
| Verification method        | Manual or automated walkthrough steps.                        |
| Evidence artifact          | Log, screenshot, marker file transcript, or checklist output. |
| Guard tier                 | Tier 0-4 from the Issue 805 regression guard policy.          |
| Guard command / checklist  | Exact command or manual checklist that catches regressions.   |
| Run cadence                | When the guard should run.                                    |
| Guard sufficiency          | Why this guard is strong enough for the row.                  |
| Owner experiment           | Experiment that created or last updated the row.              |
| Notes                      | Short context, if needed.                                     |

## Rows

| ID       | Scenario                                  | Upstream behavior / source                                                                         | Roastty behavior / path                                                                                    | Status | Verification method                                                              | Evidence artifact                                                                        | Guard tier | Guard command / checklist                                                                                                      | Run cadence                                                           | Guard sufficiency                                                                                | Owner experiment    | Notes                                                                  |
| -------- | ----------------------------------------- | -------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- | ------ | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ | ------------------- | ---------------------------------------------------------------------- |
| WALK-001 | Side-by-side debug app launch and cleanup | `vendor/ghostty/macos/build/Debug/Ghostty.app` launches and can be cleaned up by scoped debug PID. | `roastty/macos/build/Build/Products/Debug/Roastty.app` launches and can be cleaned up by scoped debug PID. | Pass   | Launch both apps, record PIDs, capture windows, and stop scoped debug processes. | `logs/issue805-exp1-live-ab-smoke.log`; `logs/issue805-exp2-combined-keyboard-pass.log`. | Tier 3     | `scripts/roastty-app/live-ab-smoke.sh --recipe smoke` plus scoped stop helpers after manual debugging.                         | Before app walkthrough milestones.                                    | It exercises the real app bundles and proves cleanup is scoped to launched debug paths.          | Experiments 1 and 2 | Installed Ghostty hosting Codex is expected context and is not killed. |
| WALK-002 | Keyboard marker command delivery          | Debug Ghostty executes a typed `touch` marker command after exact PID guard.                       | Debug Roastty executes a typed `touch` marker command after exact PID guard and focus click.               | Pass   | Run the guarded keyboard recipe from Experiment 2.                               | `logs/issue805-exp2-combined-keyboard-pass.log`.                                         | Tier 3     | Activate target PID, verify frontmost PID, focus Roastty terminal center when needed, type marker command, verify marker file. | Before keyboard-heavy walkthrough work and after input/focus changes. | The marker files prove commands executed in the target terminal sessions after exact PID guards. | Experiment 2        | Do not type when the frontmost PID check fails.                        |
