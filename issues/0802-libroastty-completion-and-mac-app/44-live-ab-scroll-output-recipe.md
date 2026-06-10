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

# Experiment 44: Phase D — scroll-output live A/B recipe

## Description

The live A/B harness now covers text, colors, clear-screen behavior, and
alternate-screen cursor addressing. Another core behavior from Experiment 20's
conformance map is ordinary output that exceeds the viewport: the terminal
should scroll to the bottom and show the final rows in order.

This experiment adds a `scroll-output` recipe. The recipe prints a timestamped
marker and a deterministic sequence of numbered rows longer than a 500px or
800x632pt window can display, then sleeps so the harness captures the settled
bottom-of-output state before the prompt returns. Strict visual parity remains a
recorded metric, not a pass requirement.

## Changes

- `scripts/roastty-app/live-ab-smoke.sh`
  - Add `scroll-output` to `--list-recipes`.
  - Add `--recipe scroll-output`.
  - Update the `--help` / usage text to include `scroll-output`.
  - The recipe command:
    - clears the screen,
    - prints a timestamped marker,
    - prints deterministic rows such as `SCROLL_ROW_001` through
      `SCROLL_ROW_080`,
    - sleeps before the prompt returns so the capture sees the final scrolled
      viewport.
  - Include the existing `recipe` JSON field with value `scroll-output`.
  - Preserve all existing recipes, screenshot policy, IOSurface-safe Roastty
    capture, `swift pngdiff.swift`, and exact launched-PID-tree cleanup.
- `scripts/roastty-app/README.md`
  - Document `scroll-output`.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Add this experiment to the index as `Designed`.
  - After implementation, record `scroll-output` under Operating notes if the
    live run succeeds.

## Verification

- Run shell syntax checks:
  - `bash -n scripts/roastty-app/live-ab-smoke.sh`
- Run recipe discovery:
  - `scripts/roastty-app/live-ab-smoke.sh --list-recipes`
  - Confirm it exits `0`, prints all prior recipes plus `scroll-output`, and
    does not launch either app.
- Run help:
  - `scripts/roastty-app/live-ab-smoke.sh --help`
  - Confirm it exits `0` and usage includes `scroll-output`.
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/44-live-ab-scroll-output-recipe.md scripts/roastty-app/README.md`
- Run `git diff --check`.
- If both debug apps are built, run the scroll recipe with permissive
  thresholds:
  - `scripts/roastty-app/live-ab-smoke.sh --recipe scroll-output --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - Confirm the harness exits `0`, prints one JSON summary object, includes
    `"recipe":"scroll-output"`, includes same-sized captures, and cleans up only
    the launched PID trees.
- Run the scroll recipe with strict thresholds:
  - `bash -lc 'scripts/roastty-app/live-ab-smoke.sh --recipe scroll-output; rc=$?; echo strict_exit=$rc; exit 0'`
  - Record the current strict verdict and metrics. Strict visual parity is not
    required for this experiment unless the current app state already achieves
    it.
- Run
  `pgrep -fl '[G]hostty.app/Contents/MacOS/ghostty|[R]oastty.app/Contents/MacOS/roastty' || true`
  and verify no launched app processes remain.
- Run `git status --short` and verify no screenshots or generated artifacts are
  in the repo.

**Pass** = `scroll-output` is discoverable, runs live through the A/B harness,
JSON identifies the recipe, screenshots stay outside the repo, strict metrics
are recorded without overclaiming parity, and launched app processes are cleaned
up.

**Partial** = the recipe is syntax-checked and documented, but a local
app-build, accessibility, screen-recording, or live-window condition prevents a
full live run; the blocker and next command are recorded.

**Fail** = the recipe makes the harness unreliable or cannot be added without a
larger rewrite.

## Design Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, read-only). **Verdict: APPROVED with no findings.**

## Result

**Result:** Pass

Added `scroll-output` to the live A/B harness recipe layer:

- `--list-recipes` now prints `smoke`, `ascii-grid`, `color-grid`,
  `clear-after`, `alt-screen`, and `scroll-output`.
- `--help` / usage text now lists
  `smoke|ascii-grid|color-grid|clear-after|alt-screen|scroll-output`.
- `--recipe scroll-output` clears the screen, prints a timestamped marker, emits
  `SCROLL_ROW_001` through `SCROLL_ROW_080`, and sleeps before the prompt
  returns so the capture sees the final scrolled viewport.
- Existing recipes, IOSurface-safe Roastty capture, `swift pngdiff.swift`,
  screenshot policy, and exact launched-PID cleanup are preserved.

Updated `scripts/roastty-app/README.md` and the Issue 802 Operating notes with
`scroll-output`. The Issue 802 experiment index now marks Experiment 44 `Pass`.

Verification:

- `bash -n scripts/roastty-app/live-ab-smoke.sh`
- `scripts/roastty-app/live-ab-smoke.sh --list-recipes`
  - Exited `0`.
  - Printed `smoke`, `ascii-grid`, `color-grid`, `clear-after`, `alt-screen`,
    and `scroll-output`.
  - Did not launch either app.
- `scripts/roastty-app/live-ab-smoke.sh --help`
  - Exited `0`.
  - Printed usage including
    `--recipe smoke|ascii-grid|color-grid|clear-after|alt-screen|scroll-output`.
- Scroll-output recipe permissive run:
  - `scripts/roastty-app/live-ab-smoke.sh --recipe scroll-output --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - Exited `0`.
  - Launched Ghostty PID `57809` and Roastty PID `57824`.
  - Captured both comparison images at `1000x1000`.
  - Printed one JSON summary object with `recipe: scroll-output`,
    `verdict: PASS`, `diff_exit_status: 0`, `mismatch_ratio: 1`, and
    `mean_channel_delta: 111.80187825`.
  - The trap killed Ghostty descendants `57817`, `57818`, Ghostty PID `57809`,
    Roastty descendant `57831`, and Roastty PID `57824`.
- Scroll-output recipe strict run:
  - `bash -lc 'scripts/roastty-app/live-ab-smoke.sh --recipe scroll-output; rc=$?; echo strict_exit=$rc; exit 0'`
  - Harness exited `1`, wrapper printed `strict_exit=1`.
  - Launched Ghostty PID `58069` and Roastty PID `58083`.
  - Captured both comparison images at `1000x1000`.
  - Printed one JSON summary object with `recipe: scroll-output`,
    `verdict: FAIL`, `diff_exit_status: 1`, `mismatch_ratio: 1`, and
    `mean_channel_delta: 107.50902775`.
  - The trap killed Ghostty descendants `58076`, `58077`, Ghostty PID `58069`,
    Roastty descendant `58090`, and Roastty PID `58083`.
- `pgrep -fl '[G]hostty.app/Contents/MacOS/ghostty|[R]oastty.app/Contents/MacOS/roastty' || true`
  - no output after cleanup.
- `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/44-live-ab-scroll-output-recipe.md scripts/roastty-app/README.md`
- `git diff --check`
- `git status --short`
  - no screenshot or PNG artifacts in the repo.

## Conclusion

The live A/B harness now has an ordinary long-output fixture that captures the
settled bottom of a scrolled terminal viewport. This moves another Experiment 20
conformance probe into the reusable Phase-D visual comparison surface.

Strict parity still fails, and this experiment does not claim otherwise. The
important progress is that long-output scrolling now has a repeatable live app
comparison recipe with machine-readable metrics.

## Completion Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, read-only). **Verdict: APPROVED with no findings.**

The reviewer independently ran `bash -n scripts/roastty-app/live-ab-smoke.sh`,
`scripts/roastty-app/live-ab-smoke.sh --list-recipes`,
`scripts/roastty-app/live-ab-smoke.sh --help`, `git diff --check`, the scoped
`pgrep` process check, `git status --short`, and source/diff inspection. It did
not run the live GUI harness.
