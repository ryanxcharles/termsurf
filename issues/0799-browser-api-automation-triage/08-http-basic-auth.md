# Experiment 8: Add Protocol HTTP Basic Auth

## Description

Experiment 1 classified HTTP Basic Auth as `Automatable after setup`.
Experiments 5 and 7 already established the right TermSurf pattern for
browser-blocking interactions: Chromium owns the browser callback, the TermSurf
protocol carries a request/reply, and the automated fake-GUI harness supplies
the reply without native UI.

The current Chromium path inherits Content Shell behavior:

- `ShellContentBrowserClient::CreateLoginDelegate(...)` is called for HTTP auth
  challenges;
- Content Shell invokes an optional test callback, then returns `nullptr`;
- returning `nullptr` cancels the auth request instead of asking the user for
  credentials.

TermSurf needs a contained protocol-mediated login flow for HTTP Basic Auth:

1. Chromium receives an origin-server auth challenge.
2. Roamium sends an `HttpAuthRequest` protobuf to connected clients.
3. The TUI displays an inline terminal credential prompt and sends
   `HttpAuthReply`.
4. Chromium runs the stored `LoginAuthRequiredCallback` with credentials or
   cancellation.

This experiment is limited to HTTP Basic Auth for origin-server page loads. It
must not implement a password manager, proxy auth UI, OS keychain integration,
credential persistence, autofill, Chrome's full `HttpAuthCoordinator`, or a
native AppKit dialog. Those can be designed later if needed.

## Changes

1. Create a new Chromium branch.

   In `chromium/src`, fork from:

   ```text
   148.0.7778.97-issue-799-exp7
   ```

   Name the new branch:

   ```text
   148.0.7778.97-issue-799-exp8
   ```

   Add it to `chromium/README.md` with a description such as:

   ```text
   Add protocol HTTP Basic Auth.
   ```

2. Extend `termsurf.proto`.

   Add two new oneof entries after `ConsoleMessage`:

   ```text
   HttpAuthRequest http_auth_request = 37;
   HttpAuthReply http_auth_reply = 38;
   ```

   Add request/reply messages:

   ```text
   message HttpAuthRequest {
     int64 tab_id = 1;
     uint64 request_id = 2;
     string url = 3;
     string auth_scheme = 4;
     string challenger = 5;
     string realm = 6;
     bool is_proxy = 7;
     bool first_auth_attempt = 8;
     bool is_primary_main_frame_navigation = 9;
     bool is_navigation = 10;
   }

   message HttpAuthReply {
     int64 tab_id = 1;
     uint64 request_id = 2;
     bool accepted = 3;
     string username = 4;
     string password = 5;
   }
   ```

   Do not renumber existing protocol fields.

   `auth_scheme` is Chromium's auth scheme, such as `basic`. `challenger` is the
   serialized `net::AuthChallengeInfo::challenger` scheme/host/port tuple, not
   just a hostname. Preserve the port because the automated localhost fixture
   depends on it.

   Passwords are allowed in this protocol message because the protocol is local
   Unix-socket IPC and the user is explicitly entering credentials into the
   local TUI. Do not log the password in Chromium, Roamium, Wezboard, webtui, or
   the harness.

