# Experiment 2: Protocol Feature Parity

## Description

Use the protocol feature groups from Experiment 1 as the first concrete parity
audit slice. This experiment compares the current mature Wezboard/TermSurf
behavior against current Ghostboard evidence for every protocol feature group,
then classifies each group as `Highly likely`, `Maybe`, or `No` for Ghostboard
risk.

This experiment should not prove every individual edge case. Its job is to find
the most likely Ghostboard gaps before ordinary app usage finds them, and to
create a ranked evidence table that later experiments or issues can use for
focused verification.

This is an audit/documentation experiment only. It must not change application
code.

## Changes

Planned files:

- `issues/0810-ghostboard-preventive-parity-audit/02-protocol-feature-parity.md`
  - record this experiment design, design review, result, completion review, and
    conclusion;
  - record the protocol feature parity audit table after implementation.
- `issues/0810-ghostboard-preventive-parity-audit/README.md`
  - add Experiment 2 to the `## Experiments` index with status `Designed`, then
    update status after the result.

No application code, generated protobuf code, historical issue files, or closed
issue files should be edited.

## Verification

Design-gate pass criteria:

- The issue README links this experiment as `Designed`.
- A fresh-context adversarial design review approves the plan.
- The plan commit exists before implementation begins.

Implementation pass criteria:

- The result covers every protocol feature group from Experiment 1:
  - browser tab and process lifecycle;
  - viewport geometry and native presentation;
  - navigation and browser chrome state;
  - input forwarding and focus;
  - appearance and environment state;
  - GUI/TUI handshake and discovery;
  - DevTools orchestration;
  - pane/split orchestration;
  - dialogs and browser-interruption flows;
  - crash reporting and recovery.
- For each group, the result records:
  - source messages;
  - inferred feature;
  - Wezboard/reference behavior evidence;
  - current Ghostboard evidence;
  - likelihood: `Highly likely`, `Maybe`, or `No`;
  - risk or impact;
  - recommended follow-up.
- Each audit row justifies its likelihood with this evidence rubric:
  - `Highly likely`: reference behavior exists, but the Ghostboard runtime path
    appears absent, clearly incomplete, parse-only, log-only, or disconnected
    from the required behavior.
  - `Maybe`: Ghostboard has partial, ambiguous, platform-specific, or untested
    runtime evidence, and the audit cannot prove whether the feature works
    without a focused experiment.
  - `No`: Ghostboard has concrete runtime implementation evidence or durable
    test/experiment evidence for the required behavior. Generated protobuf
    structs, message names, or unpack-only code do not qualify by themselves.
- The audit distinguishes generated protobuf support from implemented runtime
  behavior. Generated message structs alone are not enough to classify a feature
  as implemented.
- The result explicitly calls out any group where Ghostboard appears to parse,
  log, or name a message but does not appear to perform the corresponding
  runtime behavior.
- The result preserves uncertainty where evidence is incomplete; it should not
  label an item `Highly likely` unless the implementation evidence supports that
  risk.
- The result identifies the next audit slice. Expected next slice: a focused
  deep dive on the highest-risk `Highly likely` or `Maybe` protocol findings,
  unless the protocol comparison shows the historical issue audit should begin
  first.
- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0810-ghostboard-preventive-parity-audit/README.md \
    issues/0810-ghostboard-preventive-parity-audit/02-protocol-feature-parity.md
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

- Any protocol feature group from Experiment 1 is omitted.
- The result treats generated protobuf types as sufficient runtime parity
  evidence.
- The audit edits application code.
- The result makes vague parity claims without file references or evidence.
- The experiment starts the historical issue audit before completing the
  protocol feature comparison.

## Design Review

Fresh-context adversarial design review initially returned **CHANGES REQUIRED**.

Required finding:

- The design required `Highly likely`, `Maybe`, and `No` labels, but did not
  define an operational evidence rubric for those labels. In particular, it did
  not state what Ghostboard evidence is sufficient for `No`, or what separates
  missing, partial, parse-only, logged-only, and runtime behavior evidence.

Fix:

- Added explicit classification rules requiring every audit row to justify its
  likelihood:
  - `Highly likely` for absent, incomplete, parse-only, log-only, or
    disconnected Ghostboard runtime paths when reference behavior exists;
  - `Maybe` for partial, ambiguous, platform-specific, or untested runtime
    evidence;
  - `No` only for concrete Ghostboard runtime implementation evidence or durable
    test/experiment evidence, excluding generated protobuf structs, message
    names, or unpack-only code by themselves.

