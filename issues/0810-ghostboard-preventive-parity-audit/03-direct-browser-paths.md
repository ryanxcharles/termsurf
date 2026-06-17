# Experiment 3: Direct Browser Paths

## Description

Experiment 2 corrected an important architectural assumption: current `webtui`
connects directly to Roamium after `BrowserReady`, so missing Ghostboard
dispatcher cases do not automatically prove browser chrome, dialog/auth,
console, crash, or color-scheme gaps.

This experiment audits those direct-browser paths and the remaining compositor
fallback paths. It should turn the Experiment 2 `Maybe` findings into a clearer
ranked list:

- which behavior has convincing direct-browser runtime evidence;
- which behavior still lacks Ghostboard-specific regression evidence;
- which behavior depends on a compositor fallback that Ghostboard appears not to
  implement;
- which findings should be deferred to the historical issue audit rather than
  treated as protocol gaps.

This is an audit/documentation experiment only. It must not change application
code or test harnesses.

## Changes

Planned files:

- `issues/0810-ghostboard-preventive-parity-audit/03-direct-browser-paths.md`
  - record this experiment design, design review, result, completion review, and
    conclusion;
  - record direct-browser and fallback path evidence for each Experiment 2
    `Maybe` finding.
- `issues/0810-ghostboard-preventive-parity-audit/README.md`
  - add Experiment 3 to the `## Experiments` index with status `Designed`, then
    update status after the result.

No application code, generated protobuf code, historical issue files, closed
issue files, scripts, or test harnesses should be edited.

## Verification

Design-gate pass criteria:

- The issue README links this experiment as `Designed`.
- A fresh-context adversarial design review approves the plan.
- The plan commit exists before implementation begins.

Implementation pass criteria:

- The result covers every `Maybe` finding from Experiment 2:
  - browser chrome/status over `BrowserConnection` and compositor fallback,
    including URL, loading state, title, hover target, and console capture;
  - JavaScript dialog and HTTP auth over `BrowserConnection` and compositor
    fallback;
  - renderer crash over `BrowserConnection` and compositor fallback;
  - color scheme initial state, direct runtime state, and compositor fallback;
  - cursor shape updates and `SetGuiActive`.
- For each item, the result records:
  - source protocol messages;
  - direct-browser path evidence, or evidence that no direct path exists;
  - compositor fallback evidence, or evidence that no fallback is required;
  - Ghostboard-specific evidence from Issue 809 or current code where it exists;
  - likelihood: `Highly likely`, `Maybe`, or `No`;
  - risk or impact;
  - recommended follow-up.
- The audit must distinguish:
  - direct Roamium socket evidence from Ghostboard compositor evidence;
  - static code-path evidence from end-to-end runtime/test evidence;
  - normal post-`BrowserReady` behavior from fallback/pre-ready behavior.
- `No` is allowed only when there is concrete implementation evidence and the
  path does not require untested Ghostboard behavior, or when existing Issue 809
  evidence already proves the relevant runtime path.
- `Highly likely` is allowed only when the required normal path appears absent
  or disconnected. A missing optional fallback path should not be labeled
  `Highly likely` unless the audit shows that normal usage depends on it.
- The result identifies the next audit slice. Expected next slice: begin the
  historical issue audit if the direct-browser audit reduces the protocol
  findings to bounded follow-ups; otherwise, design one focused audit for the
  highest-risk remaining protocol path.
- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0810-ghostboard-preventive-parity-audit/README.md \
    issues/0810-ghostboard-preventive-parity-audit/03-direct-browser-paths.md
  ```

- Whitespace check passes:

  ```bash
  git diff --check
  ```

- A fresh-context completion review approves the completed result before the
  result commit.
- All real completion-review findings are fixed and recorded in this experiment
  file.
- The result commit is made after completion-review approval and before any next
  experiment is designed.

Fail criteria:

- Any Experiment 2 `Maybe` finding is omitted.
- The audit treats direct-browser code existence as equivalent to
  Ghostboard-specific end-to-end proof.
- The audit labels fallback omissions as `Highly likely` without showing normal
  usage depends on the fallback.
- The audit edits application code, generated code, scripts, harnesses, or
  closed historical issues.
- The result makes broad parity claims without file references and evidence.

## Design Review

Fresh-context adversarial design review returned **APPROVED**.

Optional finding:

- Browser chrome/status likely includes console, but Experiment 2 explicitly
  named console capture. The reviewer suggested spelling out
  URL/loading/title/target/console in the pass criteria.

Fix:

- Updated the implementation pass criteria to explicitly include URL, loading
  state, title, hover target, and console capture.

## Result

**Result:** Pass

The Experiment 2 `Maybe` findings were audited against direct Roamium socket
paths, Ghostboard compositor paths, webtui UI state handling, Roamium emission
and receive paths, Wezboard reference behavior, and Issue 809 runtime evidence.
No application code, generated code, scripts, harnesses, or closed issue files
were changed.

### Path Audit Table

| Source                                                                                         | Direct-browser path evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Compositor fallback evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Ghostboard-specific runtime evidence                                                                                                                                                                                                                                                                      | Likelihood      | Risk or impact                                                                                                                                                                                                                | Recommended follow-up                                                                                                                                                                                                                                        |
| ---------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `Navigate`, `UrlChanged`, `LoadingState`, `TitleChanged`, `TargetUrlChanged`, `ConsoleMessage` | `webtui` creates `BrowserConnection` after `BrowserReady` in `webtui/src/main.rs:1123`; the direct reader dispatches URL/loading/title/target/console events in `webtui/src/ipc.rs:502`; URL edit submits use `BrowserConnection::send_navigate` when present in `webtui/src/main.rs:898`; Roamium emits URL/loading/title/target/console through `crate::ipc::send` in `roamium/src/dispatch.rs:529`, `roamium/src/dispatch.rs:552`, `roamium/src/dispatch.rs:603`, `roamium/src/dispatch.rs:640`, and `roamium/src/dispatch.rs:705`. `roamium/src/ipc.rs:33` broadcasts outbound messages to all writers, including direct listener clients. | `webtui` falls back to compositor `send_navigate` when no `BrowserConnection` exists in `webtui/src/main.rs:900`. Ghostboard has no dispatcher cases for `Navigate`, `UrlChanged`, `LoadingState`, `TitleChanged`, `TargetUrlChanged`, or `ConsoleMessage`; unmatched messages are ignored at `ghostboard/src/apprt/termsurf.zig:557`.                                                                                                                                                                | Issue 809 Experiment 23 proved public URL edit navigation through Ghostboard with direct Roamium `Navigate`, Roamium `UrlChanged`, Ghostboard-decoded `UrlChanged`, stable geometry, and post-navigation input. It did not prove loading state, title, hover target, or console capture under Ghostboard. | `Maybe`         | Normal navigation and URL state have runtime evidence, but the broader chrome/status group still lacks Ghostboard-specific regression evidence for loading/title/hover/console, and compositor fallback is ignored.           | Add a focused browser-state regression that loads a page changing title, logging console output, exposing hover targets, and producing loading transitions. Treat compositor fallback as lower priority unless a no-direct-connection use case is confirmed. |
| `JavaScriptDialogRequest`, `JavaScriptDialogReply`, `HttpAuthRequest`, `HttpAuthReply`         | Direct request delivery is supported by Roamium request emission in `roamium/src/dispatch.rs:658` and `roamium/src/dispatch.rs:742`, `webtui` dispatch in `webtui/src/ipc.rs:542` and `webtui/src/ipc.rs:569`, and webtui UI state handling in `webtui/src/main.rs:1153` and `webtui/src/main.rs:1183`. Direct replies are sent through `BrowserConnection` in `webtui/src/ipc.rs:442` and `webtui/src/ipc.rs:451`, and Roamium handles them in `roamium/src/dispatch.rs:393` and `roamium/src/dispatch.rs:416`.                                                                                                                               | `webtui` also sends replies to the compositor in `webtui/src/main.rs:695` and `webtui/src/main.rs:767`, but Ghostboard has no dispatcher cases for dialog/auth request or reply messages; unmatched messages are ignored at `ghostboard/src/apprt/termsurf.zig:557`.                                                                                                                                                                                                                                  | No Issue 809 runtime evidence covers JavaScript dialogs or HTTP auth; Issue 809 README explicitly listed JavaScript dialogs and HTTP authentication dialogs as out of scope.                                                                                                                              | `Maybe`         | Static direct-path evidence is strong, but no Ghostboard-specific runtime regression proves the UI behavior. Compositor fallback appears absent, but normal post-`BrowserReady` behavior should not depend on it.             | Add a focused runtime audit/test for alert, confirm, prompt, beforeunload, and HTTP auth under Ghostboard. If direct path passes, record fallback as a lower-priority resilience issue.                                                                      |
| `RendererCrashed`                                                                              | Roamium emits `RendererCrashed` in `roamium/src/dispatch.rs:572`; `webtui` dispatches it in `webtui/src/ipc.rs:584` and stores crash recovery state in `webtui/src/main.rs:1104`; Roamium broadcasts events to direct clients through `roamium/src/ipc.rs:33`.                                                                                                                                                                                                                                                                                                                                                                                 | Ghostboard has no `RendererCrashed` dispatcher case; unmatched messages are ignored at `ghostboard/src/apprt/termsurf.zig:557`.                                                                                                                                                                                                                                                                                                                                                                       | No Issue 809 runtime evidence covers renderer crash recovery.                                                                                                                                                                                                                                             | `Maybe`         | Static direct-path evidence suggests the normal path should work, but renderer crash recovery is unproven under Ghostboard and fallback is ignored.                                                                           | Add a focused renderer-crash simulation or induced-crash test that verifies webtui receives crash state through `BrowserConnection`.                                                                                                                         |
| `SetColorScheme`                                                                               | Runtime color changes can use `BrowserConnection::send_set_color_scheme` in `webtui/src/ipc.rs:433`; webtui invokes it for dark/light commands in `webtui/src/main.rs:945`; Roamium applies direct `SetColorScheme` messages in `roamium/src/dispatch.rs:442`.                                                                                                                                                                                                                                                                                                                                                                                 | `webtui` also sends compositor `SetColorScheme` in `webtui/src/main.rs:948`, but Ghostboard has no dispatcher case for `SetColorScheme`; unmatched messages are ignored at `ghostboard/src/apprt/termsurf.zig:557`. Ghostboard sends `CreateTab.dark = 0` in `ghostboard/src/apprt/termsurf.zig:1196`; Wezboard also initializes pane `dark` false before later color-scheme updates in `wezboard/wezboard-gui/src/termsurf/conn.rs:771`, so this is not clearly a Ghostboard-only initial-state gap. | No Issue 809 runtime evidence covers color-scheme behavior.                                                                                                                                                                                                                                               | `Maybe`         | Direct runtime changes likely work by code path, but no Ghostboard-specific test proves them. Compositor fallback is ignored. Initial dark state is ambiguous because the reference GUI also starts panes with `dark: false`. | Add a focused color-scheme audit/test for direct post-ready dark/light commands. Defer initial theme parity to historical/config audit unless a reference requirement says initial browser creation must inherit webtui's `is_dark`.                         |
| `CursorChanged`                                                                                | Roamium emits `CursorChanged` in `roamium/src/dispatch.rs:625`, and its IPC layer broadcasts to all writers. However, `webtui` has no `CursorChanged` handling in `webtui/src/ipc.rs:490`, and cursor shape is a GUI responsibility rather than a TUI/browser direct-path responsibility.                                                                                                                                                                                                                                                                                                                                                      | Wezboard stores browser cursor type on the pane in `wezboard/wezboard-gui/src/termsurf/conn.rs:259` and maps it to terminal mouse cursors in `wezboard/wezboard-gui/src/termsurf/input.rs:575`. Ghostboard has no `CursorChanged` dispatcher case; the name only appears in `msgTypeName` at `ghostboard/src/apprt/termsurf.zig:2255`, and unmatched messages are ignored at `ghostboard/src/apprt/termsurf.zig:557`.                                                                                 | No Issue 809 runtime evidence covers browser cursor shape changes over links or text fields.                                                                                                                                                                                                              | `Highly likely` | Browser cursor shape probably remains terminal/default cursor over web content, losing hand/text cursor feedback. Direct webtui connection does not solve this because the GUI must update the visible cursor.                | Open or design a focused follow-up to implement and test Ghostboard `CursorChanged` handling, including link hover and text-field hover.                                                                                                                     |
| `SetGuiActive`                                                                                 | No useful direct-browser substitute exists because GUI activation/deactivation is known by the GUI, not by webtui or Roamium. Roamium can apply `SetGuiActive` in `roamium/src/dispatch.rs:378`.                                                                                                                                                                                                                                                                                                                                                                                                                                               | Wezboard sends `SetGuiActive` on application deactivate/activate in `wezboard/wezboard-gui/src/frontend.rs:320` through `wezboard/wezboard-gui/src/termsurf/conn.rs:643`. Ghostboard has no `SetGuiActive` sender or dispatcher evidence; the message name appears only in `msgTypeName` at `ghostboard/src/apprt/termsurf.zig:2249`.                                                                                                                                                                 | No Issue 809 runtime evidence covers app/window activation signaling to Roamium.                                                                                                                                                                                                                          | `Highly likely` | Roamium may not receive GUI active/inactive state under Ghostboard, affecting renderer focus/activity/throttling semantics when the app activates or deactivates.                                                             | Open or design a focused follow-up to implement and test Ghostboard app/window activation signaling through `SetGuiActive`.                                                                                                                                  |

### Ranked Findings

`Highly likely`:

1. `CursorChanged` handling is likely missing in Ghostboard. The normal visible
   cursor path requires the GUI to consume browser cursor updates; direct webtui
   delivery does not satisfy that requirement.
2. `SetGuiActive` signaling is likely missing in Ghostboard. The GUI is the
   source of app/window activation state, and no direct-browser substitute
   exists.

`Maybe`:

1. Loading/title/hover-target/console chrome-state behavior has direct-path
   static evidence but lacks Ghostboard-specific runtime coverage. URL
   navigation and `UrlChanged` already have Issue 809 runtime evidence.
2. JavaScript dialogs and HTTP auth have strong direct-path static evidence but
   lack Ghostboard-specific runtime coverage.
3. Renderer crash recovery has direct-path static evidence but lacks
   Ghostboard-specific runtime coverage.
4. Color-scheme runtime changes have direct-path static evidence but lack
   Ghostboard-specific runtime coverage; initial browser dark state is ambiguous
   rather than clearly Ghostboard-specific.

`No`:

1. Browser navigation URL submission and `UrlChanged` propagation for the normal
   direct-browser path are covered by Issue 809 Experiment 23.

### Verification

Commands run:

```bash
rg -n "BrowserReady|browser_socket|listen_socket|--listen-socket|buildListenSocket|sendBrowserReady" \
  ghostboard/src/apprt/termsurf.zig roamium/src/main.rs webtui/src/main.rs webtui/src/ipc.rs

