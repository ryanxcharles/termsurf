# Config Matrix

This matrix tracks Ghostty configuration option parity, config-file behavior,
defaults, diagnostics, precedence, formatting, and runtime effects.

## Row Schema

| Column                     | Meaning                                                                       |
| -------------------------- | ----------------------------------------------------------------------------- |
| ID                         | Stable row ID, prefixed `CFG-`.                                               |
| Config behavior / option   | Option, parser rule, default, diagnostic, precedence rule, or runtime effect. |
| Upstream behavior / source | Ghostty behavior and source path or user config path.                         |
| Roastty behavior / path    | Roastty behavior and source path or user config path.                         |
| Status                     | `Pass`, `Gap`, `Intentional divergence`, or `Not applicable`.                 |
| Verification method        | Concrete command, unit test, integration test, or walkthrough.                |
| Evidence artifact          | Log, test output, screenshot, or matrix row proving the status.               |
| Guard tier                 | Tier 0-4 from the Issue 805 regression guard policy.                          |
| Guard command / checklist  | Exact command or manual checklist that catches regressions.                   |
| Run cadence                | When the guard should run.                                                    |
| Guard sufficiency          | Why this guard is strong enough for the row.                                  |
| Owner experiment           | Experiment that created or last updated the row.                              |
| Notes                      | Short context, if needed.                                                     |

## Rows

| ID      | Config behavior / option       | Upstream behavior / source                         | Roastty behavior / path                                      | Status | Verification method                                                    | Evidence artifact                                                            | Guard tier | Guard command / checklist                                  | Run cadence                                                              | Guard sufficiency                                                                                 | Owner experiment | Notes                                                                     |
| ------- | ------------------------------ | -------------------------------------------------- | ------------------------------------------------------------ | ------ | ---------------------------------------------------------------------- | ---------------------------------------------------------------------------- | ---------- | ---------------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------- | ---------------- | ------------------------------------------------------------------------- |
| CFG-001 | Baseline user config file path | Ghostty user config is `~/.config/ghostty/config`. | Roastty analogous user config is `~/.config/roastty/config`. | Pass   | Compare files before A/B runs without logging private config contents. | `logs/issue805-exp1-rerun-clean-baseline.log` line `CONFIG_FILES_MATCH=yes`. | Tier 0     | `cmp -s ~/.config/ghostty/config ~/.config/roastty/config` | Before A/B visual or walkthrough experiments that depend on user config. | File equality is sufficient for the temporary baseline until per-option config parity is audited. | Experiment 1     | Later config experiments must replace this baseline with per-option rows. |
