# Experiment 151: Port RIS Full Reset

## Description

Port Ghostty's `ESC c` / RIS full-reset behavior into Roastty for the terminal
state that currently exists.

Ghostty's `Terminal.fullReset()` does three broad things:

- resets the active screen;
- resets terminal-global state such as modes, flags, tabstops, title, pwd, and
  scrolling region;
- returns to the primary screen and removes the alternate screen.

Roastty does not yet have alternate-screen storage, status-display state, or
previous-character state. This experiment should not invent those missing
subsystems. It should port the reset behavior for the state Roastty has now and
document the deferred pieces clearly.

This experiment follows Experiment 150: saved cursor state now exists, so RIS
can clear it as part of screen reset instead of leaving stale saved cursor data
behind.

## Changes

1. Extend stream actions and ESC parsing.
   - Add a `FullReset` action or equivalent.
   - Parse exact `ESC c` / RIS only.
   - Reject intermediate forms such as `ESC # c`.
   - Preserve existing ESC behavior for `ESC 7`, `ESC 8`, `ESC M`, `ESC D`,
     `ESC E`, `ESC H`, `ESC Z`, DCS, OSC, and APC.

2. Add active screen reset support.
   - Expose `PageList::reset()` to the screen layer if needed.
   - Add a `Screen::reset()` helper matching Ghostty's current active-screen
     reset model for state Roastty has:
     - reset visible and scrollback page storage to blank active rows;
     - move cursor to top-left;
     - clear cursor text style;
     - clear cursor visual style to default block;
     - clear cursor protected state;
     - clear cursor hyperlink state;
     - clear semantic prompt state back to normal output;
     - clear pending-wrap state;
     - clear saved cursor state;
     - reset charset state;
     - reset Kitty keyboard state;
     - clear current selection if current selection storage supports it.
   - Do not add Kitty graphics reset behavior; Roastty's Kitty graphics
     subsystem is not ported yet.

3. Add terminal full-reset runtime behavior.
   - On `FullReset`, reset the active screen.
   - Reset terminal mode state with `ModeState::reset()`.
   - Reset terminal flags to default.
   - Reset tabstops to the default interval.
   - Clear title and pwd.
   - Reset DCS handler state and stream parser state only if doing so is needed
     to match Ghostty's RIS behavior from a fully parsed `ESC c`; do not clear
     surrounding bytes or pending parser state that belongs to the caller.
   - Reset scrolling region to full-screen.
   - Mark the active screen dirty enough that a renderer can repaint the blank
     screen. If current dirty tracking cannot represent Ghostty's dirty-clear
     flag directly, mark every visible active cell/row dirty and document that
     mapping in the result.

4. Keep deferred Ghostty fields explicit.
   - Do not add alternate-screen support in this experiment.
   - Do not add status-display state.
   - Do not add previous-character/repeat-print state.
   - Do not add Kitty graphics.
   - Do not add public ABI, renderer, app, PTY, or browser-overlay behavior.

## Verification

1. Run formatting:

   ```bash
   cargo fmt
   ```

2. Run focused tests:

   ```bash
   cargo test -p roastty full_reset
   cargo test -p roastty ris
   ```

3. Run the full Roastty test suite:

   ```bash
   cargo test -p roastty
   ```

Required test coverage:

- Stream parser tests:
  - `ESC c` dispatches full reset.
  - Split-feed `ESC` followed by `c` dispatches full reset.
  - `ESC # c` dispatches nothing and does not leak the final byte.
  - If the handler returns an error for `ESC c`, the parser has already restored
    ground state, so the next byte parses normally.
  - Existing `ESC 7`, `ESC 8`, `ESC M`, `ESC D`, `ESC E`, `ESC H`, and `ESC Z`
    behavior still dispatches as before.
- Screen reset tests:
  - clears visible content and scrollback;
  - moves cursor to `(0, 0)`;
  - clears cursor text style;
  - resets cursor visual style to block;
  - clears cursor protected state;
  - clears cursor hyperlink state;
  - clears semantic prompt state so new cells are normal output, not prompt or
    input cells;
  - clears pending-wrap state;
  - clears saved cursor state by proving `ESC 8` after reset restores Ghostty
    defaults rather than a pre-reset save;
  - resets charset state;
  - resets Kitty keyboard state.
