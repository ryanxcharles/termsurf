# Experiment 9: Tolerate Closed Browser Sockets During Cleanup

## Description

Experiment 8 fixed the `split-right-close-sibling` harness keybinding omission
and proved that the row can pass. The same app log still showed a cleanup-time
product failure after the row passed:

```text
[Roamium] socket EOF — requesting quit
warning(termsurf): CloseTab send failed pane_id=... err=error.NotOpenForWriting
thread ... panic: reached unreachable code
???:?:?: ... in _posix.shutdown
???:?:?: ... in _apprt.termsurf.shutdownServer
???:?:?: ... in _apprt.termsurf.cleanupTuiPanes
```

Spot checks found the same pattern in previously passing rows such as
`initial-open`, `window-resize`, and `split-right`. The failure is therefore not
specific to sibling-close geometry; it is a cleanup-path robustness bug that the
matrix currently masks.

The relevant code is in `ghostboard/src/apprt/termsurf.zig`:

- `cleanupTuiPanes` and `paneClosed` snapshot a `CloseTab` and a server shutdown
  when the last pane for a browser server disappears.
- After releasing `state_mutex`, they send `CloseTab`, log
  `error.NotOpenForWriting` if the browser socket is already gone, then call
  `shutdownServer`.
- `shutdownServer` currently calls
  `std.posix.shutdown(snapshot.browser_fd, .both)`.
- `handleClient` closes every client fd in its `defer`, but only TUI connections
  currently run disconnect cleanup before that close.
- Browser connections store their fd in `servers[index].attached_fd` during
  `ServerRegister`. If the browser client exits first, the server table can
  retain a dead fd number until later TUI cleanup.

On Zig 0.15.2, `std.posix.shutdown` treats `BADF`, `INVAL`, and `NOTSOCK` as
`unreachable`, so a stale browser fd during best-effort cleanup can panic the
app instead of producing a recoverable warning. Cleanup should tolerate an
already-closed or already-disconnected browser socket. More importantly,
Ghostboard must not retain a browser fd after the browser client thread is about
to close it, because a later cleanup could act on a reused fd number.

## Changes

- `ghostboard/src/apprt/termsurf.zig`
  - Add browser-disconnect cleanup in `handleClient` for `.browser` connections.
  - Name the cleanup helper `cleanupBrowserConnection(fd)` so verification can
    assert that the browser disconnect branch calls the intended cleanup path.
  - Under `state_mutex`, find any server whose `attached_fd` matches the
    disconnecting browser fd and invalidate that fd before `handleClient` closes
    it.
  - Preserve enough server identity and child-pid information to log the detach
    and, where possible, reap an already-exiting browser child outside the
    mutex.
  - Ensure later TUI cleanup cannot snapshot or shut down a browser fd that the
    browser client thread has already closed.
  - Keep a defensive best-effort server socket shutdown helper for the normal
    TUI-initiated shutdown path. If a shutdown syscall still sees expected
    teardown errno values such as `BADF`, `NOTCONN`, `NOTSOCK`, or `INVAL`, log
    them instead of panicking.
  - Keep normal successful shutdown logging.
  - Keep unexpected errors logged as warnings.
  - Keep `CloseTab` behavior for live browser sockets; do not skip it when the
    server is still attached.
  - Do not change the TermSurf protocol, browser process launch behavior,
    `webtui/`, `roamium/`, `chromium/`, or `proto/termsurf.proto`.
- `issues/0826-update-ghostboard-to-latest-ghostty/README.md`
  - Link this experiment with status `Designed`, then update the status after
    the result is known.
- `issues/0826-update-ghostboard-to-latest-ghostty/09-tolerate-closed-browser-sockets-during-cleanup.md`
  - Record design, verification, result, reviews, and conclusion.

The intended product behavior is conservative: when the browser socket is still
live, Ghostboard sends `CloseTab` and asks the server to shut down; when the
browser client disconnects first, Ghostboard detaches that fd from the server
table before closing it, logs the disconnect, and lets later pane cleanup
continue without trying to reuse or shut down the stale fd.

## Verification

Before implementation, capture the current failure shape and the Zig stdlib
reason for the panic:

```bash
rg -n 'CloseTab send failed|panic: reached unreachable code|_posix.shutdown|cleanupTuiPanes|shutdownServer' \
  logs/ghostboard-geometry-initial-open-app-20260619-120602.log \
  logs/ghostboard-geometry-window-resize-app-20260619-120611.log \
  logs/ghostboard-geometry-split-right-app-20260619-120623.log \
  logs/ghostboard-geometry-split-right-close-sibling-app-20260619-122413.log \
  > logs/issue-0826-exp09-before-cleanup-panic-evidence.log

sed -n '3690,3735p' /opt/homebrew/Cellar/zig@0.15/0.15.2/lib/zig/std/posix.zig \
  > logs/issue-0826-exp09-zig-shutdown-evidence.log

rg -n 'handleClient|conn_type == \.tui|std\.posix\.close\(fd\)|handleServerRegister|attached_fd = fd|findServerByFd|snapshotServerShutdown|std\.posix\.shutdown' \
  ghostboard/src/apprt/termsurf.zig \
  > logs/issue-0826-exp09-before-fd-lifecycle-evidence.log
```

After implementation, run formatting and static checks:

```bash
zig fmt ghostboard/src/apprt/termsurf.zig
prettier --write --prose-wrap always --print-width 80 \
  issues/0826-update-ghostboard-to-latest-ghostty/README.md \
  issues/0826-update-ghostboard-to-latest-ghostty/09-tolerate-closed-browser-sockets-during-cleanup.md
git diff --check
```

Prove the implementation added browser-disconnect cleanup before the fd close in
`handleClient`:

```bash
awk '
  /fn handleClient/ { in_handle = 1 }
  in_handle && /if \(conn_type == \.browser\) cleanupBrowserConnection\(fd\);/ {
    saw_browser_cleanup = 1
  }
  in_handle && /std\.posix\.close\(fd\)/ {
    saw_close = 1
    exit !(saw_browser_cleanup)
  }
  END { if (!saw_close) exit 1 }
' ghostboard/src/apprt/termsurf.zig \
  > logs/issue-0826-exp09-browser-cleanup-source-evidence.log

awk '
  /fn cleanupBrowserConnection\(fd: std\.posix\.fd_t\)/ { in_fn = 1 }
  in_fn && /attached_fd == fd/ { saw_match = 1 }
  in_fn && /attached_fd = -1/ { saw_clear = 1 }
  in_fn && /^}/ {
    if (saw_match && saw_clear) ok = 1
    in_fn = 0
  }
  END { exit !ok }
' ghostboard/src/apprt/termsurf.zig \
  > logs/issue-0826-exp09-browser-detach-source-evidence.log
```

Build the debug app used by the harness:

```bash
(cd ghostboard && macos/build.nu --configuration Debug --action build \
  > ../logs/issue-0826-exp09-macos-build.log 2>&1)
```

Rerun rows that previously exposed the cleanup panic:

```bash
CLEANUP_SUMMARY="logs/issue-0826-exp09-cleanup-summary-$(date +%Y%m%d-%H%M%S).log"
SCENARIOS=(
  initial-open
  window-resize
  split-right
  split-right-close-sibling
)

set -o pipefail
for scenario in "${SCENARIOS[@]}"; do
  printf 'RUN %s\n' "$scenario" | tee -a "$CLEANUP_SUMMARY"
  if env -u TERMSURF_GHOSTBOARD_APP \
    -u TERMSURF_WEB \
    -u TERMSURF_ROAMIUM \
    -u TERMSURF_INSTALLED_ROAMIUM \
    scripts/ghostboard-geometry-matrix.sh "$scenario" 2>&1 |
    tee -a "$CLEANUP_SUMMARY"; then
    printf 'RESULT %s PASS\n' "$scenario" | tee -a "$CLEANUP_SUMMARY"
  else
    rc=$?
    printf 'RESULT %s FAIL exit=%s\n' "$scenario" "$rc" | tee -a "$CLEANUP_SUMMARY"
    exit "$rc"
  fi
done
printf 'CLEANUP SMOKE PASS\n' | tee -a "$CLEANUP_SUMMARY"
```

Extract the latest app logs and prove that the cleanup panic is gone:

```bash
printf '' > logs/issue-0826-exp09-cleanup-app-logs.txt
printf '' > logs/issue-0826-exp09-cleanup-log-evidence.log
for scenario in initial-open window-resize split-right split-right-close-sibling; do
  log="$(ls -t "logs/ghostboard-geometry-${scenario}-app-"*.log | head -1)"
  printf '%s %s\n' "$scenario" "$log" \
    >> logs/issue-0826-exp09-cleanup-app-logs.txt
  rg -n 'Browser disconnect|Server shutdown|socket already|CloseTab send failed|panic: reached unreachable code|_posix.shutdown' "$log" \
    >> logs/issue-0826-exp09-cleanup-log-evidence.log || true
  rg -n 'Browser disconnect|detached browser|Browser server detached' "$log" \
    > "logs/issue-0826-exp09-${scenario}-browser-detach.log"
done

! rg -n 'panic: reached unreachable code|_posix.shutdown' \
  $(awk '{ print $2 }' logs/issue-0826-exp09-cleanup-app-logs.txt)

rg -n 'Browser disconnect|detached browser|Browser server detached' \
  $(awk '{ print $2 }' logs/issue-0826-exp09-cleanup-app-logs.txt) \
  > logs/issue-0826-exp09-browser-detach-log-evidence.log

! rg -n 'CloseTab send failed|Server shutdown failed|panic: reached unreachable code|_posix.shutdown' \
  $(awk '{ print $2 }' logs/issue-0826-exp09-cleanup-app-logs.txt)
```

If the four cleanup-smoke rows pass without a Ghostboard cleanup panic, resume
the inherited viewport matrix from the first row that was not verified after
Experiment 8:

```bash
REMAINING_SUMMARY="logs/issue-0826-exp09-remaining-matrix-summary-$(date +%Y%m%d-%H%M%S).log"
SCENARIOS=(
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
  printf 'RUN %s\n' "$scenario" | tee -a "$REMAINING_SUMMARY"
  if env -u TERMSURF_GHOSTBOARD_APP \
    -u TERMSURF_WEB \
    -u TERMSURF_ROAMIUM \
    -u TERMSURF_INSTALLED_ROAMIUM \
    scripts/ghostboard-geometry-matrix.sh "$scenario" 2>&1 |
    tee -a "$REMAINING_SUMMARY"; then
    printf 'RESULT %s PASS\n' "$scenario" | tee -a "$REMAINING_SUMMARY"
  else
    rc=$?
    printf 'RESULT %s FAIL exit=%s\n' "$scenario" "$rc" | tee -a "$REMAINING_SUMMARY"
    exit "$rc"
  fi
done
printf 'REMAINING MATRIX PASS\n' | tee -a "$REMAINING_SUMMARY"
```

Reject masked failures and run final scope checks:

```bash
rg -n '^RUN |^RESULT |^FAIL:|CLEANUP SMOKE' "$CLEANUP_SUMMARY" \
  > logs/issue-0826-exp09-cleanup-summary-status.log
! rg -n '^FAIL:|RESULT .*FAIL' "$CLEANUP_SUMMARY"

rg -n '^RUN |^RESULT |^FAIL:|REMAINING MATRIX' "$REMAINING_SUMMARY" \
  > logs/issue-0826-exp09-summary-status.log
! rg -n '^FAIL:|RESULT .*FAIL' "$REMAINING_SUMMARY"

ps -axo pid,comm,args \
  | rg 'TermSurf\.app/Contents/MacOS/termsurf|target/debug/web|chromium/src/out/Default/roamium' \
  | rg -v 'rg|ps -axo|zsh -lc' \
  > logs/issue-0826-exp09-post-cleanup-processes.log || true
git status --short -- webtui roamium proto/termsurf.proto chromium/README.md chromium/patches \
  > logs/issue-0826-exp09-forbidden-top-status.log
git -C chromium/src status --short > logs/issue-0826-exp09-chromium-status.log
git -C chromium/src diff --name-only > logs/issue-0826-exp09-chromium-diff-name-only.log
test ! -s logs/issue-0826-exp09-forbidden-top-status.log
test ! -s logs/issue-0826-exp09-chromium-status.log
test ! -s logs/issue-0826-exp09-chromium-diff-name-only.log
```

Pass criteria:

- Browser client disconnect cleanup invalidates matching `servers[].attached_fd`
  before `handleClient` closes the fd.
- Later TUI cleanup cannot send `CloseTab` or call server shutdown on a browser
  fd that was already closed by the browser client thread.
- Source-level verification proves a `.browser` disconnect cleanup call occurs
  before `std.posix.close(fd)` in `handleClient`.
- Source-level verification proves `cleanupBrowserConnection(fd)` matches
  `attached_fd == fd` and clears the matched `attached_fd`.
- Cleanup-smoke app logs show browser fd detach/disconnect evidence and do not
  contain `CloseTab send failed` or `Server shutdown failed`.
- Each cleanup-smoke row has its own non-empty browser-detach evidence log.
- `ghostboard/src/apprt/termsurf.zig` no longer uses `std.posix.shutdown` for
  best-effort browser server teardown.
- Expected stale/disconnected browser-socket teardown errors are logged or
  ignored without panic.
- Live browser sockets still receive `CloseTab` and server shutdown.
- `zig fmt`, Prettier, and `git diff --check` are clean.
- The debug macOS app build passes.
- `initial-open`, `window-resize`, `split-right`, and
  `split-right-close-sibling` pass with overrides unset.
- The latest app logs for those rows do not contain
  `panic: reached unreachable code` or `_posix.shutdown`.
- The remaining inherited matrix rows either pass, or the first remaining
  failure is recorded with logs for the next experiment.
- Cleanup leaves no stale matching app, web, or Roamium processes.
- No forbidden paths are modified: `webtui/`, `roamium/`, `chromium/`, or
  `proto/termsurf.proto`.
- The nested `chromium/src` checkout has no uncommitted status or diff from this
  experiment.

Partial criteria:

- The cleanup panic is fixed for the four rows that previously exposed it, but a
  later viewport-matrix row fails with clear evidence.
- The implementation fixes fd detachment and removes the Zig
  `std.posix.shutdown` panic path but reveals a different cleanup or Roamium
  shutdown failure that needs a narrower follow-up experiment.

Fail criteria:

- The app can still panic in `_posix.shutdown` during the four cleanup-smoke
  rows.
- The fix leaves `servers[].attached_fd` pointing at a browser fd after the
  browser client thread closes it.
- The fix hides real protocol failures by skipping `CloseTab` or server shutdown
  when the browser socket is still usable.
- A matrix failure is hidden by shell pipeline behavior.

## Design Review

An adversarial Codex subagent reviewed the initial design with fresh context.

**Verdict:** Changes required.

Required finding and fix:

- The initial plan treated the cleanup failure mostly as a `std.posix.shutdown`
  errno-handling problem. The reviewer pointed out that `handleClient` closes
  every client fd, browser fds are stored in `servers[].attached_fd`, and there
  was no browser-disconnect cleanup clearing `attached_fd` before close. A raw
  `shutdown` wrapper would avoid the Zig panic but could still act on an fd
  number reused by a different client. Fixed by making browser-disconnect fd
  detachment the primary design requirement, with defensive raw-shutdown
  handling only for the normal live-server shutdown path.
- The first re-review found that the design required fd detachment but did not
  make verification prove it; a raw-shutdown-only fix could still have passed.
  Fixed by adding source-level checks for `.browser` cleanup before
  `std.posix.close(fd)`, detach-source evidence, required detach/disconnect log
  evidence, and hard rejection of `CloseTab send failed` or
  `Server shutdown failed` in the cleanup-smoke app logs.
- The second re-review found that the source check still only proved an
  arbitrary `.browser` branch, and the detach-source grep could match unrelated
  initialization or guards. Fixed by requiring a concrete
  `cleanupBrowserConnection(fd)` call before close, plus an `awk` check that the
  helper body matches `attached_fd == fd` and assigns `attached_fd = -1`. The
  same re-review noted that aggregate detach-log evidence could pass with only
  one row logging detach; fixed by requiring a per-scenario detach evidence log
  inside the cleanup-smoke loop.

Optional finding and fix:

- The first draft wrote forbidden-path and nested Chromium status to log files
  but did not make non-empty logs fail verification. Fixed by adding explicit
  `test ! -s` checks for all three status/diff evidence files.

The final re-review approved the design with no remaining required findings.
