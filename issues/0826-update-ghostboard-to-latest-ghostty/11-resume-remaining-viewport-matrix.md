# Experiment 11: Resume Remaining Viewport Matrix

## Description

Experiment 10 showed that the previous `display-move-backing-scale`
single-display focus failure is not currently reproducible. The row now passes
its single-display fallback: AppKit receives Enter, webtui enters Browse mode,
Ghostboard emits `FocusChanged focused=true`, Roamium records a new
`ts_set_focus focused=true` trace, and browser keyboard forwarding works.

Issue 826 still requires proof that browser overlays survive the remaining
viewport, geometry, and input cases inherited from Issue 809. This experiment
resumes the matrix after `display-move-backing-scale` and records the next real
failure, if any.

This experiment should not change product code. If a row fails, record the first
failing row, logs, and invariant, then design the next experiment from that
evidence.

## Changes

- `issues/0826-update-ghostboard-to-latest-ghostty/README.md`
  - Link this experiment with status `Designed`, then update the status after
    the result is known.
- `issues/0826-update-ghostboard-to-latest-ghostty/11-resume-remaining-viewport-matrix.md`
  - Record design, verification, result, reviews, and conclusion.

No source changes are planned. Do not modify `ghostboard/`, `webtui/`,
`roamium/`, `chromium/`, or `proto/termsurf.proto` in this experiment unless the
resumed matrix proves a narrow harness-only compatibility problem. If a
harness-only fix is needed, keep it limited to
`scripts/ghostboard-geometry-matrix.sh`, rerun the failing row, and record why
the fix is not product behavior.

## Verification

Confirm starting state and app/tool availability:

```bash
git status --short
test -x ghostboard/macos/build/Debug/TermSurf.app/Contents/MacOS/termsurf
test -x target/debug/web
test -x chromium/src/out/Default/roamium
```

Run syntax and formatting checks before the runtime matrix:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
prettier --write --prose-wrap always --print-width 80 \
  issues/0826-update-ghostboard-to-latest-ghostty/README.md \
  issues/0826-update-ghostboard-to-latest-ghostty/11-resume-remaining-viewport-matrix.md
git diff --check
```

Resume the inherited viewport matrix from the first row after
`display-move-backing-scale` with overrides explicitly unset:

```bash
SUMMARY="logs/issue-0826-exp11-remaining-matrix-summary-$(date +%Y%m%d-%H%M%S).log"
SCENARIOS=(
  fullscreen-unfullscreen
  minimize-hide-restore
  font-size-cell-metrics
  tui-overlay-resize-command
  terminal-scrollback-movement
  browser-navigation-geometry
  devtools-split-geometry
  devtools-singleton-guard
  mouse-after-geometry-change
  keyboard-after-tab-window-switch
  gui-active-multi-tab
)

set -o pipefail
FAILED_SCENARIO=""
FAILED_RC=0
RUN_SCENARIOS=()
printf '' > logs/issue-0826-exp11-artifacts.log
for scenario in "${SCENARIOS[@]}"; do
  RUN_SCENARIOS+=("$scenario")
  SCENARIO_MARKER="logs/issue-0826-exp11-${scenario}-start.marker"
  : > "$SCENARIO_MARKER"
  printf 'RUN %s\n' "$scenario" | tee -a "$SUMMARY"
  if env -u TERMSURF_GHOSTBOARD_APP \
    -u TERMSURF_WEB \
    -u TERMSURF_ROAMIUM \
    -u TERMSURF_INSTALLED_ROAMIUM \
    scripts/ghostboard-geometry-matrix.sh "$scenario" 2>&1 |
    tee -a "$SUMMARY"; then
    printf 'RESULT %s PASS\n' "$scenario" | tee -a "$SUMMARY"
  else
    FAILED_RC=$?
    FAILED_SCENARIO="$scenario"
    printf 'RESULT %s FAIL exit=%s\n' "$scenario" "$FAILED_RC" | tee -a "$SUMMARY"
  fi

  HARNESS_LOG="$(find logs -name "ghostboard-geometry-${scenario}-harness-*.log" -newer "$SCENARIO_MARKER" -print | sort | tail -1)"
  APP_LOG="$(find logs -name "ghostboard-geometry-${scenario}-app-*.log" -newer "$SCENARIO_MARKER" -print | sort | tail -1)"
  ROAMIUM_TRACE="$(find logs -name "ghostboard-geometry-${scenario}-roamium-*.log" -newer "$SCENARIO_MARKER" -print | sort | tail -1)"
  SCREENSHOT="$(find logs -name "ghostboard-geometry-${scenario}-screenshot-*.png" -newer "$SCENARIO_MARKER" -print | sort | tail -1 || true)"
  test -n "$HARNESS_LOG"
  test -n "$APP_LOG"
  test -n "$ROAMIUM_TRACE"
  {
    printf 'scenario=%s\n' "$scenario"
    printf 'harness=%s\n' "$HARNESS_LOG"
    printf 'app=%s\n' "$APP_LOG"
    printf 'roamium=%s\n' "$ROAMIUM_TRACE"
    printf 'screenshot=%s\n' "$SCREENSHOT"
  } >> logs/issue-0826-exp11-artifacts.log

  if [ -n "$FAILED_SCENARIO" ]; then
    printf 'FAILED_SCENARIO=%s\nFAILED_APP_LOG=%s\nFAILED_HARNESS_LOG=%s\nFAILED_ROAMIUM_TRACE=%s\n' \
      "$FAILED_SCENARIO" "$APP_LOG" "$HARNESS_LOG" "$ROAMIUM_TRACE" \
      > logs/issue-0826-exp11-failure-artifacts.log
    rg -n 'FAIL:|panic|error\(|warn|TermSurf geometry|ModeChanged:|FocusChanged:|KeyEvent:|SetOverlay|ClearOverlay|BrowserReady|TabReady|CloseTab' \
      "$HARNESS_LOG" "$APP_LOG" \
      > logs/issue-0826-exp11-failure-evidence.log || true
    rg -n 'resize|focus-changed|key-event|mouse-event|close-tab|shutdown|panic|error' \
      "$ROAMIUM_TRACE" \
      > logs/issue-0826-exp11-failure-roamium-evidence.log || true
    break
  fi
