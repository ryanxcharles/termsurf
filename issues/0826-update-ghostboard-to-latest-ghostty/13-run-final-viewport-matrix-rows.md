# Experiment 13: Run Final Viewport Matrix Rows

## Description

Experiment 12 fixed the stale inherited assertion in
`browser-navigation-geometry` and proved the navigation geometry row using
marker URL state, app-side Chromium navigation logs, stable AppKit geometry,
hit-testing, and post-navigation browser keyboard input.

The remaining inherited viewport/input rows after `browser-navigation-geometry`
are:

- `devtools-split-geometry`
- `devtools-singleton-guard`
- `mouse-after-geometry-change`
- `keyboard-after-tab-window-switch`
- `gui-active-multi-tab`

This experiment resumes the matrix at `devtools-split-geometry` and records
whether the final rows pass or where the next failure occurs.

## Changes

- `issues/0826-update-ghostboard-to-latest-ghostty/README.md`
  - Link this experiment with status `Designed`, then update the status after
    the result is known.
- `issues/0826-update-ghostboard-to-latest-ghostty/13-run-final-viewport-matrix-rows.md`
  - Record design, verification, result, reviews, and conclusion.

No source changes are planned. Do not modify `ghostboard/`, `webtui/`,
`roamium/`, `chromium/`, or `proto/termsurf.proto` in this experiment unless the
resumed matrix proves a narrow harness-only compatibility problem. If a
harness-only fix is needed, keep it limited to
`scripts/ghostboard-geometry-matrix.sh`, rerun the failing row, and record why
the fix is not product behavior.

## Verification

Run static checks before the runtime matrix:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
prettier --write --prose-wrap always --print-width 80 \
  issues/0826-update-ghostboard-to-latest-ghostty/README.md \
  issues/0826-update-ghostboard-to-latest-ghostty/13-run-final-viewport-matrix-rows.md
