# Experiment 8: Implement WebKit browser state callbacks

## Description

Experiments 5-7 made `libtermsurf_webkit` buildable and proved lifecycle,
navigation, resize, compositing, focus, mouse, scroll, keyboard, blur/inactive,
and color-scheme behavior in the C smoke harness. The next gap is browser state
reporting. The public header already exposes callback setters for cursor
changes, target URL changes, JavaScript dialogs, console messages, HTTP auth
requests, and renderer crashes, but most of those callbacks are only stored and
never fired. The HTTP auth callback typedef also needs to be realigned with
Roamium/Chromium before Surfari can safely emit auth requests.

This experiment should implement the browser-state callbacks that WebKit exposes
through the macOS `WKWebView` delegate surface, prove them in the smoke harness,
and document any callback that needs JavaScript injection, private WebKit SPI,
or a later WebKit source patch. It should not create the Surfari Rust process,
modify Ghostboard, modify `termsurf.proto`, or edit `webkit/src`.

## Changes

- Study and use the relevant local WebKit references:
  - `Source/WebKit/UIProcess/API/Cocoa/WKWebView.mm`;
  - `Source/WebKit/UIProcess/API/mac/WKWebViewMac.mm`;
  - `Tools/TestWebKitAPI/Helpers/cocoa/TestUIDelegate.mm`;
  - `Tools/TestWebKitAPI/Helpers/cocoa/TestNavigationDelegate.mm`;
  - `Tools/TestWebKitAPI/Tests/WebKit/WKWebView/UIDelegate.mm`.
- Add a `WKUIDelegate`-backed owner object for `WebContents`, or extend the
  existing delegate structure if that keeps ownership simpler.
- Realign Surfari's HTTP auth C callback typedef and implementation call order
  to match the Chromium/Roamium/protobuf contract before implementing HTTP auth:
  `url`, `auth_scheme`, `challenger`, `realm`, `is_proxy`, `first_auth_attempt`,
  `is_primary_main_frame_navigation`, `is_navigation`.
- Implement and prove public `WKWebView` delegate callbacks where feasible:
  - JavaScript alert/confirm/prompt requests through `WKUIDelegate`;
  - `ts_reply_javascript_dialog` completion of pending dialog requests;
  - HTTP auth requests through `WKNavigationDelegate` authentication challenge
    handling;
  - `ts_reply_http_auth` completion or cancellation of pending auth requests;
  - web content process termination through
    `webViewWebContentProcessDidTerminate:`;
  - target URL hover changes if available without WebKit source edits.
- Investigate cursor updates and console messages, but do not fake support:
  - if cursor or console capture requires private SPI, JavaScript injection, or
    a WebKit source patch, record the exact API/path and leave the public
    callback unsupported for this experiment;
  - do not use page JavaScript injection merely to claim native console support.
- Extend the smoke test pages and harness so the callbacks that are implemented
  have deterministic local evidence.
- Keep the public C header compatible with `roamium/src/ffi.rs` and
  `chromium/src/content/libtermsurf_chromium/libtermsurf_chromium.h`. Private
  smoke-test-only helpers remain in `smoke-test/test_support.h` if needed.
- Update `surfari/libtermsurf_webkit/README.md` so the implemented and
  unsupported callback lists match reality.

## Verification

Start from a clean TermSurf repo root:

```bash
git status --short
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

Build and run the smoke test:

```bash
surfari/libtermsurf_webkit/build.sh

mkdir -p logs
DYLD_FRAMEWORK_PATH="$PWD/webkit/src/WebKitBuild/Debug" \
surfari/libtermsurf_webkit/build/smoke-test \
  "$PWD/surfari/libtermsurf_webkit/test-content/index.html" \
  "$PWD/surfari/libtermsurf_webkit/test-content/navigation.html" \
  > logs/issue756-exp8-browser-state.log 2>&1
rc=$?
echo "SMOKE_EXIT_STATUS=$rc" >> logs/issue756-exp8-browser-state.log
```

The smoke log must prove:

- Experiment 7 focus and Experiment 6 input/lifecycle evidence still pass.
- Each implemented callback fires with the expected deterministic payload.
- HTTP auth payload ordering and booleans match the Roamium/protobuf field
  semantics before an auth callback is counted as implemented.
- Each implemented reply API resolves its pending request and returns `true` for
  a valid pending request.
- Invalid or stale dialog/auth reply IDs return `false`.
- Any callback left unsupported is documented with a concrete WebKit limitation
  or a precise next implementation path.

Verify symbols/linkage and checkout state:

```bash
nm -gU surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg ' _ts_|_ts_webkit_test' | sort
otool -L surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg 'WebKit|JavaScriptCore|libtermsurf'
otool -L surfari/libtermsurf_webkit/build/smoke-test | rg 'WebKit|JavaScriptCore|libtermsurf'
git diff --check
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

**Pass** = the implemented browser-state callbacks work through real WebKit
delegate paths, the smoke test exits 0, all prior lifecycle/input/focus evidence
still passes, unsupported callbacks are documented honestly, and `webkit/src`
remains unchanged.

