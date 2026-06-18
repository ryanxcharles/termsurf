# Experiment 3: Add Non-Pointer Performance Diagnostics

## Description

Experiment 1 established a fast repeated-startup smoke. Experiment 2 proved
pointer-dependent diagnostics are blocked in this VM because neither CGEvent nor
System Events click delivery produces the generic AppKit hit-test record.

Issue 820 still needs more useful lightweight coverage than startup alone. This
experiment will add non-pointer resize and split diagnostics that use the
existing app/window automation and protocol split path, while explicitly leaving
mouse, scroll, and pointer hot-path rows blocked until the VM/input-permission
problem is solved.

## Changes

Planned script changes:

- `scripts/ghostboard-geometry-matrix.sh`
  - Add a `performance-window-resize` scenario that skips only the generic
    initial hit-test prerequisite and then proves grow/shrink resize geometry,
    AppKit presented pixels, Zig presented-pixel records, and Roamium resize
    delivery without any pointer click assertions.
  - Add a `performance-split-right` scenario that skips only the generic initial
    hit-test prerequisite and then proves protocol-driven split-right geometry,
    AppKit presented pixels, and Roamium resize delivery without pointer click
    assertions.
  - Keep existing correctness scenarios unchanged; do not weaken their hit-test
    assertions.
- `scripts/ghostboard-performance-smoke.sh`
  - Keep `--fast` as the repeated-startup smoke.
  - Change `--diagnostic` to include the fast startup rows plus the new
    non-pointer resize and split rows.
  - Leave pointer-dependent rows out of `--diagnostic` for now because
    Experiment 2 proved they fail before performance can be measured.

Planned issue-document changes:

- Add `## Result` and `## Conclusion` after verification.
- Update the Issue 820 README experiment status after verification.

Explicitly out of scope:

- Ghostboard, Roamium, webtui, protocol, or app source changes.
- Fixing VM pointer injection.
- Precise FPS, CPU, memory, or frame-time benchmarking.
- Adding generated logs or screenshots to git.

## Verification

Formatting actions:

```bash
prettier --write --prose-wrap always --print-width 80 \
  issues/0820-ghostboard-performance-smoke-tests/README.md \
  issues/0820-ghostboard-performance-smoke-tests/03-add-non-pointer-performance-diagnostics.md
```

Static checks:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
bash -n scripts/ghostboard-performance-smoke.sh
git diff --check
```

Runtime checks:

```bash
scripts/ghostboard-geometry-matrix.sh performance-window-resize
scripts/ghostboard-geometry-matrix.sh performance-split-right
scripts/ghostboard-performance-smoke.sh --fast
scripts/ghostboard-performance-smoke.sh --diagnostic
```

Pass criteria:

- The two new geometry scenarios pass and do not require pointer hit-test
  events.
- Existing correctness scenarios keep their generic hit-test assertions.
- `--fast` still passes the repeated-startup smoke.
- `--diagnostic` passes startup plus the non-pointer resize/split diagnostics
  with bounded-run log paths and elapsed seconds.
- Pointer-dependent mouse/scroll/input rows remain documented as blocked, not
  silently claimed as covered.
- No generated logs or screenshots are staged.

Partial criteria:

- Startup remains green, but one non-pointer diagnostic row exposes a
  scenario-specific app or harness failure.
- The non-pointer rows work individually, but the diagnostic wrapper needs a
  follow-up to classify elapsed thresholds or logs correctly.

Fail criteria:

- The new scenarios weaken existing correctness scenarios' hit-test assertions.
- The fast repeated-startup smoke regresses.
- The diagnostic profile cannot distinguish scenario failure from timeout or
  threshold failure.

## Design Review

This experiment is plan-only until a fresh-context adversarial design review
approves it. Record the reviewer verdict here, fix all real findings, and commit
the approved plan before implementation begins.

External Codex design review using `skills/codex-review`:

- **Initial verdict:** Changes required.
- **Required finding:** Static verification used `bash -n` with two script paths
  in one command, which only parses the first file. Accepted; split the
  verification into separate `bash -n scripts/ghostboard-geometry-matrix.sh` and
  `bash -n scripts/ghostboard-performance-smoke.sh` commands.
- **Final verdict:** Approved.
- **Required findings:** None.
- **Evidence checked:** The reviewer confirmed the README links Experiment 3 as
  `Designed`, the experiment has required sections and a completion gate, scope
  follows Experiment 2's Partial result, existing hit-test correctness scenarios
  are preserved, app/Roamium/webtui/protocol source changes are out of scope,
  and the corrected syntax checks cover both planned shell scripts.

## Completion Gate

After implementation and verification:

- add `## Result` and `## Conclusion` to this experiment file;
- update the Issue 820 README experiment status from `Designed` to `Pass`,
  `Partial`, or `Fail`;
