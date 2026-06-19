# Experiment 7: Run Inherited Viewport Matrix

## Description

Experiment 6 proved the single-pane real Roamium overlay smoke path after the
upstream update and app rename. Issue 826 still requires broader proof that
browser overlays attach to the correct pane, tab, and window; resize and move
with their owning viewport; clean up when panes or tabs close; and route
keyboard and mouse input to the visible active browser.

Issue 809 already built a full Ghostboard viewport geometry harness for this
behavior. This experiment reuses that inherited matrix against the updated Issue
826 Ghostboard. The goal is to discover whether the upstream update broke any
previously proven geometry or input behavior before designing new fixes.

This experiment should not change product code. If a matrix row fails, record
the failing row, logs, and first failing invariant, then design the next
experiment from that evidence.

## Changes

- `issues/0826-update-ghostboard-to-latest-ghostty/README.md`
  - Link this experiment with status `Designed`, then update the status after
    the result is known.
- `issues/0826-update-ghostboard-to-latest-ghostty/07-run-inherited-viewport-matrix.md`
  - Record design, verification, result, reviews, and conclusion.

No source changes are planned. Do not modify `webtui/`, `roamium/`, `chromium/`,
`proto/termsurf.proto`, or Ghostboard product code in this experiment. If the
matrix requires a harness-only compatibility fix, record it explicitly and keep
the change limited to `scripts/ghostboard-geometry-matrix.sh`.

## Verification

Confirm starting state:

```bash
git status --short
test -x ghostboard/macos/build/Debug/TermSurf.app/Contents/MacOS/termsurf
test -x target/debug/web
test -x chromium/src/out/Default/roamium
```

Build the components used by the matrix:

```bash
cargo build -p webtui \
  > logs/issue-0826-exp07-webtui-build.log 2>&1
(cd ghostboard && macos/build.nu --configuration Debug --action build \
  > ../logs/issue-0826-exp07-macos-build.log 2>&1)
```

Run the inherited viewport matrix with app/web/Roamium overrides explicitly
unset:

```bash
SUMMARY="logs/issue-0826-exp07-viewport-matrix-summary-$(date +%Y%m%d-%H%M%S).log"
SCENARIOS=(
  initial-open
  window-resize
  split-right
  split-down
  split-right-resize
  split-right-equalize
  split-right-zoom
  split-right-close-sibling
  split-right-close-browser-pane
  split-right-focus-switch
  new-terminal-tab-visibility
  open-browser-in-new-tab
  close-browser-tab
  open-browser-in-new-window
  multiple-windows-with-browsers
  display-move-backing-scale
  fullscreen-unfullscreen
  minimize-hide-restore
  font-size-cell-metrics
  tui-overlay-resize-command
  terminal-scrollback-movement
  browser-navigation-geometry
  devtools-split-geometry
  mouse-after-geometry-change
  keyboard-after-tab-window-switch
)

set -o pipefail
for scenario in "${SCENARIOS[@]}"; do
  printf 'RUN %s\n' "$scenario" | tee -a "$SUMMARY"
  if env -u TERMSURF_GHOSTBOARD_APP \
    -u TERMSURF_WEB \
    -u TERMSURF_ROAMIUM \
    -u TERMSURF_INSTALLED_ROAMIUM \
    scripts/ghostboard-geometry-matrix.sh "$scenario" 2>&1 |
    tee -a "$SUMMARY"; then
    printf 'RESULT %s PASS\n' "$scenario" | tee -a "$SUMMARY"
  else
    rc=$?
    printf 'RESULT %s FAIL exit=%s\n' "$scenario" "$rc" | tee -a "$SUMMARY"
    exit "$rc"
  fi
done
printf 'FULL MATRIX PASS\n' | tee -a "$SUMMARY"
```

After the run, reject masked failures:

```bash
rg -n "^FAIL:|RESULT .*FAIL|FULL MATRIX" "$SUMMARY" \
  > logs/issue-0826-exp07-summary-status.log
! rg -n "^FAIL:|RESULT .*FAIL" "$SUMMARY"
```

Capture the latest per-scenario artifacts:

```bash
for scenario in "${SCENARIOS[@]}"; do
  {
    printf 'scenario=%s\n' "$scenario"
    ls -t "logs/ghostboard-geometry-${scenario}-harness-"*.log | head -1
    ls -t "logs/ghostboard-geometry-${scenario}-app-"*.log | head -1
    ls -t "logs/ghostboard-geometry-${scenario}-roamium-"*.log | head -1
    ls -t "logs/ghostboard-geometry-${scenario}-screenshot-"*.png 2>/dev/null | head -1 || true
  } >> logs/issue-0826-exp07-artifacts.log
done
```

For each scenario, record:

- scenario name;
- pass/fail/partial status;
- harness log path;
- app log path;
- Roamium trace path;
- screenshot path when present;
- identity tuple evidence, such as pane id, browser tab id, context id, selected
  tab id, or window id;
- the specific matrix behavior covered.

Run hygiene checks:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
prettier --write --prose-wrap always --print-width 80 \
  issues/0826-update-ghostboard-to-latest-ghostty/README.md \
  issues/0826-update-ghostboard-to-latest-ghostty/07-run-inherited-viewport-matrix.md
git diff --check
git diff --name-only
git status --short -- webtui roamium proto/termsurf.proto chromium/README.md chromium/patches
git -C chromium/src status --short
git -C chromium/src diff --name-only
```

Pass criteria:

- `cargo build -p webtui` passes.
- The debug macOS `TermSurf.app` build passes.
- Every listed inherited matrix scenario exits successfully.
- The strict summary contains `FULL MATRIX PASS` and no `FAIL:` or
  `RESULT .*FAIL` lines.
- The scenarios run without `TERMSURF_GHOSTBOARD_APP`, `TERMSURF_WEB`,
  `TERMSURF_ROAMIUM`, or `TERMSURF_INSTALLED_ROAMIUM` overrides.
- The result records a per-scenario table with the evidence paths and the
  behavior each scenario covers.
- The result explicitly notes that `display-move-backing-scale` can only prove
  the single-display path in this macOS VM unless another display is available.
- `bash -n scripts/ghostboard-geometry-matrix.sh` and `git diff --check` pass.
- No forbidden paths are modified: `webtui/`, `roamium/`, `chromium/`, or
  `proto/termsurf.proto`.
- The nested `chromium/src` checkout has no uncommitted status or diff from this
  experiment.

Partial criteria:

- Most scenarios pass, but one or more fail with clear first-failure evidence
  recorded for the next experiment.
- The matrix passes except for an environmental limitation such as
  single-display inability to move between displays.

Fail criteria:

- A scenario failure is hidden by shell pipeline behavior.
- The result claims full geometry coverage without per-scenario evidence paths.
- Product code, webtui, Roamium, Chromium, or the protocol is changed inside
  this matrix run instead of a focused follow-up experiment.

## Design Review

An adversarial Codex subagent reviewed the initial design with fresh context.

**Verdict:** Changes required.

Required findings and fixes:

- The matrix loop could still mask scenario command failures because it set
  `pipefail` but unconditionally wrote `RESULT ... PASS` and `FULL MATRIX PASS`.
  Fixed by wrapping each scenario pipeline in an explicit
  `if ...; then RESULT PASS; else RESULT FAIL exit=$rc; exit $rc; fi` block.
- The Ghostboard build snippet changed into `ghostboard/` before later
  repo-root-relative commands. Fixed by running the Ghostboard build in a
  subshell.

The first re-review found that the subshell build log redirection was evaluated
from the repo root and would write outside repo `logs/`. Fixed by moving the
redirection inside the subshell.

The final re-review approved the design with no required findings.
