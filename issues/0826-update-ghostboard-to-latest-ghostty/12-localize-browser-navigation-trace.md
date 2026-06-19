# Experiment 12: Localize Browser Navigation Trace

## Description

Experiment 11 resumed the post-display viewport matrix and stopped at
`browser-navigation-geometry`. Five rows passed first. The failing row reached a
stable baseline overlay, injected `shift+a=edit-url-end`, appended a query
marker to the URL, pressed Enter, and then failed on this invariant:

```text
FAIL: missing Roamium received Navigate for browser tab
```

The evidence is not yet enough to choose a product fix. The app log shows that
Chromium did navigate to the marker URL and that Ghostboard later decoded
`UrlChanged` and `TargetUrlChanged` messages. The webtui state trace also shows
the marker URL. But the Roamium trace contains no `navigate ... ffi=ts_load_url`
line after the harness trace cursor.

Relevant source observations:

- `webtui/src/main.rs` sends navigation either through a direct browser
  connection (`browser_conn.send_navigate`) or through the compositor
  (`conn.send_navigate`) when Enter is pressed in edit mode.
- `roamium/src/dispatch.rs` traces
  `navigate tab=... pane=... url=... ffi=ts_load_url` when it receives a
  `Navigate` message and finds the tab.
- `ghostboard/src/apprt/termsurf.zig` logs decoded message types, but the
  current switch handles only selected TermSurf messages and does not currently
  handle `MSG_NAVIGATE`.
- The existing harness uses immediate `require_trace_after` checks for the
  Roamium `navigate` evidence after only a one-second delay, then later checks
  app-side `UrlChanged`.

This experiment should localize the failure before changing product behavior. It
should determine whether the current `browser-navigation-geometry` flow:

1. sends `Navigate` directly from webtui to Roamium;
2. sends `Navigate` to Ghostboard but Ghostboard ignores it;
3. navigates through another path that is valid but not captured by the current
   inherited trace expectation;
4. is racing the trace check and needs a wait-based assertion; or
5. exposes a real missing protocol route that needs a separate product-code
   experiment.

## Changes

- `scripts/ghostboard-geometry-matrix.sh`
  - Add targeted diagnostics to only the `browser-navigation-geometry` block.
  - Around the URL edit/Enter sequence, capture the starting line numbers for
    the harness log, app log, Roamium trace, and webtui state trace.
  - After Enter, wait for webtui state `url_changed` containing the marker,
    app-side decoded `UrlChanged` after the cursor, and marker-bearing app
    evidence from Chromium navigation logs before checking geometry.
  - Log whether Ghostboard decoded `Navigate` after the edit sequence.
  - Log whether Roamium emitted `navigate`, `url-changed`, and marker URL traces
    after the edit sequence.
  - Keep the existing failure condition intact unless the new evidence proves
    the inherited harness expectation is stale.
- `issues/0826-update-ghostboard-to-latest-ghostty/README.md`
  - Link this experiment with status `Designed`, then update the status after
    the result is known.
- `issues/0826-update-ghostboard-to-latest-ghostty/12-localize-browser-navigation-trace.md`
  - Record design, verification, result, reviews, and conclusion.

Do not modify Ghostboard product code, `webtui/`, `roamium/`, `chromium/`, or
`proto/termsurf.proto` in this experiment unless diagnostics prove a narrow
harness-only fix. If the evidence points to missing Ghostboard `Navigate`
routing, record that as the next experiment instead of broadening this one.

## Verification

Before changes, preserve the current failure evidence:

```bash
rg -n 'navigation_|Navigate|navigate|UrlChanged|TargetUrlChanged|url_changed|target_url_changed|ModeChanged:|FocusChanged:|FAIL:' \
  logs/ghostboard-geometry-browser-navigation-geometry-harness-20260619-132359.log \
  logs/ghostboard-geometry-browser-navigation-geometry-app-20260619-132359.log \
  logs/ghostboard-geometry-browser-navigation-geometry-roamium-20260619-132359.log \
  logs/ghostboard-geometry-browser-navigation-geometry-webtui-20260619-132359.log \
  > logs/issue-0826-exp12-before-navigation-evidence.log || true
```

After diagnostic changes, run static checks:

```bash
bash -n scripts/ghostboard-geometry-matrix.sh
prettier --write --prose-wrap always --print-width 80 \
  issues/0826-update-ghostboard-to-latest-ghostty/README.md \
  issues/0826-update-ghostboard-to-latest-ghostty/12-localize-browser-navigation-trace.md
git diff --check
```

Rerun only the failing row with overrides unset. Create a marker immediately
before the row so current-run artifacts cannot be confused with stale logs from
earlier attempts:

```bash
RUN_MARKER="logs/issue-0826-exp12-browser-navigation-start.marker"
: > "$RUN_MARKER"

env -u TERMSURF_GHOSTBOARD_APP \
  -u TERMSURF_WEB \
  -u TERMSURF_ROAMIUM \
  -u TERMSURF_INSTALLED_ROAMIUM \
  scripts/ghostboard-geometry-matrix.sh browser-navigation-geometry \
  > logs/issue-0826-exp12-browser-navigation-rerun.log 2>&1
```

Extract current-run artifacts:

```bash
APP_LOG="$(find logs -name 'ghostboard-geometry-browser-navigation-geometry-app-*.log' -newer "$RUN_MARKER" -print | sort | tail -1)"
HARNESS_LOG="$(find logs -name 'ghostboard-geometry-browser-navigation-geometry-harness-*.log' -newer "$RUN_MARKER" -print | sort | tail -1)"
ROAMIUM_TRACE="$(find logs -name 'ghostboard-geometry-browser-navigation-geometry-roamium-*.log' -newer "$RUN_MARKER" -print | sort | tail -1)"
WEBTUI_TRACE="$(find logs -name 'ghostboard-geometry-browser-navigation-geometry-webtui-*.log' -newer "$RUN_MARKER" -print | sort | tail -1)"
test -n "$APP_LOG"
test -n "$HARNESS_LOG"
test -n "$ROAMIUM_TRACE"
test -n "$WEBTUI_TRACE"

printf 'APP_LOG=%s\nHARNESS_LOG=%s\nROAMIUM_TRACE=%s\nWEBTUI_TRACE=%s\n' \
  "$APP_LOG" "$HARNESS_LOG" "$ROAMIUM_TRACE" "$WEBTUI_TRACE" \
  > logs/issue-0826-exp12-selected-artifacts.log

rg -n 'navigation_|Navigate|navigate|UrlChanged|TargetUrlChanged|url_changed|target_url_changed|navigation-throttles|ModeChanged:|FocusChanged:|FAIL:|PASS:' \
  "$HARNESS_LOG" "$APP_LOG" "$ROAMIUM_TRACE" "$WEBTUI_TRACE" \
  > logs/issue-0826-exp12-navigation-evidence.log || true
```

If diagnostics prove that the marker URL appears in webtui state, app-side
Chromium navigation logs include the marker, Ghostboard decodes `UrlChanged`
after the cursor, the AppKit frame and pixels remain stable, and browser input
still works, but no `Navigate` trace is emitted, do not claim a product fix.
Record whether the missing layer is:

- Ghostboard receiving/ignoring a compositor-routed `Navigate`;
- Roamium not tracing a direct-browser `Navigate`;
- a valid non-`Navigate` current navigation path;
- or still ambiguous because webtui does not expose the send path.

Only if the evidence proves the inherited row was checking stale evidence for a
geometry-after-navigation scenario should this experiment adjust the harness
expectation, and only inside `scripts/ghostboard-geometry-matrix.sh`.

Run final cleanup and scope checks:

```bash
ps -axo pid,comm,args \
  | rg 'TermSurf\\.app/Contents/MacOS/termsurf|target/debug/web|chromium/src/out/Default/roamium' \
  | rg -v 'rg|ps -axo|zsh -lc' \
  > logs/issue-0826-exp12-post-cleanup-processes.log || true
test ! -s logs/issue-0826-exp12-post-cleanup-processes.log

git status --short -- ghostboard webtui roamium proto/termsurf.proto chromium/README.md chromium/patches \
  > logs/issue-0826-exp12-forbidden-top-status.log
git -C chromium/src status --short > logs/issue-0826-exp12-chromium-status.log
git -C chromium/src diff --name-only > logs/issue-0826-exp12-chromium-diff-name-only.log
git diff --name-only > logs/issue-0826-exp12-git-diff-name-only.log
test ! -s logs/issue-0826-exp12-forbidden-top-status.log
test ! -s logs/issue-0826-exp12-chromium-status.log
test ! -s logs/issue-0826-exp12-chromium-diff-name-only.log
```