- request a fresh-context completion review;
- fix all real completion-review findings and record the final verdict in this
  file; and
- commit the reviewed result separately before designing or implementing the
  next experiment.

## Result

**Result:** Pass

Implemented the non-pointer diagnostics:

- `scripts/ghostboard-geometry-matrix.sh`
  - Added `performance-window-resize`, which skips the generic initial pointer
    prime but still proves grow/shrink AppKit overlay frames, presented pixels,
    Zig pixel records, and Roamium `ts_set_view_size` delivery.
  - Added `performance-split-right`, which skips the generic initial pointer
    prime and drives the split through a direct length-prefixed protobuf
    `OpenSplit` message to Ghostboard's Unix socket. This avoids the VM
    pointer-delivery problem and the browser-focus keyboard path, while still
    proving the browser pane is resized after the split.
  - Kept the original correctness scenarios' pointer hit-test assertions intact.
- `scripts/ghostboard-performance-smoke.sh`
  - Kept `--fast` as three repeated resolver-only startup rows.
  - Changed `--diagnostic` to run the fast startup rows plus
    `performance-window-resize` and `performance-split-right`.
  - Removed pointer-dependent mouse, scroll, and browser-input rows from
    `--diagnostic`; they remain blocked by the VM pointer hit-test failure
    proven in Experiment 2.

Verification:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
bash -n scripts/ghostboard-performance-smoke.sh
git diff --check
scripts/ghostboard-geometry-matrix.sh performance-window-resize
scripts/ghostboard-geometry-matrix.sh performance-split-right
scripts/ghostboard-performance-smoke.sh --fast
scripts/ghostboard-performance-smoke.sh --diagnostic
```

Observed results:

- Static checks passed.
- `performance-window-resize` passed.
  - Harness log:
    `logs/ghostboard-geometry-performance-window-resize-harness-20260618-052628.log`
  - App log:
    `logs/ghostboard-geometry-performance-window-resize-app-20260618-052628.log`
  - Roamium trace:
    `logs/ghostboard-geometry-performance-window-resize-roamium-20260618-052628.log`
- `performance-split-right` passed.
  - Harness log:
    `logs/ghostboard-geometry-performance-split-right-harness-20260618-052614.log`
  - App log:
    `logs/ghostboard-geometry-performance-split-right-app-20260618-052614.log`
  - Roamium trace:
    `logs/ghostboard-geometry-performance-split-right-roamium-20260618-052614.log`
- `scripts/ghostboard-performance-smoke.sh --fast` passed.
  - Summary log: `logs/ghostboard-performance-smoke-fast-20260618-052628.log`
- `scripts/ghostboard-performance-smoke.sh --diagnostic` passed.
  - Summary log:
    `logs/ghostboard-performance-smoke-diagnostic-20260618-052648.log`

The diagnostic wrapper completed five bounded rows: three startup runs, one
resize run, and one split run. The split row completed in 10 seconds under the
150-second cap, and the resize row completed in 10 seconds under the 150-second
cap.

## Conclusion

Issue 820 now has a durable fast smoke and a broader diagnostic smoke that do
not depend on VM pointer delivery. The key implementation learning is that split
responsiveness can be tested through Ghostboard's existing `OpenSplit` protocol
path, which is the same logical route webtui uses for `:devtools` splits and
avoids the browser-focus keyboard ambiguity seen during the initial
implementation attempt.

Pointer-dependent rows should stay outside the default diagnostic profile until
the VM can produce generic AppKit hit-test records again. They remain useful as
separate focused correctness scenarios, but they are not reliable performance
smoke rows in this environment.

## Completion Review

External Codex completion review using `skills/codex-review`:

- **Final verdict:** Approved.
- **Required findings:** None.
- **Evidence checked:** The reviewer confirmed that the diff is limited to the
  two harness scripts and Issue 820 docs, the original `window-resize` and
  `split-right` correctness scenarios keep their pointer hit-test assertions,
  the direct `OpenSplit` protobuf sender matches the TermSurf protocol field
  numbers, `--fast` remains three startup rows, `--diagnostic` runs only the
  non-pointer resize and split rows, and the recorded pass result is backed by
  the cited logs and static checks.
