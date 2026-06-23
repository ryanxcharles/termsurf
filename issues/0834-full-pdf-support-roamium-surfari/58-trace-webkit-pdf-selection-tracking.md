# Experiment 58: Trace WebKit PDF Selection Tracking

## Description

Experiments 56 and 57 ruled out two outer-layer explanations for embedded
Surfari PDF selection/copy failure:

- generic AppKit responder activation does not recover the missing right-side
  tokens;
- alternate mouse dispatch paths through the hidden Surfari host window,
  `WKWebView`, `WKFlippedView`, and `WKPDFHUDView` do not recover the missing
  right-side tokens.

The next diagnostic step is to trace WebKit's PDF selection path directly. The
goal is not to fix selection yet. The goal is to determine whether WebKit's PDF
plugin receives the same selection-relevant events that Surfari sends, which PDF
plugin implementation is active, what PDF/document/page coordinates WebKit
computes, and whether WebKit's internal selection string is already left-only
before copy reaches the pasteboard.

This experiment should add an environment-gated trace inside the local WebKit
checkout and run the embedded Surfari calibrated-selection harness against that
locally built WebKit. The trace must be disabled by default and must not change
production behavior.

## Changes

- `webkit/src/`
  - Create a new WebKit issue branch from the recorded base:
    `webkit-1452a439-issue-834-exp58`.
  - Add env-gated tracing around WebKit's PDF selection handling, prioritizing
    `Source/WebKit/WebProcess/Plugins/PDF/UnifiedPDF/UnifiedPDFPlugin.mm`
    because prior local source inspection showed the modern selection path
    there:
    - active plugin path;
    - mouse event type/button/click-count/modifiers;
    - root/plugin/document/page coordinates used by the PDF plugin;
    - page index;
    - selection-tracking transitions such as begin, continue, and stop;
    - current selection string, emptiness, and available selection rect counts
      after tracking transitions;
    - copy/editing-command path and selection string at copy time.
  - If the build path still compiles legacy
    `Source/WebKit/WebProcess/Plugins/PDF/PDFPlugin.mm`, add a lighter matching
    trace there too so the logs can prove whether the unified or legacy PDF path
    is active.
  - Use environment variables such as:
    - `TERMSURF_WEBKIT_PDF_SELECTION_TRACE=1`;
    - `TERMSURF_WEBKIT_PDF_SELECTION_TRACE_FILE=/path/to/trace.jsonl`.
  - Commit the WebKit source change inside `webkit/src`, then archive patches
    under `webkit/patches/issue-834/` with `git format-patch`.
- `webkit/README.md`
  - Record the new WebKit branch in the Branches table.
  - Update Current State if the active documented branch changes.
- `scripts/test-issue-834-webkit-pdf-selection-trace.sh`
  - Add a focused harness that runs the embedded Surfari separated-token PDF
    selection cells against the local WebKit debug build by setting
    `DYLD_FRAMEWORK_PATH=$ROOT/webkit/src/WebKitBuild/Debug`.
  - Prove the harness is using the repo-built Surfari stack, not an installed or
    stale helper, by setting `TERMSURF_SURFARI_PATH=$ROOT/target/debug/surfari`
    or by recording equivalent spawn evidence in the summary.
  - Keep the prior gates that make this comparable to earlier experiments:
    - Experiment 50 separated-token oracle gate;
    - Experiment 54 standalone calibration gate;
    - Experiment 55 embedded baseline reproduction;
    - fixture identity match;
    - clipboard restoration.
  - Run a small but diagnostic cell set first: at minimum the matched calibrated
    `oracle-base`, `oracle-x-tight`, and `oracle-x-wide` style cells that
    previously separated standalone full-token success from embedded left-token
    failure.
  - Unset stale Experiment 52/56/57 probe variables unless this harness
    explicitly sets them. This trace is meant to observe the baseline WebKit PDF
    path, not combine unrelated probes.
  - Write a machine-readable JSON summary and trace paths under `logs/`.

## Verification

The experiment passes if it produces a usable WebKit PDF selection trace and
classifies the embedded Surfari failure into one of these outcomes:

- **webkit-selection-left-only:** WebKit's own PDF selection string is already
  left-only after selection tracking and before pasteboard copy. This means the
  next fix should focus on WebKit/PDFKit coordinate or selection tracking.
- **webkit-copy-routing-gap:** WebKit's internal PDF selection string contains
  all expected tokens, but the copied pasteboard text is left-only. This means
  the next fix should focus on copy-command routing or pasteboard ownership.
- **webkit-coordinate-transform-gap:** the traced plugin/document/page points
  are inconsistent with the selected fixture text, indicating a coordinate-space
  or scaling problem before PDFKit selection state is formed.
- **webkit-plugin-path-identified:** the trace proves which PDF plugin path is
  active but does not yet expose enough selection state to classify the bug.
  This is only a partial result and should lead to a narrower follow-up trace.
- **harness-insufficient:** the local WebKit build, trace, or harness cannot
  reproduce the current embedded baseline. This is a failure unless the cause is
  explicitly identified and fixed in this experiment.

Required checks:

```bash
bash -n scripts/test-issue-834-webkit-pdf-selection-trace.sh
cargo fmt -p surfari -- --check
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
webkit/src/Tools/Scripts/build-webkit --debug
scripts/test-issue-834-webkit-pdf-selection-trace.sh
git diff --check
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

The result write-up must include:

- the WebKit branch name and commit hash;
- the generated patch filenames in `webkit/patches/issue-834/`;
- the trace file path;
- the summary JSON path;
- proof that the embedded test used the repo-built Surfari binary and local
  WebKit debug frameworks;
- proof that the trace contains nonempty records from the patched active WebKit
  PDF plugin path;
- the classification above;
- the next experiment implied by the classification.

## Design Review

Codex reviewed the Experiment 58 design before implementation and agreed that a
WebKit-internal PDF selection trace is the correct next diagnostic step after
Experiments 56 and 57 ruled out the outer responder and mouse-dispatch layers.

The review required two stronger gates before the plan could be committed:

- add top-level repo status and WebKit shallow-check verification;
- prove that the harness uses the repo-built Surfari stack and records nonempty
  trace rows from the patched active WebKit PDF plugin path.

Both requirements were added to the design. The plan is approved for
implementation after the plan commit.

## Result

**Result:** Fail

The focused harness reproduced the embedded Surfari PDF selection/copy failure
in all three calibrated cells, but it did not produce any WebKit PDF plugin
trace records from the patched local WebKit build.

Latest summary:

- `logs/issue-834-exp58-webkit-pdf-selection-trace/webkit-pdf-selection-trace-summary.json`
- run id: `20260623-040511`
- classification: `harness-insufficient`
- overall result: `fail`
- oracle gate: open
- standalone calibration gate: open
- matched cells: true
- fixture identity match: true
- repo stack paths: repo-built Surfari and
  `webkit/src/WebKitBuild/Debug/WebKit.framework`
- trace records: `0`

The user-visible failure remained stable:

- `oracle-base`: clipboard contained only `LEFT834`
- `oracle-x-tight`: clipboard contained only `LEFT834`
- `oracle-x-wide`: clipboard contained only `LEFT834`

The experiment also proved that the patched trace strings were present in the
local debug framework:

```bash
strings webkit/src/WebKitBuild/Debug/WebKit.framework/WebKit \
  | rg "termsurf-webkit-pdf-selection|performCopyEditingOperation-before"
```

However, live process inspection showed existing WebContent processes mapped
system WebKit and system JavaScriptCore images, not the local debug framework:

```text
/System/Library/Frameworks/WebKit.framework/Versions/A/WebKit
/System/Library/Frameworks/WebKit.framework/Versions/A/Frameworks/WebCore.framework/Versions/A/WebCore
/System/Library/Frameworks/JavaScriptCore.framework/Versions/A/JavaScriptCore
```

The harness was updated to mirror WebKit's own local-run environment more
closely by passing:

- `DYLD_FRAMEWORK_PATH`
- `DYLD_LIBRARY_PATH`
- `__XPC_DYLD_FRAMEWORK_PATH`
- `__XPC_DYLD_LIBRARY_PATH`
- `WEBKIT_UNSET_DYLD_FRAMEWORK_PATH=YES`

Those changes were not sufficient to make the patched WebKit PDF plugin active
inside the WebContent process. Because the trace gate required nonempty records
from the active patched PDF plugin path, this experiment failed.

No WebKit patch was archived for this experiment. The temporary WebKit source
trace diff was reverted after the failed proof because it did not become active
in the process under test.

Final repository state:

- top-level status contained only the Experiment 58 documentation and harness
  changes;
- WebKit branch: `webkit-1452a439-issue-834-exp58`;
- WebKit HEAD: `1452a43959523449099b2616793fd2c5b6a6487e`;
- WebKit shallow checkout: `true`;
- WebKit source status: clean;
- WebKit patch archive: none, because the temporary trace diff was reverted and
  never became active in WebContent.

Verification run:

```bash
bash -n scripts/test-issue-834-webkit-pdf-selection-trace.sh
bash -n scripts/test-issue-834-surfari-pdf-selection-copy.sh
cargo fmt -p surfari -- --check
cargo build -p surfari
scripts/test-issue-834-webkit-pdf-selection-trace.sh
git diff --check
git -C webkit/src status --short
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

## Completion Review

Codex reviewed the completed Experiment 58 result before the result commit.

The review agreed that the `Fail` / `harness-insufficient` classification was
justified because the embedded baseline failure reproduced, the oracle and
calibration gates remained open, fixture identity matched, and the required
trace gate failed with zero trace records.

The review required two fixes before commit:

- gate the extra local-WebKit XPC/library environment variables so the shared
  Experiment 44 selection-copy harness does not change behavior for every
  baseline run;
- add the required WebKit branch, HEAD, shallow-check, clean-status, and
  `git diff --check` evidence to this result.

Both fixes were applied. The extra local-WebKit environment is now enabled only
when `TERMSURF_SURFARI_USE_LOCAL_WEBKIT_ENV=1`, and the Experiment 58 wrapper
sets that flag explicitly.

## Conclusion

Experiment 58 did not give the desired WebKit-internal selection trace. The
strongest direct finding is that local WebKit tracing is blocked by the
WebContent process continuing to use system WebKit despite the local framework
path being present in the harness.

The next experiment should not depend on WebKit-internal trace records. Source
inspection of Surfari's current AppKit embedding path exposed a more direct
hypothesis: `ts_set_view_size` receives browser dimensions in pixels but applies
them directly as AppKit points, and the mouse forwarding path likewise treats
pixel-space input as point-space `NSEvent` coordinates. On a 2x Retina display,
that can make the hidden `WKWebView` twice as large in point space and make a
visually full-width drag select only the left side of the PDF. The next
experiment should test and, if proven, fix Surfari's point/pixel conversion for
view sizing and mouse input.
