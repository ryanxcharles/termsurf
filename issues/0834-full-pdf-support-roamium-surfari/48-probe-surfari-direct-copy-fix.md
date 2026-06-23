# Experiment 48: Probe Surfari Direct Copy Fix

## Description

Experiment 47 showed that Surfari's embedded `WKWebView` is first responder, but
AppKit cannot find a normal `copy:` target because the hidden `TSHostWindow` is
not key/main and `NSApp.keyWindow` is `nil`. It also showed that direct
`WKWebView.copy(nil)` can be invoked, but the existing Surfari PDF copy harness
selection was narrow enough to copy `TS834PDFCOPYQXJ`, missing the final `Z`.

This experiment should test a focused candidate fix without committing it as
product behavior yet: when Surfari receives forwarded `Cmd+C`, use an env-gated
direct `WKWebView.copy(nil)` route, and use widened PDF selection coordinates so
the harness targets the full marker.

## Changes

- Add or reuse an env-gated Surfari direct-copy behavior flag, tentatively
  `TERMSURF_SURFARI_PDF_COPY_DIRECT=1`.
- Under that flag only, when `ts_forward_key_event` receives forwarded `Cmd+C`:
  - run the normal existing key forwarding path first;
  - invoke direct `WKWebView.copy(nil)` after the normal path;
  - trace clipboard state before and after the direct copy;
  - keep the behavior disabled unless the environment flag is set.
- Add a focused harness, tentatively
  `scripts/test-issue-834-surfari-direct-copy-fix.sh`, that runs the real
  Surfari-in-Ghostboard PDF fixture with:
  - the passive baseline path;
  - the direct-copy flag path;
  - widened drag coordinates that cover the full visible `TS834PDFCOPYQXJZ`
    marker, based on Experiment 46's standalone WKWebView correction.
- Keep coordinate and behavior variables separate:
  - first prove the widened drag coordinates still reproduce the baseline
    external-copy failure without the direct-copy flag;
  - then run the same widened drag coordinates with the direct-copy flag.
- Record:
  - drag coordinates and screenshots;
  - Surfari trace lines around `Cmd+C`;
  - clipboard before/after samples and hashes;
  - whether the copied text contains the full accepted marker
    `TS834PDFCOPYQXJZ`;
  - whether the direct-copy route changed clipboard content only under the
    env-gated flag.
- Protect clipboard state:
  - save the original clipboard exactly once at harness start;
  - restore it from a trap on every exit path;
  - use distinct sentinels for baseline and direct-copy-flag runs where the
    nested harness does not already do so;
  - record final restoration status in the summary;
  - downgrade/fail the result if restoration fails.
- Apply this outcome matrix:
  - **direct-copy-fix-candidate:** widened baseline still fails, but widened
    direct-copy flag copies the full marker;
  - **coordinate-fix-only:** widened baseline copies the full marker without the
    direct-copy flag;
  - **direct-copy-partial-selection:** direct-copy flag copies text but not the
    full marker;
  - **direct-copy-no-effect:** direct-copy flag does not change clipboard
    content;
  - **harness-insufficient:** screenshots or traces cannot prove the widened
    drag targeted the full marker.
- Map result status:
  - **Pass:** `direct-copy-fix-candidate`, `coordinate-fix-only`,
    `direct-copy-partial-selection`, or `direct-copy-no-effect`, with complete
    evidence;
  - **Partial:** `harness-insufficient` with useful logs;
  - **Fail:** clipboard restore failure, missing baseline/direct run, or
    non-env-gated behavior change.
- Do not make the direct-copy route permanent in this experiment. If the
  candidate works, the next experiment should review and implement the product
  fix deliberately.
- Update this experiment file with the result.

## Verification

Run hygiene checks:

```bash
bash -n scripts/test-issue-834-surfari-direct-copy-fix.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
```

Run the diagnostic harness:

```bash
rm -rf logs/issue-834-exp48-surfari-direct-copy-fix
scripts/test-issue-834-surfari-direct-copy-fix.sh
```

Pass criteria:

- baseline and direct-copy-flag runs both use the same widened drag coordinates;
- baseline and direct-copy-flag clipboard evidence is recorded separately;
- screenshots and coordinate logs show the widened drag targeted the full
  marker;
- direct-copy behavior is gated behind the experiment environment variable;
- copied text is checked against the full accepted marker;
- clipboard restoration succeeds and final restoration status is recorded;
- build/format checks pass;
- completion review is recorded.

Partial criteria:

- widened coordinates cannot be proven visually but produce useful clipboard or
  trace evidence;
- the direct-copy route copies partial text but screenshots cannot prove whether
  the missing text is a selection or copy-route problem;
- the harness cannot run one of the two modes, but the other mode produces
  useful diagnostic evidence.

Failure criteria:

- clipboard state is not restored;
- product behavior changes without the environment flag;
- the result claims a product fix without comparing baseline and direct-copy
  flag runs.

## Design Review

An external Codex review checked the design.

Initial verdict: **Changes required**.

Finding:

- clipboard safety was not explicit enough for a copy experiment.

Resolution:

- required saving the original clipboard exactly once at harness start;
- required trap-based restore on every exit path;
- required distinct sentinels for baseline and direct-copy-flag runs where the
  nested harness does not already provide them;
- required final restoration status in the summary and successful restoration in
  the pass criteria.

Follow-up verdict: **Approved**.

The reviewer found no remaining must-fix design issues and approved the
Experiment 48 plan commit.

## Result

**Result:** Pass

The direct-copy probe was added behind `TERMSURF_SURFARI_PDF_COPY_DIRECT=1`. The
existing Surfari PDF selection/copy harness now accepts drag-ratio overrides,
and the Experiment 48 wrapper was added as
`scripts/test-issue-834-surfari-direct-copy-fix.sh`.

Verification:

```bash
bash -n scripts/test-issue-834-surfari-direct-copy-fix.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
rm -rf logs/issue-834-exp48-surfari-direct-copy-fix
scripts/test-issue-834-surfari-direct-copy-fix.sh
```

The diagnostic run was `20260622-232631`. Its summary is:

```text
logs/issue-834-exp48-surfari-direct-copy-fix/surfari-direct-copy-fix-summary.json
```

The run classified the result as:

```json
{
  "classification": "direct-copy-partial-selection",
  "overall_result": "pass"
}
```

Key evidence:

- Both baseline and direct-copy-flag modes used the same widened drag ratios:
  start x `0.58`, end x `0.99`, y `0.43`.
- The widened baseline still reproduced the existing Surfari partial failure:
  the clipboard remained the sentinel after normal external `Cmd+C`.
- The direct-copy flag did not copy the marker on the first normal `Cmd+C`
  attempt; the clipboard remained the sentinel after direct `sendAction` and
  direct `WKWebView.copy(nil)` calls.
- During the later fallback copy path, the direct-copy flag changed the
  clipboard to `TS834PDFCOPYQXJ`, which is still missing the final `Z` and does
  not contain the full accepted marker `TS834PDFCOPYQXJZ`.
- Clipboard restoration succeeded.

Important artifacts:

- `logs/issue-834-exp48-surfari-direct-copy-fix/baseline-exp44-summary-20260622-232631.json`
- `logs/issue-834-exp48-surfari-direct-copy-fix/direct-exp44-summary-20260622-232631.json`
- `logs/issue-834-exp48-surfari-direct-copy-fix/baseline-copy-trace-20260622-232631.log`
- `logs/issue-834-exp48-surfari-direct-copy-fix/direct-copy-trace-20260622-232631.log`

The WebKit bridge build emitted the existing macOS SDK warning about building
for macOS 26.0 while linking a WebKit framework built for 26.5, but completed
successfully.

## Conclusion

The direct-copy candidate is not sufficient as a product fix. It can make the
embedded Surfari path copy some PDF text during the fallback copy path, but it
does not copy the full accepted marker and does not fix the first normal
external `Cmd+C` attempt.

The remaining problem now appears to involve the PDF selection extent and/or the
timing/state of the PDF plugin selection, not only AppKit's missing normal copy
target. The next experiment should target selection geometry more directly: use
a marker or drag path that can prove whole-text selection in Surfari, or add a
PDF fixture with wider spacing/sentinel tokens so partial-selection boundaries
are easier to diagnose.

## Completion Review

An external Codex completion review checked the result, implementation, harness
changes, and final summary.

Verdict: **Approved after recording this review**.

Finding:

- the experiment file needed to record the completion review before the result
  commit.

Resolution:

- this section records the completion review verdict and finding.

The reviewer found no implementation must-fix issues. It agreed that the `Pass`
/ `direct-copy-partial-selection` classification is supported by the evidence,
that the behavior is diagnostic and env-gated behind
`TERMSURF_SURFARI_PDF_COPY_DIRECT=1`, and that the result language does not
claim Surfari PDF copy is fixed.
