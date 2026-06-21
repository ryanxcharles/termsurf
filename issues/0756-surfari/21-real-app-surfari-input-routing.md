# Experiment 21: Prove real-app Surfari input routing

## Description

Experiment 20 proved the real Ghostboard app can launch Surfari, present its
WebKit CAContext overlay, navigate to a deterministic fixture, receive scroll,
resize, and shut down cleanly. It did not prove keyboard input reaching Surfari:
the harness logged `WARN: missing Surfari received keyboard input`.

This experiment should focus on real-app input fidelity before expanding to the
full pane, split, tab, and window matrix. The goal is to prove, with objective
logs and page-visible state, that keyboard and pointer input can travel through
the real Ghostboard app to Surfari's WebKit view. If keyboard input fails, this
experiment should identify the exact failing boundary and fix it if the fix is
reasonably scoped.

The experiment should cover only the single-window, single-tab, single-pane
case. It should not broaden into split panes, tab switching, window switching,
restart, profile isolation, or crash handling. Those belong after the basic
input path is proven.

## Changes

- Add or extend a focused real-app input harness under `scripts/`.
- Use a deterministic local fixture page with visible state changes for:
  - keyboard input into a focused text field;
  - click or pointer focus;
  - scroll or wheel input;
  - optional drag only if the existing injection helpers make it practical
    without broadening the experiment.
- Collect objective evidence under
  `logs/issue-756-exp21-real-app-surfari-input-routing/`.
- Prefer evidence from multiple layers:
  - harness command output;
  - Ghostboard input/geometry trace logs;
  - Surfari trace logs such as `key-event`, `mouse-event`, or `scroll-event`;
  - fixture-page state visible through title changes, URL/hash changes, console
    logs, screenshot text, or another deterministic observable.
- If input fails, instrument only the narrow boundary required to locate it:
  macOS event injection, Ghostboard hit testing, Ghostboard protocol forwarding,
  Surfari Rust dispatch, or `libtermsurf_webkit` event injection.
- Fix a failing boundary inside this experiment only if the fix is directly
  required for keyboard or pointer input in the single-pane real-app case and is
  small enough to verify here.
- Do not modify `webkit/src` unless the evidence proves the failure is inside
  the WebKit patch layer and no smaller TermSurf-side fix can make progress.

## Verification

Pass criteria:

