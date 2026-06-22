# Experiment 6: Implement core WebKit input API

## Description

Experiment 5 created a buildable `libtermsurf_webkit` scaffold and proved the
first browser-view lifecycle through the C ABI. The next unchecked Issue 756
requirement is the core browser API that Surfari will need before a Rust process
can drive WebKit usefully.

This experiment should implement the missing core `libtermsurf_webkit` behavior
that maps directly to existing Roamium FFI calls and TermSurf protocol messages:
focus, GUI active state, color scheme, mouse click/move, wheel scroll, keyboard
input, browser-state reporting, and explicit lifecycle behavior around resize,
destroy, and quit.

This experiment should not create the Surfari Rust binary, modify Ghostboard,
modify `termsurf.proto`, patch WebKit source, implement DevTools, or claim
support for JavaScript dialogs, HTTP auth, renderer crash recovery, downloads,
history, or bookmarks. Anything still unsupported must remain exported as an
explicitly documented unsupported stub.

## Changes

- Extend `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm` so the following
  exported functions have real behavior:
  - `ts_forward_mouse_event`;
  - `ts_forward_mouse_move`;
  - `ts_forward_scroll_event`;
  - `ts_forward_key_event`;
  - `ts_set_focus`;
  - `ts_set_gui_active`;
  - `ts_set_color_scheme`.
- Keep the ABI names and signatures compatible with `roamium/src/ffi.rs`.
- Forward mouse, scroll, and keyboard input through Cocoa/WebKit event paths
  rather than ad hoc JavaScript calls. If a specific WebKit event path is not
  viable from the public `WKWebView` boundary, record the exact limitation and
  mark the experiment **Partial** rather than faking success.
- Implement dark/light color-scheme behavior using WebKit/Cocoa APIs where
  available. If WebKit exposes no reliable public setting for this in the
  current source build, record the limitation and leave the function documented
  as unsupported.
- Implement focus and GUI-active behavior by making the `WKWebView`/window the
  active recipient when focused/active, and by resigning or suppressing activity
  when inactive where Cocoa allows it.
- Add DOM-observable deterministic smoke-test content under
  `surfari/libtermsurf_webkit/test-content/` that records:
  - mouse down/up/click coordinates;
  - mouse move coordinates;
  - wheel scroll deltas or resulting scroll position;
  - keydown/keyup text or key code;
  - focus/blur events;
  - color-scheme state if implemented.
- Extend `surfari/libtermsurf_webkit/smoke-test/smoke_test.c` so it drives the
  new functions after the existing lifecycle proof and verifies their effects
  through browser-visible state. Prefer reading deterministic DOM state through
  a small test-only callback or existing state callback rather than using timing
  alone.
- If test-only hooks are needed, prefer a private smoke-test header or
  implementation-local hook. Do not add public test symbols unless there is no
  cleaner option, and do not pollute the Roamium-compatible ABI with
  non-protocol behavior.
- Update `surfari/libtermsurf_webkit/README.md` with the now-supported and
  still-unsupported API surface.
- Do not modify `webkit/src`. If WebKit source changes are needed for real input
  forwarding, record **Partial** and design the next experiment around a WebKit
  patch on the Issue 756 branch.

## Verification

Start from a clean TermSurf repo root:

```bash
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

Build and run the library smoke test:

```bash
surfari/libtermsurf_webkit/build.sh

mkdir -p logs
DYLD_FRAMEWORK_PATH="$PWD/webkit/src/WebKitBuild/Debug" \
surfari/libtermsurf_webkit/build/smoke-test \
  "$PWD/surfari/libtermsurf_webkit/test-content/index.html" \
  "$PWD/surfari/libtermsurf_webkit/test-content/navigation.html" \
  > logs/issue756-exp6-smoke.log 2>&1
