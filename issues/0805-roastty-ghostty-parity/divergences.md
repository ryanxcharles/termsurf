# Divergences

This matrix records accepted intentional divergences and not-applicable items.
Rows here are not escape hatches: every row still needs evidence and either an
automated regression guard or a documented manual walkthrough guard.

## Row Schema

| Column                    | Meaning                                                                                      |
| ------------------------- | -------------------------------------------------------------------------------------------- |
| ID                        | Stable row ID, prefixed `DIV-`.                                                              |
| Upstream behavior         | Ghostty behavior or source path.                                                             |
| Roastty behavior          | Roastty behavior or source path.                                                             |
| Status / outcome          | `Intentional divergence` or `Not applicable`.                                                |
| Reason                    | Why the difference exists or why upstream behavior does not apply.                           |
| User impact               | Expected user-visible or integration-visible impact.                                         |
| Acceptance rationale      | Why the issue accepts this row as non-blocking.                                              |
| Evidence artifact         | Log, screenshot, source reference, checklist, or other proof.                                |
| Guard tier                | Tier 0-4 from the Issue 805 regression guard policy.                                         |
| Guard command / checklist | Exact command or manual checklist that catches regressions or confirms continued acceptance. |
| Run cadence               | When the guard should run.                                                                   |
| Guard sufficiency         | Why this guard is strong enough for the row.                                                 |
| Owner experiment          | Experiment that created or last updated the row.                                             |
| Notes                     | Short context, if needed.                                                                    |

## Rows

No accepted divergences or not-applicable items have been recorded yet.