3. Add a TermSurf HTTP auth delegate in Chromium.

   Update:

   ```text
   chromium/src/content/libtermsurf_chromium/ts_browser_client.h
   chromium/src/content/libtermsurf_chromium/ts_browser_client.cc
   chromium/src/content/libtermsurf_chromium/libtermsurf_chromium.h
   chromium/src/content/libtermsurf_chromium/libtermsurf_chromium.cc
   chromium/src/content/libtermsurf_chromium/ts_browser_main_parts.h
   ```

   Add a small `content::LoginDelegate` implementation under
   `content/libtermsurf_chromium/`, for example:

   ```text
   ts_http_auth_delegate.h
   ts_http_auth_delegate.cc
   ```

   `TsBrowserClient::CreateLoginDelegate(...)` should:
   - handle origin-server Basic auth challenges for a non-null `WebContents`;
   - create a monotonic `request_id`;
   - return a `LoginDelegate` that owns the pending `LoginAuthRequiredCallback`;
   - notify Roamium through a new callback such as
     `TsNotifyHttpAuthRequest(...)`;
   - include `url`, `auth_scheme`, `challenger`, `realm`, `is_proxy`,
     `first_auth_attempt`, `is_request_for_primary_main_frame_navigation`, and
     `is_request_for_navigation` in the notification;
   - invalidate the pending request safely if `WebContents` is destroyed or the
     delegate is destroyed before a reply arrives;
   - ignore late or duplicate replies instead of running the callback twice.

   The reply entry point should look like:

   ```text
   ts_reply_http_auth(wc, request_id, accepted, username, password)
   ```

   Required reply behavior:
   - if `accepted` is false, run the stored callback with `std::nullopt`;
   - if `accepted` is true, convert username/password to UTF-16 and run the
     stored callback with `net::AuthCredentials`;
   - erase the pending request before running the callback;
   - return false for unknown, stale, duplicate, or destroyed requests.

   Lifecycle rule:
   - user/protocol cancellation runs the stored auth callback with
     `std::nullopt`;
   - delegate destruction or `WebContentsDestroyed` only unregisters and
     invalidates the pending request;
   - destruction must not call the auth callback, matching Chromium's
     `LoginDelegate` contract.

   Do not call the auth callback reentrantly from inside
   `CreateLoginDelegate(...)`. Sending the protocol request may happen during
   construction, but the credential callback must only run from a later reply or
   explicit protocol cancellation.

   Proxy auth, Negotiate/NTLM, saved credentials, autofill, retry UI, and Chrome
   password-manager integration are out of scope. If a non-Basic or proxy
   challenge reaches this path, cancel it deterministically and log the scheme
   without logging credentials.

4. Route auth requests through Roamium.

   Update:

   ```text
   roamium/src/ffi.rs
   roamium/src/dispatch.rs
   roamium/src/main.rs
   ```

   Add FFI for the new callback setter and reply function.

   Required behavior:
   - register `ts_set_on_http_auth_request(...)` at startup;
   - resolve the Chromium `TsWebContents` handle to the correct TermSurf
     `tab_id`;
   - build `Msg::HttpAuthRequest` and send it to connected clients;
   - handle incoming `Msg::HttpAuthReply` by calling `ts_reply_http_auth(...)`;
   - log stable `[termsurf-http-auth]` request/reply diagnostics without logging
     passwords.

   Blocking request safety:
   - if Roamium knows no client can receive the request, it must immediately
     call `ts_reply_http_auth(...)` with `accepted = false`;
   - if sending the request to the connected peer fails, it must also cancel the
     pending auth request;
   - a request must never be dropped in a way that leaves Chromium's
     `LoginDelegate` pending forever.

   Run `cargo fmt` after Rust edits and accept formatter output.

5. Route auth requests through Wezboard.

   Update:

   ```text
   wezboard/wezboard-gui/src/termsurf/conn.rs
   ```

   Route `HttpAuthRequest` from the browser process to the pane's TUI/direct
   browser connection in the same style as `JavaScriptDialogRequest`. Route
   `HttpAuthReply` from the TUI back to the browser process in the same style as
   `JavaScriptDialogReply`.

   Required behavior:
   - forward when the destination connection exists;
   - if no pane/server exists, send `HttpAuthReply { accepted: false }` back
     toward Roamium instead of dropping the blocking request;
   - if forwarding fails, send the same cancel reply back toward Roamium;
   - never show native UI;
   - never log passwords.

   Run `cargo fmt` after Rust edits and accept formatter output.

