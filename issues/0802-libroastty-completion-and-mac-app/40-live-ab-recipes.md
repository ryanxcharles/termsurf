+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex-adversarial"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 40: Phase D — named live A/B recipes

## Description

Experiment 39 proved the mechanics of launching Ghostty and Roastty, driving one
shared marker command, capturing both windows, and diffing the images. The next
Phase-D step is to turn that one-off smoke command into named, repeatable
recipes that can grow into the feature-by-feature conformance matrix.

This experiment adds a small recipe layer to `live-ab-smoke.sh`, starting with
the existing `smoke` recipe and one deterministic `ascii-grid` recipe. The new
recipe should render stable ASCII content while the shell command is still
sleeping, so captures do not include a returned shell prompt as part of the
oracle. The recipe layer must stay conservative: it should not claim visual
parity while strict diffs still fail, and it should not try to solve all Phase-D
feature coverage in one step.

## Changes

- `scripts/roastty-app/live-ab-smoke.sh`
  - Add `--recipe <name>` with at least:
    - `smoke`: the existing `clear; echo ISSUE802_AB_SMOKE_<timestamp>` command.
    - `ascii-grid`: a deterministic ASCII command that clears the screen, prints
      a recipe marker plus several fixed rows of letters, digits, and
      punctuation, then sleeps long enough for the harness to capture before the
      prompt returns.
  - Add `--list-recipes` so future experiments can discover supported recipes
    without reading the script.
  - Include `recipe` in the JSON summary.
  - Keep the default recipe as `smoke` so Experiment 39 behavior stays
    compatible.
  - Keep screenshots outside the repo, retain the IOSurface-safe Roastty
    full-screen-plus-crop path, invoke `pngdiff.swift` through `swift`, and keep
    exact launched-PID-tree cleanup.
- `scripts/roastty-app/README.md`
  - Document `--recipe`, `--list-recipes`, and the initial recipes.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Add this experiment to the index as `Designed`.
  - After implementation, record the recipe usage under Operating notes if the
    live run succeeds.

## Verification

- Run shell syntax checks:
  - `bash -n scripts/roastty-app/live-ab-smoke.sh`
- Run recipe discovery:
  - `scripts/roastty-app/live-ab-smoke.sh --list-recipes`
  - Confirm it exits `0`, prints the supported recipe names, and does not launch
    either app.
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/40-live-ab-recipes.md scripts/roastty-app/README.md`
- Run `git diff --check`.
- If both debug apps are built, run the default smoke recipe with no `--recipe`
  argument:
  - `scripts/roastty-app/live-ab-smoke.sh --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - Confirm the harness exits `0`, prints one JSON summary object, includes
    `"recipe":"smoke"`, and preserves Experiment 39's default behavior.
- If both debug apps are built, run the ASCII recipe with permissive thresholds:
  - `scripts/roastty-app/live-ab-smoke.sh --recipe ascii-grid --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - Confirm the harness exits `0`, prints one JSON summary object, includes
    `"recipe":"ascii-grid"`, includes same-sized captures, and cleans up only
    the launched PID trees.
- Run the ASCII recipe with strict thresholds:
  - `bash -lc 'scripts/roastty-app/live-ab-smoke.sh --recipe ascii-grid; rc=$?; echo strict_exit=$rc; exit 0'`
  - Record the current strict verdict and metrics. Strict visual parity is not
    required for this experiment unless the current app state already achieves
    it.
- Run
  `pgrep -fl '[G]hostty.app/Contents/MacOS/ghostty|[R]oastty.app/Contents/MacOS/roastty' || true`
  and verify no launched app processes remain.
- Run `git status --short` and verify no screenshots or generated artifacts are
  in the repo.

**Pass** = named recipes work without regressing the default smoke behavior,
`ascii-grid` can run live through the A/B harness, JSON identifies the recipe,
screenshots remain outside the repo, and launched app processes are cleaned up.

**Partial** = the recipe layer is syntax-checked and documented, but a local
app-build, accessibility, screen-recording, or live-window condition prevents a
full live run; the blocker and next command are recorded.

**Fail** = the recipe layer makes the harness unreliable or cannot be added
without a larger rewrite.

## Design Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, read-only). **Verdict: APPROVED after fixes.**

The first review returned `CHANGES REQUIRED` with one Required finding: the
design promised default `smoke` compatibility but only verified the new
`ascii-grid` recipe. Fixed by adding a permissive live run with no `--recipe`
argument and requiring the JSON summary to include `"recipe":"smoke"`.

The focused re-review approved the fix and found no new Required issues.

## Result

**Result:** Pass

Added named recipe support to `scripts/roastty-app/live-ab-smoke.sh`:

- `--list-recipes` prints supported recipes without launching either app.
- `--recipe <name>` selects the command recipe.
- `smoke` remains the default recipe and preserves Experiment 39's no-argument
  behavior.
- `ascii-grid` clears the terminal, prints a timestamped recipe marker plus
  fixed ASCII rows, and sleeps long enough for the harness to capture before the
  prompt returns.
- The JSON summary now includes `recipe`.
- The harness still keeps screenshots outside the repo, uses the IOSurface-safe
  Roastty full-screen-plus-crop path, invokes `pngdiff.swift` through `swift`,
  and cleans up only the launched PID trees after expected-path verification.

Updated `scripts/roastty-app/README.md` with recipe usage and recorded the
durable recipe commands in the Issue 802 Operating notes. The Issue 802
experiment index now marks Experiment 40 `Pass`.

Verification:

- `bash -n scripts/roastty-app/live-ab-smoke.sh`
- `scripts/roastty-app/live-ab-smoke.sh --list-recipes`
  - Exited `0`.
  - Printed `smoke` and `ascii-grid`.
  - Did not launch either app.
- Default smoke compatibility run:
  - `scripts/roastty-app/live-ab-smoke.sh --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - Exited `0`.
  - Launched Ghostty PID `49670` and Roastty PID `49685`.
  - Printed one JSON summary object with `recipe: smoke`, `verdict: PASS`,
    `marker: ISSUE802_AB_SMOKE_20260610-100021`, `diff_exit_status: 0`,
    `mismatch_ratio: 1`, and `mean_channel_delta: 107.9859555`.
  - The trap killed Ghostty descendants `49678`, `49679`, Ghostty PID `49670`,
    Roastty descendant `49692`, and Roastty PID `49685`.
