# Source Audit

This matrix tracks source-level parity audits between pinned Ghostty and
Roastty/libroastty.

## Row Schema

| Column                     | Meaning                                                                                  |
| -------------------------- | ---------------------------------------------------------------------------------------- |
| ID                         | Stable row ID, prefixed `SRC-`.                                                          |
| Subsystem / source area    | Ghostty subsystem, ABI surface, terminal behavior, renderer path, app bridge, or helper. |
| Upstream behavior / source | Ghostty source path and expected behavior.                                               |
| Roastty behavior / path    | Roastty source path and implemented behavior.                                            |
| Status                     | `Pass`, `Gap`, `Intentional divergence`, or `Not applicable`.                            |
| Verification method        | Source audit method, test, build, or oracle.                                             |
| Evidence artifact          | Log, diff, test output, or matrix row proving the status.                                |
| Guard tier                 | Tier 0-4 from the Issue 805 regression guard policy.                                     |
| Guard command / checklist  | Exact command or manual checklist that catches regressions.                              |
| Run cadence                | When the guard should run.                                                               |
| Guard sufficiency          | Why this guard is strong enough for the row.                                             |
| Owner experiment           | Experiment that created or last updated the row.                                         |
| Notes                      | Short context, if needed.                                                                |

## Rows

| ID      | Subsystem / source area         | Upstream behavior / source                                                               | Roastty behavior / path                                               | Status | Verification method                                                              | Evidence artifact                                                                                                                        | Guard tier | Guard command / checklist                                                                          | Run cadence                                                 | Guard sufficiency                                                                         | Owner experiment | Notes                                                                          |
| ------- | ------------------------------- | ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | ------ | -------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ---------- | -------------------------------------------------------------------------------------------------- | ----------------------------------------------------------- | ----------------------------------------------------------------------------------------- | ---------------- | ------------------------------------------------------------------------------ |
| SRC-001 | Pinned Ghostty reference source | `vendor/ghostty` at `2c62d182cec246764ff725096a70b9ef44996f7f` builds from clean source. | Roastty parity target is compared against that clean upstream source. | Pass   | Build pinned Ghostty with plain Homebrew `zig` and Ghostty's macOS build script. | `logs/issue805-clean-ghostty-zig-build.log`; `logs/issue805-clean-ghostty-app-build.log`; `logs/issue805-exp1-rerun-clean-baseline.log`. | Tier 2     | `cd vendor/ghostty && zig build -Demit-macos-app=false && nu macos/build.nu --configuration Debug` | Before source audit milestones and after toolchain changes. | A clean upstream build proves the reference app can be used without local source patches. | Experiment 1     | This does not prove Roastty source parity; it pins the source audit reference. |