6. Add minimal webtui auth UI.

   Update:

   ```text
   webtui/src/ipc.rs
   webtui/src/main.rs
   ```

   Add a minimal inline terminal credential prompt.

   Required behavior:
   - receive `HttpAuthRequest` from either direct browser or compositor route;
   - enter a temporary auth mode that does not hide the web overlay;
   - show origin/realm/auth scheme and whether this is a retry, but do not show
     or log the password;
   - collect username and password;
   - mask password input in the UI;
   - send `HttpAuthReply` with `accepted = true` on submit;
   - send `HttpAuthReply` with `accepted = false` on Esc/cancel;
   - return to normal browsing mode after reply.

   The UI may be simple and keyboard-only. It does not need password-manager
   save prompts, autofill, paste filtering beyond existing input behavior,
   credential persistence, or multi-step retry styling.

   Run `cargo fmt` after Rust edits and accept formatter output.

7. Extend the Issue 799 harness.

   Update:

   ```text
   scripts/test-issue-799-browser-api-audit.py
   ```

   Add a local HTTP Basic Auth fixture. The server should protect a path such
   as:

   ```text
   /auth/basic/success.html
   ```

   It should return:

   ```text
   401 Unauthorized
   WWW-Authenticate: Basic realm="TermSurf Issue 799"
   ```

   until it receives:

   ```text
   Authorization: Basic <base64("termsurf:correct horse battery staple")>
   ```

   On success, return an HTML page that reports completion through the existing
   harness report path.

   Add at least these probes:
   - `http-basic-auth-success`: the fake GUI receives `HttpAuthRequest`, replies
     with the expected username/password, and the protected page reports
     success.
   - `http-basic-auth-cancel`: the fake GUI receives `HttpAuthRequest`, replies
     with `accepted = false`, and the navigation does not reach the protected
     success page. The probe should classify as a clean cancellation, not a
     crash, bad Mojo, or process exit.

   Optional if the first two probes are stable:
   - `http-basic-auth-retry`: the fake GUI first replies with wrong credentials,
     receives a second request with `first_auth_attempt = false`, then replies
     with correct credentials and reaches the protected page.

   The harness should capture top-level `HttpAuthRequest` and `HttpAuthReply`
   protobufs and classify the success probe as:

   ```text
   http_auth_completed
   ```

   only if:
   - the auth request arrives over the protocol;
   - `tab_id` matches the created tab;
   - `auth_scheme` is `basic`;
   - `challenger` includes the fixture server host and port;
   - `realm` is `TermSurf Issue 799`;
   - `is_proxy` is false;
   - the reply uses the same `tab_id` and `request_id` as the request;
   - local server evidence shows the sequence: unauthenticated request, `401`
     challenge, protocol reply, authenticated request, success page;
   - the protected page loads and reports the expected nonce;
   - no password appears in `messages.log`, `roamium.stderr`, or the result
     JSON;
   - no bad-Mojo, missing-binder, or crash signature appears.

   The cancel probe should classify as:

   ```text
   http_auth_cancelled
   ```

   only if the request arrives, the harness sends a cancel reply, the protected
   success page does not report completion, and the same tab is immediately
   usable afterward. Prove tab usability by navigating the same tab to an
   unprotected local page and requiring that page to report completion. A live
   process without post-cancel navigation success is not enough.

   Update `coverage-map.md` and `reference-coverage-map.md` output so the new
   classifications have accurate next-action text.

8. Run formatters.

   Required markdown formatting:

   ```bash
   prettier --write --prose-wrap always --print-width 80 \
     issues/0799-browser-api-automation-triage/README.md \
     issues/0799-browser-api-automation-triage/08-http-basic-auth.md
   ```

   Required Rust formatting:

   ```bash
   PATH="/Users/ryan/.rustup/toolchains/1.92.0-aarch64-apple-darwin/bin:$PATH" \
     /Users/ryan/.rustup/toolchains/1.92.0-aarch64-apple-darwin/bin/cargo fmt
   ```

   Run `cargo fmt` from each edited Rust crate as needed. Accept formatter
   output as-is.

