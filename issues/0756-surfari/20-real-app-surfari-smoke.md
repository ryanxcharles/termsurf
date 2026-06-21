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
