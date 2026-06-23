# Experiment 51: Rerun Embedded Selection Bounds With Oracle

## Description

Experiment 49 ran the embedded Surfari separated-token matrix and found a strong
right-edge clue: direct-copy diagnostic cells repeatedly copied `LEFT834 MID834`
while missing `RIGHT834`. Experiment 49 stayed Partial because the separated
token fixture did not yet have a standalone PDFKit/WKWebView oracle.

Experiment 50 closed that gap. The exact separated-token fixture copied
`LEFT834 MID834 RIGHT834` from standalone PDFKit and standalone `WKWebView`
through CGEvent, in-process AppKit copy, and menu copy routes. Its embedded
interpretation gate is open.

This experiment should rerun the embedded Surfari matrix with the Experiment 50
oracle evidence wired into the summary. If the same missing-`RIGHT834` pattern
reappears, this experiment should classify it as an embedded right-edge
selection gap rather than a harness/oracle gap.

## Changes

- Add or update a focused embedded matrix harness, tentatively
  `scripts/test-issue-834-surfari-pdf-selection-bounds-with-oracle.sh`.
- Reuse the existing embedded matrix behavior from
  `scripts/test-issue-834-surfari-pdf-selection-bounds.sh`:
  - separated-token fixture;
  - normal external-copy baseline cells;
  - env-gated direct-copy diagnostic cells;
  - drag direction, x-span, y-offset, and delayed-copy probes;
  - clipboard save/restore behavior.
- Require an Experiment 50 oracle summary before interpreting embedded cells:
  - default to
    `logs/issue-834-exp50-separated-token-copy-oracle/separated-token-copy-oracle-summary.json`;
  - allow overriding the path with an environment variable;
  - require `classification = "separated-token-oracle-pass"`;
  - require `embedded_interpretation_gate = "open"`;
  - require PDFKit and standalone `WKWebView` trusted copy evidence for
    `LEFT834 MID834 RIGHT834`;
  - if the oracle summary is missing or not open, classify as `oracle-not-open`
    and do not interpret embedded behavior.
- Before interpreting embedded cells, mechanically compare the Experiment 50
  oracle fixture identity to the current embedded fixture metadata:
  - page geometry must match;
  - font name, size, and encoding must match;
  - text operators, token text, and token positions must match;
  - token box JSON must match;
  - if any fixture identity field differs, classify as `fixture-identity-gap`
    and do not interpret embedded behavior.
- Rerun the embedded Surfari matrix from a clean log directory.
- Record for every embedded cell:
  - cell name and direct-copy mode;
  - fixture extraction status;
  - drag ratios and copy delay;
  - copied tokens and clipboard sample;
  - whether all tokens were copied;
  - whether only left/middle tokens were copied.
- Apply this outcome matrix:
  - **embedded-right-edge-selection-gap:** oracle gate is open, at least one
    direct-copy all-token/over-wide cell copies `LEFT834 MID834`, no embedded
    cell copies `RIGHT834`, the missing token is consistently the rightmost
    token, the required direct all-token and over-wide cells are present, and
    drag/coordinate/screenshot evidence shows those cells target safely past the
    right edge of `RIGHT834`;
  - **embedded-right-edge-candidate:** oracle gate is open and embedded cells
    again suggest a missing-rightmost-token pattern, but recurrence or targeting
    evidence is not strong enough for the full right-edge classification;
  - **embedded-geometry-fix-candidate:** oracle gate is open and at least one
    normal external-copy baseline cell copies all three tokens;
  - **embedded-direct-copy-fix-candidate:** oracle gate is open, normal
    external-copy baseline cells do not copy all tokens, but at least one
    direct-copy diagnostic cell copies all three tokens;
  - **embedded-vertical-selection-gap:** oracle gate is open and copied-token
    success depends primarily on y-offset;
  - **embedded-timing-sensitive-selection:** oracle gate is open and the same
    over-wide geometry succeeds only after a longer mouse-up-to-copy delay or
    repeated copy;
  - **embedded-copy-routing-gap:** oracle gate is open, fixture extraction
    passes, but no embedded path copies meaningful token text;
  - **oracle-not-open:** the Experiment 50 oracle summary is missing, stale, or
    does not open the embedded interpretation gate;
  - **fixture-identity-gap:** the Experiment 50 oracle fixture identity does not
    match the current embedded fixture metadata;
  - **harness-insufficient:** required embedded cells, screenshots, traces,
    clipboard evidence, or restoration evidence are missing.