9. Build and run automated verification.

   Build Chromium:

   ```bash
   cd chromium/src
   export PATH="$HOME/dev/termsurf/chromium/depot_tools:$PATH"
   autoninja -C out/Default libtermsurf_chromium
   ```

   Build Rust components that changed:

   ```bash
   PATH="/Users/ryan/.rustup/toolchains/1.92.0-aarch64-apple-darwin/bin:$PATH" \
     ./scripts/build.sh roamium
   PATH="/Users/ryan/.rustup/toolchains/1.92.0-aarch64-apple-darwin/bin:$PATH" \
     ./scripts/build.sh wezboard
   PATH="/Users/ryan/.rustup/toolchains/1.92.0-aarch64-apple-darwin/bin:$PATH" \
     ./scripts/build.sh webtui
   ```

   Run focused auth probes:

   ```bash
   python3 scripts/test-issue-799-browser-api-audit.py \
     --probe http-basic-auth-success \
     --seconds 10

   python3 scripts/test-issue-799-browser-api-audit.py \
     --probe http-basic-auth-cancel \
     --seconds 10
   ```

   Then run the full Issue 799 harness:

   ```bash
   python3 scripts/test-issue-799-browser-api-audit.py --seconds 10
   ```

10. Archive Chromium only after a passing implementation.

    If the experiment passes, commit the Chromium branch and regenerate:

    ```bash
    cd chromium/src
    rm -rf ../../chromium/patches/issue-799/
    git format-patch 148.0.7778.97..HEAD -o ../../chromium/patches/issue-799/
    ```

    Commit the main repo changes, including the updated patch archive and issue
    result. If the experiment is Partial or Fail, record the result first and do
    not archive an incoherent Chromium branch unless the partial branch is the
    intended base for the next experiment.

11. Get Codex review before and after implementation.

    Before implementation, run `codex-review` against this experiment design and
    fix all real findings before starting code changes.

    After implementation and result recording, run `codex-review` again against
    the diff, test output, and recorded result. Do not mark the experiment Pass
    until Codex agrees there are no blocking issues or all real issues are
    fixed.

## Verification

This experiment passes if:

- Chromium builds with `autoninja -C out/Default libtermsurf_chromium`;
- Roamium, Wezboard, and webtui build successfully;
- focused `http-basic-auth-success` classifies as `http_auth_completed`;
- focused `http-basic-auth-cancel` classifies as `http_auth_cancelled`;
- the success probe proves the protected page loads only after the protocol
  credential reply;
- the cancel probe proves cancellation does not load the protected success page
  and does not hang/crash the browser;
- no password appears in harness result JSON, `messages.log`, `roamium.stderr`,
  or stable diagnostic logs;
- the full Issue 799 harness still completes with previously passing download,
  JavaScript-dialog, page-zoom, and console-capture probes green;
- no renderer bad-Mojo, missing-binder, or crash signatures appear in auth probe
  logs;
- HTTP auth does not require native UI, OS permissions, external accounts,
  manual testing, password persistence, DevTools, or Chrome's full password
  manager;
- Codex reviews the completed experiment and no blocking findings remain.

This experiment is partial if:

- Chromium auth requests and fake-GUI replies work, but webtui only logs the
  request and does not yet provide an inline credential prompt;
- success works but cancellation needs a more precise automated classification;
- origin-server Basic auth works but proxy auth, Negotiate, NTLM, or persisted
  credentials remain unsupported and explicitly out of scope;
- the protocol is correct but a routing layer needs a follow-up experiment.

This experiment fails if:

- it opens a native auth dialog;
- it logs passwords;
- it returns credentials synchronously/reentrantly from inside
  `CreateLoginDelegate(...)`;
- it only hardcodes test credentials in Chromium instead of using a protocol
  request/reply;
- it changes downloads, JavaScript dialogs, page zoom, console capture, PDF
  behavior, or normal unauthenticated navigation while adding HTTP auth.

## Result

**Result:** Pass

Experiment 8 added protocol-mediated HTTP Basic Auth for origin-server Basic
auth challenges.

Implemented behavior:

- `termsurf.proto` now includes `HttpAuthRequest http_auth_request = 37` and
  `HttpAuthReply http_auth_reply = 38`.
- Chromium overrides `TsBrowserClient::CreateLoginDelegate(...)` and returns a
  TermSurf `LoginDelegate` for non-proxy Basic auth challenges with a non-null
  `WebContents`.
