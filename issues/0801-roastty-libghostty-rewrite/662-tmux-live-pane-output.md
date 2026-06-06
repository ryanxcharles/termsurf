+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5"
reasoning = "medium"
+++

# Experiment 662: Tmux Live Pane Output

## Description

Experiment 661 completed restoration of every field in upstream Ghostty's
current `receivedPaneState` block. The next tmux viewer gap is live `%output`
notifications: Roastty's control parser already parses them into
`ControlNotification::Output`, but `TmuxViewer` currently drops that
notification in command-queue state.

This experiment replays live pane output into the tracked pane terminal,
matching upstream Ghostty's `receivedOutput`: find the pane by ID, feed the
already-parsed output into that pane's VT stream, return no viewer actions, and
ignore unknown pane IDs.

This experiment works within Roastty's current control parser boundary:
`ControlNotification::Output` stores `data` as a UTF-8 `String`. Upstream
carries pane output as raw bytes. Raw byte and tmux-escaped `%output` parity is
a future parser-level experiment; this slice wires the existing parsed
notification through the viewer.

PTY writes and App integration remain out of scope.

## Changes

- `roastty/src/terminal/tmux.rs`
  - Route `ControlNotification::Output { pane_id, data }` in command-queue state
    to a new live-output handler instead of dropping it.
  - Feed `data.as_bytes()` to the tracked pane terminal with
    `Terminal::next_slice`.
  - Do not consume or mutate the in-flight command queue; live output is not a
    command result.
  - Return no actions for live output.
  - Ignore unknown pane IDs, matching upstream.
  - Treat terminal stream errors as non-fatal for the viewer. Upstream logs live
    output errors and keeps the viewer alive; Roastty can preserve that behavior
    by returning no action and leaving state unchanged.
- Tests in `roastty/src/terminal/tmux.rs`
  - Verify live output writes to the tracked pane's active screen.
  - Verify live output for an unknown pane is ignored without changing tracked
    panes.
  - Verify live output does not consume a pending command or emit the next
    queued command.
  - Verify live output with an empty command queue still emits no actions.
  - Verify live output targets the pane terminal's current active screen,
    including an already-active alternate screen.
  - Keep parser coverage for `%output` as-is; this experiment wires UTF-8
    `ControlNotification::Output` data through the viewer and defers raw byte
    payload parity.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/662-tmux-live-pane-output.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty terminal::tmux`
- `git diff --check`

## Design Review

**Result:** Approved with scope clarification.

Codex confirmed the viewer behavior is compatible with upstream: live `%output`
should feed the tracked pane terminal's current active screen, ignore unknown
panes, return no actions, and avoid consuming the command queue.

Codex found one scope risk: upstream carries output as raw bytes, while
Roastty's current parser stores `%output` data as a UTF-8 `String`. The design
now explicitly limits this experiment to wiring the existing parsed notification
through the viewer and defers raw byte or tmux-escaped payload parity to a
future parser-level experiment. Codex also requested an empty-command-queue
live-output test, which is now part of the plan.

## Result

**Result:** Pass.

`TmuxViewer` now routes `ControlNotification::Output` to a live-output handler
in command-queue state. The handler finds the tracked pane by ID, feeds the
existing UTF-8 output data into that pane terminal with `Terminal::next_slice`,
returns no actions, leaves the command queue untouched, and ignores unknown
panes.

Focused tests cover tracked panes, unknown panes, pending commands, an empty
command queue, and output delivered while the alternate screen is active.

Verification passed:

- `cargo fmt -p roastty`
- `cargo test -p roastty terminal::tmux` — 148 passed, 0 failed

## Conclusion

Roastty now applies live tmux pane output to the pane terminal instead of
dropping it. This matches the approved viewer-scope behavior from upstream
Ghostty while preserving the known parser limitation: raw byte and tmux-escaped
`%output` parity remains a future parser-level slice.

## Completion Review

**Result:** Approved.

Codex found no concrete bugs, regressions, or missing tests blocking the result
commit. The review confirmed that `ControlNotification::Output` is routed in
command-queue state, `received_output` feeds the tracked pane terminal and
returns no actions, the command queue remains untouched, and the tests cover
tracked panes, unknown panes, pending commands, empty queues, and active
alternate-screen routing.

The review also confirmed that the recorded result and conclusion accurately
describe the implemented viewer-scope behavior and the deferred raw byte parser
parity.