Re-review verdict: **APPROVED**.

The reviewer confirmed the prior required finding is resolved and no new
required findings were introduced.

## Result

**Result:** Pass

The protocol feature groups from Experiment 1 were compared against Wezboard,
webtui, Issue 809 evidence, and current Ghostboard runtime paths. The audit did
not edit application code.

### Parity Table

| Source                                                                                                               | Inferred feature                                                                                                           | Reference behavior                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Ghostboard evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               | Likelihood | Risk or impact                                                                                                                                                                                            | Recommended follow-up                                                                                                                                                                                |
| -------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `CreateTab`, `CreateDevtoolsTab`, `CloseTab`, `ServerRegister`, `TabReady`, `BrowserReady`                           | Browser tab and process lifecycle.                                                                                         | Wezboard accepts `SetOverlay`, spawns or reuses a server, sends tab creation messages, maps `TabReady`, and forwards `BrowserReady` to the TUI. `webtui/src/ipc.rs:535` consumes `BrowserReady`.                                                                                                                                                                                                                                                                                                                                | Ghostboard handles `SetOverlay`, `SetDevtoolsOverlay`, `ServerRegister`, and `TabReady` in the dispatcher at `ghostboard/src/apprt/termsurf.zig:536`; `handleTabReady` maps tab ids and `sendBrowserReady` sends the direct browser socket to the TUI at `ghostboard/src/apprt/termsurf.zig:1638` and `ghostboard/src/apprt/termsurf.zig:1686`. Issue 809 proved normal and DevTools tab lifecycle paths through the geometry matrix.                                                                             | `No`       | Lifecycle parity looks implemented for the audited surface. Remaining risk is ordinary regression risk, not a likely missing feature.                                                                     | Keep lifecycle covered by the existing geometry/devtools regression harness. No new follow-up from this audit row.                                                                                   |
| `Resize`, `CaContext`, `SetOverlay`, `SetDevtoolsOverlay`                                                            | Viewport geometry and native presentation.                                                                                 | Wezboard converts overlay cells to pixels, sends `Resize`, consumes `CaContext`, and presents native layers.                                                                                                                                                                                                                                                                                                                                                                                                                    | Ghostboard handles `SetOverlay`, `SetDevtoolsOverlay`, and `CaContext` at `ghostboard/src/apprt/termsurf.zig:536`; `handleCaContext` maps browser tab ids to panes and calls `presentOverlay` through the AppKit bridge at `ghostboard/src/apprt/termsurf.zig:1706`. Issue 809 full matrix passed for initial open, splits, resize, zoom, fullscreen, tab, and DevTools geometry.                                                                                                                                 | `No`       | This is the best-covered Ghostboard protocol surface.                                                                                                                                                     | Keep Issue 809 matrix as the durable regression guard. No new follow-up from this audit row.                                                                                                         |
| `Navigate`, `UrlChanged`, `LoadingState`, `TitleChanged`, `TargetUrlChanged`, `ConsoleMessage`                       | Browser navigation and browser chrome state update the URL bar, loading/progress UI, title, hover target, and console log. | Wezboard explicitly matches `UrlChanged`, `LoadingState`, `TitleChanged`, `Navigate`, and `ConsoleMessage` in `wezboard/wezboard-gui/src/termsurf/conn.rs:193`. Current `webtui` also has an intentional direct-browser path: after `BrowserReady`, it connects to Roamium with `BrowserConnection` in `webtui/src/main.rs:1123`; that direct reader dispatches URL/loading/title/target/console messages in `webtui/src/ipc.rs:502`, and navigation prefers `BrowserConnection::send_navigate` in `webtui/src/main.rs:898`.    | Ghostboard sends `BrowserReady` with the direct browser socket at `ghostboard/src/apprt/termsurf.zig:1686`, so the normal post-ready browser chrome path may bypass Ghostboard. Ghostboard's compositor dispatcher still has no cases for `Navigate`, `UrlChanged`, `LoadingState`, `TitleChanged`, `TargetUrlChanged`, or `ConsoleMessage`; unmatched messages fall into the ignored branch at `ghostboard/src/apprt/termsurf.zig:557`. That is fallback-path risk, not proof the normal direct path is missing. | `Maybe`    | Browser chrome may work through the direct Roamium socket, but fallback navigation/status behavior before or without `BrowserConnection` is unproven and likely ignored by Ghostboard.                    | Focused follow-up should test normal direct-browser chrome/status first, then explicitly force or simulate no direct browser connection to test compositor fallback behavior.                        |
| `MouseEvent`, `MouseMove`, `ScrollEvent`, `KeyEvent`, `FocusChanged`, `CursorChanged`, `ModeChanged`, `SetGuiActive` | Input forwarding, focus, browse mode, GUI active state, and cursor changes.                                                | Wezboard forwards keyboard/mouse/scroll/focus to Chromium in `wezboard/wezboard-gui/src/termsurf/input.rs:92`; it stores cursor type from `CursorChanged` at `wezboard/wezboard-gui/src/termsurf/conn.rs:259`; and it sends `SetGuiActive` from app/window activity in `wezboard/wezboard-gui/src/termsurf/conn.rs:632`.                                                                                                                                                                                                        | Ghostboard has concrete keyboard, mouse, scroll, `ModeChanged`, and `FocusChanged` runtime paths at `ghostboard/src/apprt/termsurf.zig:1362` and `ghostboard/src/apprt/termsurf.zig:1452`, and Issue 809 proved input routing across the geometry matrix. However, the dispatcher does not handle inbound `CursorChanged` or `SetGuiActive`; those names appear only in `msgTypeName` at `ghostboard/src/apprt/termsurf.zig:2249`, and unmatched messages are ignored at `ghostboard/src/apprt/termsurf.zig:557`. | `Maybe`    | Core keyboard/mouse input is covered, but browser cursor shape and GUI active/inactive signaling may be missing or incomplete. This can affect cursor UX and renderer focus/throttling/activity behavior. | Focused follow-up should test cursor changes over links/text fields and app/window activation/deactivation behavior, then split any missing `CursorChanged` or `SetGuiActive` work into a fix issue. |
| `SetColorScheme`                                                                                                     | Appearance and dark/light color-scheme propagation.                                                                        | `webtui/src/ipc.rs:280` sends compositor `SetColorScheme`; direct `BrowserConnection::send_set_color_scheme` exists in `webtui/src/ipc.rs:433`; Wezboard stores the dark flag for the pane in `wezboard/wezboard-gui/src/termsurf/conn.rs:221`.                                                                                                                                                                                                                                                                                 | Ghostboard sends `BrowserReady`, so runtime color-scheme changes may use the direct browser path after webtui connects to Roamium. Ghostboard still has no dispatcher case for compositor `SetColorScheme`; it is only named in `msgTypeName` at `ghostboard/src/apprt/termsurf.zig:2248`, unmatched messages are ignored at `ghostboard/src/apprt/termsurf.zig:557`, and `sendCreateTab` currently sets `dark = 0` at `ghostboard/src/apprt/termsurf.zig:1196`.                                                  | `Maybe`    | Runtime theme changes may work directly, but initial tab dark-state and compositor fallback behavior are ambiguous.                                                                                       | Focused follow-up should test initial dark mode, direct post-ready `webtui` color-scheme commands, and no-direct-connection fallback behavior separately.                                            |
| `HelloRequest`, `HelloReply`, `QueryLastRequest`, `QueryLastReply`, `QueryTabsRequest`, `QueryTabsReply`, `TabInfo`  | TUI discovers GUI defaults, last browser pane, and tab inventory.                                                          | Wezboard replies to hello, last-pane, devtools, and tabs queries in `wezboard/wezboard-gui/src/termsurf/conn.rs:446` and `wezboard/wezboard-gui/src/termsurf/conn.rs:567`. `webtui/src/ipc.rs:162` consumes those replies.                                                                                                                                                                                                                                                                                                      | Ghostboard handles `HelloRequest`, `QueryLastRequest`, `QueryDevtoolsRequest`, and `QueryTabsRequest` in the dispatcher at `ghostboard/src/apprt/termsurf.zig:490`, `ghostboard/src/apprt/termsurf.zig:505`, and `ghostboard/src/apprt/termsurf.zig:523`. Issue 809 DevTools evidence also proved `QueryDevtoolsRequest`/reply in the public DevTools path.                                                                                                                                                       | `No`       | Discovery/query parity appears implemented for the audited request/reply surface.                                                                                                                         | Later historical audit may still find edge cases, but no protocol-level follow-up is needed from this row.                                                                                           |
| `SetDevtoolsOverlay`, `QueryDevtoolsRequest`, `QueryDevtoolsReply`, `CreateDevtoolsTab`                              | DevTools orchestration.                                                                                                    | Wezboard validates DevTools requests, opens a split, handles `SetDevtoolsOverlay`, and sends `CreateDevtoolsTab`.                                                                                                                                                                                                                                                                                                                                                                                                               | Ghostboard handles `QueryDevtoolsRequest` at `ghostboard/src/apprt/termsurf.zig:505`, `SetDevtoolsOverlay` at `ghostboard/src/apprt/termsurf.zig:539`, and sends `CreateDevtoolsTab` from `handleSetDevtoolsOverlay`. Issue 809 Experiment 24 proved public `:devtools right` with `QueryDevtoolsRequest`, `QueryDevtoolsReply`, `OpenSplit`, `SetDevtoolsOverlay`, `CreateDevtoolsTab`, DevTools `CaContext`, geometry, mouse, focus, and keyboard evidence.                                                     | `No`       | DevTools protocol orchestration is covered by recent end-to-end evidence.                                                                                                                                 | Keep the Issue 809 DevTools split geometry scenario as the regression guard.                                                                                                                         |
| `OpenSplit`                                                                                                          | TUI requests a terminal split for a related workflow such as DevTools.                                                     | `webtui/src/ipc.rs:291` sends `OpenSplit`; Wezboard handles it at `wezboard/wezboard-gui/src/termsurf/conn.rs:597`.                                                                                                                                                                                                                                                                                                                                                                                                             | Ghostboard handles `OpenSplit` at `ghostboard/src/apprt/termsurf.zig:554` and calls `termsurf_open_split` in `handleOpenSplit` at `ghostboard/src/apprt/termsurf.zig:1092`. Issue 809 DevTools split evidence proves the public split path.                                                                                                                                                                                                                                                                       | `No`       | Split orchestration looks implemented for the audited protocol path.                                                                                                                                      | Keep DevTools split coverage. No new follow-up from this row.                                                                                                                                        |
| `JavaScriptDialogRequest`, `JavaScriptDialogReply`, `HttpAuthRequest`, `HttpAuthReply`                               | Browser interruption flows route JavaScript dialogs and HTTP auth prompts to webtui and route replies back to the browser. | Wezboard forwards dialog/auth requests to the pane TUI and replies back to the browser in `wezboard/wezboard-gui/src/termsurf/conn.rs:274` and `wezboard/wezboard-gui/src/termsurf/conn.rs:351`. Current `webtui` also handles these through the direct browser connection: request dispatch is in `webtui/src/ipc.rs:547`, and replies prefer `BrowserConnection` in `webtui/src/main.rs:692` and `webtui/src/main.rs:764`. Roamium handles direct replies in `roamium/src/dispatch.rs:393` and `roamium/src/dispatch.rs:416`. | Ghostboard's dispatcher has no cases for JavaScript dialog or HTTP auth request/reply messages; unmatched messages are ignored at `ghostboard/src/apprt/termsurf.zig:557`. That is a compositor fallback gap, but it does not prove the normal direct-browser path is missing after `BrowserReady`.                                                                                                                                                                                                               | `Maybe`    | Dialog/auth flows may work through the direct Roamium socket, but compositor fallback and pre-direct-connection behavior are unproven.                                                                    | Focused follow-up should trigger alert/confirm/prompt/beforeunload and a local HTTP auth challenge under Ghostboard, verifying both direct request/reply delivery and fallback behavior if possible. |
| `RendererCrashed`                                                                                                    | Browser renderer crash reporting and recovery UI.                                                                          | Wezboard forwards `RendererCrashed` to the pane TUI at `wezboard/wezboard-gui/src/termsurf/conn.rs:325`. Roamium emits `RendererCrashed` in `roamium/src/dispatch.rs:572`, and `webtui/src/ipc.rs:584` consumes it through the shared reader path.                                                                                                                                                                                                                                                                              | Ghostboard's dispatcher has no `RendererCrashed` case; unmatched messages are ignored at `ghostboard/src/apprt/termsurf.zig:557`. Because webtui connects directly to Roamium after `BrowserReady`, missing Ghostboard dispatcher handling is only fallback-path evidence.                                                                                                                                                                                                                                        | `Maybe`    | Renderer crash recovery may work over the direct socket, but fallback behavior is unproven and the direct path lacks a current Ghostboard-specific regression result.                                     | Focused follow-up should induce or simulate a renderer crash and verify that webtui receives crash state under the normal Ghostboard direct-browser path.                                            |

