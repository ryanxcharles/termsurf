+++
[implementer]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 150: Port ESC Cursor State Controls

## Description

Port the next coherent ESC-level cursor-control slice from Ghostty into Roastty:

- `ESC 7` / DECSC save cursor;
- `ESC 8` / DECRC restore cursor;
- `ESC M` / RI reverse index.

These controls share the same parser/runtime boundary and are small enough to
verify precisely, but they are not a one-byte micro-slice: together they add the
remaining basic ESC cursor-state behavior needed before broader reset and
alternate-screen work.

Ghostty's save/restore cursor behavior stores more than x/y position. It saves
cursor position, text style, protected pen state, pending-wrap state, origin
mode, and charset state. Roastty already has cursor text style, protected state,
pending-wrap state, origin mode, and charset state, so this experiment should
store and restore those fields now. Cursor visual style from Experiment 149 is
not part of Ghostty's saved cursor payload and must not be saved/restored here.

Ghostty's reverse index moves the cursor up one row unless the cursor is on the
top row of the active scrolling region and inside the horizontal margins. In
that top-margin case it scrolls the region down by one row.

`ESC c` / RIS full reset is intentionally out of scope. RIS touches screen
memory reset, modes reset, flags, tabstops, title, pwd, alternate-screen state,
and dirty handling. It deserves its own experiment after this cursor-state slice
lands.

## Changes

1. Extend stream actions and ESC parsing.
   - Add actions for `SaveCursor`, `RestoreCursor`, and `ReverseIndex`.
   - Parse exact ESC forms only:
     - `ESC 7` -> save cursor;
     - `ESC 8` -> restore cursor;
     - `ESC M` -> reverse index.
   - Reject intermediate forms such as `ESC # 7`, `ESC # 8`, and `ESC # M`.
   - Preserve existing ESC behavior for `ESC D` index, `ESC E` next line,
     `ESC H` tab set, `ESC Z` primary device attributes, DCS, OSC, and APC.

2. Add saved cursor state.
   - Add a saved cursor struct that stores:
     - x/y cursor position;
     - cursor text `style::Style`;
     - cursor protected pen state;
     - pending-wrap state;
     - `Mode::Origin`;
     - screen charset state.
   - Store it beside the active screen state, matching Ghostty's per-screen
     model as closely as Roastty's current single-screen implementation allows.
   - Add internal screen helpers to snapshot and restore the cursor/charset
     fields. Clamp restored x/y to the current terminal size.
   - Do not store cursor visual style. Ghostty's saved cursor payload stores
     cursor text style, not cursor shape.
   - Do not store cursor hyperlink state or semantic-prompt state; Ghostty does
     not include those in `SavedCursor`.

3. Wire terminal runtime behavior.
   - `SaveCursor` records the current saved cursor snapshot.
   - `RestoreCursor` restores the saved snapshot, or restores Ghostty's default
     snapshot if no save exists:
     - x/y = 0/0;
     - default text style;
     - protected = false;
     - pending_wrap = false;
     - origin mode = false;
     - default charset state.
   - Restoring origin mode should update `Mode::Origin` directly. It must not
     run the normal origin-mode set handler because Ghostty restores the saved
     cursor position after setting the mode.
   - Restore must not write PTY responses or mutate visible cell contents.

4. Wire reverse index runtime behavior.
   - If the cursor is on the scrolling region's top row and inside the
     horizontal margins, call the existing scroll-down-by-one path.
   - Otherwise move the cursor up by one row using existing cursor movement
     semantics.
   - Preserve Ghostty's behavior that the top-margin scroll path uses the
     current scrolling region and horizontal margins.

5. Keep scope narrow.
   - Do not implement RIS/full reset.
   - Do not implement alternate-screen save-cursor behavior.
   - Do not implement DECALN or protected-mode ESC helpers.
   - Do not add public ABI, renderer, app, PTY, or browser-overlay behavior.

## Verification

1. Run formatting:

   ```bash
   cargo fmt
   ```

2. Run focused tests:

   ```bash
   cargo test -p roastty save_cursor
   cargo test -p roastty restore_cursor
   cargo test -p roastty reverse_index
   ```

3. Run the full Roastty test suite:

   ```bash
   cargo test -p roastty
   ```

Required test coverage:

- Stream parser tests:
  - `ESC 7`, `ESC 8`, and `ESC M` dispatch the new actions.
  - Split-feed forms preserve parser state.
  - Intermediate forms such as `ESC # 7`, `ESC # 8`, and `ESC # M` dispatch
    nothing and do not leak final bytes.
  - If the handler returns an error for `ESC 7`, `ESC 8`, or `ESC M`, the parser
    has already restored ground state, so the next byte parses normally after
    the caller retries or continues.
  - Existing `ESC D`, `ESC E`, `ESC H`, and `ESC Z` behavior still dispatches as
    before.
- Terminal save/restore tests:
  - save/restore round-trips cursor x/y position.
  - save/restore round-trips cursor text style.
  - save/restore round-trips cursor protected state.
  - save/restore round-trips pending-wrap state.
  - save/restore round-trips origin mode without moving to origin as a side
    effect.
  - save/restore round-trips charset state using formatter charset extras.
  - restore without prior save restores Ghostty defaults.
  - restored x/y clamps to the current terminal size if the terminal was resized
    between save and restore, if current resize helpers support this test.
  - cursor visual style is not restored by saved cursor state.
  - cursor hyperlink state and semantic-prompt state are not restored.
  - save/restore does not mutate visible cells, dirty rows, or PTY responses.
