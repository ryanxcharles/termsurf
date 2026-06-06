# Experiment 657: Tmux Pane State Modes

## Description

Experiment 656 started applying parsed pane state by restoring cursor position
and cursor shape. Upstream Ghostty's next pane-state section restores cursor
visibility/blinking and core terminal modes before mouse modes, scroll region,
and tab stops.

This experiment applies the next non-mouse state subset to tracked pane
terminals: cursor visible, cursor blinking, insert, wraparound, keypad keys,
cursor keys, origin, focus events, and bracketed paste. It should keep
mouse-event/mouse-format modes, scroll region, tab stops, and alternate saved
cursor restoration for later experiments.

## Changes

- `roastty/src/terminal/terminal.rs`
  - Add a narrow tmux-facing helper to apply cursor and core terminal mode
    booleans to the pane terminal's mode state.
  - Assign directly to `ModeState`, matching upstream's `t.modes.set(...)`
    restoration path. Do not run through normal terminal mode execution paths
    that perform side effects; in particular, restoring `origin_flag` must not
    move the cursor after Experiment 656 restored it.
  - Apply only these fields from `TmuxPaneState`: `cursor_flag`,
    `cursor_blinking`, `insert_flag`, `wrap_flag`, `keypad_flag`,
    `keypad_cursor_flag`, `origin_flag`, `focus_flag`, and `bracketed_paste`.
- `roastty/src/terminal/tmux.rs`
  - Extend successful `PaneState` handling to apply this mode subset after the
    cursor subset from Experiment 656.
  - Keep stale pane IDs ignored, malformed state output defuncting, and
    command-queue continuation unchanged.
- Tests in `roastty/src/terminal/tmux.rs`
  - Verify a pane state line sets every included mode to the parsed boolean
    values, including both true and false values overriding defaults.
  - Verify `origin_flag = true` sets origin mode without changing the cursor
    position restored by Experiment 656.
  - Verify stale pane IDs are ignored while a later valid line still applies
    modes.
  - Verify malformed state output still defuncts the viewer.
  - Verify successful pane state handling still emits the next queued command.
  - Verify at least one representative mouse event mode and one mouse format
    mode remain unchanged when pane state contains mouse flags; scroll region,
    tab stops, and alternate saved cursor remain out of scope for this
    experiment.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/657-tmux-pane-state-modes.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty terminal::tmux`
- `git diff --check`

## Design Review

**Result:** Not approved on first review.

Codex found one blocker: pane-state mode restoration must assign directly to
`ModeState`, matching upstream's `t.modes.set(...)`, rather than using normal
terminal mode execution paths that perform side effects. This matters for
`origin_flag`, because Roastty's normal origin-mode handling moves the cursor
home and would undo the cursor position restored by Experiment 656. The design
was revised to require direct assignment and a cursor-preservation test for
`origin_flag = true`.

The review also suggested strengthening verification for both set and clear
directions and checking representative out-of-scope mouse modes. Those
suggestions were added.

**Re-review result:** Approved.

Codex confirmed the previous blocker was resolved and the design now matches
upstream's direct mode-restoration behavior. It suggested keeping the terminal
helper narrow by passing explicit booleans rather than making terminal core
depend on `TmuxPaneState`; the implementation will follow that boundary.