### Ranked Findings

`Highly likely`:

1. None from this protocol slice after accounting for webtui's direct
   Roamium/browser socket.

`Maybe`:

1. Browser chrome/status behavior may depend on the direct Roamium socket;
   compositor fallback for navigation URL, loading state, title, hover target,
   and console capture appears unhandled.
2. JavaScript dialog and HTTP auth flows may work over the direct Roamium
   socket, but compositor fallback is unproven.
3. Renderer crash recovery may work over the direct Roamium socket, but lacks a
   Ghostboard-specific regression result.
4. Color-scheme propagation may work after `BrowserReady`, but initial
   `CreateTab.dark` and compositor fallback behavior are ambiguous.
5. Cursor shape updates and `SetGuiActive` app/window activity signaling may be
   missing even though keyboard, mouse, scroll, mode, and focus routing are
   covered.

`No`:

1. Browser/process lifecycle, viewport geometry, GUI/TUI discovery, DevTools,
   and `OpenSplit` have current runtime and/or Issue 809 evidence.

### Verification

Commands run:

```bash
rg -n "UrlChanged|LoadingState|TitleChanged|CursorChanged|TargetUrlChanged|ConsoleMessage|JavaScriptDialog|HttpAuth|RendererCrashed|SetColorScheme|FocusChanged|SetGuiActive|Navigate" \
  ghostboard/src/apprt/termsurf.zig ghostboard/macos ghostboard/src \
  -g '*.zig' -g '*.swift'

rg -n "Msg::UrlChanged|Msg::LoadingState|Msg::TitleChanged|Msg::CursorChanged|Msg::JavascriptDialogRequest|Msg::ConsoleMessage|Msg::RendererCrashed|Msg::HttpAuthRequest|Msg::Navigate|Msg::SetColorScheme|Msg::OpenSplit|Msg::SetDevtoolsOverlay|Msg::QueryTabsRequest|Msg::QueryLastRequest" \
  wezboard/wezboard-gui/src/termsurf/conn.rs

rg -n "UrlChanged|LoadingState|TitleChanged|TargetUrlChanged|ConsoleMessage|RendererCrashed|JavaScriptDialogRequest|HttpAuthRequest|send_navigate|send_set_color_scheme|BrowserReady" \
  webtui/src/main.rs webtui/src/ipc.rs

prettier --write --prose-wrap always --print-width 80 \
  issues/0810-ghostboard-preventive-parity-audit/README.md \
  issues/0810-ghostboard-preventive-parity-audit/02-protocol-feature-parity.md

git diff --check
```