- Build or confirm the required repo binaries:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
cargo build -p webtui
cd ghostboard && zig build
```

- Run the real Debug `TermSurf.app` with:
  - `TERMSURF_SURFARI_PATH` pointing at `target/debug/surfari`;
  - `DYLD_FRAMEWORK_PATH=$PWD/webkit/src/WebKitBuild/Debug`;
  - input and geometry tracing enabled.
- Launch repo-built `target/debug/web --browser surfari` against the fixture
  page from inside the app.
- Prove the fixture receives keyboard input in the Surfari WebKit view. At least
  one objective browser-side signal must show the typed value changed, such as a
  title update, console trace, hash update, screenshot, or Surfari callback.
- Prove pointer input reaches the Surfari WebKit view. The proof may be click,
  mouse move, or wheel/scroll, but it must include Surfari-side or page-visible
  evidence rather than only successful event injection.
- Prove the harness fails if keyboard evidence is missing. A keyboard miss must
  not be logged as a warning followed by final `PASS`.
- If keyboard cannot be made to work in this experiment, record `Partial` or
  `Fail` with the exact failing boundary and the next proposed fix; do not mark
  `Pass`.
- Run hygiene checks for edited files:

```bash
git diff --check
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/21-real-app-surfari-input-routing.md
```

Run formatting/checks for any source files touched by a fix:

```bash
cargo fmt -- <rust-files>
zig fmt <zig-files>
```

Result classification:

- `Pass` means real-app keyboard input and pointer input both have objective
  Surfari/WebKit evidence in the single-pane case, and the harness fails when
  required keyboard evidence is missing.
- `Partial` means some input path works, or the exact failing boundary is
  identified, but keyboard input is not yet proven end to end.
- `Fail` means the experiment cannot reach the real Surfari overlay or cannot
  produce enough evidence to localize the input path.

## Design Review

Adversarial design review returned `APPROVED` with no findings. The reviewer
confirmed that the README links Experiment 21 as `Designed`, the experiment has
the required sections, the scope stays single-window/single-tab/single-pane,
keyboard evidence is required for `Pass`, pointer input requires Surfari-side or
page-visible evidence, and the plan commit had not already been made.

## Result

**Result:** Partial

The real-app harness now proves keyboard input reaches Surfari and the fixture
page, but it does not yet prove page-visible pointer input.

Implemented changes:

- Added `scripts/test-issue-756-real-app-surfari-input-routing.sh`.
- Kept Surfari's WebKit host windows offscreen and non-interactive so the engine
  process does not steal activation from the real Ghostboard app.
- Stopped `ts_set_focus(true)` and `ts_set_gui_active(true)` from activating or
  keying the Surfari process. Ghostboard remains the frontmost app and owns the
  OS keyboard/mouse events.
- Converted TermSurf top-left web coordinates to AppKit local coordinates before
  WebKit hit testing/event delivery.

Verification evidence:

- `surfari/libtermsurf_webkit/build.sh` passed.
- `cargo build -p surfari` passed.
- Harness run `20260621-181307` reached the real Debug `TermSurf.app`, launched
  repo `target/debug/web --browser surfari`, launched repo
  `target/debug/surfari`, presented the Surfari WebKit CAContext overlay,
  entered Browse mode, and focused Surfari.
- Keyboard evidence in run `20260621-181307`:
  - Ghostboard stayed frontmost before keyboard injection.
  - Ghostboard logged `key_down` and `key_down_forwarded` for key code `0`.
  - Surfari logged `key-event ... type=down windows_key_code=65 utf8_len=1`.
  - WebTUI logged fixture console output
    `ISSUE756_EXP21_INPUT ... kind=input value=a ... active=field`.
- Pointer forwarding evidence in run `20260621-181307`:
  - Ghostboard logged browser-overlay hit tests at `web_point={160, 170}`.
  - Surfari logged `mouse-event ... type=down button=left`.
  - Surfari logged `scroll-event ... ffi=ts_forward_scroll_event`.
- Pointer page evidence is still missing:
  - The fixture did not log `kind=click`.
  - The fixture did not log `kind=wheel`.
  - The harness correctly failed instead of marking the experiment `Pass`.

I also tested two narrower WebKit-side hypotheses and did not retain them:

- Swizzling `NSEvent.pressedMouseButtons` and `buttonNumber` around mouse
  dispatch, mirroring WebKitTestRunner's event sender, did not make DOM click or
  wheel events appear.
- Making the hidden Surfari `WKWebView` first responder without activating or
  keying the Surfari window did not make DOM click or wheel events appear.

## Conclusion

The OS/VM permission problem and Ghostboard activation problem are no longer the
keyboard blocker. Keyboard input works end to end in the real app when Surfari
does not activate its hidden WebKit host window.

The remaining boundary is pointer event injection inside `libtermsurf_webkit`:
Ghostboard hit testing and IPC forwarding are proven, and Surfari receives mouse
and wheel messages, but the synthetic AppKit events do not become DOM
click/wheel events in `WKWebView`. The next experiment should focus only on
WebKit pointer injection. It should compare Surfari against WebKitTestRunner and
WebKit private APIs such as `_simulateMouseMove`,
`_doAfterProcessingAllPendingMouseEvents`, automation-event marking, and direct
`WebPageProxy` event paths before expanding to panes, tabs, or windows.

## Completion Review

Adversarial completion review returned `APPROVED` with no Required findings. The
reviewer confirmed that:

- the result remains uncommitted before the result commit;
- the scope stays single-window, single-tab, single-pane;
- the `Partial` classification is correct;
- the README status matches the experiment result;
- the final harness run fails at missing page-visible wheel evidence and does
  not print final `PASS`;
- keyboard misses are fatal in the harness;
- the logs support the written evidence: keyboard reaches the page, Surfari
  receives mouse and wheel, and page click/wheel evidence is absent.

The reviewer also reran `git diff --check`,
`bash -n scripts/test-issue-756-real-app-surfari-input-routing.sh`,
`prettier --check`, `cargo build -p surfari`, `cargo build -p webtui`, and
`surfari/libtermsurf_webkit/build.sh`. The WebKit library build reproduced the
known macOS/WebKit dylib version warning and completed successfully.
