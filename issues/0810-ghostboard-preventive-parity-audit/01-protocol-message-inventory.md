# Experiment 1: Protocol Message Inventory

## Description

Start the preventive parity audit with the protocol itself.
`proto/termsurf.proto` is the most compact description of the TermSurf behavior
surface: every browser, GUI, and TUI feature that crosses process boundaries is
represented by a protobuf message or message group.

This experiment will not decide final Ghostboard parity for every feature. Its
job is to build the audit backbone:

- enumerate every `TermSurfMessage` variant and concrete protobuf message;
- group messages into logical feature areas;
- infer the feature each message group implies;
- identify the primary reference code paths to inspect in later experiments;
- create the initial audit table schema that later Wezboard/Ghostboard and
  historical-issue passes will fill in.

This is an audit/documentation experiment only. It must not change application
code.

## Changes

Planned files:

- `issues/0810-ghostboard-preventive-parity-audit/01-protocol-message-inventory.md`
  - record the protocol inventory design, review, result, and conclusion;
  - record the initial message-group table after the experiment runs.
- `issues/0810-ghostboard-preventive-parity-audit/README.md`
  - add Experiment 1 to the `## Experiments` index with status `Designed`, then
    update status after the result.

No application code, generated protobuf code, or historical issue files should
be edited.

## Verification

Design-gate pass criteria:

- The issue README links this experiment as `Designed`.
- A fresh-context adversarial design review approves the plan.
- The plan commit exists before implementation begins.

Implementation pass criteria:

- The experiment result enumerates every `TermSurfMessage` oneof variant from
  `proto/termsurf.proto`.
- The concrete message inventory covers all message definitions in
  `proto/termsurf.proto`, including helper reply payloads such as `TabInfo`.
- Every message is assigned to a logical feature group, such as:
  - tab lifecycle and browser process lifecycle;
  - viewport geometry and native layer presentation;
  - navigation and browser chrome state;
  - input forwarding and focus;
  - GUI/TUI handshake and browser discovery;
  - DevTools;
  - split/pane orchestration;
  - dialogs, console capture, HTTP auth, and crash reporting.
- The result records an audit table schema with the required Issue 810 fields:
  source, inferred feature, reference behavior, Ghostboard evidence, likelihood,
  risk or impact, and recommended follow-up.
- The result identifies the next audit slice. Expected next slice: compare the
  protocol message groups against Wezboard and Ghostboard implementation
  evidence.