rc=$?
echo "SMOKE_EXIT_STATUS=$rc" >> logs/issue756-exp6-smoke.log
```

The smoke log must prove all Experiment 5 lifecycle guarantees still pass, plus
the new core API behavior:

- `ts_set_focus(true)` causes browser-visible focus state.
- `ts_set_focus(false)` or inactive GUI state causes browser-visible blur or a
  documented Cocoa/WebKit inactive state.
- `ts_forward_mouse_move` produces browser-visible mousemove coordinates.
- `ts_forward_mouse_event` produces browser-visible mouse down/up/click
  coordinates and button state.
- `ts_forward_scroll_event` produces browser-visible scrolling or wheel event
  state.
- `ts_forward_key_event` produces browser-visible keydown/keyup/text state.
- `ts_set_color_scheme` produces browser-visible dark/light state, or the result
  explicitly records why this cannot be implemented through the current WebKit
  boundary.
- Existing lifecycle callbacks still pass: initialized, tab ready, CA context
  ID, loading state, URL change, title change, navigation, resize, destroy, and
  quit.
- Unsupported stubs remain documented and are not falsely reported as working.

Verify symbols and linkage:

```bash
nm -gU surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg ' _ts_' | sort
otool -L surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg 'WebKit|JavaScriptCore|libtermsurf'
otool -L surfari/libtermsurf_webkit/build/smoke-test | rg 'WebKit|JavaScriptCore|libtermsurf'
git diff --check
```

**Pass** = the library builds, the smoke test proves real browser-visible
behavior for focus, GUI active/inactive behavior, mouse move, mouse click,
scroll, keyboard input, and color scheme where supported, all Experiment 5
lifecycle behavior still passes, unsupported stubs are still explicit, and
`webkit/src` remains unchanged.

**Partial** = some core API behavior works but one or more required behaviors
cannot be implemented through the current public `WKWebView`/Cocoa boundary or
needs a WebKit source patch. The result must identify each missing behavior and
the exact next experiment needed.

**Fail** = the library no longer builds or the Experiment 5 lifecycle smoke test
regresses.

Before recording the result, capture:

```bash
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

The TermSurf worktree must contain only the intended library, smoke-test, docs,
and issue changes plus ignored `logs/` and build output.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

**Verdict:** Approved.

Required findings: none.

Optional/nit findings accepted and fixed:

- Tightened the test-only hook guidance to prefer private smoke-test headers or
  implementation-local hooks.
- Added `mkdir -p logs` to the smoke-test verification command.

## Result

**Result:** Partial

Experiment 6 implemented most of the core `libtermsurf_webkit` input API through
real Cocoa/WebKit event paths and extended the smoke test to prove the behavior
from DOM-visible state. The library still does not fully satisfy the focus
requirement: `ts_set_focus(true)` makes the `WKWebView` the AppKit first
responder and subsequent keyboard input reaches the page, but the page did not
observe a DOM `focus` event or `document.hasFocus() == true` during the smoke
test.

Implemented changes:

- `ts_forward_mouse_event` now creates Cocoa mouse down/up events and delivers
  them to the hit-tested WebKit view.
- `ts_forward_mouse_move` now sends mouse-enter, mouse-move, and dragged
  movement events through the WebKit view path. The dragged event was needed for
  WebKit to emit browser-visible DOM `mousemove` state from the synthetic event
  path.
- `ts_forward_scroll_event` now follows WebKit's own macOS test pattern: create
  a `CGEventCreateScrollWheelEvent2` event, set screen-flipped global
  coordinates, wrap it as an `NSEvent`, and deliver it to the hit-tested WebKit
  view.
- `ts_forward_key_event` now creates Cocoa key down/up events and delivers them
  to the `WKWebView`.
- `ts_set_focus` and `ts_set_gui_active` now activate/order the window and set
  or clear the first responder.
- `ts_set_color_scheme` now assigns light/dark `NSAppearance` to the
  `WKWebView`.
- Added private smoke-test-only hooks in
  `surfari/libtermsurf_webkit/smoke-test/test_support.h`:
  - `ts_webkit_test_evaluate_javascript`;
  - `ts_webkit_test_post_delayed_task`.
- Extended the deterministic navigation test page to record DOM-visible pointer,
  wheel, keyboard, blur, and color-scheme state.
- Updated `surfari/libtermsurf_webkit/README.md` with the supported and
  unsupported API surface.

Build command:

```bash
surfari/libtermsurf_webkit/build.sh
```

Output:

```text
ld: warning: building for macOS-26.0, but linking with dylib '/System/Library/Frameworks/WebKit.framework/Versions/A/WebKit' which was built for newer version 26.5
built surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib
built surfari/libtermsurf_webkit/build/smoke-test
```

As in Experiment 5, the linker warning is emitted before `install_name_tool`
rewrites the produced dylib dependency to `@rpath/WebKit.framework/...`.

Smoke-test command:

```bash
mkdir -p logs
DYLD_FRAMEWORK_PATH="$PWD/webkit/src/WebKitBuild/Debug" \
surfari/libtermsurf_webkit/build/smoke-test \
  "$PWD/surfari/libtermsurf_webkit/test-content/index.html" \
  "$PWD/surfari/libtermsurf_webkit/test-content/navigation.html" \
  > logs/issue756-exp6-smoke.log 2>&1
rc=$?
echo "SMOKE_EXIT_STATUS=$rc" >> logs/issue756-exp6-smoke.log
```

