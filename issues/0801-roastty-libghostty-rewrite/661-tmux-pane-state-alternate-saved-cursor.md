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

# Experiment 661: Tmux Pane State Alternate Saved Cursor

## Description

Experiment 660 restored tmux pane-state tab stops. The remaining field in
upstream Ghostty's current `receivedPaneState` restore block is the alternate
screen saved cursor position, carried by `alternate_saved_x` and
`alternate_saved_y`.

Upstream labels this as saved cursor restoration, but the code applies it by
calling `cursorAbsolute` on the alternate screen if that screen already exists.
This experiment should mirror the behavior: set the alternate screen's cursor
position from the parsed 0-based tmux coordinates, without allocating an
alternate screen and without mutating Roastty's richer `ScreenSavedCursor`
snapshot.

Live pane output, PTY writes, and App integration remain out of scope.

## Changes

- `roastty/src/terminal/terminal.rs`
  - Add a narrow tmux-facing helper to apply `alternate_saved_x/y` to the
    alternate screen cursor position.
  - Do nothing if the alternate screen has not been initialized.
  - Treat tmux coordinates as 0-based columns/rows.
  - Check the alternate screen's own bounds before assignment, matching
    upstream's `alt_screen.pages.cols/rows` check.
  - Ignore values that parsed as `usize` but cannot fit in `CellCountInt`, or
    values that fit but are outside the alternate screen bounds. tmux can send
    sentinel-like large values, such as `4294967295`, when no saved cursor
    exists. Values too large to parse as `usize` remain malformed pane-state
    output and keep the existing defunct behavior.
  - Do not call `save_cursor`, `restore_saved_cursor`, or otherwise mutate
    `ScreenSavedCursor`; this is an alternate screen cursor-position restore in
    Roastty's model.
- `roastty/src/terminal/tmux.rs`
  - Call the helper during successful pane-state restoration after the target
    screen cursor state is applied and before the terminal mode/mouse/scroll/tab
    state writes, matching upstream ordering.
  - Preserve existing behavior for malformed pane-state output, stale pane IDs,
    active screen switching, and command-queue continuation.
  - Extend pane-state fixture support so tests can supply explicit
    `alternate_saved_x/y` values while keeping other fields focused.
- Tests in `roastty/src/terminal/tmux.rs`
  - Verify an existing alternate screen receives the parsed alternate saved
    cursor position even when `alternate_on = false`.
  - Verify upstream ordering when `alternate_on = true`: the target-screen
    cursor restore runs first, then `alternate_saved_x/y` wins when both target
    the alternate screen.
  - Verify applying alternate saved cursor position does not switch the active
    screen.
  - Verify missing alternate screens are not initialized by pane-state
    restoration.
  - Verify last-cell coordinates (`cols - 1`, `rows - 1`) are accepted.
  - Verify out-of-bounds and `CellCountInt`-overflowing values are ignored and
    preserve the existing alternate cursor position.
  - Verify `ScreenSavedCursor` is not mutated by saving a distinct alternate
    screen cursor snapshot, applying pane state with different
    `alternate_saved_x/y`, restoring the saved cursor, and checking that the
    original saved snapshot still wins.
  - Verify stale pane IDs do not apply alternate saved cursor changes while a
    later valid pane state line still does.
  - Keep malformed pane-state output and command-queue continuation coverage in
    the tmux pane-state test set.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/661-tmux-pane-state-alternate-saved-cursor.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty terminal::`
- `git diff --check`

## Design Review

**Result:** Not approved on first review.

Codex confirmed the core design is correct: upstream updates the existing
alternate screen cursor position and does not restore a saved-cursor snapshot.
It found missing proof points in the test plan.

The design now requires a test for upstream ordering when `alternate_on = true`,
where target-screen cursor restoration runs first and `alternate_saved_x/y` wins
on the alternate screen. It also requires a behavioral guard proving
`ScreenSavedCursor` remains unchanged, last-cell boundary acceptance, and
ignored `CellCountInt` overflow values such as tmux's `4294967295` sentinel. The
design now explicitly distinguishes those parsed-but-overflowing values from
decimal values too large for `usize`, which remain malformed pane-state output.
Verification broadens to `cargo test -p roastty terminal::` so any terminal-core
saved-cursor guard is covered if it cannot stay in tmux tests.

**Re-review result:** Approved.

Codex confirmed the revised design resolves all prior findings: ordering when
`alternate_on = true`, `ScreenSavedCursor` non-mutation, last-cell acceptance,
`CellCountInt` overflow semantics, and alternate-screen bounds. It noted the
main implementation watchpoint is to place the helper call between the existing
target cursor restore and mode writes, and to use the alternate screen's own
bounds rather than terminal size.

## Result

**Result:** Pass.

Roastty now applies the final field from upstream Ghostty's current
`receivedPaneState` restore block: `alternate_saved_x/y`. The new terminal
helper updates the existing alternate screen's current cursor position using
0-based tmux coordinates, checks the alternate screen's own bounds, accepts the
last cell, and ignores missing alternate screens, out-of-bounds coordinates, and
`CellCountInt`-overflowing sentinel values such as `4294967295`.

The helper does not call `save_cursor`, `restore_saved_cursor`, or mutate
`ScreenSavedCursor`. The tests prove this behaviorally by saving a distinct
alternate-screen cursor snapshot, applying pane state with different
`alternate_saved_x/y`, restoring the saved cursor, and verifying the saved
snapshot still wins.

The tmux restore call is placed after target screen cursor restoration and
before mode, mouse, scroll-region, and tab-stop writes. Tests cover
`alternate_on = false`, the `alternate_on = true` ordering where
`alternate_saved_x/y` wins over target alternate cursor restoration, active
screen preservation, missing alternate screens, last-cell acceptance,
out-of-bounds and overflow ignore behavior, stale pane IDs with a later valid
line, and the saved-cursor snapshot guard.

Verification passed:

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/661-tmux-pane-state-alternate-saved-cursor.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty terminal::` — 2344 passed, 0 failed
- `git diff --check`

## Conclusion

Roastty now restores every field in upstream Ghostty's current tmux
`receivedPaneState` block: target cursor position/shape, alternate saved cursor
position, cursor and terminal modes, mouse modes, focus and bracketed paste,
vertical scroll region, and tab stops. The remaining tmux work is outside
pane-state restoration: live pane output, PTY writes, and App integration.

## Completion Review

**Result:** Approved.

Codex found no issues. It confirmed that the tmux restore path calls
`Terminal::apply_tmux_alternate_saved_cursor_state` immediately after target
cursor restoration and before mode, mouse, scroll-region, and tab-stop writes,
matching upstream ordering. It also confirmed the helper only touches an
existing alternate screen, converts parsed `usize` values through
`CellCountInt`, checks the alternate screen's own `cols()` and `rows()`, and
then updates the current cursor position.

Codex judged the test coverage sufficient for this slice: normal application
with `alternate_on = false`, `alternate_on = true` ordering where
`alternate_saved_x/y` wins, missing alternate screen, last-cell acceptance,
out-of-bounds and overflow ignore behavior, `ScreenSavedCursor` non-mutation,
and stale pane handling with a later valid line. It also confirmed the recorded
result, conclusion, README status, and checklist updates are accurate.
