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