Smoke-test evidence from `logs/issue756-exp6-smoke.log`:

```text
CALLBACK initialized
CALLBACK tab_ready tab_id=1
CALLBACK ca_context_id context_id=2431042180 width=320 height=240
CALLBACK loading_state loading=1 url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/index.html
CALLBACK url_changed url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/index.html
CALLBACK url_changed url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/index.html
CALLBACK title_changed title=Surfari ABI First Page
CALLBACK loading_state loading=0 url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/index.html
CALLBACK ca_context_id context_id=2431042180 width=320 height=240
CALLBACK loading_state loading=1 url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/navigation.html
CALLBACK url_changed url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/navigation.html
CALLBACK url_changed url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/navigation.html
CALLBACK title_changed title=Surfari ABI Navigation Page
CALLBACK loading_state loading=0 url=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/navigation.html
CALLBACK ca_context_id context_id=2431042180 width=320 height=240
CALLBACK ca_context_id context_id=2431042180 width=640 height=480
CALLBACK input_state {"focus":false,"blur":true,"move":"120,130","click":"140,150,0","scroll":-120,"key":"a","colorScheme":"dark"}
SMOKE_PASS initialized=1 tab_ready=1 ca_context=4 url=4 loading_started=2 loading_finished=2 title=2 navigations=2 resized=1 input=1
SMOKE_EXIT_STATUS=0
```

What passed:

- Experiment 5 lifecycle behavior still passes.
- Tab ready, CA context, loading, URL, title, navigation, resize, destroy, and
  quit still work.
- Mouse click is DOM-visible: `"click":"140,150,0"`.
- Mouse movement is DOM-visible: `"move":"120,130"`.
- Wheel input is DOM-visible: `"scroll":-120`.
- Keyboard input is DOM-visible: `"key":"a"`.
- Inactive/blur state is DOM-visible: `"blur":true`.
- Dark color scheme is DOM-visible: `"colorScheme":"dark"`.

What did not fully pass:

- Focus activation is not DOM-visible. The smoke-test state remained
  `"focus":false` after `ts_set_focus(true)`. AppKit first-responder assignment
  and keyboard delivery work, but WebKit did not emit page-level focus state
  through this public `WKWebView` event path.

Symbol check:

```bash
nm -gU surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg ' _ts_|_ts_webkit_test' | sort
```

The dylib still exports all Roamium-shaped `ts_*` symbols plus the two private
smoke-test-only `ts_webkit_test_*` hooks.

Linkage check:

```bash
otool -L surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg 'WebKit|JavaScriptCore|libtermsurf'
otool -L surfari/libtermsurf_webkit/build/smoke-test | rg 'WebKit|JavaScriptCore|libtermsurf'
```

Output:

```text
surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib:
  @rpath/libtermsurf_webkit.dylib (compatibility version 0.0.0, current version 0.0.0)
  @rpath/WebKit.framework/Versions/A/WebKit (compatibility version 1.0.0, current version 625.1.21)
surfari/libtermsurf_webkit/build/smoke-test:
  @rpath/libtermsurf_webkit.dylib (compatibility version 0.0.0, current version 0.0.0)
```

Final checks:

```text
$ git diff --check
<no output>

$ git -C webkit/src status --short
<clean>

$ git -C webkit/src rev-parse HEAD
1452a43959523449099b2616793fd2c5b6a6487e

$ git -C webkit/src rev-parse --abbrev-ref HEAD
webkit-1452a439-issue-756

$ git -C webkit/src rev-parse --is-shallow-repository
true
```

## Conclusion

The core `libtermsurf_webkit` input API is now mostly real and smoke-tested
through browser-visible state, but the focus requirement is not fully satisfied.
The next experiment should investigate WebKit focus semantics from the
source-built API boundary and determine whether Surfari needs a WebKit source
patch, a private WebKit API call, or a different activation sequence to make
`ts_set_focus(true)` produce reliable page-visible focus state.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

**Initial verdict:** Changes required.

Required finding:

- `surfari/libtermsurf_webkit/README.md` overclaimed focus support by listing
  "focus and GUI active/inactive state" as implemented even though the
  experiment result is **Partial** because DOM-visible focus remains unresolved.

Fix:

- Updated the README to describe the implemented behavior as AppKit
  first-responder assignment and GUI active/inactive state.
- Added DOM-visible focus state from `ts_set_focus(true)` to the unsupported
  list.

**Re-review verdict:** Approved.

The reviewer confirmed the README now qualifies the implemented focus behavior
as AppKit first-responder and GUI active/inactive state, and lists DOM-visible
focus from `ts_set_focus(true)` as unsupported. No new required findings were
introduced.