- Map result status:
  - **Pass:** any embedded classification except `oracle-not-open` or
    `fixture-identity-gap` or `harness-insufficient`, with complete evidence and
    clipboard restoration;
  - **Partial:** `oracle-not-open`, `fixture-identity-gap`, or
    `harness-insufficient` with useful logs;
  - **Fail:** clipboard restoration failure, missing embedded matrix summary,
    product behavior changed outside an experiment flag, or no real embedded
    Surfari run.
- Do not make a product fix in this experiment. If the embedded right-edge gap
  is proven, design the next experiment around fixing that specific embedded
  Surfari/PDF selection boundary.
- Update this experiment file with the result.

## Verification

Run hygiene checks:

```bash
bash -n scripts/test-issue-834-surfari-pdf-selection-bounds-with-oracle.sh
bash -n scripts/test-issue-834-surfari-pdf-selection-bounds.sh
bash -n scripts/test-issue-834-surfari-pdf-selection-copy.sh
git diff --check
git -C webkit/src status --short
```

Run the embedded matrix:

```bash
rm -rf logs/issue-834-exp51-surfari-pdf-selection-bounds-with-oracle
scripts/test-issue-834-surfari-pdf-selection-bounds-with-oracle.sh
```

Pass criteria:

- the Experiment 50 oracle summary is loaded and proves the embedded
  interpretation gate is open;
- the current embedded fixture metadata mechanically matches the Experiment 50
  oracle fixture identity before embedded results are interpreted;
- embedded Surfari is run from a clean log directory;
- each required embedded matrix cell records fixture, drag, trace, screenshot,
  and clipboard evidence;
- `embedded-right-edge-selection-gap`, if selected, is supported by recurrence
  across direct all-token, direction, y-offset, over-wide, and delayed-copy
  cells plus evidence that the drag targeted beyond `RIGHT834`;
- at least one explicit embedded outcome class is selected;
- clipboard restoration succeeds;
- no product behavior change is made;
- completion review is recorded.

Partial criteria:

- the oracle summary is unavailable or not open, but the embedded matrix still
  produces useful logs;
- fixture identity comparison fails but identifies the mismatch cleanly;
- the embedded matrix runs but evidence is incomplete enough to prevent a strong
  embedded classification.

Failure criteria:

- clipboard state is not restored;
- no real embedded Surfari run occurs;
- the result claims a product fix instead of a diagnostic classification;
- product behavior changes without an experiment flag.

## Design Review

An external Codex review checked the design.

Initial verdict: **Changes required**.

Findings:

- the design did not require proving the current embedded fixture still matches
  the Experiment 50 oracle fixture before applying the oracle;
- the `embedded-right-edge-selection-gap` class could be assigned from too
  little targeting evidence.

Resolution:

- added a mechanical fixture-identity comparison gate requiring page geometry,
  font, text operators, token text, token positions, and token boxes to match
  the Experiment 50 oracle fixture identity;
- tightened `embedded-right-edge-selection-gap` to require recurrence across the
  planned direct all-token, direction, y-offset, over-wide, and delayed-copy
  cells plus drag/coordinate/screenshot evidence that the selection targeted
  safely past `RIGHT834`;
- added `fixture-identity-gap` and `embedded-right-edge-candidate` outcome
  classes for useful but weaker evidence.

Follow-up verdict: **Approved**.

The reviewer found no remaining must-fix design issues and approved the
Experiment 51 plan commit.

## Result

**Result:** Pass

