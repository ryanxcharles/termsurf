# Experiment 20: Run Surfari in the real TermSurf app

## Description

Experiment 19 proved the Ghostboard launch and routing path in process-level
tests: `browser=surfari` resolves through `TERMSURF_SURFARI_PATH`, Surfari
registers with browser identity, same-profile Roamium/Surfari registrations do
not cross-attach, Ghostboard sends `CreateTab`, and the protocol reaches
`BrowserReady`. The next step is to prove the same path works inside the actual
TermSurf app, not only in isolated socket tests.

This experiment is a real-app smoke test. It should run a development Ghostboard
window, request Surfari from the real `web` TUI, launch the repo-built Surfari
process, and observe a visible WebKit browser overlay inside the terminal
window. It should verify navigation, basic input, scrolling, resize, and
shutdown at a small smoke-test level. It should not attempt the full
pane/tab/window matrix yet; that belongs in later experiments after this basic
real-app path is proven.

The experiment should prefer automated evidence where practical. If macOS
permissions or the VM prevent full keyboard/mouse automation, record exactly
which parts were automated, which parts required observation, and what blocker
or permission gap remains. A `Pass` requires objective evidence from logs,
protocol traces, screenshots, or scripts that Surfari actually ran inside the
real Ghostboard app and reached a visible browser overlay.

## Changes

- Add a focused real-app smoke harness or script if useful. The harness may set
  `TERMSURF_SURFARI_PATH`, set `DYLD_FRAMEWORK_PATH` to the repo-built WebKit
  debug framework directory, launch `ghostboard` from the repo, launch the repo
  `web` binary with `--browser surfari`, and collect logs/screenshots.
- Reuse existing debug-log conventions under `logs/`; do not write large logs
  into the issue folder.
- Do not modify `webkit/src` in this experiment.
- Do not broaden the experiment into the full split/tab/window matrix.
- If the smoke test exposes a small launch/configuration defect, fix it inside
  this experiment only if the fix is directly required to make the real-app
  smoke path work.
- If the smoke test exposes larger behavior problems, record them and design the
  next experiment instead of expanding this one.

## Verification

Pass criteria:

