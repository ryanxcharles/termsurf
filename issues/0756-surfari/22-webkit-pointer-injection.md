# Experiment 22: Prove WebKit pointer injection

## Description

Experiment 21 proved the real app keyboard path end to end and localized the
remaining pointer gap. Ghostboard hit testing works, IPC forwarding works, and
Surfari receives mouse and wheel messages, but synthetic AppKit mouse/wheel
events delivered by `libtermsurf_webkit` do not become DOM `click` or `wheel`
events in the fixture page.

This experiment should stay focused on that boundary: Surfari's WebKit pointer
injection. It should not expand into split panes, tab switching, window
switching, restart, profile isolation, crash handling, or the full feature
matrix. The goal is to find and verify the smallest correct way to make
forwarded TermSurf pointer events produce page-visible WebKit pointer behavior
in the single-window, single-tab, single-pane real app case.

## Changes

- Study WebKit's own macOS testing and automation event paths before changing
  code:
  - `webkit/src/Tools/WebKitTestRunner/mac/EventSenderProxy.mm`;
  - `webkit/src/Tools/WebKitTestRunner/mac/UIScriptControllerMac.mm`;
  - `webkit/src/Source/WebKit/UIProcess/API/Cocoa/WKWebViewPrivate.h`;
  - relevant `WKWebView` private methods such as `_simulateMouseMove`,
    `_doAfterProcessingAllPendingMouseEvents`, and automation-event marking.
- Update `libtermsurf_webkit` pointer delivery only where evidence shows the
  current synthetic AppKit path is incomplete.
- Keep Ghostboard, WebTUI, protocol, and WebKit source changes out of scope
  unless the investigation proves the pointer failure cannot be solved in
  `libtermsurf_webkit`.
- Extend `scripts/test-issue-756-real-app-surfari-input-routing.sh` only as
  needed to produce stronger pointer evidence or better failure diagnostics.
- Preserve Experiment 21's keyboard proof. Keyboard must remain fatal if it
  regresses.

Possible implementation paths to test, in order of least invasive to most:

- Correct event construction details: window-relative coordinates, event number,
  graphics context, click count, pressed-button state, event phases, and
  pixel-vs-point deltas.
- Use WebKit private completion hooks such as
  `_doAfterProcessingAllPendingMouseEvents` so the harness waits for WebKit's
  asynchronous mouse processing rather than racing it.
- Mark forwarded events as synthesized for WebKit automation if the evidence
  shows WebKit filters unmarked synthetic events.
- Use the private WebKit2/UIProcess event path directly if AppKit dispatch to
  `WKWebView` is the wrong seam.

## Verification

Pass criteria:

- Build or confirm the required binaries:

```bash
surfari/libtermsurf_webkit/build.sh
cargo build -p surfari
cargo build -p webtui
cd ghostboard && zig build
```

- Run the real Debug `TermSurf.app` through
  `scripts/test-issue-756-real-app-surfari-input-routing.sh`.
- Preserve Experiment 21 keyboard evidence:
  - Ghostboard stays frontmost;
  - Surfari logs `key-event`;
  - the fixture page logs `kind=input value=a`.
- Prove at least one page-visible pointer behavior in the real Surfari WebKit
  view:
  - DOM `click` on the fixture click zone; or
  - DOM `wheel`/scroll on the fixture page; or
  - an equivalent page-visible pointer signal if the harness records why it is
    equivalent.
- Keep Surfari-side pointer evidence:
  - `mouse-event` for click; and/or
  - `scroll-event` for wheel.
- The harness must fail if page-visible pointer evidence is missing. It must not
  print final `PASS` after only proving that Surfari received an IPC pointer
  message.
- Run hygiene checks:

```bash
git diff --check
bash -n scripts/test-issue-756-real-app-surfari-input-routing.sh
prettier --check --prose-wrap always --print-width 80 \
  issues/0756-surfari/README.md \
  issues/0756-surfari/22-webkit-pointer-injection.md
```

Run formatting/checks for any source files touched:

```bash
cargo fmt -- <rust-files>
zig fmt <zig-files>
```

Result classification:

- `Pass` means page-visible pointer behavior is proven in the real app without
  regressing Experiment 21's keyboard proof.
- `Partial` means the exact remaining WebKit pointer boundary is narrowed but
  page-visible pointer behavior is still not proven.