- Markdown is formatted:

  ```bash
  prettier --write --prose-wrap always --print-width 80 \
    issues/0810-ghostboard-preventive-parity-audit/README.md \
    issues/0810-ghostboard-preventive-parity-audit/01-protocol-message-inventory.md
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

- Any `TermSurfMessage` variant or concrete message definition is omitted.
- The result makes final parity claims without Wezboard/Ghostboard evidence.
- The experiment edits application code.
- The result collapses protocol messages into vague categories that cannot guide
  later audits.

## Design Review

Fresh-context adversarial design review initially returned **CHANGES REQUIRED**.

Required finding:

- The design did not explicitly require the result workflow gate: completed
  result recording, fresh-context completion review, fixing and recording real
  review findings, and result commit before the next experiment.

Fix:

- Added implementation pass criteria requiring completion-review approval,
  recording/fixing real review findings, and committing the result before the
  next experiment is designed.

Re-review verdict: **APPROVED**.

The reviewer confirmed the prior required finding is resolved and no new
required findings were introduced.

## Result

**Result:** Pass

`proto/termsurf.proto` contains:

- 39 `TermSurfMessage` oneof wire variants;
- 41 `message` definitions, including the wrapper `TermSurfMessage` and the
  helper payload `TabInfo`.

The inventory below is the audit backbone for later experiments. It groups every
wire variant into an inferred feature area, records the durable feature
question, and identifies the primary implementation paths to inspect next. It
does not make final Ghostboard parity claims.

### Audit Table Schema

Later audit tables should use these fields:

| Field                 | Meaning                                                              |
| --------------------- | -------------------------------------------------------------------- |
| Source                | Protobuf message, code path, issue number, or document.              |
| Inferred feature      | User-visible or system-visible behavior implied by the source.       |
| Reference behavior    | Wezboard, historical issue, Roamium, webtui, or protocol behavior.   |
| Ghostboard evidence   | Current Ghostboard code, logs, tests, or missing evidence.           |
| Likelihood            | `Highly likely`, `Maybe`, or `No`.                                   |
| Risk or impact        | Why the finding matters if true.                                     |
| Recommended follow-up | Later experiment or issue needed to verify, reject, or fix the item. |

### Protocol Feature Groups

| Group                                     | Wire variants / helper messages                                                                                      | Inferred feature surface                                                                                                                                                          | Primary next evidence paths                                                                                                                                                                                              |
| ----------------------------------------- | -------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Browser tab and process lifecycle         | `CreateTab`, `CreateDevtoolsTab`, `CloseTab`, `ServerRegister`, `TabReady`, `BrowserReady`                           | GUI launches/attaches browser profile servers, creates browser and DevTools tabs for panes, maps browser tab ids to panes, advertises direct browser sockets, and cleans up tabs. | `wezboard/wezboard-gui/src/termsurf/conn.rs`, `ghostboard/src/apprt/termsurf.zig`, `roamium/src/dispatch.rs`, `webtui/src/ipc.rs`, Issue 809 geometry/cleanup evidence.                                                  |
| Viewport geometry and native presentation | `Resize`, `CaContext`, `SetOverlay`, `SetDevtoolsOverlay`                                                            | TUI declares overlay cell geometry; GUI converts it to pixel/native layer geometry; browser reports CALayer context and receives resizes.                                         | Issue 809 final matrix, `wezboard/wezboard-gui/src/termsurf/conn.rs`, `ghostboard/src/apprt/termsurf.zig`, Ghostboard macOS layer bridge files, Roamium resize dispatch.                                                 |
| Navigation and browser chrome state       | `Navigate`, `UrlChanged`, `LoadingState`, `TitleChanged`, `TargetUrlChanged`, `ConsoleMessage`                       | TUI or GUI navigates a tab; browser state updates URL bar, loading/progress UI, title, hover target, and console capture.                                                         | `webtui/src/main.rs`, `webtui/src/ipc.rs`, `wezboard/wezboard-gui/src/termsurf/conn.rs`, `ghostboard/src/apprt/termsurf.zig`, `roamium/src/dispatch.rs`, Issues 809 and historical webtui/chrome issues.                 |
| Input forwarding and focus                | `MouseEvent`, `MouseMove`, `ScrollEvent`, `KeyEvent`, `FocusChanged`, `CursorChanged`, `ModeChanged`, `SetGuiActive` | GUI gates pointer/scroll/keyboard input by active pane/tab/window and browsing mode; browser focus follows mode and app/window activity; browser cursor changes reach the GUI.    | Issue 809 input rows, `wezboard/wezboard-gui/src/termsurf/input.rs`, `wezboard/wezboard-gui/src/termsurf/conn.rs`, `ghostboard/src/apprt/termsurf.zig`, Ghostboard macOS input/cursor bridge, `roamium/src/dispatch.rs`. |
| Appearance and environment state          | `SetColorScheme`                                                                                                     | TUI/GUI/browser propagate dark/light appearance for initial tab creation and later updates.                                                                                       | Wezboard color-scheme handling, Ghostboard `SetColorScheme` routing, Roamium tab color handling, historical theme/config issues.                                                                                         |
| GUI/TUI handshake and discovery           | `HelloRequest`, `HelloReply`, `QueryLastRequest`, `QueryLastReply`, `QueryTabsRequest`, `QueryTabsReply`, `TabInfo`  | TUI discovers GUI defaults, available browser names, previous browser tab for a pane/profile, and current tab inventory.                                                          | `webtui/src/ipc.rs`, `wezboard/wezboard-gui/src/termsurf/conn.rs`, `ghostboard/src/apprt/termsurf.zig`, Roamium query fallback behavior.                                                                                 |
| DevTools orchestration                    | `SetDevtoolsOverlay`, `QueryDevtoolsRequest`, `QueryDevtoolsReply`, `CreateDevtoolsTab`                              | TUI asks for DevTools for an inspected browser tab; GUI creates or finds a DevTools pane/tab and connects it to the browser process.                                              | Issue 809 DevTools row, `webtui/src/ipc.rs`, `wezboard/wezboard-gui/src/termsurf/conn.rs`, `ghostboard/src/apprt/termsurf.zig`, `roamium/src/dispatch.rs`.                                                               |
| Pane/split orchestration                  | `OpenSplit`                                                                                                          | TUI asks GUI to open a terminal split with a command, usually to host DevTools or a related browser workflow.                                                                     | Issue 809 split rows, Issue 808 split work, `webtui/src/ipc.rs`, `wezboard/wezboard-gui/src/termsurf/conn.rs`, `ghostboard/src/apprt/termsurf.zig`, `ghostboard/macos/Sources/App/macOS/AppDelegate+TermSurf.swift`.     |
| Dialogs and browser-interruption flows    | `JavaScriptDialogRequest`, `JavaScriptDialogReply`, `HttpAuthRequest`, `HttpAuthReply`                               | Browser asks TUI/automation to present and answer JavaScript dialogs and HTTP auth prompts; replies return to the browser process.                                                | `webtui/src/main.rs`, `webtui/src/ipc.rs`, `wezboard/wezboard-gui/src/termsurf/conn.rs`, `ghostboard/src/apprt/termsurf.zig`, `roamium/src/dispatch.rs`, historical dialog/auth/PDF automation issues.                   |
| Crash reporting and recovery              | `RendererCrashed`                                                                                                    | Browser process reports renderer failure to the TUI/automation path so the UI can display or offer recovery.                                                                      | `webtui/src/main.rs`, `webtui/src/ipc.rs`, `wezboard/wezboard-gui/src/termsurf/conn.rs`, `ghostboard/src/apprt/termsurf.zig`, `roamium/src/dispatch.rs`.                                                                 |

### Complete Wire Variant Inventory

| #   | Variant                   | Direction / protocol section              | Feature group                               |
| --- | ------------------------- | ----------------------------------------- | ------------------------------------------- |
| 1   | `CreateTab`               | GUI -> Chromium, tab lifecycle            | Browser tab and process lifecycle           |
| 2   | `CreateDevtoolsTab`       | GUI -> Chromium, tab lifecycle            | Browser tab and process lifecycle; DevTools |
| 3   | `Resize`                  | GUI -> Chromium, tab lifecycle / geometry | Viewport geometry and native presentation   |
| 4   | `CloseTab`                | GUI -> Chromium, tab lifecycle            | Browser tab and process lifecycle           |
| 5   | `Navigate`                | GUI -> Chromium, TUI -> GUI               | Navigation and browser chrome state         |
| 6   | `MouseEvent`              | GUI -> Chromium, input                    | Input forwarding and focus                  |
| 7   | `MouseMove`               | GUI -> Chromium, input                    | Input forwarding and focus                  |
| 8   | `ScrollEvent`             | GUI -> Chromium, input                    | Input forwarding and focus                  |
| 9   | `KeyEvent`                | GUI -> Chromium, input                    | Input forwarding and focus                  |
| 10  | `FocusChanged`            | GUI -> Chromium, state                    | Input forwarding and focus                  |
| 11  | `SetColorScheme`          | GUI -> Chromium, TUI -> GUI state         | Appearance and environment state            |
| 12  | `ServerRegister`          | Chromium -> GUI                           | Browser tab and process lifecycle           |
| 13  | `TabReady`                | Chromium -> GUI                           | Browser tab and process lifecycle           |
| 14  | `CaContext`               | Chromium -> GUI                           | Viewport geometry and native presentation   |
| 15  | `UrlChanged`              | Chromium -> GUI                           | Navigation and browser chrome state         |
| 16  | `LoadingState`            | Chromium -> GUI                           | Navigation and browser chrome state         |
| 17  | `TitleChanged`            | Chromium -> GUI                           | Navigation and browser chrome state         |
| 18  | `CursorChanged`           | Chromium -> GUI                           | Input forwarding and focus                  |
| 19  | `SetOverlay`              | TUI -> GUI                                | Viewport geometry and native presentation   |
| 20  | `SetDevtoolsOverlay`      | TUI -> GUI                                | DevTools; viewport geometry                 |
| 21  | `OpenSplit`               | TUI -> GUI                                | Pane/split orchestration                    |
| 22  | `ModeChanged`             | GUI -> TUI                                | Input forwarding and focus                  |
| 23  | `HelloRequest`            | TUI <-> GUI request/reply                 | GUI/TUI handshake and discovery             |
| 24  | `HelloReply`              | TUI <-> GUI request/reply                 | GUI/TUI handshake and discovery             |
| 25  | `QueryLastRequest`        | TUI <-> GUI request/reply                 | GUI/TUI handshake and discovery             |
| 26  | `QueryLastReply`          | TUI <-> GUI request/reply                 | GUI/TUI handshake and discovery             |
| 27  | `QueryDevtoolsRequest`    | TUI <-> GUI request/reply                 | DevTools                                    |
| 28  | `QueryDevtoolsReply`      | TUI <-> GUI request/reply                 | DevTools                                    |
| 29  | `QueryTabsRequest`        | TUI <-> GUI request/reply                 | GUI/TUI handshake and discovery             |
| 30  | `QueryTabsReply`          | TUI <-> GUI request/reply                 | GUI/TUI handshake and discovery             |
| 31  | `BrowserReady`            | GUI -> TUI, direct browser connection     | Browser tab and process lifecycle           |
| 32  | `TargetUrlChanged`        | Chromium -> GUI                           | Navigation and browser chrome state         |
| 33  | `SetGuiActive`            | GUI -> Chromium, state                    | Input forwarding and focus                  |
| 34  | `JavaScriptDialogRequest` | Chromium -> TUI / automation harness      | Dialogs and browser-interruption flows      |
| 35  | `JavaScriptDialogReply`   | TUI / automation harness -> Chromium      | Dialogs and browser-interruption flows      |
| 36  | `ConsoleMessage`          | Chromium -> TUI / automation harness      | Navigation and browser chrome state         |
| 37  | `HttpAuthRequest`         | Chromium -> TUI / automation harness      | Dialogs and browser-interruption flows      |
| 38  | `HttpAuthReply`           | TUI / automation harness -> Chromium      | Dialogs and browser-interruption flows      |
| 39  | `RendererCrashed`         | Chromium -> TUI / automation harness      | Crash reporting and recovery                |

### Complete Message Definition Inventory

| Message                   | In oneof? | Feature group                               |
| ------------------------- | --------- | ------------------------------------------- |
| `TermSurfMessage`         | Wrapper   | Wire envelope                               |
| `CreateTab`               | Yes       | Browser tab and process lifecycle           |
| `CreateDevtoolsTab`       | Yes       | Browser tab and process lifecycle; DevTools |
| `Resize`                  | Yes       | Viewport geometry and native presentation   |
| `CloseTab`                | Yes       | Browser tab and process lifecycle           |
| `Navigate`                | Yes       | Navigation and browser chrome state         |
| `MouseEvent`              | Yes       | Input forwarding and focus                  |
| `MouseMove`               | Yes       | Input forwarding and focus                  |
| `ScrollEvent`             | Yes       | Input forwarding and focus                  |
| `KeyEvent`                | Yes       | Input forwarding and focus                  |
| `FocusChanged`            | Yes       | Input forwarding and focus                  |
| `SetColorScheme`          | Yes       | Appearance and environment state            |
| `SetGuiActive`            | Yes       | Input forwarding and focus                  |
| `ServerRegister`          | Yes       | Browser tab and process lifecycle           |
| `TabReady`                | Yes       | Browser tab and process lifecycle           |
| `CaContext`               | Yes       | Viewport geometry and native presentation   |
| `UrlChanged`              | Yes       | Navigation and browser chrome state         |
| `LoadingState`            | Yes       | Navigation and browser chrome state         |
| `TitleChanged`            | Yes       | Navigation and browser chrome state         |
| `CursorChanged`           | Yes       | Input forwarding and focus                  |
| `TargetUrlChanged`        | Yes       | Navigation and browser chrome state         |
| `JavaScriptDialogRequest` | Yes       | Dialogs and browser-interruption flows      |
| `JavaScriptDialogReply`   | Yes       | Dialogs and browser-interruption flows      |
| `ConsoleMessage`          | Yes       | Navigation and browser chrome state         |
| `HttpAuthRequest`         | Yes       | Dialogs and browser-interruption flows      |
| `HttpAuthReply`           | Yes       | Dialogs and browser-interruption flows      |
| `RendererCrashed`         | Yes       | Crash reporting and recovery                |
| `SetOverlay`              | Yes       | Viewport geometry and native presentation   |
| `SetDevtoolsOverlay`      | Yes       | DevTools; viewport geometry                 |
| `OpenSplit`               | Yes       | Pane/split orchestration                    |
| `ModeChanged`             | Yes       | Input forwarding and focus                  |
| `BrowserReady`            | Yes       | Browser tab and process lifecycle           |
| `HelloRequest`            | Yes       | GUI/TUI handshake and discovery             |
| `HelloReply`              | Yes       | GUI/TUI handshake and discovery             |
| `QueryLastRequest`        | Yes       | GUI/TUI handshake and discovery             |
| `QueryLastReply`          | Yes       | GUI/TUI handshake and discovery             |
| `QueryDevtoolsRequest`    | Yes       | DevTools                                    |
| `QueryDevtoolsReply`      | Yes       | DevTools                                    |
| `QueryTabsRequest`        | Yes       | GUI/TUI handshake and discovery             |
| `TabInfo`                 | Helper    | GUI/TUI handshake and discovery             |
| `QueryTabsReply`          | Yes       | GUI/TUI handshake and discovery             |

### Verification

Commands run:

```bash
awk '
  /oneof msg/ {in_oneof=1; next}
  in_oneof && /^  }/ {in_oneof=0}
  in_oneof && /^[ ]{4}[A-Za-z]/ {count++}
  END {print count}
