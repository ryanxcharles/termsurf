# Experiment 16: Prove Surfari fake-GUI IPC

## Description

Experiment 15 created a buildable Surfari Rust binary linked to
`libtermsurf_webkit`, but it did not prove the binary can run as a real TermSurf
browser process. The next requirement in Issue 756 is to run Surfari outside
Ghostboard with a small test driver or harness and prove the Rust process can
drive WebKit through `libtermsurf_webkit`.

This experiment should add a narrow fake-GUI IPC harness for Surfari. It should
follow the proven shape of `scripts/test-issue-792-fake-gui.py`, but target
`target/debug/surfari` and WebKit instead of `chromium/src/out/Default/roamium`.
The harness should bind a TermSurf Unix socket, launch Surfari with
`--ipc-socket` and `--user-data-dir`, accept `ServerRegister`, send `CreateTab`,
record browser messages, send `Resize` after `TabReady`, and close the tab
cleanly. It should fail if the expected messages do not arrive.

This experiment should not integrate Surfari into Ghostboard, change webtui
browser selection, update install/release scripts, implement DevTools, or modify
`termsurf.proto`. Its only production-code changes should be whatever is needed
to make Surfari's current Rust/C ABI process work in this fake-GUI protocol
proof.

## Changes

- Add a Surfari-specific fake-GUI harness, for example
  `scripts/test-issue-756-surfari-fake-gui.py`.
- Reuse the minimal protobuf encoder/decoder style from
  `scripts/test-issue-792-fake-gui.py` unless a generated protobuf dependency is
  already available without adding setup friction.
- Launch `target/debug/surfari` from the repo root with:
  - `--ipc-socket=<fake GUI socket>`;
  - `--user-data-dir=<log-dir>/profile`.
- Set the environment needed for local WebKit development:
  - `DYLD_FRAMEWORK_PATH=$PWD/webkit/src/WebKitBuild/Debug`;
  - any trace variable needed to prove Surfari received protocol messages, such
    as `TERMSURF_PDF_INPUT_TRACE=1` and
    `TERMSURF_PDF_INPUT_TRACE_FILE=<log-dir>/surfari-trace.log`.
- Use deterministic local file URLs from
  `surfari/libtermsurf_webkit/test-content/`, preferably `navigation.html`, so
  the test does not depend on the network.
- The fake GUI should prove at least:
  - Surfari connects to the socket;
  - `ServerRegister` is received with the expected profile name;
  - `CreateTab` is sent;
  - `TabReady` is received with a positive tab id;
  - `CaContext` is received with a nonzero context id and positive dimensions;
  - `UrlChanged`, `LoadingState`, and `TitleChanged` are received for the
    deterministic test page;
  - `Resize` is sent and Surfari's trace records the resize dispatch to
    `ts_set_view_size`;
  - `CloseTab` is sent and Surfari exits cleanly, or the harness terminates it
    only after recording why clean shutdown was not observable.
- If the harness reveals a missing Surfari runtime behavior, fix the narrow root
  cause in the Surfari Rust/C ABI layer and keep the fix inside this experiment.
- Do not modify `roamium/`, `webtui/`, `ghostboard/`, `termsurf.proto`, or
  WebKit source.

## Verification

Start from a clean repo root:

```bash
git status --short
git -C webkit/src status --short
```

Build the required pieces:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
```

Run the fake-GUI IPC proof:

```bash
mkdir -p logs/issue756-exp16-surfari-fake-gui
scripts/test-issue-756-surfari-fake-gui.py \
  --log-dir logs/issue756-exp16-surfari-fake-gui \
  "file://$PWD/surfari/libtermsurf_webkit/test-content/navigation.html"
```

The harness log must prove:

- `ServerRegister` arrived from Surfari.
- `CreateTab`, `Resize`, and `CloseTab` were sent by the fake GUI.
- `TabReady`, `CaContext`, `UrlChanged`, `LoadingState`, and `TitleChanged`
  arrived from Surfari.
- The Surfari trace contains `surfari create-tab`, `surfari resize`, and
  `surfari close-tab` lines.
- The Surfari process exits cleanly after `CloseTab`, or the result records the
  exact reason clean shutdown is not yet observable.

Run focused checks:

```bash
cargo fmt -p surfari -- --check
git diff --check
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/16-surfari-fake-gui-ipc.md
python3 -m py_compile scripts/test-issue-756-surfari-fake-gui.py
rg -n 'Roamium|roamium|chromium/src/out/Default' scripts/test-issue-756-surfari-fake-gui.py
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

The final `rg` should return no matches.

**Pass** = the fake-GUI harness exits 0, proves Surfari's socket registration,
tab creation, CA context, page state callbacks, resize dispatch, and close-path
behavior, all checks pass, and `webkit/src` remains unchanged.

**Partial** = Surfari registers and creates a WebKit tab, but one expected
callback or clean shutdown behavior is missing. Record the precise missing piece
and whether the next experiment should fix it before Ghostboard integration.

**Fail** = Surfari cannot connect to the fake GUI, cannot create a tab through
the Rust/protobuf path, crashes before producing useful evidence, or requires
Ghostboard/protocol/WebKit-source changes to proceed.

## Design Review

Adversarial design review approved the experiment with no findings. The reviewer
confirmed the README links Experiment 16 as `Designed`, the required sections
are present, the scope follows Experiment 15, the fake-GUI proof is mechanically
plausible against the current Surfari IPC/dispatch/protobuf paths, verification
has concrete pass/partial/fail criteria, hygiene checks are listed, and the plan
explicitly excludes Ghostboard, webtui, protocol, and WebKit source changes.
