+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 48: Phase D — hold live A/B recipe frames through capture

## Description

Experiment 47 made live A/B recipe delivery reliable by running recipes from
launch-time shell bootstrap files. The current captures now show each recipe
executing in both apps, but they also show a harness-induced mismatch after the
recipe finishes: Ghostty and Roastty return to different shell prompts/cwds (`~`
vs `termsurf`) and render different cursor/prompt state. That prompt difference
is not the recipe behavior under test, and it pollutes the screenshot diff
before we can reason about stricter visual thresholds.

This experiment keeps each recipe's final test frame active through capture.
Instead of letting the startup shell return to its prompt before screenshots are
taken, each recipe should sleep long enough after drawing the intended frame.
The harness still kills the launched app PID trees at the end, so the held shell
does not need to return naturally.

## Changes

- `scripts/roastty-app/live-ab-smoke.sh`
  - Add one configurable hold duration for live A/B recipes, defaulting to a
    value long enough for the current launch, sizing, and capture flow.
  - Replace per-recipe short sleeps with that hold duration.
  - Preserve recipe names, marker generation, launch-time bootstrap delivery,
    full-screen-plus-crop capture, JSON output, and exact launched-PID-tree
    cleanup.
  - Do not hide or crop out app chrome in this experiment; only remove
    post-recipe shell prompt/cwd noise.
- `scripts/roastty-app/README.md`
  - Document the held-frame behavior and the optional hold-duration environment
    variable.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Add this experiment to the index as `Designed`.
  - After implementation, update Operating notes with the held-frame result and
    any improved diff metrics.

## Verification

- Run shell syntax checks:
  - `bash -n scripts/roastty-app/live-ab-smoke.sh`
  - `bash -n scripts/roastty-app/live-ab-matrix.sh`
- Run non-GUI recipe discovery:
  - `scripts/roastty-app/live-ab-smoke.sh --list-recipes`
- Run representative live A/B recipes:
  - `scripts/roastty-app/live-ab-smoke.sh --recipe smoke --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - `scripts/roastty-app/live-ab-smoke.sh --recipe ascii-grid --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - Confirm both captures visibly contain the marker/expected rows and do not
    show the returned shell prompt.
  - Record the new diff metrics and compare them to the pre-Experiment-48
    permissive metrics from Experiment 47.
- Run the full default matrix:
  - `scripts/roastty-app/live-ab-matrix.sh`
  - Confirm it exits `0`, emits one JSON Lines object for every recipe, and
    every recipe's captures have direct execution evidence without returned
    prompt/cwd noise.
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/48-live-ab-held-recipe-frame.md scripts/roastty-app/README.md`
- Run `git diff --check`.
- Run
  `pgrep -fl '[G]hostty.app/Contents/MacOS/ghostty|[R]oastty.app/Contents/MacOS/roastty' || true`
  and verify no launched app processes remain.
- Run `find /tmp -maxdepth 1 -name 'termsurf-ab-bootstrap.*' -print` and verify
  no bootstrap temp dirs remain.
- Run `git status --short` and verify no screenshots or generated artifacts are
  in the repo.

**Pass** = representative and matrix recipes still execute in both apps, the
captured frames no longer include returned shell prompts, diff metrics improve
or the remaining differences are attributable to app/rendering rather than
post-recipe prompt/cwd noise, screenshots remain outside the repo, and no app
processes or bootstrap temp dirs remain.

**Partial** = held frames work for representative recipes but the full matrix is
blocked by local app/window/screen-recording conditions; record the exact
blocker and next command.

**Fail** = the harness cannot reliably capture before prompts return without a
larger recipe/capture redesign.

## Design Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, read-only). **Verdict: APPROVED.**

The reviewer found no Required issues. It noted one Optional concern: prompt
absence is still manually verified by visual inspection, which is acceptable for
this visual harness but may need an automated cue or OCR/check helper if prompt
contamination recurs. It also noted a documentation nit: the README update
should be unconditional because the design adds a configurable hold duration;
fixed before the plan commit.

## Result

**Result:** Pass

Implemented held live A/B recipe frames:

- `scripts/roastty-app/live-ab-smoke.sh`
  - adds `HOLD_SECONDS="${TERMSURF_AB_HOLD_SECONDS:-20}"`;
  - replaces each recipe's short `sleep 2` with the configurable hold duration;
  - keeps launch-time bootstrap, full-screen-plus-crop capture, recipe names,
    JSON output, and exact launched-PID-tree cleanup unchanged;
  - hardens activation verification after a full-matrix run exposed a transient
    System Events state where asking for the global frontmost process returned
    no process during rapid app launches. The harness now queries the exact
    launched target process by Unix PID, checks that process's own `frontmost`
    property, and fails capture if activation cannot be proven.
- `scripts/roastty-app/README.md`
  - documents held-frame behavior and `TERMSURF_AB_HOLD_SECONDS`.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - records the operating note and marks Experiment 48 `Pass`.

Verification:

- `bash -n scripts/roastty-app/live-ab-smoke.sh`
- `bash -n scripts/roastty-app/live-ab-matrix.sh`
- `scripts/roastty-app/live-ab-smoke.sh --list-recipes`
  - Printed `smoke`, `ascii-grid`, `color-grid`, `clear-after`, `alt-screen`,
    and `scroll-output`.
- Representative smoke recipe:
  - `scripts/roastty-app/live-ab-smoke.sh --recipe smoke --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - Exited `0`.
  - Visual inspection confirmed both captures contained the marker and no
    returned shell prompt.
  - Diff metric improved from Exp 47's smoke matrix metric
    `mean_channel_delta=1.8266029222705695`,
    `mismatch_ratio=0.06408425632911392` to
    `mean_channel_delta=1.68709590090981`, `mismatch_ratio=0.06260878164556961`
    in the representative run.