**Partial** = at least one browser-state callback works, but a callback expected
to work from public WebKit delegates proves to require private SPI, injection,
or a WebKit source patch. The result must record exact APIs tried and the next
experiment.

**Fail** = the implementation regresses Experiment 7 focus or Experiment 6
input/lifecycle behavior, or cannot identify a concrete path for unsupported
callbacks.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

**Verdict:** Approved after one required fix.

Required finding fixed:

- The original design overstated Surfari's current HTTP auth ABI compatibility.
  Surfari's header did not yet match Roamium/Chromium HTTP auth callback field
  order, so firing an auth callback through the current typedef would cause Rust
  or protocol consumers to misinterpret fields.

Fix applied:

- The design now explicitly requires realigning Surfari's HTTP auth callback
  typedef and call order to the Chromium/Roamium/protobuf contract before HTTP
  auth can be counted as implemented. It also names the exact field order and
  requires smoke verification of payload ordering and boolean semantics.

The reviewer re-reviewed the fix and approved it with no remaining required
findings.

## Result

**Result:** Partial

This experiment implemented the first browser-state callback slice: JavaScript
alert, confirm, and prompt requests now flow through public `WKUIDelegate`
methods in `surfari/libtermsurf_webkit/src/libtermsurf_webkit.mm`. Each request
gets a monotonically increasing request ID and is stored until
`ts_reply_javascript_dialog` resolves it. Valid replies return `true`; stale or
unknown request IDs return `false`.

The experiment also fixed the Surfari HTTP auth callback typedef in
`surfari/libtermsurf_webkit/include/libtermsurf_webkit.h` so the field order now
matches Roamium, Chromium, and `proto/termsurf.proto`:

```text
url, auth_scheme, challenger, realm, is_proxy, first_auth_attempt,
is_primary_main_frame_navigation, is_navigation
```

The smoke harness now registers `ts_set_on_javascript_dialog_request`, evaluates
a deterministic script that calls `alert`, `confirm`, and `prompt`, replies
through `ts_reply_javascript_dialog`, and checks both the JavaScript-visible
results and stale-reply rejection.

The passing smoke log recorded:

```text
CALLBACK focus_state {"focus":true,"focusIn":false,"hasFocus":true,"activeElement":""}
CALLBACK input_state {"focus":true,"focusIn":false,"blur":true,"move":"120,130","click":"140,150,0","scroll":-120,"key":"a","colorScheme":"dark"}
CALLBACK javascript_dialog request_id=1 type=alert origin=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/navigation.html message=surfari-alert default=
CALLBACK javascript_dialog request_id=2 type=confirm origin=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/navigation.html message=surfari-confirm default=
CALLBACK javascript_dialog request_id=3 type=prompt origin=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/navigation.html message=surfari-prompt default=default-prompt
CALLBACK javascript_dialog_state {"alert":"done","confirm":true,"prompt":"surfari-prompt-reply"}
SMOKE_PASS initialized=1 tab_ready=1 ca_context=4 url=4 loading_started=2 loading_finished=2 title=2 navigations=2 resized=1 focus=1 input=1 js_dialogs=1
SMOKE_EXIT_STATUS=0
```

Additional verification passed:

```text
surfari/libtermsurf_webkit/build.sh
nm -gU surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg ' _ts_|_ts_webkit_test' | sort
otool -L surfari/libtermsurf_webkit/build/libtermsurf_webkit.dylib | rg 'WebKit|JavaScriptCore|libtermsurf'
otool -L surfari/libtermsurf_webkit/build/smoke-test | rg 'WebKit|JavaScriptCore|libtermsurf'
git diff --check
git -C webkit/src status --short
git -C webkit/src rev-parse HEAD
git -C webkit/src rev-parse --abbrev-ref HEAD
git -C webkit/src rev-parse --is-shallow-repository
```

`webkit/src` remained unchanged at `1452a43959523449099b2616793fd2c5b6a6487e` on
branch `webkit-1452a439-issue-756`, and the checkout is still shallow.

## Conclusion

The browser-state callback work is only partially complete. JavaScript dialogs
are now implemented and proven through real WebKit delegate callbacks, and the
HTTP auth ABI mismatch is fixed before any auth requests can be emitted.

HTTP auth request/reply handling, renderer crash reporting, target URL changes,
cursor updates, and console messages remain unsupported. The next experiment
should implement HTTP auth using `WKNavigationDelegate` now that the C ABI field
order matches Roamium and the protocol.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

**Verdict:** Approved after two required fixes.

Required findings fixed:

- The public JavaScript dialog callback typedef in
  `surfari/libtermsurf_webkit/include/libtermsurf_webkit.h` used misleading
  string argument names/order. It now matches the implementation, Roamium,
  Chromium, and `proto/termsurf.proto`: `dialog_type`, `origin_url`, `message`,
  `default_prompt_text`.
- `surfari/libtermsurf_webkit/README.md` did not list console messages as
  unsupported even though the public callback setter exists and the experiment
  leaves console capture for later. The README now lists console messages under
  unsupported callbacks.

The reviewer re-reviewed both fixes, confirmed the refreshed smoke log still
ends with `SMOKE_EXIT_STATUS=0`, and approved the result with no remaining
required findings.
