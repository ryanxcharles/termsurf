# Experiment 52: Probe Embedded Right-Edge Correction

## Description

Experiment 51 proved that embedded Surfari PDF selection has a right-edge
boundary gap: direct-copy diagnostic cells repeatedly copy `LEFT834 MID834` but
never `RIGHT834`, even though the exact fixture copies all three tokens in
standalone PDFKit and standalone `WKWebView`.

The next step is to test fix candidates without making product behavior
permanent. The most likely boundary is Surfari's synthetic mouse-event path:
Ghostboard forwards web coordinates, Surfari converts them into `NSEvent`
locations, and WebKit/PDFKit's embedded PDF selection appears to stop short of
the right edge. This experiment should probe whether an env-gated coordinate or
drag-extension correction makes embedded Surfari include `RIGHT834`.

## Changes

- Add env-gated diagnostic correction support in
  `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm`, enabled only by
  experiment variables such as:
  - `TERMSURF_SURFARI_PDF_SELECTION_EDGE_PROBE=1`;
  - `TERMSURF_SURFARI_PDF_SELECTION_EDGE_DELTA_X={points}`.
- Under the flag only, adjust drag/mouse-up behavior for PDF selection probes
  without changing normal Surfari behavior:
  - candidate A: add a positive x delta to drag and mouse-up events while the
    left button is down;
  - candidate B: emit one extra synthetic drag event farther right before
    mouse-up;
  - candidate C: test whether using the hit-tested target for drag events
    instead of always calling `web_view.mouseDragged` changes selection bounds.
- Keep candidates independently selectable so the result can identify which
  correction works, if any.
- Extend or add a focused harness, tentatively
  `scripts/test-issue-834-surfari-pdf-right-edge-correction.sh`, that:
  - first runs the current no-correction embedded matrix/control cell to
    reproduce `LEFT834 MID834` without `RIGHT834`;
  - then runs the same all-token/over-wide cells with each correction candidate;
  - keeps `TERMSURF_SURFARI_PDF_COPY_DIRECT=1` enabled for the diagnostic copy
    extraction path;
  - keeps the Experiment 50 oracle summary and fixture identity checks from
    Experiment 51 before interpreting any correction result.
- Use a bounded candidate matrix:
  - no correction baseline;
  - positive x deltas such as `8`, `16`, `32`, and `64` points;
  - optional extra-drag variant for the smallest delta that shows movement
    toward `RIGHT834`;
  - stop early only if a candidate copies all three tokens and the summary
    records the skipped cells explicitly.
- For every candidate cell, record:
  - correction mode and x delta;
  - original and adjusted coordinates;
  - mouse trace lines showing the adjustment;
  - primary post-selection/direct-copy tokens and clipboard sample;
  - fallback/select-all tokens and clipboard sample, separately from primary
    evidence;
  - whether `RIGHT834` was copied;
  - whether all three tokens were copied;
  - whether the primary sample is a clean expected-token copy or includes extra
    page text;
  - screenshots and coordinate evidence from the embedded harness.
- Apply this outcome matrix:
  - **edge-delta-fix-candidate:** an env-gated positive x delta copies all three
    tokens in embedded Surfari in the primary post-selection/direct-copy sample;
  - **extra-drag-fix-candidate:** an env-gated extra drag event copies all three
    tokens in the primary post-selection/direct-copy sample;
  - **target-routing-fix-candidate:** changing drag delivery target copies all
    three tokens in the primary post-selection/direct-copy sample;
  - **fallback-only-copy:** expected tokens appear only through fallback/select
    all evidence, not the primary post-selection/direct-copy sample;
  - **right-edge-persists:** oracle and fixture gates are open, corrections run,
    and all candidates still miss `RIGHT834`;
  - **harness-insufficient:** the probe cannot prove whether correction
    candidates were applied or which tokens were copied.
- Map result status:
  - **Pass:** any fix-candidate class or `right-edge-persists`, with complete
    evidence and clipboard restoration;
  - **Partial:** `harness-insufficient` with useful logs;
  - **Fail:** clipboard restoration failure, missing correction traces,
    non-gated behavior change, or no real embedded Surfari run.
- Do not make the correction permanent in this experiment. If a candidate works,
  the next experiment should review and implement the product fix deliberately.
- Update this experiment file with the result.

## Verification

Run hygiene checks:

```bash
bash -n scripts/test-issue-834-surfari-pdf-right-edge-correction.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
```

Run the correction probe:

```bash
rm -rf logs/issue-834-exp52-surfari-pdf-right-edge-correction
scripts/test-issue-834-surfari-pdf-right-edge-correction.sh
```

Pass criteria:

- all behavior-changing code is gated behind explicit experiment environment
  variables;
- the no-correction baseline reproduces the missing-`RIGHT834` pattern;
- correction traces prove which candidate and delta were applied;
- primary post-selection/direct-copy evidence identifies whether `RIGHT834` and
  all three tokens were copied;
- fallback/select-all evidence is recorded separately and cannot by itself
  produce a fix-candidate classification;
- if a candidate copies all expected tokens plus unrelated page text, the result
  records that distinction instead of calling it a clean correction;
- oracle and fixture identity gates remain open before interpreting candidates;
- clipboard restoration succeeds;
- no permanent product behavior change is claimed;
- completion review is recorded.

Partial criteria:

- candidate traces are useful but copied-token evidence is incomplete;
- one correction path cannot be tested, but others produce useful evidence;
- public WebKit behavior prevents proving why a candidate did or did not work.

Failure criteria:

- a correction affects normal behavior without the experiment flag;
- clipboard state is not restored;
- the probe cannot run embedded Surfari;
- the result claims a product fix instead of a fix candidate.

## Design Review

An external Codex review checked the design.

Initial verdict: **Changes required**.

Finding:

- fix-candidate classifications could be polluted by fallback/select-all copy
  evidence, because earlier bounds wrappers combined primary and fallback
  samples when deriving token presence.

Resolution:

- required every candidate to record primary post-selection/direct-copy evidence
  separately from fallback/select-all evidence;
- required fix-candidate classifications to come only from the primary sample;
- added `fallback-only-copy` for expected-token evidence that appears only
  through fallback recovery;
- required the result to distinguish clean expected-token copies from copies
  that include extra unrelated page text.

Follow-up verdict: **Approved**.

The reviewer found no remaining must-fix design issues and approved the
Experiment 52 plan commit.
