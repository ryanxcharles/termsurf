# Experiment 56: Probe Embedded PDF Responder Activation

## Description

Experiment 55 showed that embedded Surfari behaves differently from matched
standalone `WKWebView` under calibrated PDF selection gestures. Matched
standalone cells have key/main windows and resolve `copy:` targets to
`WKWebView`. Embedded Surfari cells have a `TSHostWindow` that is not key/main,
and AppKit resolves both `target_nil` and `target_webview` to `nil`. Under those
conditions, calibrated embedded gestures copy only `LEFT834` while the matched
standalone cells copy `LEFT834 MID834 RIGHT834`.

This experiment should isolate whether making the embedded Surfari host
window/app/responder state comparable to standalone changes PDF selection or
copy behavior. It should use env-gated probes first, not permanent product
behavior.

## Changes

- Add env-gated responder activation probes in
  `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm`. Proposed flags:
  - `TERMSURF_SURFARI_PDF_RESPONDER_PROBE=1`;
  - `TERMSURF_SURFARI_PDF_RESPONDER_MODE=baseline|activate-app|key-window|main-window|key-main-window|explicit-first-responder|explicit-copy-target`;
  - optional `TERMSURF_SURFARI_PDF_RESPONDER_TRACE=1` if separate trace routing
    is useful.
- Keep normal behavior unchanged unless the probe flag is present.
- Run two baseline controls before interpreting probe modes:
  - **normal-control:** no `TERMSURF_SURFARI_PDF_RESPONDER_PROBE` flag;
  - **flagged-baseline:** `TERMSURF_SURFARI_PDF_RESPONDER_PROBE=1` with
    `TERMSURF_SURFARI_PDF_RESPONDER_MODE=baseline`. Both controls must reproduce
    the Experiment 55 `responder-gap-candidate` baseline before activation modes
    can be interpreted.
- For each mode, apply the minimum relevant action before selection and copy:
  - `baseline`: no additional action;
  - `activate-app`: activate the Surfari helper app before the gesture and again
    before copy, if AppKit allows it;
  - `key-window`: attempt to make the `TSHostWindow` key before the gesture and
    again before copy;
  - `main-window`: attempt to make the `TSHostWindow` main before the gesture
    and again before copy;
  - `key-main-window`: attempt both key and main before the gesture and again
    before copy;
  - `explicit-first-responder`: make the `WKWebView` first responder before the
    gesture and again before copy;
  - `explicit-copy-target`: keep selection unchanged but route copy explicitly
    through the `WKWebView` target path for primary copy diagnostics.
- Add a harness, tentatively
  `scripts/test-issue-834-surfari-pdf-responder-activation.sh`, that:
  - requires/open-checks the Experiment 50 oracle summary;
  - requires/open-checks the Experiment 54 calibration summary;
  - uses the same calibrated embedded cells from Experiment 55;
  - runs normal-control, flagged-baseline, and responder probe modes;
  - carries the Experiment 55 cell name/ratios and matched Experiment 54
    successful standalone baseline for every calibrated cell/mode;
  - closes the harness gate if any calibrated cell is missing its matched
    standalone baseline, has mismatched ratios, or did not copy all tokens in
    standalone;
  - records primary post-selection copy tokens, fallback/select-all tokens, and
    direct-probe tokens separately;
  - records explicit-copy-target tokens separately from primary external Cmd+C;
  - records matched standalone responder baselines from Experiment 54;
  - records embedded responder/copy-target state before and after the probe
    action;
  - records whether each mode changes `key_window`, `main_window`, `target_nil`,
    `target_webview`, copied tokens, or trace completeness.
- Use the Experiment 55 classification as the baseline control: both
  normal-control and flagged-baseline must reproduce `responder-gap-candidate`
  before interpreting probe modes.
- Keep result language diagnostic. A successful mode is a product-fix candidate,
  not a product fix, until a follow-up experiment converts it into normal
  behavior and regression guards. This experiment may identify responder or
  activation candidates and next targets only; it must not claim the final root
  cause without a later fix-validation experiment.
- Apply this outcome matrix:
  - **activation-fix-candidate:** a responder mode makes embedded primary copy
    all three tokens through primary external Cmd+C for at least one calibrated
    cell and improves responder state toward the matched standalone baseline.
    `explicit-copy-target` is excluded from this class;
  - **responder-state-improved-selection-unchanged:** a responder mode improves
    key/main or copy-target state, but calibrated embedded copies still miss
    tokens;
  - **responder-state-unchanged:** the probe modes do not materially change
    key/main or copy-target state;
  - **explicit-copy-target-only:** explicit copy targeting changes copied tokens
    but key/main responder state remains non-comparable;
  - **harness-insufficient:** oracle/calibration/baseline gates are closed,
    traces are missing, clipboard restoration fails, or probe modes cannot run.
- Apply this classification precedence:
  1. `harness-insufficient` for closed gates, missing baseline reproduction,
     missing traces, or clipboard restoration failure.
  2. `activation-fix-candidate` for non-`explicit-copy-target` modes that copy
     all tokens through primary external Cmd+C while improving responder state.
  3. `explicit-copy-target-only` for copied-token improvement caused by explicit
     target routing.
  4. `responder-state-improved-selection-unchanged` if responder state improves
     but copied tokens do not.
  5. `responder-state-unchanged` if modes do not materially alter the responder
     gap.
- Update this experiment file with the result.

## Verification

Run hygiene checks:

```bash
bash -n scripts/test-issue-834-surfari-pdf-responder-activation.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
git diff --check
git -C webkit/src status --short
```

Run the responder probe:

```bash
rm -rf logs/issue-834-exp56-surfari-pdf-responder-activation
scripts/test-issue-834-surfari-pdf-responder-activation.sh
```

Pass criteria:

- Experiment 50 oracle gate is open;
- Experiment 54 calibration gate is open;
- normal-control reproduces Experiment 55 `responder-gap-candidate`;
- flagged-baseline reproduces Experiment 55 `responder-gap-candidate`;
- every calibrated cell/mode is mechanically matched by name and ratios to a
  successful Experiment 54 standalone cell;
- each probe mode records responder state, copy-target state, copied-token
  evidence by route, explicit-copy evidence separately when used, matched
  standalone baseline, and trace paths;
- one explicit non-`harness-insufficient` outcome is selected;
- normal behavior is unchanged without the env-gated probe flag;
- completion review is recorded.

Partial criteria:

- baseline reproduces and some probe evidence is useful, but no mode can be
  classified confidently;
- probe modes run but AppKit refuses key/main changes in a way that needs a
  narrower follow-up;
- explicit copy targeting changes behavior but cannot be separated from
  selection geometry.

Failure criteria:

- clipboard state is not restored;
- oracle, calibration, or baseline gates are closed;
- probe flags alter normal behavior when disabled;
- the result claims a product fix or final root cause instead of a candidate.

## Design Review

Codex reviewed the design and agreed Experiment 56 is the correct next step
after Experiment 55. The initial review required stricter controls:

- add separate no-flag normal-control and flagged-baseline controls;
- exclude `explicit-copy-target` from `activation-fix-candidate`;
- require per-cell matching to successful Experiment 54 standalone baselines;
- separate primary external Cmd+C from explicit, fallback, and direct routes;
- prevent final-root-cause overclaims;
- pin probe timing before the gesture and before copy.

The design was updated for each finding. A follow-up Codex review confirmed the
required findings were resolved and approved the design for the plan commit.
