# Experiment 7: Add Protocol Console Capture

## Description

Experiment 1 classified console capture as `Automatable now`. Experiment 6
completed page zoom, so the next deterministic browser feature is JavaScript
console capture.

TermSurf currently has no app-facing console stream. Developers can open
DevTools, but DevTools is a separate browser surface and is too heavy for simple
"what did this page log?" feedback. Issue 616 recorded the original ts1 console
capture as JavaScript injection to stdout/stderr. That approach is not suitable
for TermSurf now because it changes page JavaScript behavior, can be bypassed,
and does not capture browser-generated console messages.

Chromium already provides the correct embedder hook:

```text
content::WebContentsObserver::OnDidAddMessageToConsole(...)
```

This experiment should route that hook into the TermSurf protocol as a
Chromium-to-client event, then verify it with the Issue 799 fake-GUI harness.
The fix must not depend on DevTools, remote debugging ports, injected
`console.*` wrappers, stderr scraping, or Content Shell's web-test-only console
behavior.

## Changes

1. Create a new Chromium branch.

   In `chromium/src`, fork from:

   ```text
   148.0.7778.97-issue-799-exp6
   ```

   Name the new branch:

   ```text
   148.0.7778.97-issue-799-exp7
   ```

   Add it to `chromium/README.md` with a description such as:

   ```text
   Add protocol console capture.
   ```

2. Extend `termsurf.proto`.

   Add a new oneof entry after the JavaScript dialog messages:

   ```text
   ConsoleMessage console_message = 36;
   ```

   Add a message:

   ```text
   message ConsoleMessage {
     int64 tab_id = 1;
     string level = 2;
     string message = 3;
     int32 line_no = 4;
     string source_id = 5;
   }
   ```

   Use string levels for stability across Rust, Python, and C++:

   ```text
   verbose
   info
   warning
   error
   unknown
   ```

   Do not renumber existing protocol fields. Do not add a request/reply pair;
   console capture is a one-way browser event.

3. Emit console messages from Chromium.

   Update:

   ```text
   chromium/src/content/libtermsurf_chromium/ts_tab_observer.h
   chromium/src/content/libtermsurf_chromium/ts_tab_observer.cc
   chromium/src/content/libtermsurf_chromium/libtermsurf_chromium.h
   chromium/src/content/libtermsurf_chromium/libtermsurf_chromium.cc
   chromium/src/content/libtermsurf_chromium/ts_browser_main_parts.h
   chromium/src/content/libtermsurf_chromium/ts_browser_main_parts.cc
   ```

   Add `TsTabObserver::OnDidAddMessageToConsole(...)` and call a new callback
   such as `TsNotifyConsoleMessage(...)`.

   Required behavior:
   - include the owning TermSurf `tab_id`;
   - convert `message` and `source_id` from UTF-16 to UTF-8;
   - map `blink::mojom::ConsoleMessageLevel` to the stable level strings above;
   - include `line_no`;
   - emit messages from all frames in the tab, not only the primary main frame;
   - do not return through `Shell::DidAddMessageToConsole()` or rely on
     `--run-web-tests`;
   - do not suppress Chromium's normal console handling.

   `untrusted_stack_trace` is explicitly out of scope for this experiment. It is
   optional, absent for most messages, and would make the protocol surface
   larger than needed for the first console-capture pass.

4. Route console messages through Roamium.

   Update:

   ```text
   roamium/src/ffi.rs
   roamium/src/dispatch.rs
   ```

   Add an FFI binding for the new callback setter and an extern callback that
   builds `Msg::ConsoleMessage`.

   Required behavior:
   - resolve the Chromium `TsWebContents` handle to the correct `tab_id`;
   - preserve the level/message/source/line fields exactly;
   - log stable diagnostic lines with a prefix such as `[termsurf-console]`;
   - do not crash or drop the browser process if there is no connected client.

   Run `cargo fmt` after Rust edits and accept its output.

5. Route console messages through Wezboard.

   Update:

   ```text
   wezboard/wezboard-gui/src/termsurf/conn.rs
   ```

   Route `ConsoleMessage` from a browser process to the pane's TUI/direct
   browser connection in the same style as `UrlChanged`, `TitleChanged`,
   `TargetUrlChanged`, and `JavaScriptDialogRequest`.

   Required behavior:
   - if the TUI/direct connection exists, forward the message;
   - if it does not exist yet, drop the message safely and log the drop;
   - do not turn console messages into modal UI, native notifications, or popup
     prompts.

   Run `cargo fmt` after Rust edits and accept its output.