- Terminal full-reset tests:
  - resets modes to defaults, including saved mode state;
  - resets flags such as modify-other-keys state;
  - resets tabstops to default interval;
  - clears title and pwd;
  - resets scrolling region to full-screen;
  - does not write PTY responses;
  - marks the active screen dirty/repaintable according to the dirty strategy
    chosen in Changes step 3.

## Non-Negotiable Invariants

- RIS/full reset must reset only Roastty state that exists today; do not invent
  alternate-screen, status-display, previous-character, or Kitty graphics
  subsystems.
- Full reset must clear saved cursor state.
- Full reset must restore cursor visual style to default block, but saved cursor
  restore must still not include cursor visual style.
- Existing ESC, CSI, OSC, DCS, and APC parsing must not regress.
- Do not add `ghostty_*` names. Use Roastty names except when citing upstream
  Ghostty source paths or behavior.
- Run `cargo fmt` and accept its output.

## Failure Criteria

This experiment fails if:

- `ESC c` is ignored.
- malformed or intermediate RIS-like forms dispatch full reset.
- RIS leaves visible content, scrollback, cursor style, cursor visual style,
  cursor hyperlink, semantic prompt state, saved cursor, charset state, Kitty
  keyboard state, modes, flags, tabstops, title, pwd, or scrolling region stale.
- RIS writes PTY responses.
- RIS leaves the active screen not repaintable under current dirty tracking.
- The patch adds alternate-screen support, status-display state,
  previous-character state, Kitty graphics, public ABI, renderer/app behavior,
  PTY behavior, or browser overlay behavior.

## Design Review

Initial Codex review found two real design issues:

1. The draft omitted semantic prompt reset even though Roastty already has OSC
   133 semantic prompt state and Ghostty's screen reset disables semantic prompt
   state.
2. The draft required mouse shape and implicit hyperlink ID reset without enough
   Ghostty provenance in the cited reset path.

This design was updated to require semantic prompt reset coverage and to remove
mouse-shape and implicit-hyperlink-ID reset from Experiment 151's required
scope. Cursor hyperlink state remains in scope because it is active screen
cursor state and should be cleared with the rest of the active screen cursor.

Follow-up Codex review approved the design with no blocking findings. Codex
noted one non-blocking implementation reminder: the DCS/stream parser reset
bullet is intentionally conditional, and implementation should leave parser
framing alone for a fully parsed `ESC c` unless concrete Roastty state needs
clearing.

## Result

**Result:** Pass

Implemented RIS/full reset for the Roastty terminal state that exists today.

Code changes:

- Added `Action::FullReset` and exact `ESC c` dispatch in the stream parser.
- Kept intermediate forms such as `ESC # c` invalid/ignored.
- Added `Screen::reset()` using the existing `PageList::reset()` machinery.
- Made `PageList::reset()` available to the screen layer and added a helper to
  mark all active rows dirty after reset.
- Reset current screen state: visible content, scrollback, cursor position,
  cursor text style, cursor visual style, protected state, cursor hyperlink
  state, semantic prompt state, pending wrap, saved cursor, charset state, and
  Kitty keyboard state.
- Reset terminal-global state: modes, saved mode state, flags, tabstops, title,
  pwd, DCS handler state, and scrolling region.
- Left parser framing alone for a fully parsed `ESC c`, matching the design
  review guidance.
- Updated the prior unsupported-escape test so it no longer treats `ESC c` as
  unsupported.

Verification:

```bash
cargo fmt
cargo test -p roastty full_reset
cargo test -p roastty ris
cargo test -p roastty
```

Observed results:

- `cargo test -p roastty full_reset`: 3 passed.
- `cargo test -p roastty ris`: 2 passed.
- `cargo test -p roastty`: 1656 unit tests passed, 1 ABI harness test passed, 0
  doc tests.

## Conclusion

Roastty now handles `ESC c` / RIS as a real full reset for the terminal and
screen state currently implemented. The deferred Ghostty reset pieces remain
unchanged: alternate-screen storage, status-display state, previous-character /
repeat-print state, and Kitty graphics are still future subsystem work.

The next experiment should continue with the next missing terminal-control
subsystem that builds on the now-resettable screen and terminal-global state.

## Result Review

Codex reviewed the implementation diff and recorded result after verification.
It reported no findings and approved the result.