rg -n "CursorChanged|SetGuiActive|set_gui_active|cursor_type" \
  wezboard/wezboard-gui/src/termsurf wezboard -g '*.rs'

rg -n "CursorChanged|SetGuiActive|set_gui_active|cursor_type" \
  ghostboard/src/apprt/termsurf.zig ghostboard/src/main_c.zig ghostboard/macos \
  -g '*.zig' -g '*.swift'

rg -n "BrowserReady|url|UrlChanged|LoadingState|TitleChanged|ConsoleMessage|TargetUrlChanged|color|dialog|HttpAuth|RendererCrashed|CursorChanged|SetGuiActive|direct" \
  issues/0809-ghostboard-viewport-geometry -g '*.md'

prettier --write --prose-wrap always --print-width 80 \
  issues/0810-ghostboard-preventive-parity-audit/README.md \
  issues/0810-ghostboard-preventive-parity-audit/03-direct-browser-paths.md

git diff --check
```

## Conclusion

The direct-browser audit reduces the protocol findings to two likely Ghostboard
implementation gaps and several bounded regression-coverage gaps.

The likely implementation gaps are `CursorChanged` and `SetGuiActive`. Unlike
browser chrome or dialog/auth messages, these cannot be satisfied by webtui's
direct Roamium socket: cursor shape and GUI active state are GUI-owned behavior.
Ghostboard appears to name but not implement those messages, while Wezboard has
runtime paths for both.

The browser chrome/status, dialog/auth, renderer crash, and color-scheme
findings should become focused regression/proof work rather than immediate
parity-fix claims. Static direct-path evidence is strong for those groups, but
Issue 810 still needs later follow-up candidates to be ranked by runtime
evidence. The next experiment should begin the historical issue audit unless we
decide to open a separate focused issue for the two `Highly likely` protocol
gaps first.

## Completion Review

Fresh-context adversarial completion review returned **APPROVED**.

Findings: none.

Reviewer checks passed:

- Experiment 3 covers every Experiment 2 `Maybe` finding.
- `Highly likely`, `Maybe`, and `No` labels follow the stated rubric.
- `CursorChanged` and `SetGuiActive` are justified as `Highly likely` and are
  not contradicted by direct-browser paths.
- Browser chrome/status, dialog/auth, renderer crash, and color scheme are kept
  as bounded `Maybe`/coverage questions, not overclaimed fixes.
- README status is `Pass`.
- Only issue docs are changed.
- `git diff --check` passes.
- The result commit had not yet been made.
