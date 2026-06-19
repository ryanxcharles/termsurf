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

## Result

**Result:** Partial

The remaining matrix resumed from the first row after
`display-move-backing-scale` with `TERMSURF_GHOSTBOARD_APP`, `TERMSURF_WEB`,
`TERMSURF_ROAMIUM`, and `TERMSURF_INSTALLED_ROAMIUM` unset.

Summary log:

```text
logs/issue-0826-exp11-remaining-matrix-summary-20260619-132215.log
```

Passed rows:

- `fullscreen-unfullscreen`
- `minimize-hide-restore`
- `font-size-cell-metrics`
- `tui-overlay-resize-command`
- `terminal-scrollback-movement`

First failing row:

- `browser-navigation-geometry`

Summary evidence:

```text
RUN fullscreen-unfullscreen
RESULT fullscreen-unfullscreen PASS
RUN minimize-hide-restore
RESULT minimize-hide-restore PASS
RUN font-size-cell-metrics
RESULT font-size-cell-metrics PASS
RUN tui-overlay-resize-command
RESULT tui-overlay-resize-command PASS
RUN terminal-scrollback-movement
RESULT terminal-scrollback-movement PASS
RUN browser-navigation-geometry
FAIL: missing Roamium received Navigate for browser tab
RESULT browser-navigation-geometry FAIL exit=1
```

Current-run artifact capture worked and recorded required harness, app, and
Roamium paths for every attempted row in:

```text
logs/issue-0826-exp11-artifacts.log
```

The failing row artifacts are:

```text
FAILED_SCENARIO=browser-navigation-geometry
FAILED_APP_LOG=logs/ghostboard-geometry-browser-navigation-geometry-app-20260619-132359.log
FAILED_HARNESS_LOG=logs/ghostboard-geometry-browser-navigation-geometry-harness-20260619-132359.log
FAILED_ROAMIUM_TRACE=logs/ghostboard-geometry-browser-navigation-geometry-roamium-20260619-132359.log
```

Focused failure evidence was written to:

```text
logs/issue-0826-exp11-failure-artifacts.log
logs/issue-0826-exp11-failure-evidence.log
logs/issue-0826-exp11-failure-roamium-evidence.log
logs/issue-0826-exp11-navigation-focused-evidence.log
```

The failing harness reached the navigation test with stable baseline overlay
identity:

```text
navigation_baseline_window_id=13432
navigation_baseline_surface_id=7EA5BCA6-2C9C-4E7B-9151-68498FA993DB
navigation_baseline_selected_tab_id=13432
navigation_baseline_pane_id=7EA5BCA6-2C9C-4E7B-9151-68498FA993DB
navigation_baseline_browser_tab_id=1
navigation_baseline_context_id=4032185689
navigation_baseline_grid=158x43+1+1
navigation_baseline_frame={{8, 17}, {1264, 731}}
navigation_baseline_appkit_pixel=2528x1462
navigation_append_command_text=?termsurf_issue809_exp23=20260619-132359
navigation_edit_key=shift+a=edit-url-end
FAIL: missing Roamium received Navigate for browser tab
```

The app log shows that Chromium did navigate after the edit-key step:

```text
[termsurf-pdf] navigation-throttles frame_tree_node_id=1 url=https://example.com/?termsurf_issue809_exp23=20260619-132359 ...
info(termsurf): TermSurf message decoded type=UrlChanged
info(termsurf): TermSurf message decoded type=TargetUrlChanged
```

The app log also shows the browser entered Browse mode after the navigation
sequence:

```text
info(termsurf): ModeChanged: pane_id=7EA5BCA6-2C9C-4E7B-9151-68498FA993DB browsing=true
info(termsurf): FocusChanged: pane_id=7EA5BCA6-2C9C-4E7B-9151-68498FA993DB tab_id=1 focused=true
```

But the Roamium trace for that run contains resize, mouse, and focus evidence
only; it does not contain the expected `navigate tab=1 pane=... url=...` line:

```text
roamium resize tab_id=1 pane_id=7EA5BCA6-2C9C-4E7B-9151-68498FA993DB pixel_width=2528 pixel_height=1462 ...
roamium mouse-event tab=1 pane=7EA5BCA6-2C9C-4E7B-9151-68498FA993DB ...
roamium focus-changed tab=1 pane=7EA5BCA6-2C9C-4E7B-9151-68498FA993DB ffi=ts_set_focus focused=false
roamium focus-changed tab=1 pane=7EA5BCA6-2C9C-4E7B-9151-68498FA993DB ffi=ts_set_focus focused=true
```

This means the row failed specifically at the expected Roamium `Navigate` trace
evidence. The current evidence does not yet prove whether the product failed to
send a `Navigate` message, Roamium performed navigation through a different path
than the trace expects, or the inherited harness expectation is stale for the
current webtui/Ghostboard navigation flow.

Final checks:

- `bash -n scripts/ghostboard-geometry-matrix.sh` passed.
- Prettier passed for the issue README and this experiment file.
- `git diff --check` passed.
- Cleanup left no stale matching app, web, or Roamium processes:
  `logs/issue-0826-exp11-post-cleanup-processes.log` is empty.
- Forbidden product/source paths are clean:
  `logs/issue-0826-exp11-forbidden-top-status.log` is empty.
- The nested Chromium checkout is clean:
  - `logs/issue-0826-exp11-chromium-status.log` is empty.
  - `logs/issue-0826-exp11-chromium-diff-name-only.log` is empty.
- `logs/issue-0826-exp11-git-diff-name-only.log` contains only the issue README
  and Experiment 11 result documentation changes.

## Conclusion

Experiment 11 proved five more post-display viewport rows on the updated
Ghostboard build. It stopped at the first remaining failure,
`browser-navigation-geometry`, where the page appears to navigate and app-side
URL state changes arrive, but the inherited harness does not see the expected
Roamium `navigate` trace.

The next experiment should localize `browser-navigation-geometry` before any
product change: determine whether `shift+a=edit-url-end` is expected to produce
a TermSurf `Navigate` message in the current webtui flow, whether Ghostboard
sends such a message to Roamium, and whether Roamium should trace that path.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

**Verdict:** Approved.

Required findings: none.

Optional finding and fix:

- The result initially said `logs/issue-0826-exp11-git-diff-name-only.log`
  contained only the README status change, but the result documentation itself
  was added after that log was captured. Fixed by regenerating the diff-name log
  after result recording and updating the final-check note to say it contains
  only the issue README and Experiment 11 result documentation changes.

The reviewer independently verified that
`bash -n scripts/ghostboard-geometry-matrix.sh` and `git diff --check` passed,
that the result commit had not already been made, that the summary log shows
five passing rows followed by the first failure at
`browser-navigation-geometry`, that the failure evidence matches the missing
Roamium `Navigate` trace, and that forbidden product/source paths, cleanup logs,
and Chromium scope logs are clean.