- Chromium stores the auth callback by `WebContents`/`request_id`, emits a
  protocol request, and runs the callback exactly once on accepted/canceled
  protocol replies.
- Delegate destruction and `WebContentsDestroyed` invalidate the pending request
  without running the callback, matching Chromium's `LoginDelegate` contract.
- Roamium routes auth requests/replies and defers no-client/missing-tab
  cancellation through `ts_post_task`, avoiding reentrant callbacks from inside
  `CreateLoginDelegate(...)`.
- Wezboard forwards auth requests to the pane TUI and sends a cancel reply back
  toward Roamium if no pane/server route exists.
- webtui displays a minimal inline auth prompt, masks password input, and sends
  accepted/canceled replies.
- The Issue 799 harness now serves a local Basic Auth fixture and verifies both
  success and cancellation.

Security/logging fixes:

- The harness no longer runs Roamium with `--v=1`; that verbose Chromium flag
  exposed HTTP request headers, including `Authorization: Basic ...`, in
  `roamium.stderr`.
- The auth verifier now fails if stderr/stdout/messages contain the fixture
  password, `username:password`, the base64 Basic credential, or
  `Authorization: Basic`.
- An explicit grep across the new focused/full auth log directories found no
  credential material.

Formatting and build evidence:

- Ran `cargo fmt` for edited Rust crates and accepted formatter output.
- Ran Chromium `clang-format` on the edited C++ files.
- `python3 -m py_compile scripts/test-issue-799-browser-api-audit.py` passed.
- `git diff --check` passed for the main repo.
- `git -C chromium/src diff --check` passed for Chromium.
- `autoninja -C out/Default libtermsurf_chromium` passed.
- `PATH="/Users/ryan/.rustup/toolchains/1.92.0-aarch64-apple-darwin/bin:$PATH" ./scripts/build.sh roamium`
  passed.
- `PATH="/Users/ryan/.rustup/toolchains/1.92.0-aarch64-apple-darwin/bin:$PATH" ./scripts/build.sh webtui`
  passed.
- `PATH="/Users/ryan/.rustup/toolchains/1.92.0-aarch64-apple-darwin/bin:$PATH" ./scripts/build.sh wezboard`
  passed.

Automated verification:

- Focused success probe:
  `logs/issue-799-browser-api-audit/20260531-013830-175631`
  - `http-basic-auth-success`: `http_auth_completed`
  - `missing_interfaces`: `[]`
- Focused cancel probe:
  `logs/issue-799-browser-api-audit/20260531-013844-558097`
  - `http-basic-auth-cancel`: `http_auth_cancelled`
  - `missing_interfaces`: `[]`
- Full Issue 799 harness:
  `logs/issue-799-browser-api-audit/20260531-013858-852868`
  - 23 probes completed.
  - `http-basic-auth-success`: `http_auth_completed`
  - `http-basic-auth-cancel`: `http_auth_cancelled`
  - `download-attachment`: `download_completed`
  - `download-blob`: `download_completed`
  - JavaScript dialog probes: `dialog_completed`
  - `page-zoom-shortcuts`: `page_zoom_completed`
  - `console-capture-basic`: `console_capture_completed`
  - `missing_interfaces`: `[]`

Codex reviewed the first implementation and found real blockers: credential
leakage through verbose Chromium logs, possible reentrant no-client
cancellation, a missing-tab drop path, and weak ordering verification. Those
were fixed. Codex re-reviewed the updated implementation and found no blocking
findings remaining.

## Conclusion

HTTP Basic Auth is now a contained, automatable TermSurf browser feature for
origin-server Basic auth challenges. It uses the same request/reply protocol
pattern as JavaScript dialogs, avoids native UI, does not persist credentials,
does not integrate a password manager, and proves both success and cancellation
without manual testing.

Out of scope remains explicit: proxy auth, Negotiate/NTLM, saved credentials,
autofill, OS keychain integration, retry UI polish, and Chrome's full
`HttpAuthCoordinator` product stack.

The next Issue 799 experiment should continue with the remaining automatable
queue, most likely renderer crash recovery UX or default-deny permission/API
hardening.