Added `scripts/test-issue-834-surfari-pdf-selection-bounds-with-oracle.sh`, a
wrapper around the existing embedded Surfari separated-token matrix. The wrapper
loads the Experiment 50 oracle summary, verifies the oracle gate is open,
compares the current embedded fixture metadata against the oracle fixture
identity, then reruns the 18-cell embedded matrix and reclassifies it with the
oracle evidence available.

Verification:

```bash
bash -n scripts/test-issue-834-surfari-pdf-selection-bounds-with-oracle.sh
bash -n scripts/test-issue-834-surfari-pdf-selection-bounds.sh
bash -n scripts/test-issue-834-surfari-pdf-selection-copy.sh
git diff --check
git -C webkit/src status --short
rm -rf logs/issue-834-exp51-surfari-pdf-selection-bounds-with-oracle
scripts/test-issue-834-surfari-pdf-selection-bounds-with-oracle.sh
```

The successful run was `20260623-002902`. Its summary is:

```text
logs/issue-834-exp51-surfari-pdf-selection-bounds-with-oracle/surfari-pdf-selection-bounds-with-oracle-summary.json
```

The run classified the result as:

```json
{
  "classification": "embedded-right-edge-selection-gap",
  "fixture_identity_match": true,
  "oracle_gate_open": true,
  "overall_result": "pass",
  "right_edge_recurrence": true,
  "targeting_complete": true
}
```

Key evidence:

- The Experiment 50 oracle gate was open.
- The current embedded fixture metadata matched the Experiment 50 oracle fixture
  identity, including page geometry, font, text operators, token text, token
  positions, and token boxes.
- The embedded matrix ran 18 real Surfari cells.
- No embedded cell copied `RIGHT834`.
- Required direct all-token/over-wide cells copied `LEFT834 MID834` while
  missing `RIGHT834`:
  - `all-ltr`;
  - `all-rtl`;
  - `all-y-high`;
  - `all-y-low`;
  - `overwide-delay-025`;
  - `overwide-delay-100`;
  - `overwide-delay-200`.
- Targeting evidence was complete for the same required cells, meaning the drag
  paths reached safely past the expected right edge of `RIGHT834`.
- Clipboard restoration succeeded through the embedded matrix wrapper.

The repeated clipboard sample for the required direct all-token and over-wide
cells was `LEFT834 MID834 LEFT834 MID834`. That proves embedded Surfari can
select/copy PDF text through the direct-copy diagnostic path, but consistently
loses the rightmost token.

## Conclusion

The embedded Surfari PDF copy failure is now classified as an embedded
right-edge selection gap. It is no longer explained by a bad PDF fixture, a bad
standalone WebKit/PDFKit copy oracle, missing embedded fixture identity, a
simple y-offset issue, drag direction, or copy timing delay.

The next experiment should diagnose and fix why embedded Surfari's PDF selection
boundary excludes the right edge of the selected text. The fix should still be
deliberate: this experiment only proves the failure class; it does not ship a
product behavior change.

## Completion Review

An external Codex completion review checked the wrapper, result language, and
final summary.

Initial verdict: **Changes required**.

Findings:

- the first wrapper synthesized page geometry and font constants instead of
  reading them from the current embedded fixture summary;
- the result text therefore overstated the fixture identity proof;
- the experiment file needed to record completion review before the result
  commit.

Resolution:

- updated `scripts/test-issue-834-surfari-pdf-selection-copy.sh` to emit
  `page_geometry` and `font` in every fixture summary;
- updated `scripts/test-issue-834-surfari-pdf-selection-bounds-with-oracle.sh`
  to read those fields from embedded cell summaries before comparing them to the
  Experiment 50 oracle fixture identity;
- reran the full matrix as `20260623-002902` and updated the result text to
  reference that run.

Follow-up verdict: **Approved**.

The reviewer found no remaining technical must-fix issues. It agreed that the
fixture identity gate is now sufficiently mechanical and that the rerun supports
`Pass` / `embedded-right-edge-selection-gap`: oracle open, fixture identity
match, 18 embedded cells, no `RIGHT834`, required direct cells copied
`LEFT834 MID834`, recurrence and targeting true, and clipboard restoration
succeeded.
