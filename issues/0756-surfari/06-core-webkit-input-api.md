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