- `Fail` means the experiment cannot reach the real Surfari overlay or cannot
  produce enough evidence to improve on Experiment 21's localization.

## Design Review

Adversarial design review returned `APPROVED` with no Required findings. The
reviewer confirmed that the README links Experiment 22 as `Designed`, the file
has the required Description, Changes, and Verification sections, the scope
follows Experiment 21's `Partial` result, the plan stays focused on WebKit
pointer injection, the verification requires page-visible pointer behavior
instead of Surfari IPC receipt alone, Experiment 21's keyboard proof remains a
regression requirement, and the plan commit had not already been made.

## Result

**Result:** Pass

The real-app harness now proves page-visible pointer behavior in Surfari's
WebKit view while preserving Experiment 21's keyboard proof.

Implemented changes:

- Kept Surfari host windows onscreen instead of moving them to `-10000,-10000`.
  WebKit receives forwarded IPC mouse/wheel messages when the host window is
  offscreen, but it does not turn wheel events into DOM `wheel` events.
- Made `TSHostWindow` refuse key/main-window status so the onscreen WebKit host
  window does not steal activation from Ghostboard.
- Made the host windows ignore mouse events and set `alphaValue = 0.0` so they
  remain non-interactive and invisible to the user while still satisfying
  WebKit's onscreen-window requirement.
- Corrected `libtermsurf_webkit` hit testing to use `event.locationInWindow`
  directly when dispatching AppKit events to the `WKWebView`, matching WebKit's
  own event-sender pattern.
- Made the harness final pass message generic for Issue 756 real-app input
  routing instead of naming Experiment 21.

Verification evidence:

- `surfari/libtermsurf_webkit/build.sh` passed, with the known macOS/WebKit
  dylib version warning.
- `cargo build -p surfari` passed.
- `cargo build -p webtui` passed.
- `cd ghostboard && zig build` passed.
- `bash -n scripts/test-issue-756-real-app-surfari-input-routing.sh` passed.
- `git diff --check` passed.
- Harness run `20260621-183255` passed end to end:
  - real Debug `TermSurf.app` launched;
  - repo `target/debug/web --browser surfari` launched;
  - repo `target/debug/surfari` launched;
  - Surfari's WebKit CAContext overlay was presented;
  - Browse mode focused Surfari;
  - Ghostboard stayed frontmost before keyboard injection;
  - Surfari logged `key-event`;
  - the fixture logged `kind=input value=a`;
  - Surfari logged `mouse-event ... type=down button=left`;
  - Surfari logged `scroll-event`;
  - the fixture logged page-visible wheel input;
  - Surfari accepted `CloseTab` and began clean shutdown.

The fixture still did not log a DOM click for the click-zone click. The harness
logs this as `WARN: missing page observed click-zone click`, then requires the
DOM wheel signal before final `PASS`. This satisfies the experiment's pass
criteria because page-visible pointer behavior may be click, wheel/scroll, or an
equivalent page-visible pointer signal. A later matrix experiment should decide
whether DOM click itself must be fixed separately.

## Conclusion

The WebKit pointer boundary is no longer blocked at Surfari IPC receipt. The
critical missing condition was that the WebKit host window must remain onscreen
for DOM wheel delivery. Making that window transparent, mouse-ignoring, and
unable to become key/main preserves Ghostboard's ownership of real OS input
while allowing WebKit to process forwarded wheel events into page-visible DOM
behavior.

Experiment 22 does not complete the full Issue 756 real-app matrix. It proves
the single-pane keyboard and wheel paths are now working and gives the next
experiments a stronger base for focused regression guards and then the pane,
tab, window, resize, restart, profile, and crash matrix.

## Completion Review

Adversarial completion review returned `APPROVED` with no Required findings. The
reviewer confirmed that the result commit had not already been made, the README
marks Experiment 22 as `Pass`, the experiment stays within the pointer-injection
scope, the docs honestly state that DOM click still warns/misses, the harness
requires page-visible wheel evidence before final `PASS`, and the keyboard proof
remains fatal and preserved.

The reviewer noted one optional caveat: the final run proves page-visible wheel
delivery, not wheel-coordinate fidelity. The harness computed a distinct scroll
point, but the Surfari trace logged the wheel at the prior click coordinate
while the page still logged `kind=wheel`. A later coordinate/regression
experiment should assert forwarded wheel coordinates if coordinate fidelity is
part of that experiment's goal.