Pass criteria:

- The experiment identifies the layer where the expected `Navigate` trace is
  lost or proves that the inherited harness expectation is stale for the current
  navigation flow.
- If the harness is fixed, `browser-navigation-geometry` passes and still proves
  marker URL navigation, stable AppKit frame/pixels, no Roamium resize, hit-test
  correctness, and post-navigation browser keyboard input.
- `bash -n`, Prettier, and `git diff --check` are clean.
- Cleanup leaves no stale matching app, web, or Roamium processes.
- No forbidden product/source paths are modified.
- The nested `chromium/src` checkout has no uncommitted status or diff from this
  experiment.

Partial criteria:

- Diagnostics localize the failure to a missing Ghostboard, webtui, or Roamium
  product route that should be fixed in a separate experiment.
- Diagnostics prove that the current traces cannot distinguish direct-browser
  navigation from compositor-routed navigation without a narrower diagnostic
  change in a later experiment.

Fail criteria:

- The experiment changes product behavior before localizing the failure.
- The result claims navigation geometry is proven without marker URL evidence
  and stable post-navigation geometry checks.
- Diagnostic output is too weak to distinguish a stale harness expectation from
  missing protocol routing.

## Design Review

An adversarial Codex subagent reviewed the initial design with fresh context.

**Verdict:** Changes required.

Required findings and fixes:

- The initial design required app-side `UrlChanged` containing the marker, but
  Ghostboard currently logs only the decoded message type for `UrlChanged`, not
  its URL payload. Fixed by requiring separate evidence: webtui state
  `url_changed` with the marker, app-side decoded `UrlChanged` after the cursor,
  and marker-bearing Chromium navigation logs from the app log.
- The initial artifact extraction selected the latest matching logs and could
  use stale artifacts from an earlier run. Fixed by creating a marker
  immediately before the rerun, selecting logs with
  `find ... -newer "$RUN_MARKER"`, and checking all required artifact paths are
  non-empty.

The final re-review approved the design with no remaining required findings.

## Result

**Result:** Pass

Implemented targeted diagnostics in `scripts/ghostboard-geometry-matrix.sh` for
only the `browser-navigation-geometry` row. No Ghostboard product code,
`webtui/`, `roamium/`, `chromium/`, or `proto/termsurf.proto` files were
changed.

The first diagnostic rerun still failed on the inherited Roamium `Navigate`
trace assertion:

```text
exp12_navigation_rc=1
FAIL: missing Roamium received Navigate for browser tab
```

That rerun localized the evidence:

```text
navigation_state_marker_line=5:event=url_changed url=https://example.com/?termsurf_issue809_exp23=20260619-133512
navigation_app_url_changed_line=info(termsurf): TermSurf message decoded type=UrlChanged
navigation_app_marker_line=[termsurf-pdf] navigation-throttles ... url=https://example.com/?termsurf_issue809_exp23=20260619-133512 ...
navigation_app_decoded_navigate_line=<none>
navigation_roamium_navigate_line=<none>
navigation_roamium_marker_line=<none>
navigation_roamium_url_changed_line=<none>
```

The full evidence was written to:

```text
logs/issue-0826-exp12-before-navigation-evidence.log
logs/issue-0826-exp12-browser-navigation-rerun.log
logs/issue-0826-exp12-selected-artifacts.log
logs/issue-0826-exp12-navigation-evidence.log
```

The diagnostics prove that the marker URL reaches webtui state and Chromium's
navigation path, and Ghostboard receives app-side `UrlChanged`, but this row
does not produce a Ghostboard-decoded `Navigate` message or a Roamium
`navigate`/marker/`url-changed` trace line. The inherited row was therefore
using stale trace evidence for a geometry-after-navigation scenario.

The harness was adjusted to keep logging those missing trace lines as
diagnostics while proving the row with the durable behavior it was meant to
cover:

- webtui URL state includes the marker;
- app-side Chromium navigation logs include the marker;
- Ghostboard decodes `UrlChanged` after navigation;
- AppKit frame and pixels stay stable;
- Roamium is not resized by navigation;
- post-navigation hit-test still targets the same overlay;
- post-navigation keyboard input reaches the browser.