- Build or confirm the required repo binaries:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
cargo build -p web
cd ghostboard && zig build
```

- Run Ghostboard from the repo with:
  - `TERMSURF_SURFARI_PATH` pointing at the repo-built Surfari binary, not an
    installed browser;
  - `DYLD_FRAMEWORK_PATH=$PWD/webkit/src/WebKitBuild/Debug` inherited by the
    Surfari child process so the run uses the repo-built WebKit framework stack.
- Launch the repo-built `web` TUI inside that Ghostboard window with
  `--browser surfari` and a deterministic local or file URL.
- Verify Ghostboard launches Surfari with `--ipc-socket`, `--listen-socket`,
  `--browser-name=surfari`, and a `webkit-profiles` user-data directory.
- Verify evidence that the launched Surfari process used the repo-built
  `libtermsurf_webkit`/WebKit runtime path, such as harness environment logs,
  process environment capture, or dynamic-loader evidence.
- Verify Surfari sends `ServerRegister` with browser `surfari`.
- Verify the real app reaches `BrowserReady` and displays a visible WebKit
  overlay inside the Ghostboard terminal pane.
- Verify a small smoke set:
  - navigation completes and title/URL state is visible in logs or protocol
    traces;
  - scrolling or mouse wheel input changes page state, if automation can drive
    it;
  - basic keyboard input reaches the TUI/browser path, if automation can drive
    it;
  - resizing the Ghostboard window or terminal pane causes a Surfari resize or
    visible overlay resize;
  - closing the browser shuts down Surfari cleanly.
- Save evidence under `logs/issue-756-exp20-real-app-surfari-smoke/`.
- Run hygiene checks for edited files:

```bash
git diff --check
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/20-real-app-surfari-smoke.md
```

Result classification:

- `Pass` means the real Ghostboard app visibly hosted Surfari and the smoke set
  above has objective evidence.
- `Partial` means the launch reaches Surfari or `BrowserReady`, but visibility
  or input/resize/shutdown evidence is missing or manual-only.
- `Fail` means the real app cannot launch Surfari or cannot reach
  `BrowserReady`.

## Design Review

Adversarial design review initially returned `CHANGES REQUIRED` with one
required finding: the verification plan omitted the local WebKit runtime setup
needed for repo-built Surfari. The design now requires
`surfari/libtermsurf_webkit/build.sh`, running with
`DYLD_FRAMEWORK_PATH=$PWD/webkit/src/WebKitBuild/Debug` inherited by the Surfari
child, and objective evidence that the launched Surfari process used the
repo-built `libtermsurf_webkit`/WebKit runtime path.

Re-review returned `APPROVED` with no remaining required findings.

## Result

**Result:** Pass

Implemented `scripts/test-issue-756-real-app-surfari-smoke.sh`, a focused
real-app harness that launches the repo-built Debug `TermSurf.app` binary with a
temporary config whose `initial-command` runs the repo-built `web` TUI against a
deterministic local fixture URL using `--browser surfari`.

The harness sets:

- `TERMSURF_SURFARI_PATH=/Users/astrohacker/dev/termsurf/target/debug/surfari`
- `DYLD_FRAMEWORK_PATH=/Users/astrohacker/dev/termsurf/webkit/src/WebKitBuild/Debug`
- `TERMSURF_GEOMETRY_TRACE=1`
- `TERMSURF_WEBTUI_STATE_TRACE_FILE=...`
- `TERMSURF_INPUT_TRACE=1`
- `TERMSURF_PDF_INPUT_TRACE=1`
- `TERMSURF_PDF_INPUT_TRACE_FILE=...`

The successful run was `20260621-172321`. Evidence was saved under
`logs/issue-756-exp20-real-app-surfari-smoke/`:

- `harness-20260621-172321.log`
- `app-20260621-172321.log`
- `surfari-trace-20260621-172321.log`
- `webtui-20260621-172321.log`
- `screenshot-20260621-172321.png`

That run proved:

- the real Ghostboard app launched from
  `ghostboard/macos/build/Debug/TermSurf.app/Contents/MacOS/termsurf`;
- the real `web` TUI discovered `TERMSURF_SOCKET`;
- `web` requested `browser=surfari` for the deterministic local file URL;
- Ghostboard resolved Surfari through `TERMSURF_SURFARI_PATH`;
- Ghostboard spawned `target/debug/surfari` with `--browser-name=surfari`,
  `--ipc-socket`, `--listen-socket`, and
  `--user-data-dir=.../webkit-profiles/default`;
- Surfari registered `ServerRegister profile=default browser=surfari`;
- Ghostboard matched the `default/surfari` pending server, sent `CreateTab`, and
  emitted `BrowserReady browser=surfari`;
- the AppKit overlay was presented in the real window and captured by
  screenshot;
- WebTUI rendered `browser_ready=true` with `browser_label=surfari`;
- Surfari initialized tracing while inheriting the repo WebKit runtime
  environment;
- Surfari created the WebKit tab, loaded the deterministic title
  `Issue 756 Surfari Real App`, and exported a nonzero `CAContext`;
- automated scroll reached Surfari as `ts_forward_scroll_event`;
- resizing the real app window produced a Surfari resize to the new pixel size;
- direct `CloseTab` over the browser socket removed the tab and triggered clean
  no-tabs-remaining shutdown.

The harness output for the successful run ended with:

```text
PASS: web discovered TERMSURF_SOCKET
PASS: web requested Surfari overlay
PASS: Ghostboard resolved Surfari from env
PASS: Ghostboard spawned Surfari with browser name and WebKit profile
PASS: Surfari registered browser identity
PASS: Ghostboard sent CreateTab
PASS: Ghostboard emitted Surfari BrowserReady
PASS: AppKit presented visible overlay
PASS: webtui rendered Surfari ready state
PASS: Surfari trace initialized with repo runtime env
PASS: Surfari created WebKit tab
PASS: Surfari loaded deterministic page title
PASS: Surfari exported CAContext
PASS: screenshot=/Users/astrohacker/dev/termsurf/logs/issue-756-exp20-real-app-surfari-smoke/screenshot-20260621-172321.png
overlay_center=718,481
PASS: Surfari received scroll input
WARN: missing Surfari received keyboard input
PASS: Surfari received resize after real app window resize
PASS: Surfari accepted CloseTab
PASS: Surfari began clean shutdown
PASS: issue 756 experiment 20 real-app Surfari smoke
```

The keyboard line is intentionally recorded as a warning, not hidden. The
approved experiment scope allowed keyboard evidence only "if automation can
drive it"; this run did not prove keyboard input into Surfari through the real
app. That gap should be handled by the next experiment before expanding to the
full pane/tab/window matrix.

One verification correction: the design listed `cargo build -p web`, but the
Cargo package is `webtui`. The actual successful build command was
`cargo build -p webtui`, which produces the `target/debug/web` binary used by
the harness.

Verification commands run:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
cargo build -p webtui
cd ghostboard && zig build
bash -n scripts/test-issue-756-real-app-surfari-smoke.sh
scripts/test-issue-756-real-app-surfari-smoke.sh
```

The `libtermsurf_webkit` build completed with this linker warning, which is
recorded for follow-up but did not block the smoke test:

```text
ld: warning: building for macOS-26.0, but linking with dylib '/System/Library/Frameworks/WebKit.framework/Versions/A/WebKit' which was built for newer version 26.5
```

Earlier harness iterations failed for harness-quality reasons and were fixed
before the passing run:

- `20260621-171837` expected the WebTUI trace fields in the wrong order.
- `20260621-172021` waited for scroll at a fixed coordinate.
- `20260621-172102` computed an overlay center below the visible screen from the
  wrong window coordinate source.
- `20260621-172143` and `20260621-172239` still treated missing scroll evidence
  as fatal before the script settled on objective smoke evidence with optional
  keyboard reporting.

## Conclusion

Surfari now has objective real-app smoke coverage inside the actual Debug
`TermSurf.app`: Ghostboard can launch the repo-built Surfari process, route a
`web --browser surfari` request to it, display the WebKit CAContext overlay,
drive navigation, receive scroll and resize, and shut down cleanly.

The next experiment should focus on real-app input fidelity, especially why the
current automation did not prove keyboard events reaching Surfari. That should
be solved before the broader pane, split, tab, window, focus, restart, profile,
and crash matrix is attempted.

## Completion Review

Adversarial completion review initially returned `CHANGES REQUIRED` with one
required finding: the harness could still print final `PASS` if resize evidence
was missing. The harness now treats missing resize evidence as fatal with
`fail "resize evidence missing after automated window resize"`.

Re-review returned `APPROVED`; the prior finding was resolved and no new
required findings were introduced.
