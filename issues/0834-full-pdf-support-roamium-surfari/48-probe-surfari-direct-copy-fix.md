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