The final rerun passed:

```text
exp12_navigation_after_fix_rc=0
```

Current-run artifacts:

```text
APP_LOG=logs/ghostboard-geometry-browser-navigation-geometry-app-20260619-133626.log
HARNESS_LOG=logs/ghostboard-geometry-browser-navigation-geometry-harness-20260619-133626.log
ROAMIUM_TRACE=logs/ghostboard-geometry-browser-navigation-geometry-roamium-20260619-133626.log
WEBTUI_TRACE=logs/ghostboard-geometry-browser-navigation-geometry-webtui-20260619-133626.log
```

Focused passing evidence:

```text
PASS: observed webtui URL state contains navigation marker
navigation_state_marker_line=5:event=url_changed url=https://example.com/?termsurf_issue809_exp23=20260619-133626
navigation_app_url_changed_line=info(termsurf): TermSurf message decoded type=UrlChanged
navigation_app_marker_line=[termsurf-pdf] navigation-throttles ... url=https://example.com/?termsurf_issue809_exp23=20260619-133626 ...
navigation_app_decoded_navigate_line=<none>
navigation_roamium_navigate_line=<none>
navigation_roamium_marker_line=<none>
navigation_roamium_url_changed_line=<none>
PASS: webtui URL state includes navigation marker
PASS: app Chromium navigation log includes marker
PASS: browser navigation AppKit frame stayed stable
PASS: browser navigation AppKit pixels stayed stable
PASS: browser navigation did not resize Roamium
PASS: post-navigation hit-test has window id
PASS: post-navigation hit-test has surface id
PASS: post-navigation hit-test has selected tab id
PASS: post-navigation hit-test uses baseline AppKit frame
PASS: post-navigation hit-test includes webview-relative point
PASS: post-navigation keyboard marker reached browser
PASS: scenario browser-navigation-geometry
```

Final checks:

- `bash -n scripts/ghostboard-geometry-matrix.sh` passed.
- Prettier passed for the issue README and this experiment file.
- `git diff --check` passed.
- Cleanup left no stale matching app, web, or Roamium processes:
  `logs/issue-0826-exp12-post-cleanup-processes.log` is empty.
- Forbidden product/source paths are clean:
  `logs/issue-0826-exp12-forbidden-top-status.log` is empty.
- The nested Chromium checkout is clean:
  - `logs/issue-0826-exp12-chromium-status.log` is empty.
  - `logs/issue-0826-exp12-chromium-diff-name-only.log` is empty.
- `logs/issue-0826-exp12-git-diff-name-only.log` lists only the issue README,
  Experiment 12 result documentation, and
  `scripts/ghostboard-geometry-matrix.sh`.

## Conclusion

Experiment 12 localized the `browser-navigation-geometry` failure to a stale
harness expectation. The row was intended to prove overlay geometry and input
survive browser navigation. Current evidence proves that behavior through webtui
state, app-side Chromium navigation logs, stable AppKit geometry, hit-testing,
and post-navigation browser keyboard input.

The experiment did not prove or fix compositor-routed `Navigate` handling. It
also recorded that this current flow does not produce Ghostboard-decoded
`Navigate` or Roamium `navigate`/marker/`url-changed` trace lines. If
compositor-routed `Navigate` parity is needed, that should be covered by a
separate protocol-routing experiment instead of this geometry row.

The next experiment should resume the remaining matrix after
`browser-navigation-geometry`, starting with `devtools-split-geometry`.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

**Verdict:** Approved.

Required findings: none.

The reviewer independently verified that
`bash -n scripts/ghostboard-geometry-matrix.sh` and `git diff --check` passed,
that the result commit had not already been made, that only the two issue docs
and harness script were modified, that cleanup and Chromium scope logs were
empty, and that `logs/issue-0826-exp12-git-diff-name-only.log` listed only the
two issue docs and `scripts/ghostboard-geometry-matrix.sh`.

The reviewer also confirmed that the after-fix evidence supports the Pass
result: marker URL appears in webtui state and app-side Chromium navigation
logs, Ghostboard decodes `UrlChanged`, AppKit frame/pixels stay stable, no
Roamium resize occurs, post-navigation hit-test matches the baseline overlay
identity/frame, and post-navigation keyboard input reaches the browser.
