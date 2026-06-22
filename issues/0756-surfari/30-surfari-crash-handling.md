# Experiment 30: Prove Surfari crash handling

## Description

Experiment 29 proved Surfari profile isolation. The remaining real-app Surfari
matrix row before the final Ghostboard/Roamium comparison is `Crash handling`.

Experiment 14 already proved `libtermsurf_webkit` can report WebKit web-content
process termination through WebKit's delegate path in the C smoke harness.
Experiment 30 should prove the same class of failure inside the real TermSurf
app:

- Surfari detects a WebKit web-content process termination for the active tab.
- Surfari sends a `RendererCrashed` protocol message to Ghostboard/WebTUI with
  the correct tab ID, status, status code, URL, and reloadability.
- WebTUI records the crash state and renders it as active without leaving the
  loading bar stuck.
- Stale post-crash loading/state events do not clear the crash state before an
  explicit recovery navigation.
- A recovery navigation reloads real content in the same Surfari tab/process and
  clears WebTUI's crash state.

Use Roamium's `renderer-crash-smoke` scenario in
`scripts/ghostboard-geometry-matrix.sh` as the behavioral reference, but do not
reuse `chrome://crash/` blindly: WebKit does not share Chromium's crash URL.
Surfari already has the C test helper
`ts_webkit_test_kill_web_content_process(ts_web_contents_t wc)` from
Experiment 14. This experiment may add an environment-gated Surfari test hook
that calls that helper for a deterministic real-app crash trigger.

This experiment should not expand into the final Ghostboard/Roamium comparison
or unrelated crash-reporting infrastructure.

## Changes

- Add a focused Surfari crash-handling real-app harness under `scripts/`.
- Launch the real Debug `TermSurf.app` with repo-built `web --browser surfari`
  and repo-built `surfari`, explicitly setting
  `TERMSURF_SURFARI_PATH=$ROOT/target/debug/surfari` for Ghostboard.
- Serve deterministic local HTTP fixtures:
  - an initial page that logs an initial-ready marker and sets a stable title;
  - a recovery page that logs a recovery marker and sets a stable title.
- Trigger a deterministic WebKit web-content process termination from the real
  Surfari process. Preferred path:
  - add a Surfari-only, environment-gated test hook such as
    `TERMSURF_SURFARI_TEST_RENDERER_CRASH_URL`;
  - when Surfari receives a `Navigate` to that exact URL and the environment
    variable is set, call
    `ts_webkit_test_kill_web_content_process(ts_web_contents_t wc)` for the
    target tab instead of trying to load the URL;
  - for this deterministic helper path, expect status `requested`, status code
    `0`, `can_reload=true`, and the pre-crash page URL as the crash URL;
  - keep the hook disabled by default and unreachable unless the explicit test
    environment variable is present.
- If the direct C helper is not enough in the real app, localize the failure to
  the narrowest boundary: WebKit delegate callback, `libtermsurf_webkit` C ABI,
  Surfari Rust dispatch, Ghostboard routing, or WebTUI state handling.
- Audit the renderer-crash boolean semantics. The protocol field is
  `RendererCrashed.can_reload`, but the current WebKit C callback names its
  boolean `visible`. If real-app evidence shows Surfari is forwarding visibility
  as reloadability, fix the boundary so WebTUI receives a true `can_reload`
  value for recoverable Surfari crashes.
- Reuse the existing direct browser-socket `Navigate` helper pattern from
  Experiments 25 and 29 where practical, so recovery does not depend on the
  separate URL editor or Browse-mode reload shortcut.
- Update `issues/0756-surfari/real-app-matrix.md` only if the experiment
  directly proves the `Crash handling` row.

## Verification

Pass criteria:

- Build or confirm required artifacts:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
cargo build -p webtui
cd ghostboard && zig build
cd ghostboard && macos/build.nu --configuration Debug --action build
```

- Run the new Surfari crash-handling harness.
- The harness must prove, in the real app:
  - the initial page loads and WebTUI observes its URL, title, and console
    marker;
  - the crash trigger is explicitly environment-gated and run only in the
    harness;
  - Surfari trace logs a `renderer-crashed` event for the active tab/pane;
  - the Surfari trace and WebTUI state trace both report exact deterministic
    helper values: `status=requested`, `code=0`, `url=<initial page URL>`, and
    `can_reload=true`;
  - WebTUI state trace logs `event=renderer_crashed` for the same tab;
  - WebTUI render state shows `renderer_crash_active=true`,
    `renderer_crash_tab_id=<tab_id>`, `renderer_crash_status=requested`,
    `loading_bar_active=false`, and `can_reload=true`;
  - no stale post-crash loading or render event clears the crash state before
    explicit recovery;
  - explicit recovery navigation reaches the recovery page in the same Surfari
    tab/pane;
  - WebTUI observes recovery URL/title/console marker;
  - WebTUI render state clears `renderer_crash_active` after recovery;
  - Surfari remains connected long enough to deliver the recovery title or
    console marker.
- The harness must fail if:
  - the crash callback is missing;
  - `RendererCrashed.tab_id` does not match the active Surfari tab;
  - `RendererCrashed.termination_status` is not `requested`;
  - `RendererCrashed.termination_status_code` is not `0`;
  - `RendererCrashed.url` is not the initial page URL for the crashed tab;
  - crash state is cleared before recovery;
  - loading stays active after the crash;
  - `can_reload` is false for the recoverable test crash;
  - recovery requires launching a new Surfari profile/process instead of
    recovering the existing tab/process.
- The harness must fail if the app binary it launches is missing or older than
  the source/build inputs needed for this experiment; the preferred verification
  path is to rebuild the Debug app bundle immediately before running it.
- Run hygiene checks:

```bash
git diff --check
bash -n <new-surfari-crash-handling-harness>
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/30-surfari-crash-handling.md \
  issues/0756-surfari/real-app-matrix.md
```

Run formatting/checks for any source files touched:

```bash
cargo fmt -- <rust-files>
zig fmt <zig-files>
```

Result classification:

- `Pass` means Surfari real-app crash detection, WebTUI crash state, stale-event
  suppression, same-tab recovery, and crash-state clearing are all directly
  proven, allowing `Crash handling` to become `Proven`.
- `Partial` means Surfari emits a renderer crash event but WebTUI state,
  reloadability, or recovery remains unproven or broken.
- `Fail` means the harness cannot trigger a deterministic Surfari renderer crash
  or cannot produce stronger evidence than Experiment 14's C smoke test.

## Design Review

Adversarial design review initially returned `CHANGES REQUIRED` with two
Required findings:

- The verification required correct crash status, status code, URL, and
  reloadability, but did not specify exact expected values for the deterministic
  Surfari helper path.
- The build verification could run against a stale Debug `TermSurf.app` bundle
  even though the harness launches that app bundle.

The design was updated to require exact deterministic crash values:
`status=requested`, `code=0`, `url=<initial page URL>`, and `can_reload=true`.
It was also updated to rebuild the Debug app bundle with
`macos/build.nu --configuration Debug --action build` and to fail the harness if
the app binary is missing or stale.

Focused re-review returned `APPROVED` with no Required findings. The reviewer
confirmed both prior findings were resolved and no new Required finding was
introduced.

## Result

**Result:** Pass

Passing run:

```bash
git diff --check
bash -n scripts/test-issue-756-surfari-crash-handling.sh
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/30-surfari-crash-handling.md \
  issues/0756-surfari/real-app-matrix.md