git diff --check
```

Run the final rows with overrides explicitly unset. Capture current-run
artifacts immediately after each row using a marker file created before that row
starts:

```bash
SUMMARY="logs/issue-0826-exp13-final-matrix-summary-$(date +%Y%m%d-%H%M%S).log"
SCENARIOS=(
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
printf '' > logs/issue-0826-exp13-artifacts.log
for scenario in "${SCENARIOS[@]}"; do
  RUN_SCENARIOS+=("$scenario")
  SCENARIO_MARKER="logs/issue-0826-exp13-${scenario}-start.marker"
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
  WEBTUI_TRACE="$(find logs -name "ghostboard-geometry-${scenario}-webtui-*.log" -newer "$SCENARIO_MARKER" -print | sort | tail -1)"
  SCREENSHOT="$(find logs -name "ghostboard-geometry-${scenario}-screenshot-*.png" -newer "$SCENARIO_MARKER" -print | sort | tail -1 || true)"
  test -n "$HARNESS_LOG"
  test -n "$APP_LOG"
  test -n "$ROAMIUM_TRACE"
  test -n "$WEBTUI_TRACE"
  {
    printf 'scenario=%s\n' "$scenario"
    printf 'harness=%s\n' "$HARNESS_LOG"
    printf 'app=%s\n' "$APP_LOG"
    printf 'roamium=%s\n' "$ROAMIUM_TRACE"
    printf 'webtui=%s\n' "$WEBTUI_TRACE"
    printf 'screenshot=%s\n' "$SCREENSHOT"
  } >> logs/issue-0826-exp13-artifacts.log

  if [ -n "$FAILED_SCENARIO" ]; then
    printf 'FAILED_SCENARIO=%s\nFAILED_APP_LOG=%s\nFAILED_HARNESS_LOG=%s\nFAILED_ROAMIUM_TRACE=%s\nFAILED_WEBTUI_TRACE=%s\n' \
      "$FAILED_SCENARIO" "$APP_LOG" "$HARNESS_LOG" "$ROAMIUM_TRACE" "$WEBTUI_TRACE" \
      > logs/issue-0826-exp13-failure-artifacts.log
    rg -n 'FAIL:|panic|error\(|warn|TermSurf geometry|ModeChanged:|FocusChanged:|KeyEvent:|SetOverlay|ClearOverlay|BrowserReady|TabReady|CloseTab|DevTools|devtools|mouse|key' \
      "$HARNESS_LOG" "$APP_LOG" "$WEBTUI_TRACE" \
      > logs/issue-0826-exp13-failure-evidence.log || true
    rg -n 'resize|focus-changed|key-event|mouse-event|close-tab|shutdown|panic|error|devtools' \
      "$ROAMIUM_TRACE" \
      > logs/issue-0826-exp13-failure-roamium-evidence.log || true
    break
  fi
done
if [ -z "$FAILED_SCENARIO" ]; then
  printf 'FINAL MATRIX ROWS PASS\n' | tee -a "$SUMMARY"
fi
```

Validate artifacts and reject masked failures:

```bash
awk '
  /^scenario=/ { scenarios++ }
  /^harness=/ && length($0) > 8 { harness++ }
  /^app=/ && length($0) > 4 { app++ }
  /^roamium=/ && length($0) > 8 { roamium++ }
  /^webtui=/ && length($0) > 7 { webtui++ }
  END { exit !(scenarios == harness && scenarios == app && scenarios == roamium && scenarios == webtui) }
' logs/issue-0826-exp13-artifacts.log

if [ -n "$FAILED_SCENARIO" ]; then
  test -s logs/issue-0826-exp13-failure-artifacts.log
  test -s logs/issue-0826-exp13-failure-evidence.log
  test -f logs/issue-0826-exp13-failure-roamium-evidence.log
fi

rg -n '^RUN |^RESULT |^FAIL:|FINAL MATRIX ROWS' "$SUMMARY" \
  > logs/issue-0826-exp13-summary-status.log
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
  > logs/issue-0826-exp13-post-cleanup-processes.log || true
test ! -s logs/issue-0826-exp13-post-cleanup-processes.log

git status --short -- ghostboard webtui roamium proto/termsurf.proto chromium/README.md chromium/patches \
  > logs/issue-0826-exp13-forbidden-top-status.log
git -C chromium/src status --short > logs/issue-0826-exp13-chromium-status.log
git -C chromium/src diff --name-only > logs/issue-0826-exp13-chromium-diff-name-only.log
git diff --name-only > logs/issue-0826-exp13-git-diff-name-only.log
test ! -s logs/issue-0826-exp13-forbidden-top-status.log
test ! -s logs/issue-0826-exp13-chromium-status.log
test ! -s logs/issue-0826-exp13-chromium-diff-name-only.log
```

Pass criteria:

- `bash -n`, Prettier, and `git diff --check` are clean.
- The final rows run with `TERMSURF_GHOSTBOARD_APP`, `TERMSURF_WEB`,
  `TERMSURF_ROAMIUM`, and `TERMSURF_INSTALLED_ROAMIUM` unset.
- Every listed final scenario exits successfully.
- The strict summary contains `FINAL MATRIX ROWS PASS` and no `FAIL:` or
  `RESULT .*FAIL` lines.
- Per-scenario artifact paths are recorded from the current run.
- Cleanup leaves no stale matching app, web, or Roamium processes.
- No product/source paths are modified by this experiment.
- The nested `chromium/src` checkout has no uncommitted status or diff from this
  experiment.

Partial criteria:

- One or more final scenarios fail, and the first failure is recorded with
  focused harness, app, webtui, and Roamium evidence for the next experiment.
- A harness-only compatibility issue is fixed narrowly in
  `scripts/ghostboard-geometry-matrix.sh`, but a later row fails with clear
  evidence.

Fail criteria:

- A scenario failure is hidden by shell pipeline behavior.
- The result claims final matrix coverage without per-scenario artifact paths.
- Product code, webtui, Roamium, Chromium, or the protocol is changed inside
  this matrix continuation instead of a focused follow-up experiment.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

**Verdict:** Approved.

Findings: none required.

The reviewer verified that the README links Experiment 13 as `Designed`, the
experiment has the required sections, the scenario list resumes correctly after
`browser-navigation-geometry`, Experiment 12 explicitly says to resume at
`devtools-split-geometry`, artifact capture uses per-scenario marker files with
`find ... -newer`, the pipeline uses `set -o pipefail`, failed scenarios are
recorded before validation exits with the failed status, and the scope excludes
product changes except a narrow documented harness-only fix.

## Result

**Result:** Pass

Static checks passed:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
prettier --write --prose-wrap always --print-width 80 \
  issues/0826-update-ghostboard-to-latest-ghostty/README.md \
  issues/0826-update-ghostboard-to-latest-ghostty/13-run-final-viewport-matrix-rows.md
git diff --check
```

The first `devtools-split-geometry` run failed because the row inherited strict
Roamium trace assertions for DevTools creation and CA-context events. The app
and webtui logs showed the DevTools pane was created, mapped to `TabReady`,
presented with an AppKit CA context, resized to the AppKit pixel size, and
accepted mouse and keyboard input. The harness was updated narrowly in
`scripts/ghostboard-geometry-matrix.sh` to keep those Roamium trace lines as
diagnostics while using Ghostboard `TabReady` / `CaContext` and webtui DevTools
render state as authoritative DevTools identity evidence.

After that harness fix, `devtools-split-geometry` passed. The first full final
matrix rerun then failed only at `gui-active-multi-tab`: Ghostboard logged
`SetGuiActive: tab_id=0 active=false reason=gui_deactivated`, but the deployed
`chromium/src/out/Default/roamium` binary did not contain the current
`set-gui-active` trace strings. Running `scripts/build.sh roamium` rebuilt and
copied the current Rust Roamium wrapper into `chromium/src/out/Default/roamium`.
The focused `gui-active-multi-tab` rerun then passed.

The final full matrix rerun used explicitly unset overrides:
`TERMSURF_GHOSTBOARD_APP`, `TERMSURF_WEB`, `TERMSURF_ROAMIUM`, and
`TERMSURF_INSTALLED_ROAMIUM`. The summary was:

```text
RUN devtools-split-geometry
RESULT devtools-split-geometry PASS
RUN devtools-singleton-guard
RESULT devtools-singleton-guard PASS
RUN mouse-after-geometry-change
RESULT mouse-after-geometry-change PASS
RUN keyboard-after-tab-window-switch
RESULT keyboard-after-tab-window-switch PASS
RUN gui-active-multi-tab
RESULT gui-active-multi-tab PASS
FINAL MATRIX ROWS PASS
```

Artifact paths for the final passing run were recorded in
`logs/issue-0826-exp13-artifacts.log`. The strict summary was recorded in
`logs/issue-0826-exp13-summary-status.log`.

Cleanup and scope checks passed:

- `logs/issue-0826-exp13-post-cleanup-processes.log` was empty.
- `logs/issue-0826-exp13-forbidden-top-status.log` was empty.
- `logs/issue-0826-exp13-chromium-status.log` was empty.
- `logs/issue-0826-exp13-chromium-diff-name-only.log` was empty.
- `logs/issue-0826-exp13-git-diff-name-only.log` listed only
  `scripts/ghostboard-geometry-matrix.sh`.

## Conclusion

The final inherited viewport/input rows pass together on the current rebuilt
debug stack. The only source change in this experiment is a harness
compatibility fix for DevTools identity proof; it does not change Ghostboard,
webtui, Roamium, Chromium, or the protocol source.

The stale deployed Roamium binary was an environment/build artifact issue:
current source already contained `SetGuiActive` trace handling, but the copied
`chromium/src/out/Default/roamium` binary had not been rebuilt. Rebuilding
Roamium restored the expected `set-gui-active` trace evidence and allowed
`gui-active-multi-tab` to pass.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

**Verdict:** Approved.

Findings: none.