6. Handle console messages in `webtui`.

   Update:

   ```text
   webtui/src/main.rs
   ```

   Minimal required user-facing behavior:
   - receive `ConsoleMessage` from the direct browser connection or compositor
     route;
   - store a small in-memory ring buffer, such as the most recent 100 messages;
   - expose the latest warning/error level message in the existing terminal UI
     status/footer area without hiding the web overlay or adding a new browser
     mode;
   - keep ordinary browsing and JavaScript dialog UI behavior unchanged.

   This experiment does not need a full console drawer, filtering UI, search,
   persistence, copy support, or DevTools replacement. A richer console viewer
   can be designed later if users need it.

   Run `cargo fmt` after Rust edits and accept its output.

7. Extend the Issue 799 harness.

   Update:

   ```text
   scripts/test-issue-799-browser-api-audit.py
   ```

   Add a probe such as:

   ```text
   console-capture-basic
   ```

   The local fixture should emit deterministic, unique messages from both the
   top-level frame and a same-origin iframe:

   ```text
   console.log("ts-console-top-log-<nonce>")
   console.info("ts-console-top-info-<nonce>")
   console.warn("ts-console-top-warn-<nonce>")
   console.error("ts-console-top-error-<nonce>")
   iframe: console.warn("ts-console-frame-warn-<nonce>")
   ```

   Add one deterministic non-`console.*` browser-console source too:

   ```text
   setTimeout(() => { throw new Error("ts-console-throw-<nonce>"); }, 0)
   ```

   The thrown error is required because it proves the implementation is using
   Chromium console plumbing, not only JavaScript wrappers around `console.*`.

   Expected level mapping:

   | Fixture source        | Expected protocol level |
   | --------------------- | ----------------------- |
   | `console.log(...)`    | `info`                  |
   | `console.info(...)`   | `info`                  |
   | `console.warn(...)`   | `warning`               |
   | `console.error(...)`  | `error`                 |
   | uncaught `Error(...)` | `error`                 |

   The fake-GUI harness should capture top-level `ConsoleMessage` protobufs and
   classify the probe as:

   ```text
   console_capture_completed
   ```

   only if:
   - all expected top-level, iframe, and uncaught-error nonce-tagged messages
     arrive over the protocol;
   - each expected message has the expected level;
   - each message has the correct `tab_id`;
   - top-level `source_id` contains the local top-level probe URL;
   - iframe `source_id` contains the local iframe fixture URL;
   - `line_no` is greater than zero;
   - no bad-Mojo, missing-binder, or crash signature appears.

   If the page reports completion but the protocol messages do not arrive,
   classify as:

   ```text
   console_capture_failed
   ```

   Update `coverage-map.md` and `reference-coverage-map.md` output so the new
   classifications have accurate next-action text.

8. Run formatters.

   Required formatting:

   ```bash
   prettier --write --prose-wrap always --print-width 80 \
     issues/0799-browser-api-automation-triage/README.md \
     issues/0799-browser-api-automation-triage/07-console-capture.md
   ```

   For Rust edits, run:

   ```bash
   PATH="/Users/ryan/.rustup/toolchains/1.92.0-aarch64-apple-darwin/bin:$PATH" \
     /Users/ryan/.rustup/toolchains/1.92.0-aarch64-apple-darwin/bin/cargo fmt
   ```

   from each edited Rust crate as needed. Accept formatter output as-is.

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

   Run the focused console probe:

   ```bash
   python3 scripts/test-issue-799-browser-api-audit.py \
     --probe console-capture-basic \
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
- the focused `console-capture-basic` probe classifies as
  `console_capture_completed`;
- the focused probe records the expected top-level log/info/warn/error messages,
  the same-origin iframe warning message, and the uncaught error message with
  correct level, tab id, source id, and positive line number;
- the full Issue 799 harness still completes with the previously passing
  download, JavaScript-dialog, and page-zoom probes green;
- no renderer bad-Mojo, missing-binder, or crash signatures appear in the
  console probe logs;
- console capture does not require DevTools, stderr scraping, injected
  `console.*` wrappers, native UI, or manual testing;
- Codex reviews the completed experiment and no blocking findings remain.

This experiment is partial if:

- Chromium emits console messages and the harness receives them, but webtui only
  stores/logs them and does not yet expose any user-facing status;
- the main-frame console messages work but subframe messages are dropped and the
  missing route is diagnosed;
- the focused probe passes but the full Issue 799 harness regresses in an
  unrelated existing probe and the cause is diagnosed;
- the protocol is correct but a Rust routing layer needs a follow-up experiment.

This experiment fails if:

- it relies on DevTools or remote debugging as the primary console stream;
- it captures console output by injecting JavaScript wrappers around
  `console.*`;
- it only scrapes process stderr/stdout instead of sending a protocol event;
- it drops messages silently when a TUI client exists;
- it changes page behavior, navigation, dialogs, downloads, page zoom, or PDF
  behavior while adding console capture.
