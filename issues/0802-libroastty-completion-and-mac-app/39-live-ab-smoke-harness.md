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

# Experiment 39: Phase D — live A/B smoke harness

## Description

Phase D needs a repeatable in-session harness that launches the real Ghostty app
and the copied Roastty app in the same run, drives the same simple terminal
content into each app, captures both windows outside the repo, and compares the
captures with the `pngdiff.swift` metric from Experiment 38.

This experiment intentionally proves only the first live A/B smoke path. It does
not define the full UI feature matrix, crop every interesting region, or tune
feature-specific thresholds. The goal is to compose the already-proven app
helpers into one script that later experiments can extend: start both apps,
activate each app before keyboard injection, send a deterministic ASCII command,
capture both app windows, run the PNG diff, print a machine-readable verdict,
and stop the launched apps even on failure.

## Changes

- `scripts/roastty-app/live-ab-smoke.sh`
  - Add a Bash harness for the first live A/B visual smoke check.
  - Start Ghostty with `scripts/ghostty-app/start-app.sh` and Roastty with
    `scripts/roastty-app/start-app.sh`.
  - Use the existing scoped stop scripts in a trap so launched app processes are
    cleaned up on success, failure, or interruption.
  - Activate each app before driving keyboard input, with the existing
    activate-first and warmup-key lessons from Experiment 5.
  - Drive a deterministic ASCII shell command into both apps, producing a unique
    marker line that should render in each terminal window.
  - Capture the Ghostty window with the existing
    `scripts/ghostty-app/screenshot.sh` wrapper.
  - Capture the Roastty window with the known IOSurface-safe path from
    Experiments 15 and 20: bring the window on screen, take a full-screen
    capture outside the repo, resolve the Roastty window bounds with
    `scripts/roastty-app/list-windows.swift`, crop the full-screen capture with
    `scripts/roastty-app/crop.swift`, and keep both the full-screen and cropped
    images under `${TERMSURF_SHOT_DIR:-$HOME/.cache/termsurf/shots}`.
  - Verify both captures exist and have dimensions accepted by the diff helper.
  - Compare the two captures with
    `swift scripts/roastty-app/pngdiff.swift <ghostty.png> <roastty-crop.png>`
    and pass through optional threshold flags: `--max-mismatch-ratio` and
    `--max-mean-channel-delta`.
  - Print one JSON summary object to stdout containing the harness verdict,
    Ghostty/Roastty PIDs, screenshot paths, diff metrics, thresholds, and the
    command marker. Diagnostics go to stderr.
  - Never write screenshots, logs, or generated artifacts inside the repo.
- `scripts/roastty-app/README.md`
  - Document the live A/B smoke harness usage and screenshot policy.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Add this experiment to the index as `Designed`.
  - After implementation, record the harness under Operating notes and update
    the Phase-D roadmap checkbox only if the script successfully points the
    Phase-A harness at the Roastty app in a repeatable run.

## Verification

- Run shell syntax checks:
  - `bash -n scripts/roastty-app/live-ab-smoke.sh`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/39-live-ab-smoke-harness.md scripts/roastty-app/README.md`
- Run `git diff --check`.
- If both debug apps are already built, run the harness once:
  - `scripts/roastty-app/live-ab-smoke.sh --max-mismatch-ratio 1 --max-mean-channel-delta 255`
  - The permissive thresholds make this first smoke prove harness mechanics
    rather than visual parity.
  - Confirm stdout is one JSON summary object.
  - Confirm the summary includes Ghostty, Roastty full-screen, and Roastty
    cropped screenshot paths outside the repo, plus the nested diff metrics from
    `pngdiff.swift`.
  - Confirm the harness exits `0` with permissive thresholds.
- Run a strict-threshold variant if the first run succeeds:
  - `scripts/roastty-app/live-ab-smoke.sh`
  - Record the actual diff verdict and metrics; do not require exact visual
    parity yet unless the current app state genuinely achieves it.
- Run `scripts/ghostty-app/stop-app.sh` and `scripts/roastty-app/stop-app.sh`
  after live verification, even though the harness has a trap.
- Run `git status --short` and verify no screenshots or generated artifacts are
  in the repo.

**Pass** = the script can run a live Ghostty/Roastty A/B smoke check, capture
both windows outside the repo, invoke `pngdiff.swift`, print a machine-readable
summary, clean up spawned apps, and leave no screenshot artifacts in the working
tree.

**Partial** = the script is correct and syntax-checked, but a local app-build,
screen-recording, accessibility, or live-window condition prevents a full run;
the blocker and next command are recorded.

**Fail** = the existing app helpers cannot be composed into a reliable A/B smoke
harness without a different approach.

## Design Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, read-only). **Verdict: APPROVED after fixes.**

The first review returned `CHANGES REQUIRED` with two Required findings:

- The design used the existing Roastty screenshot wrapper, but prior live-render
  experiments showed Roastty's IOSurface/Metal window defeats `screencapture -l`
  / `-R`; fixed by designing the Roastty capture path around full-screen
  capture, `list-windows.swift`, and `crop.swift`, with dimension verification
  before diffing.
- The design invoked `pngdiff.swift` directly even though it is a Swift source
  file; fixed by specifying
  `swift scripts/roastty-app/pngdiff.swift <ghostty.png> <roastty-crop.png>`.

The focused re-review approved both fixes and found no new Required issues.