## Conclusion

The protocol feature comparison found that Ghostboard's recently tested runtime
surface is strong for lifecycle, geometry, input, discovery, DevTools, and split
orchestration. The important correction from completion review is that current
`webtui` intentionally connects directly to Roamium after `BrowserReady`, so
missing Ghostboard dispatcher cases do not by themselves prove missing browser
chrome, dialog/auth, console, or crash behavior.

The next experiment should deep-dive the direct-browser paths and their fallback
behavior. It should first prove the normal Ghostboard runtime path for
URL/loading/title/target/console, dialog/auth, crash reporting, and color scheme
over `BrowserConnection`, then separately decide whether the compositor fallback
gaps matter enough to fix.

## Completion Review

Reviewer: Russell.

Initial verdict: **CHANGES REQUIRED**.

Required findings:

- The browser chrome/status row treated absent Ghostboard dispatcher handling as
  proof of missing behavior, but current `webtui` connects directly to Roamium
  after `BrowserReady` and uses that reader for URL/loading/title/target/console
  messages.
- The dialog/auth row had the same path error: current `webtui` can send replies
  through `BrowserConnection`, and Roamium handles those replies directly.
- The renderer-crash row was unsupported for the same reason: Roamium emits
  `RendererCrashed`, and `webtui` consumes it through the shared direct-reader
  path.
- The conclusion incorrectly identified Roamium-to-Ghostboard-to-webtui
  forwarding as the highest-risk gap even though current runtime intentionally
  provides a direct Roamium-to-webtui socket.

Fix:

- Reclassified browser chrome/status, dialog/auth, renderer crash, and color
  scheme findings as `Maybe` direct-path/fallback questions instead of
  `Highly likely` Ghostboard dispatcher gaps.
- Rewrote the ranked findings so this protocol slice has no `Highly likely`
  items after accounting for `BrowserConnection`.
- Rewrote the conclusion to focus the next experiment on proving direct-browser
  behavior first, then separately evaluating compositor fallback gaps.

Re-review verdict: **APPROVED**.
