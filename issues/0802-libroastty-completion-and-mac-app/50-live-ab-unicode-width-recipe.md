+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 50: Phase E — Unicode-width live A/B recipe

## Description

Phase D now has a repeatable content-region live A/B oracle against the real
Ghostty app. Phase E moves into terminal correctness: Unicode width,
grapheme-break, symbol/Nerd-Font width, and `Terminal::print()` behavior. Before
changing the terminal internals, we need a focused live recipe that exercises
the relevant width and grapheme cases in both apps and records the current
visual gap.

This experiment adds a deterministic `unicode-width` live A/B recipe to the
existing harness. The recipe should draw ASCII guide columns plus representative
Unicode cases using shell escapes instead of literal non-ASCII source text:
combining marks, CJK wide characters, emoji, variation selectors, box/symbol
characters, and a cursor-addressed alignment row. The goal is not to make the
recipe pass strict parity yet; the goal is to make the mismatch measurable and
repeatable so later Phase-E experiments can port the Unicode tables and
`Terminal::print()` with an app-level oracle already in place.

## Changes

- `scripts/roastty-app/live-ab-smoke.sh`
  - Add `unicode-width` to the recipe list and validation.
  - Add a launch-time bootstrap recipe that clears the screen, prints a marker,
    prints ASCII guide columns, then prints deterministic Unicode-width cases
    using escaped codepoints.
  - Hold the final frame through capture with the existing configurable hold
    duration.
  - Preserve the content-region/default comparison behavior from Experiment 49.
- `scripts/roastty-app/README.md`
  - Document the new `unicode-width` recipe and the classes of Unicode cases it
    exercises.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Add this experiment to the index as `Designed`.
  - After implementation, add an Operating note with the current recipe metrics
    and the next Phase-E target.

## Verification

- Run shell syntax checks:
  - `bash -n scripts/roastty-app/live-ab-smoke.sh`
  - `bash -n scripts/roastty-app/live-ab-matrix.sh`
- Run recipe discovery:
  - `scripts/roastty-app/live-ab-smoke.sh --list-recipes`
  - Confirm `unicode-width` is listed with the existing recipes.
- Run the new recipe with permissive thresholds:
  - `scripts/roastty-app/live-ab-smoke.sh --recipe unicode-width --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - Confirm it exits `0`, emits both full-window and content-region metrics, and
    writes screenshots outside the repo.
  - Visually inspect the content-region captures and confirm both apps show the
    marker, guide rows, and Unicode test rows.
  - Record the current content-region metrics.
- Run the full default matrix:
  - `scripts/roastty-app/live-ab-matrix.sh`
  - Confirm it exits `0` and includes `unicode-width` in the JSON Lines output.
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/50-live-ab-unicode-width-recipe.md scripts/roastty-app/README.md`
- Run `git diff --check`.
- Run cleanup checks:
  - `scripts/roastty-app/stop-app.sh || true`
  - `scripts/ghostty-app/stop-app.sh || true`
  - `pgrep -fl '[G]hostty.app/Contents/MacOS/ghostty|[R]oastty.app/Contents/MacOS/roastty' || true`
  - `find /tmp -maxdepth 1 -name 'termsurf-ab-bootstrap.*' -print`
- Run `git status --short` and verify no screenshots or generated artifacts are
  in the repo.

**Pass** = `unicode-width` is a repeatable live A/B recipe, appears in the
matrix, emits content-region metrics, visually exercises the intended Unicode
classes in both apps, and leaves no processes/temp dirs/artifacts behind.

**Partial** = the recipe exists and runs standalone, but full-matrix inclusion
or visual inspection is blocked by local app/window/screen-recording conditions;
record the blocker and next command.

**Fail** = the harness cannot deliver escaped Unicode codepoints reliably enough
for a repeatable live A/B recipe.

## Design Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, read-only). **Verdict: APPROVED.**

The reviewer found no issues. It verified that the issue README links Experiment
50 as `Designed`, the experiment has the required sections, the scope is narrow,
the test-first Unicode-width recipe is sensible before terminal internals
changes, and the verification plan includes concrete syntax, recipe discovery,
standalone run, matrix run, formatting, cleanup, and artifact-hygiene checks.

## Result

**Result:** Pass

Implemented the `unicode-width` live A/B recipe:

- `scripts/roastty-app/live-ab-smoke.sh`
  - adds `unicode-width` to recipe discovery and validation;
  - draws a marker, ASCII guide columns, combining marks, CJK wide text,
    emoji/variation-selector samples, box/symbol glyphs, and a cursor-addressed
    alignment row;
  - uses escaped UTF-8 byte sequences in the harness source, so the script stays
    mostly ASCII while the launched bash recipe prints real codepoints at
    runtime;
  - reuses the existing held-frame and content-region diff behavior.
- `scripts/roastty-app/README.md`
  - documents the new recipe and the Unicode classes it exercises.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - records the current metrics and marks Experiment 50 `Pass`.

Verification:

- `bash -n scripts/roastty-app/live-ab-smoke.sh`
- `bash -n scripts/roastty-app/live-ab-matrix.sh`
- `scripts/roastty-app/live-ab-smoke.sh --list-recipes`
  - Printed `smoke`, `ascii-grid`, `color-grid`, `clear-after`, `alt-screen`,
    `scroll-output`, and `unicode-width`.
- Standalone Unicode-width recipe:
  - `scripts/roastty-app/live-ab-smoke.sh --recipe unicode-width --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - Exited `0`.
  - Content-region metric: `mean_channel_delta=3.812397048611111`,
    `mismatch_ratio=0.040785416666666664`.
  - Full-window metric: `mean_channel_delta=3.9459993077531648`,
    `mismatch_ratio=0.08596073971518987`.
  - Visual inspection of
    `/Users/ryan/.cache/termsurf/shots/ghostty-ab-content-20260610-120551.png`
    and
    `/Users/ryan/.cache/termsurf/shots/roastty-ab-content-20260610-120551.png`
    confirmed both apps showed the marker, guide rows, and Unicode test rows.
    The captures also showed expected Roastty differences in width/fallback
    behavior, especially around CJK, emoji, symbol, and alignment rows.
- Full default matrix:
  - `scripts/roastty-app/live-ab-matrix.sh`
  - Exited `0`.
  - Emitted seven JSON Lines objects, including `unicode-width`.
  - Final matrix `unicode-width` content-region metric:
    `mean_channel_delta=3.8124979166666666`,
    `mismatch_ratio=0.04077708333333333`.
- `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/50-live-ab-unicode-width-recipe.md scripts/roastty-app/README.md`

## Conclusion

Phase E now has a live app-level Unicode-width oracle. The new recipe does not
fix Unicode width or grapheme behavior, but it makes the gap repeatable and
visible in both standalone and full-matrix A/B runs. The next Phase-E experiment
should start porting Unicode width/grapheme behavior behind this recipe, using
the content-region metric and visual alignment rows as the regression target.

## Completion Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, read-only). **Verdict: APPROVED.**

The reviewer found no issues. It independently verified shell syntax for both
harness scripts, recipe discovery, `git diff --check`, worktree scope, cleanup
checks for launched app binaries and bootstrap temp dirs, and that no result
commit existed before review. It did not rerun the standalone smoke or full
matrix because those launch GUI apps and the review was read-only.