done
if [ -z "$FAILED_SCENARIO" ]; then
  printf 'REMAINING MATRIX PASS\n' | tee -a "$SUMMARY"
fi
```

Capture the latest per-scenario artifacts for the rows that actually ran. The
harness, app, and Roamium logs are required for every attempted row; screenshots
are optional because not every row creates one. Each artifact path is captured
inside the matrix loop above using a marker file created immediately before that
scenario starts, so stale logs from earlier runs cannot satisfy this check.

```bash
awk '
  /^scenario=/ { scenarios++ }
  /^harness=/ && length($0) > 8 { harness++ }
  /^app=/ && length($0) > 4 { app++ }
  /^roamium=/ && length($0) > 8 { roamium++ }
  END { exit !(scenarios == harness && scenarios == app && scenarios == roamium) }
' logs/issue-0826-exp11-artifacts.log
```

If a row fails, the matrix loop above extracts focused failure evidence before
breaking. Validate that the failure evidence exists:

```bash
if [ -n "$FAILED_SCENARIO" ]; then
  test -s logs/issue-0826-exp11-failure-artifacts.log
  test -s logs/issue-0826-exp11-failure-evidence.log
  test -f logs/issue-0826-exp11-failure-roamium-evidence.log
fi
```

Reject masked failures after artifacts and focused failure evidence have been
captured:

```bash
rg -n '^RUN |^RESULT |^FAIL:|REMAINING MATRIX' "$SUMMARY" \
  > logs/issue-0826-exp11-summary-status.log
if [ -n "$FAILED_SCENARIO" ]; then
  exit "$FAILED_RC"
fi
! rg -n '^FAIL:|RESULT .*FAIL' "$SUMMARY"
```

Run final cleanup and scope checks:

```bash
ps -axo pid,comm,args \
  | rg 'TermSurf\\.app/Contents/MacOS/termsurf|target/debug/web|chromium/src/out/Default/roamium' \
  | rg -v 'rg|ps -axo|zsh -lc' \
  > logs/issue-0826-exp11-post-cleanup-processes.log || true
test ! -s logs/issue-0826-exp11-post-cleanup-processes.log

git status --short -- ghostboard webtui roamium proto/termsurf.proto chromium/README.md chromium/patches \
  > logs/issue-0826-exp11-forbidden-top-status.log
git -C chromium/src status --short > logs/issue-0826-exp11-chromium-status.log
git -C chromium/src diff --name-only > logs/issue-0826-exp11-chromium-diff-name-only.log
git diff --name-only > logs/issue-0826-exp11-git-diff-name-only.log
test ! -s logs/issue-0826-exp11-forbidden-top-status.log
test ! -s logs/issue-0826-exp11-chromium-status.log
test ! -s logs/issue-0826-exp11-chromium-diff-name-only.log
```

Pass criteria:

- `bash -n`, Prettier, and `git diff --check` are clean.
- The remaining matrix runs with `TERMSURF_GHOSTBOARD_APP`, `TERMSURF_WEB`,
  `TERMSURF_ROAMIUM`, and `TERMSURF_INSTALLED_ROAMIUM` unset.
- Every listed remaining scenario exits successfully.
- The strict summary contains `REMAINING MATRIX PASS` and no `FAIL:` or
  `RESULT .*FAIL` lines.
- Per-scenario artifact paths are recorded.
- Cleanup leaves no stale matching app, web, or Roamium processes.
- No product/source paths are modified by this experiment.
- The nested `chromium/src` checkout has no uncommitted status or diff from this
  experiment.

Partial criteria:

- One or more remaining scenarios fail, and the first failure is recorded with
  focused harness, app, and Roamium evidence for the next experiment.
- A harness-only compatibility issue is fixed narrowly in
  `scripts/ghostboard-geometry-matrix.sh`, but a later row fails with clear
  evidence.

Fail criteria:

- A scenario failure is hidden by shell pipeline behavior.
- The result claims full remaining matrix coverage without per-scenario artifact
  paths.
- Product code, webtui, Roamium, Chromium, or the protocol is changed inside
  this matrix continuation instead of a focused follow-up experiment.

## Design Review

An adversarial Codex subagent reviewed the initial design with fresh context.

**Verdict:** Changes required.

Required findings and fixes:

- The original matrix loop exited immediately after `RESULT ... FAIL`, making
  the later focused failure extraction unreachable. Fixed by tracking
  `FAILED_SCENARIO` and `FAILED_RC`, breaking only after current-run artifacts
  and focused failure evidence are captured, and re-emitting the failed exit
  only after validation.
- The original artifact capture selected the latest matching log files and could
  therefore pass with stale logs from earlier runs. Fixed by creating a
  per-scenario marker immediately before each harness invocation and requiring
  harness, app, and Roamium logs newer than that marker before recording the
  artifact paths.

The first re-review confirmed that failure evidence extraction was reachable but
still required current-run artifact correlation. The final re-review approved
the marker-based artifact capture and found no remaining required findings.