- Reverse-index tests:
  - outside the top row of the scrolling region, `ESC M` moves the cursor up one
    row.
  - at row 0, outside a constrained scrolling region, `ESC M` clamps at the top
    without scrolling.
  - at the top row of the scrolling region and inside horizontal margins,
    `ESC M` scrolls the region down by one row.
  - at the top row but outside horizontal margins, `ESC M` moves/clamps the
    cursor instead of scrolling.
  - reverse index preserves existing scroll-down dirty-row behavior.

## Non-Negotiable Invariants

- Saved cursor state must not conflate cursor text style with cursor visual
  style.
- Saved cursor restore must not invoke the ordinary origin-mode set handler in a
  way that loses the restored cursor position.
- Reverse index must use the current scrolling region and horizontal margins.
- Existing ESC `D`, `E`, `H`, `Z`, DCS, OSC, and APC behavior must not regress.
- Do not add `ghostty_*` names. Use Roastty names except when citing upstream
  Ghostty source paths or behavior.
- Run `cargo fmt` and accept its output.

## Failure Criteria

This experiment fails if:

- `ESC 7`, `ESC 8`, or `ESC M` are ignored.
- Intermediate or malformed ESC forms dispatch the new actions.
- Saved cursor restore fails to restore text style, protected state,
  pending-wrap state, origin mode, or charset state.
- Saved cursor restore incorrectly restores cursor visual style, hyperlink
  state, or semantic-prompt state.
- Restore without a prior save does not restore Ghostty defaults.
- Reverse index scrolls outside the scrolling region top-row/horizontal-margin
  condition.
- Reverse index fails to scroll down inside the top-row/horizontal-margin
  condition.
- The patch implements RIS/full reset, alternate-screen behavior, DECALN,
  protected-mode ESC helpers, public ABI, renderer/app behavior, PTY behavior,
  or browser overlay behavior.

## Design Review

Codex reviewed the initial design and approved the slice choice, Ghostty
behavior model, scope boundary, and verification plan. It requested one
test-strengthening addition: parser error-recovery coverage for handler errors
on the new `ESC 7`, `ESC 8`, and `ESC M` actions, matching the rigor of the
surrounding stream tests. The design was updated with that requirement.

Codex reviewed the revised design and approved it with no findings. It confirmed
that the updated verification plan is strong enough to prove the result and that
the experiment is ready for implementation.

## Result

**Result:** Pass

Experiment 150 ported the ESC cursor-state controls:

- `ESC 7` dispatches and executes save cursor.
- `ESC 8` dispatches and executes restore cursor.
- `ESC M` dispatches and executes reverse index.

Implemented changes:

- Added `SaveCursor`, `RestoreCursor`, and `ReverseIndex` stream actions.
- Wired exact ESC parsing for `ESC 7`, `ESC 8`, and `ESC M`.
- Kept intermediate forms such as `ESC # 7`, `ESC # 8`, and `ESC # M` inert.
- Added saved cursor state to `Screen`, including x/y position, cursor text
  style, protected pen state, pending-wrap state, origin mode, and charset
  state.
- Restored unsaved cursors to Ghostty defaults.
- Preserved the design distinction from Experiment 149: saved cursor state does
  not restore cursor visual style.
- Preserved Ghostty's exclusion of cursor hyperlink state and semantic-prompt
  state from saved cursor restore.
- Added reverse-index runtime behavior using the current scrolling region and
  horizontal margins.

Verification commands:

```bash
cargo fmt
cargo test -p roastty save_cursor
cargo test -p roastty restore_cursor
cargo test -p roastty reverse_index
cargo test -p roastty
```

Verification results:

- `cargo test -p roastty save_cursor`: 4 passed, 0 failed.
- `cargo test -p roastty restore_cursor`: 5 passed, 0 failed.
- `cargo test -p roastty reverse_index`: 8 passed, 0 failed.
- `cargo test -p roastty`: 1651 unit tests passed, 1 ABI harness test passed, 0
  doc tests.

## Conclusion

Roastty now supports Ghostty's basic ESC cursor-state controls. Save/restore
cursor now covers the state Ghostty stores, excludes the state Ghostty excludes,
and avoids the origin-mode restore trap by setting origin mode directly before
restoring the saved cursor position. Reverse index now matches Ghostty's
scrolling-region rule: scroll down only at the top row of the active region when
the cursor is inside the horizontal margins; otherwise move/clamp the cursor up.

The next experiment should continue with another coherent terminal-control
slice. RIS/full reset remains deliberately unimplemented and is a good candidate
for a future focused reset-state experiment once the surrounding reset
preconditions are clear.

## Result Review

Codex reviewed the completed implementation and initially found one result/code
alignment issue: the result text said restore sets origin mode before restoring
cursor position, while the first implementation restored cursor position first.
The implementation was changed to match the design: it now obtains the saved
cursor/default snapshot, sets `Mode::Origin` directly, then restores the saved
cursor fields and position.

After the fix, Codex re-reviewed the implementation, result record, and
verification summary. It reported no findings and confirmed that parser
behavior, handler-error recovery, saved/restored state, excluded state,
reverse-index margin behavior, and verification results all line up.