cargo fmt -p surfari -p webtui -- --check
cargo build -p surfari
cargo build -p webtui
surfari/libtermsurf_webkit/build.sh
cd ghostboard && zig build
scripts/test-issue-756-surfari-crash-handling.sh
```

Run ID: `20260621-205236`.

Logs:

- `logs/issue-756-exp30-surfari-crash-handling/harness-20260621-205236.log`
- `logs/issue-756-exp30-surfari-crash-handling/app-20260621-205236.log`
- `logs/issue-756-exp30-surfari-crash-handling/surfari-trace-20260621-205236.log`
- `logs/issue-756-exp30-surfari-crash-handling/webtui-20260621-205236.log`

The harness launched the real Debug `TermSurf.app` with repo-built
`web --browser surfari` and repo-built `surfari`, explicitly pinning Ghostboard
with `TERMSURF_SURFARI_PATH=$ROOT/target/debug/surfari`. It used an
environment-gated `TERMSURF_SURFARI_TEST_RENDERER_CRASH_URL` hook so normal
navigation cannot trigger the WebKit crash helper.

The passing run proved:

- the initial page loaded and WebTUI observed the initial URL, title, and
  console marker;
- Surfari received a direct `Navigate` to
  `termsurf://issue756-exp30-renderer-crash` and invoked
  `ts_webkit_test_kill_web_content_process`;
- Surfari logged a `renderer-crashed` trace for the active tab and pane with
  `status=requested`, `code=0`, the pre-crash page URL, `visible=true`, and
  `can_reload=true`;
- WebTUI logged `event=renderer_crashed` for the same tab with
  `status=requested`, `code=0`, the pre-crash page URL, and `can_reload=true`;
- WebTUI render state showed `renderer_crash_active=true`,
  `renderer_crash_tab_id=1`, `renderer_crash_status=requested`, and
  `renderer_crash_can_reload=true`, with `loading_bar_active=false`;
- stale post-crash events did not clear the crash state or restart loading
  before explicit recovery;
- explicit direct `Navigate` to the recovery page reached the same Surfari tab
  and pane;
- WebTUI observed the recovery URL, title, and console marker;
- WebTUI render state cleared `renderer_crash_active` after recovery;
- Surfari stayed alive through recovery long enough to emit the recovery title;
- no new default Surfari browser process spawned after the initial
  `BrowserReady` baseline.

Implementation changes:

- Added `scripts/test-issue-756-surfari-crash-handling.sh`.
- Added the Surfari FFI binding for the existing C helper
  `ts_webkit_test_kill_web_content_process`.
- Added a Surfari-only environment-gated crash trigger for exact URL matches on
  `TERMSURF_SURFARI_TEST_RENDERER_CRASH_URL`.
- Fixed the Surfari renderer-crash protocol boundary so the WebKit callback's
  `visible` boolean is traced as visibility while the TermSurf
  `RendererCrashed.can_reload` field is sent as `true` for this recoverable
  WebKit crash path.
- Added `renderer_crash_can_reload` to WebTUI's existing `render_state` trace so
  the crash UI state trace records reloadability directly.

The first harness run, `20260621-204616`, failed only at the final no-respawn
assertion. The log evidence showed one Surfari PID, one `BrowserReady`, same-tab
recovery, and no actual respawn. The failure came from the harness using a
Surfari-trace line baseline as an offset into the app log, which included the
original spawn. The assertion was fixed to use the app log line after the
initial `BrowserReady` as the no-respawn baseline, and the corrected harness
passed in run `20260621-205236`.

## Conclusion

Surfari now has real-app evidence for deterministic recoverable WebKit
web-content process termination. The real-app matrix marks `Crash handling`
`Proven`. The only remaining Issue 756 matrix gap is the final
Ghostboard/Roamium comparison against Surfari.

## Completion Review

Adversarial completion review returned `APPROVED` with no findings. The reviewer
inspected the current uncommitted result diff, including `webtui/src/main.rs`,
the Surfari dispatch/FFI changes, the new crash-handling harness, the issue
README, the real-app matrix, and the passing `20260621-205236` run logs.

The reviewer independently ran the documented non-mutating checks and reran the
crash harness successfully as run `20260621-205258`. The reviewer confirmed the
environment-gated crash trigger, exact deterministic crash values,
`renderer_crash_can_reload=true` in WebTUI render-state traces, recovery
clearing the crash state, no post-`BrowserReady` Surfari respawn, updated matrix
status, and that the result commit had not yet been made.