- ASCII recipe permissive run:
  - `scripts/roastty-app/live-ab-smoke.sh --recipe ascii-grid --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - Exited `0`.
  - Launched Ghostty PID `48304` and Roastty PID `48318`.
  - Printed one JSON summary object with `recipe: ascii-grid`, `verdict: PASS`,
    `diff_exit_status: 0`, `mismatch_ratio: 1`, and
    `mean_channel_delta: 107.5086925`.
  - The trap killed Ghostty descendants `48311`, `48312`, Ghostty PID `48304`,
    Roastty descendant `48325`, and Roastty PID `48318`.
- ASCII recipe strict run:
  - `bash -lc 'scripts/roastty-app/live-ab-smoke.sh --recipe ascii-grid; rc=$?; echo strict_exit=$rc; exit 0'`
  - Harness exited `1`, wrapper printed `strict_exit=1`.
  - Launched Ghostty PID `48542` and Roastty PID `48556`.
  - Printed one JSON summary object with `recipe: ascii-grid`, `verdict: FAIL`,
    `diff_exit_status: 1`, `mismatch_ratio: 1`, and
    `mean_channel_delta: 110.90070025`.
  - The trap killed Ghostty descendants `48550`, `48549`, Ghostty PID `48542`,
    Roastty descendant `48564`, and Roastty PID `48556`.
- `pgrep -fl '[G]hostty.app/Contents/MacOS/ghostty|[R]oastty.app/Contents/MacOS/roastty' || true`
  - no output after cleanup.
- `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/40-live-ab-recipes.md scripts/roastty-app/README.md`
- `git diff --check`
- `git status --short`
  - no screenshot or PNG artifacts in the repo.

## Conclusion

The live A/B harness now has the start of a feature-recipe surface instead of a
single baked-in smoke command. `ascii-grid` gives later Phase-D work a stable
ASCII content recipe that captures while the command is still sleeping, and the
JSON output identifies which recipe produced the metric.

Strict parity still fails, as expected, and this experiment does not claim
otherwise. The next Phase-D experiments can add recipes for colors, clear,
scrollback, selection, and other known feature areas, or start turning recipe
metrics into per-feature thresholds.

## Completion Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, read-only). **Verdict: APPROVED after fixes.**

The first completion review found one Required compatibility issue: the default
`smoke` recipe changed the Experiment 39 marker contract from
`ISSUE802_AB_SMOKE_<timestamp>` to `ISSUE802_AB_smoke_<timestamp>`. Fixed by
restoring the exact uppercase `ISSUE802_AB_SMOKE_<timestamp>` marker for the
`smoke` recipe while keeping recipe-specific markers for other recipes.

After the fix, the default permissive live run exited `0` and its JSON summary
included `recipe: smoke` with `marker: ISSUE802_AB_SMOKE_20260610-100021`.

The focused re-review approved the marker fix and found no new Required issues.
