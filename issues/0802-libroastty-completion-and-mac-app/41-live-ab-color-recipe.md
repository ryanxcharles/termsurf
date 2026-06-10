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

# Experiment 41: Phase D — color live A/B recipe

## Description

Experiment 40 added the recipe layer for the live Ghostty/Roastty A/B harness.
The next useful feature recipe is color rendering: ANSI palette colors,
background colors, bold brightening, and truecolor are core terminal behaviors
that later conformance work needs to compare against the real app.

This experiment adds a `color-grid` recipe to `live-ab-smoke.sh`. The recipe is
a visual oracle only: it prints deterministic ANSI / truecolor rows and records
the current A/B screenshot-diff metrics. It does not require strict visual
parity yet, because the existing strict A/B recipes still fail. The value is
creating a repeatable live color fixture that later renderer/config work can use
as a regression target.

## Changes

- `scripts/roastty-app/live-ab-smoke.sh`
  - Add `color-grid` to `--list-recipes`.
  - Add `--recipe color-grid`.
  - Update the `--help` / usage text to include `color-grid`.
  - The recipe clears the screen, moves the cursor home, prints a timestamped
    marker, then prints deterministic rows covering:
    - basic ANSI foreground colors,
    - ANSI background colors,
    - bold/bright foreground colors,
    - truecolor foreground/background samples.
  - Keep the command self-contained and sleeping long enough for capture before
    the shell prompt returns.
  - Include the existing `recipe` JSON field with value `color-grid`.
  - Preserve `smoke` default compatibility, the `ascii-grid` recipe, screenshot
    policy, IOSurface-safe Roastty capture path, `swift pngdiff.swift`, and
    exact launched-PID-tree cleanup.
- `scripts/roastty-app/README.md`
  - Document `color-grid`.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Add this experiment to the index as `Designed`.
  - After implementation, record `color-grid` under Operating notes if the live
    run succeeds.

## Verification

- Run shell syntax checks:
  - `bash -n scripts/roastty-app/live-ab-smoke.sh`
- Run recipe discovery:
  - `scripts/roastty-app/live-ab-smoke.sh --list-recipes`
  - Confirm it exits `0`, prints `smoke`, `ascii-grid`, and `color-grid`, and
    does not launch either app.
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/41-live-ab-color-recipe.md scripts/roastty-app/README.md`
- Run `git diff --check`.
- If both debug apps are built, run the color recipe with permissive thresholds:
  - `scripts/roastty-app/live-ab-smoke.sh --recipe color-grid --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - Confirm the harness exits `0`, prints one JSON summary object, includes
    `"recipe":"color-grid"`, includes same-sized captures, and cleans up only
    the launched PID trees.
- Run the color recipe with strict thresholds:
  - `bash -lc 'scripts/roastty-app/live-ab-smoke.sh --recipe color-grid; rc=$?; echo strict_exit=$rc; exit 0'`
  - Record the current strict verdict and metrics. Strict visual parity is not
    required for this experiment unless the current app state already achieves
    it.
- Run
  `pgrep -fl '[G]hostty.app/Contents/MacOS/ghostty|[R]oastty.app/Contents/MacOS/roastty' || true`
  and verify no launched app processes remain.
- Run `git status --short` and verify no screenshots or generated artifacts are
  in the repo.

**Pass** = `color-grid` is discoverable, runs live through the A/B harness, JSON
identifies the recipe, screenshots stay outside the repo, strict metrics are
recorded without overclaiming parity, and launched app processes are cleaned up.

**Partial** = the recipe is syntax-checked and documented, but a local
app-build, accessibility, screen-recording, or live-window condition prevents a
full live run; the blocker and next command are recorded.

**Fail** = the recipe makes the harness unreliable or cannot be added without a
larger rewrite.

## Design Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, read-only). **Verdict: APPROVED.**

The reviewer found no Required issues. It noted one Optional completeness issue:
the design mentioned `--list-recipes`, docs, and `--recipe color-grid`, but did
not explicitly call out the script's `--help` / usage text, which still listed
only `smoke|ascii-grid`. Fixed by adding the usage text update to the planned
changes.

## Result

**Result:** Pass

