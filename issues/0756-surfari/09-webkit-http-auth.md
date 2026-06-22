# Experiment 9: Implement WebKit HTTP auth callbacks

## Description

Experiment 8 fixed the HTTP auth callback typedef ordering so Surfari's public C
ABI now matches Roamium, Chromium, and `proto/termsurf.proto`. The callback
itself is still unsupported: `TSNavigationDelegate` does not implement
`webView:didReceiveAuthenticationChallenge:completionHandler:`, and
`ts_reply_http_auth` still returns `false`.

This experiment should implement HTTP auth request/reply handling for
`libtermsurf_webkit` through WebKit's real `WKNavigationDelegate` authentication
challenge path. The implementation must preserve the corrected ABI field order
from Experiment 8 and prove both successful credential replies and stale/invalid
reply rejection.

This experiment should not create the Surfari Rust binary, modify Ghostboard,
modify `termsurf.proto`, implement unrelated browser-state callbacks, or edit
`webkit/src`.

## Changes

- Study local WebKit auth references:
  - `Source/WebKit/UIProcess/API/Cocoa/WKNavigationDelegate.h`;
  - `Tools/TestWebKitAPI/Helpers/cocoa/TestNavigationDelegate.mm`;
  - `Tools/TestWebKitAPI/Tests/WebKit/WKWebView/Proxy.mm`;
  - `Tools/TestWebKitAPI/Tests/WebKit/WKWebView/Navigation.mm`.
- Add pending HTTP auth request storage to `WebContents`, analogous to the
  pending JavaScript dialog storage from Experiment 8.
- Implement `webView:didReceiveAuthenticationChallenge:completionHandler:` in
  `TSNavigationDelegate`.
- Map `NSURLAuthenticationChallenge` into the Roamium/protobuf field semantics:
  - `url` from the web view or challenge context;
  - `auth_scheme` normalized to Chromium-compatible values, starting with
    `basic` for `NSURLAuthenticationMethodHTTPBasic`;
  - `challenger` serialized in Chromium-compatible `scheme://host[:port]` form,
    omitting the port only for the default port of the URL scheme;
  - `realm` from `challenge.protectionSpace.realm`;
  - `is_proxy` from `challenge.protectionSpace.proxy`;
  - `first_auth_attempt` from `previousFailureCount == 0`;
  - `is_primary_main_frame_navigation` and `is_navigation` conservatively true
    for the main-frame smoke navigation unless WebKit exposes more precise
    public data.
- Only create pending requests for supported challenges in this experiment:
  non-proxy HTTP Basic challenges with a registered TermSurf callback. For
  unsupported challenges or when no callback is registered, synchronously invoke
  the WebKit completion handler with an appropriate rejection/cancel disposition
  and do not leave a pending request.
- Implement `ts_reply_http_auth`:
  - valid accepted replies call the stored completion handler with an
    `NSURLCredential` built from username/password;
  - valid rejected replies cancel or reject the challenge;
  - unknown/stale request IDs return `false`.
- Extend the smoke harness with deterministic local HTTP Basic auth coverage.
  Prefer an in-process loopback HTTP server owned by the smoke test so the test
  does not depend on external network state. The server should:
  - send a `401 Unauthorized` with `WWW-Authenticate: Basic realm="surfari"`;
  - accept the expected `Authorization` header after `ts_reply_http_auth`;
  - return a small HTML page that the smoke harness can verify.
- Include two deterministic auth navigations: one where the callback accepts
  with the expected username/password, and one where the callback rejects with
  `accepted=false` so the smoke harness proves valid rejected replies complete
  and then reject repeated/stale replies.
- Keep Experiment 6/7/8 smoke coverage intact: lifecycle, navigation, resize,
  focus, mouse, scroll, keyboard, color scheme, JavaScript dialogs, and stale
  JavaScript dialog replies must still pass.
- Update `surfari/libtermsurf_webkit/README.md` so HTTP auth moves from
  unsupported to implemented only if the real WebKit challenge path is proven.

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
  > logs/issue756-exp9-http-auth.log 2>&1
rc=$?
echo "SMOKE_EXIT_STATUS=$rc" >> logs/issue756-exp9-http-auth.log
```

The smoke log must prove:

- Experiment 6/7/8 evidence still passes.
- WebKit invokes the `WKNavigationDelegate` auth challenge path for the local
  protected HTTP resource.
- The C callback receives the expected deterministic auth payload:
  `auth_scheme="basic"`, `challenger="http://127.0.0.1:<port>"`,
  `realm="surfari"`, `is_proxy=false`, `first_auth_attempt=true`,
  `is_primary_main_frame_navigation=true`, and `is_navigation=true`.
- `ts_reply_http_auth` returns `true` for the pending request and allows the
  protected page to load with the expected title/body evidence.
- A valid rejected auth reply returns `true` and produces deterministic
  navigation rejection/cancellation evidence.
- Repeated or unknown auth reply IDs return `false`.

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

**Pass** = HTTP auth request/reply works through real WebKit delegate
authentication challenges, the smoke test exits 0, all prior evidence still
passes, stale auth replies are rejected, the README reflects support, and
`webkit/src` remains unchanged.

**Partial** = the library can represent pending auth requests or replies, but
the smoke test cannot prove the real `WKNavigationDelegate` challenge path
without a new harness or WebKit source/test helper. The result must record the
exact blocker and next experiment.

**Fail** = the implementation regresses prior lifecycle/input/focus/dialog
coverage, misorders auth fields, or cannot identify a concrete next step.

## Design Review

An adversarial Codex subagent reviewed the design with fresh context.

**Verdict:** Approved after three required fixes.

Required findings fixed:

- The initial design mapped WebKit authentication methods directly into
  `auth_scheme`, which was not faithful to Chromium/Roamium. The design now
  requires Chromium-compatible normalization, including `auth_scheme="basic"`
  and `challenger` serialized as `scheme://host[:port]`.
