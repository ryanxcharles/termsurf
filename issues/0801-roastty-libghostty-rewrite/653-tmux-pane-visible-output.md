# Experiment 653: Tmux Pane Visible Output

## Description

The tmux viewer now owns a `Terminal` for each tracked pane and queues
`capture-pane` commands for the primary and alternate visible regions, but the
command output is still ignored. This experiment applies `PaneVisible` command
output to the matching pane terminal so Roastty starts reconstructing pane
screen contents from tmux control mode.

Upstream Ghostty's `Viewer.receivedPaneVisible` switches the pane terminal to
the requested screen, clears the active area, homes the cursor, and then streams
the captured bytes through the terminal parser. Roastty should match that shape
without implementing history capture, pane state restoration, live pane output,
PTY startup, or App integration in this experiment.

## Changes

- `roastty/src/terminal/terminal.rs`
  - Add a narrow tmux-facing helper that switches a terminal to a requested
    primary or alternate screen using the existing `switch_screen` path.
  - Add a narrow tmux-facing helper that clears the active display and homes the
    cursor before capture content is replayed. The helper should use a complete
    erase with protected cells disabled and move to tmux/VT position `1,1`,
    matching upstream visible capture setup.
- `roastty/src/terminal/tmux.rs`
  - Route `TmuxCommand::PaneVisible` command output to the tracked
    `TmuxPane.terminal`.
  - Preserve command-queue behavior: consuming a `PaneVisible` response must
    still emit the next queued command when one is pending, so pane bootstrap
    can continue through primary history, primary visible, alternate history,
    alternate visible, and pane state.
  - Ignore visible output for unknown panes, matching upstream's non-fatal
    behavior for stale pane IDs.
  - Treat terminal replay or screen setup failures as viewer-defunct conditions,
    because the viewer can no longer trust its reconstructed pane state.
  - Keep `PaneHistory`, `PaneState`, live output, PTY, and App integration as
    no-ops/future work.
- Tests in `roastty/src/terminal/tmux.rs`
  - Verify primary visible capture clears stale content, homes the cursor, and
    replays into the primary screen.
  - Verify alternate visible capture writes to the alternate screen without
    polluting primary content.
  - Verify consuming `PaneVisible` emits the next queued command when the queue
    still has work.
  - Verify stale pane IDs are consumed without defuncting the viewer or changing
    tracked panes.
  - Verify a terminal replay failure defuncts the viewer if a practical
    malformed input fixture exists; otherwise document why this path is not
    directly fixture-tested.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/653-tmux-pane-visible-output.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty terminal::tmux`
- `git diff --check`

## Design Review

**Result:** Not approved on first review.

Codex found two blockers: the original design did not explicitly require
`PaneVisible` to preserve command-queue continuation by emitting the next queued
command after consuming visible output, and the verification list omitted the
required write-mode `cargo fmt -p roastty` step. Both findings were valid and
the design was revised to include them.

**Re-review result:** Approved.

Codex confirmed the blockers were resolved, the scope remained narrow, and the
design matched upstream `receivedPaneVisible`: stale pane IDs are non-fatal, and
tracked panes switch screen, complete-erase with protected cells disabled, home
to `1,1`, and stream content through the terminal parser.

## Result

**Result:** Pass.

Roastty now applies `PaneVisible` command output to the tracked pane terminal.
The handler switches the pane terminal to the requested primary or alternate
screen, clears the active display with protected cells disabled, homes the
cursor to the top-left, and replays the captured bytes through
`Terminal::next_slice`.

Unknown pane IDs remain non-fatal and simply consume the command output,
matching upstream's stale-pane behavior. Command-queue continuation is
preserved: consuming a visible capture still emits the next queued command when
one is pending.

No separate malformed replay fixture was added. `Terminal::next_slice` accepts
arbitrary byte slices through the normal parser path, and the focused testable
contract for this slice is successful setup/replay plus queue continuation and
stale-pane handling.

Verification passed:

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/653-tmux-pane-visible-output.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty terminal::tmux` — 105 passed, 0 failed
- `git diff --check`

## Conclusion

Pane terminal reconstruction now has the visible-region half of bootstrap wired.
The next tmux experiment should handle `PaneHistory` output so historical
scrollback can be replayed without leaving copied history in the active area.

## Completion Review

**Result:** Approved.

Codex found no blocking issues. It confirmed the implementation matches the
approved visible capture behavior: unknown panes are non-fatal, tracked panes
switch to the requested screen, complete-erase with protected cells disabled,
home to `1,1`, and replay through `Terminal::next_slice`. It also confirmed
command-queue continuation is preserved and covered by the new test.

The only notes were non-blocking: the test-only pane screen accessor mutates the
active screen while inspecting it, and a single combined helper may be useful if
the tmux capture setup sequence is reused later.