Added `color-grid` to the live A/B harness recipe layer:

- `--list-recipes` now prints `smoke`, `ascii-grid`, and `color-grid`.
- `--help` / usage text now lists `smoke|ascii-grid|color-grid`.
- `--recipe color-grid` emits a deterministic fixture with:
  - basic ANSI foreground colors,
  - ANSI background colors,
  - bold/bright foreground colors,
  - truecolor foreground/background samples.
- The recipe command clears the screen, moves the cursor home, prints a
  timestamped marker, and sleeps before the prompt returns so the capture sees
  the fixture.
- Existing `smoke` and `ascii-grid` recipes, IOSurface-safe Roastty capture,
  `swift pngdiff.swift`, screenshot policy, and exact launched-PID cleanup are
  preserved.

Updated `scripts/roastty-app/README.md` and the Issue 802 Operating notes with
`color-grid`. The Issue 802 experiment index now marks Experiment 41 `Pass`.

Verification:

- `bash -n scripts/roastty-app/live-ab-smoke.sh`
- `scripts/roastty-app/live-ab-smoke.sh --list-recipes`
  - Exited `0`.
  - Printed `smoke`, `ascii-grid`, and `color-grid`.
  - Did not launch either app.
- `scripts/roastty-app/live-ab-smoke.sh --help`
  - Exited `0`.
  - Printed usage including `--recipe smoke|ascii-grid|color-grid`.
- Color recipe permissive run:
  - `scripts/roastty-app/live-ab-smoke.sh --recipe color-grid --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - Exited `0`.
  - Launched Ghostty PID `52124` and Roastty PID `52139`.
  - Captured both comparison images at `1600x1264`.
  - Printed one JSON summary object with `recipe: color-grid`, `verdict: PASS`,
    `diff_exit_status: 0`, `mismatch_ratio: 0.9541193631329113`, and
    `mean_channel_delta: 9.873654692444621`.
  - The trap killed Ghostty descendants `52132`, `52133`, Ghostty PID `52124`,
    Roastty descendant `52146`, and Roastty PID `52139`.
- Color recipe strict run:
  - `bash -lc 'scripts/roastty-app/live-ab-smoke.sh --recipe color-grid; rc=$?; echo strict_exit=$rc; exit 0'`
  - Harness exited `1`, wrapper printed `strict_exit=1`.
  - Launched Ghostty PID `52369` and Roastty PID `52383`.
  - Captured both comparison images at `1000x1000`.
  - Printed one JSON summary object with `recipe: color-grid`, `verdict: FAIL`,
    `diff_exit_status: 1`, `mismatch_ratio: 1`, and
    `mean_channel_delta: 107.5086925`.
  - The trap killed Ghostty descendants `52376`, `52377`, Ghostty PID `52369`,
    Roastty descendant `52390`, and Roastty PID `52383`.
- `pgrep -fl '[G]hostty.app/Contents/MacOS/ghostty|[R]oastty.app/Contents/MacOS/roastty' || true`
  - no output after cleanup.
- `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/41-live-ab-color-recipe.md scripts/roastty-app/README.md`
- `git diff --check`
- `git status --short`
  - no screenshot or PNG artifacts in the repo.

## Conclusion

The live A/B harness now has a color fixture covering ANSI foregrounds,
backgrounds, bold/bright colors, and truecolor samples. This gives future
renderer/config work a repeatable live app comparison target for color behavior.

Strict parity still fails, and this experiment does not claim otherwise. The
important progress is that color behavior is now represented in the Phase-D
recipe surface with machine-readable metrics.

## Completion Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, read-only). **Verdict: APPROVED.**

The reviewer found no Required issues. It reported one Optional stale-doc issue:
`scripts/roastty-app/README.md` still said the harness drives an "ASCII marker
command" even though `color-grid` now drives ANSI color escape sequences. Fixed
by rewording the README to "selected recipe command."

The reviewer independently ran `bash -n scripts/roastty-app/live-ab-smoke.sh`,
`scripts/roastty-app/live-ab-smoke.sh --list-recipes`,
`scripts/roastty-app/live-ab-smoke.sh --help`, `git diff --check`, and the
scoped `pgrep` process check. It did not run the live GUI harness.