- The initial design did not specify completion behavior for unsupported
  challenges or missing TermSurf callbacks. The design now requires synchronous
  completion with a rejection/cancel disposition and no pending request for
  those cases.
- The initial verification did not prove valid rejected replies. The design now
  requires deterministic smoke coverage for both accepted and rejected auth
  replies, plus stale/unknown reply rejection.

The reviewer re-reviewed these fixes and approved the design with no remaining
required findings.

## Result

**Result:** Pass

`libtermsurf_webkit` now implements HTTP Basic auth requests through WebKit's
real `WKNavigationDelegate` authentication challenge path. The implementation
creates pending auth requests only for non-proxy
`NSURLAuthenticationMethodHTTPBasic` challenges when a TermSurf callback is
registered. Unsupported or no-callback challenges synchronously complete with
`NSURLSessionAuthChallengeRejectProtectionSpace` and do not leave pending state.

The emitted C callback payload is normalized to match Roamium/Chromium/proto
semantics:

- `auth_scheme` is `basic`;
- `challenger` is serialized as `scheme://host[:port]`;
- `realm` comes from the protection space;
- `is_proxy` is false for the supported path;
- `first_auth_attempt` is derived from `previousFailureCount == 0`;
- `is_primary_main_frame_navigation` and `is_navigation` are true for the proven
  main-frame smoke navigations.

`ts_reply_http_auth` now returns `true` for a pending request, creates an
`NSURLCredential` for accepted replies, cancels rejected replies, removes the
pending request, and returns `false` for repeated or unknown request IDs.

The smoke harness now owns a deterministic loopback HTTP server. It serves a
protected `/auth-accept` path that succeeds only after the expected Basic
Authorization header and a protected `/auth-reject` path used to prove valid
rejected replies complete the WebKit challenge with cancellation.

The passing smoke log recorded:

```text
CALLBACK focus_state {"focus":true,"focusIn":false,"hasFocus":true,"activeElement":""}
CALLBACK input_state {"focus":true,"focusIn":false,"blur":true,"move":"120,130","click":"140,150,0","scroll":-120,"key":"a","colorScheme":"dark"}
CALLBACK javascript_dialog request_id=1 type=alert origin=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/navigation.html message=surfari-alert default=
CALLBACK javascript_dialog request_id=2 type=confirm origin=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/navigation.html message=surfari-confirm default=
CALLBACK javascript_dialog request_id=3 type=prompt origin=file:///Users/astrohacker/dev/termsurf/surfari/libtermsurf_webkit/test-content/navigation.html message=surfari-prompt default=default-prompt
CALLBACK javascript_dialog_state {"alert":"done","confirm":true,"prompt":"surfari-prompt-reply"}
CALLBACK http_auth request_id=4 url=http://127.0.0.1:49992/auth-accept scheme=basic challenger=http://127.0.0.1:49992 realm=surfari proxy=0 first=1 primary=1 navigation=1
CALLBACK http_auth_accept_state Surfari Auth OK:auth-ok
CALLBACK http_auth request_id=5 url=http://127.0.0.1:49992/auth-reject scheme=basic challenger=http://127.0.0.1:49992 realm=surfari proxy=0 first=1 primary=1 navigation=1
SMOKE_PASS initialized=1 tab_ready=1 ca_context=5 url=6 loading_started=4 loading_finished=4 title=3 navigations=4 resized=1 focus=1 input=1 js_dialogs=1 http_auth=1
SMOKE_EXIT_STATUS=0
```

The rejected auth navigation logs WebKit's expected cancellation:

```text
provisional navigation failed: Error Domain=NSURLErrorDomain Code=-999 "cancelled"
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

HTTP Basic auth is now implemented and proven in `libtermsurf_webkit`. The next
browser-state gaps are renderer crash reporting, target URL changes, cursor
updates, and console messages.

## Completion Review

An adversarial Codex subagent reviewed the completed experiment with fresh
context.

**Verdict:** Approved.

Required findings: none.

The reviewer independently verified that:

- implementation scope matches Experiment 9;
- auth payload order and normalization match proto/Roamium/Chromium
  expectations;
- unsupported/no-callback challenges complete immediately with reject
  disposition;
- accepted, rejected, stale, and unknown replies are covered by smoke logic;
- prior lifecycle/input/focus/dialog smoke coverage still passes;
- README status is `Pass` and HTTP auth support is documented;
- `webkit/src` is unchanged, shallow, and on branch `webkit-1452a439-issue-756`;
- `git diff --check`, build, smoke test, symbols, and linkage checks passed.