' proto/termsurf.proto

awk '/^message / {count++} END {print count}' proto/termsurf.proto

prettier --write --prose-wrap always --print-width 80 \
  issues/0810-ghostboard-preventive-parity-audit/README.md \
  issues/0810-ghostboard-preventive-parity-audit/01-protocol-message-inventory.md

git diff --check
```

Results:

- oneof variants: `39`;
- message definitions: `41`;
- markdown formatting passed;
- whitespace check passed.

## Conclusion

The protocol audit backbone is established. The next experiment should compare
these protocol feature groups against Wezboard and Ghostboard implementation
evidence, starting with the high-risk runtime groups: lifecycle, geometry,
navigation/chrome state, input/focus, DevTools, and browser-interruption flows.

## Completion Review

Reviewer: Kierkegaard.

Initial verdict: **CHANGES REQUIRED**.

Required finding:

- `CursorChanged` was present in `proto/termsurf.proto` and the complete wire
  inventory, but missing from the Protocol Feature Groups table's input/focus
  row.

Fix:

- Added `CursorChanged` to the input/focus feature group, added cursor behavior
  to the inferred feature surface, and included the Ghostboard input/cursor
  bridge in the evidence path.

Re-review verdict: **APPROVED**.

The reviewer confirmed the prior finding is resolved and found no new blocking
issues for committing the Experiment 1 result.