- Representative ASCII recipe:
  - `scripts/roastty-app/live-ab-smoke.sh --recipe ascii-grid --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - Exited `0`.
  - Visual inspection confirmed both captures contained the marker and expected
    ASCII rows with no returned shell prompt.
  - Diff metric improved from Exp 47's ASCII matrix metric
    `mean_channel_delta=2.8211517256724683`,
    `mismatch_ratio=0.07424643987341772` to
    `mean_channel_delta=2.708811931368671`, `mismatch_ratio=0.07295638844936708`
    in the representative run.
- First full default matrix attempt:
  - Exposed a transient hard activation failure in rapid repeated launches:
    System Events sometimes reported no frontmost process, causing recipes to
    fail before capture despite launched PID-tree cleanup succeeding.
  - Fixed by verifying the launched target process's own `frontmost` property by
    Unix PID after setting it frontmost, instead of querying the global
    frontmost process or the first process with a matching name.
- First exact-PID matrix attempt:
  - Failed closed before capture because the initial AppleScript selector did
    not resolve a process by Unix PID in this environment.
  - Fixed by selecting `first process whose unix id is target_pid`, then
    confirmed with a representative smoke run before repeating the matrix.
- Full default matrix after the activation-verification fix:
  - `scripts/roastty-app/live-ab-matrix.sh`
  - Exited `0`.
  - Emitted six JSON Lines objects, one each for `smoke`, `ascii-grid`,
    `color-grid`, `clear-after`, `alt-screen`, and `scroll-output`.
  - Visual spot-check confirmed held prompt-free frames persisted beyond the
    representative recipes, including `color-grid`.
  - Final matrix metrics:
    - `smoke`: `mean_channel_delta=1.6865060077136076`,
      `mismatch_ratio=0.06261471518987342`
    - `ascii-grid`: `mean_channel_delta=2.678706363726266`,
      `mismatch_ratio=0.07274970332278481`
    - `color-grid`: `mean_channel_delta=5.112856259889241`,
      `mismatch_ratio=0.1104207871835443`
    - `clear-after`: `mean_channel_delta=2.1038679291930378`,
      `mismatch_ratio=0.0668507713607595`
    - `alt-screen`: `mean_channel_delta=2.1731665348101266`,
      `mismatch_ratio=0.0672987539556962`
    - `scroll-output`: `mean_channel_delta=5.360994239517405`,
      `mismatch_ratio=0.09954113924050632`
- `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/48-live-ab-held-recipe-frame.md scripts/roastty-app/README.md`
- `git diff --check`
- `scripts/roastty-app/stop-app.sh || true`
- `scripts/ghostty-app/stop-app.sh || true`
- `pgrep -fl '[G]hostty.app/Contents/MacOS/ghostty|[R]oastty.app/Contents/MacOS/roastty' || true`
  - no output after cleanup.
- `find /tmp -maxdepth 1 -name 'termsurf-ab-bootstrap.*' -print`
  - no output after cleanup.
- `git status --short`
  - no screenshots or generated artifacts in the repo.

## Conclusion

The live A/B harness now captures recipe frames before Ghostty and Roastty
return to different shell prompts/cwds. That removes a known harness-induced
source of visual noise and slightly improves representative diff metrics. The
remaining differences are now more plausibly app/rendering differences: app
chrome/titlebar, debug banner text, font metrics/wrapping, colors, and cursor
placement. The next Phase-D parity step should either normalize the comparison
region or start fixing those concrete rendering deltas.

## Completion Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, read-only). **Initial verdict: CHANGES REQUIRED. Final verdict:
APPROVED.**

The reviewer found one Required issue: activation was still keyed by process
name, so another existing Ghostty/Roastty process with the same name could have
satisfied the frontmost gate while the launched target process remained
occluded. Fixed by making `activate` require the launched PID and by selecting
`first process whose unix id is target_pid` in System Events before setting and
checking that exact process's `frontmost` property. A first exact-PID selector
attempt failed closed before capture, was corrected, and the representative
smoke plus full six-recipe matrix passed after the correction. The reviewer then
approved the completed experiment.
