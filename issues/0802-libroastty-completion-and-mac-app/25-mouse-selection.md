+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"
+++

# Experiment 25: Phase C — mouse-drag text selection (deferred Exp-20 probe)

## Description

Exp 20 deferred **mouse selection + clipboard**. Recon shows mouse-drag
selection is **unwired**:

- `Surface::mouse_button` (`lib.rs:3769`) only records the button + calls
  `dispatch_mouse_report`; it never starts a selection.
- `Surface::mouse_pos` (`lib.rs:3754`) only updates the position + dispatches a
  motion report; it never extends a selection.
- The `SelectionGesture` state machine (`terminal/selection_gesture.rs`:
  `press`/`drag`/`release`, driving
  `terminal.drag_select_cells/word/line/output`) is **complete**, but is only
  reachable via the standalone `roastty_selection_gesture_*` FFI — which the
  renamed app does **not** call from its mouse handlers (Swift
  `mouseDragged → mouseMoved → roastty_surface_mouse_pos`).

Upstream handles selection in the **core**: `Surface.zig` owns
`self.mouse.selection_gesture`, and `mouseButtonCallback` (left-press →
`activeLeftClickPin` → start) + `cursorPosCallback` (drag → extend) drive it
(`Surface.zig:1216`, `:1593`, `:4485`). The renamed Roastty app — faithful to
Ghostty — forwards **raw** mouse events
(`mouseDown`→`roastty_surface_mouse_button`,
`mouseDragged`→`roastty_surface_mouse_pos`); so for selection to work in the
faithful app, the core `mouse_button`/`mouse_pos` must drive it, exactly as
upstream's callbacks do. This experiment wires that (cell-drag selection — the
core gesture); double/triple-click word/line and clipboard copy are follow-ups
(Exp 26+).

## Approach

**Own a gesture on the Surface.** Add a `SelectionGesture` field; wire the core
mouse handlers to drive it, faithful to upstream's callbacks:

1. **`mouse_button`, left button:**
   - **Press**, when selection should run (the gate below): convert
     `self.mouse.position` → a terminal **viewport** pin (see step 3), call
     `gesture.press(...)`, apply the returned selection (`set_selection`/clear),
     mark dirty.
   - **Release:** `gesture.release(...)`.
   - **Press while actually mouse-reporting + no shift:** keep dispatching the
     report **and** clear any selection + `gesture.reset(...)` (upstream
     `Surface.zig:3886-3892`, so a report→no-report transition can't resume a
     stale selection).
2. **`mouse_pos`, left button held** (drag): convert the position → viewport
   pin, call `gesture.drag(...)`, apply the returned selection, mark dirty.
   (Autoscroll-on-drag-past-edge is deferred — note it.)
3. **Pixel → VIEWPORT pin helper:** factor the cell mapping (already in
   `mouse_report_geometry` / the report path at `lib.rs:4056-4057`, which maps
   to a **Viewport** grid*ref) into a `position_to_viewport_pin(x, y)`.
   **Viewport, not active** — upstream anchors selection in viewport space
   (`Surface.zig:3924`/`:4629` `pin(.{ .viewport })`), and since Exp 23 made
   rendering viewport-aware, a cell under the cursor while scrolled into
   scrollback is a \_viewport* row; `active_pin` would resolve it to the live
   active region (wrong rows). Use `terminal.viewport_pin(coord)` /
   `grid_ref(Viewport, …)`.
4. **Gate:** drive selection when **not actually mouse-reporting** —
   `mouse_report_context().is_some() == false` (the compound check: the coarse
   `self.mouse_reporting` flag defaults `true`, so the bare flag would disable
   selection entirely; same lesson as Exp 23's scroll gate at `lib.rs:3794`).
   Shift held overrides the report for selection (upstream); the _conditional_
   shift-capture nuance (`if mods.shift and !shift_capture`, `Surface.zig:3882`)
   is **deferred** (note it).
5. **Borrow:** `gesture.press`/`drag` take `&mut Terminal` (via
   `worker.with_termio_mut`) while also needing `&mut self.gesture` — bind/clone
   the worker `Arc` before the closure so `self.gesture` and
   `self.termio_worker` don't conflict.

Cell-behavior (single-click-drag) only this experiment; word/line (double/triple
click) is wired by the same `gesture` but exercised in a follow-up. **Only
`libroastty`** (`lib.rs` mouse handlers + the gesture field + the pixel→pin
helper). No app change (the app already forwards the raw events).

## Verification

1. **Headless regression test (two cases):** drive `mouse_button(Press, Left)`
   at a start cell + `mouse_pos` to an end cell + `mouse_button(Release, Left)`;
   assert an **active selection** (`terminal.active_selection()`,
   `terminal.rs:2022`) whose **selected text** (`screen::selection_string_map`,
   `screen.rs:305`) equals the dragged substring.
   - **(a) unscrolled** — basic drag selects the right substring.
   - **(b) scrolled into scrollback first** (fill past the screen, scroll up,
     then drag): the selection must land on the **scrolled-back** rows and its
     text equal the _history_ substring — this case **distinguishes the viewport
     pin from active** (on a fresh surface viewport==active, so (a) alone passes
     vacuously). Fails pre-wire (no selection) and would fail with an active-pin
     mapping; passes with the viewport pin.
2. **No regression:** `cargo test -p roastty` (full) green — mouse-reporting
   surfaces still get reports (the gate), no selection side-effects when
   mouse-reporting.
3. **Live confirmation** (screen unlocked — check `CGSSessionScreenIsLocked`):
   build a CGEvent **mouse-drag driver** (`scripts/roastty-app/drag.swift`:
   `leftMouseDown` → `leftMouseDragged` steps → `leftMouseUp` at
   `.cghidEventTap`, window-under-cursor like the scroll driver). Launch with
   known text, drag across a word/line, capture — the dragged cells render
   **highlighted** (selection background). App + descendant tree killed (0
   dangling); shots out-of-repo.
4. Faithful to upstream `mouseButtonCallback`/`cursorPosCallback` selection
   (cite).

**Pass** = the gesture is wired into `mouse_button`/`mouse_pos`, the headless
test (drag → correct selected text) passes, the suite is green, and the live app
highlights a dragged selection.

**Partial** = cell-drag selection works + tested, but a sub-aspect (autoscroll
past edge, or the pixel→pin mapping for partial cells) is deferred — documented.

**Fail** = the gesture can't be driven from the core mouse handlers (documented
with the blocker).

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: CHANGES REQUIRED → addressed (re-review below).** It
**confirmed the core-side architecture is faithful** (upstream drives selection
in the core `mouseButtonCallback`/ `cursorPosCallback`; the renamed app forwards
raw events and never calls `roastty_selection_gesture_*` — that FFI is a
separate programmatic ABI). Two Required + three Optional/Nit, folded in:

- **Required — anchor in VIEWPORT, not active.** Upstream uses
  `pin(.{ .viewport })` and roastty's own report path maps to a Viewport ref;
  `active_pin` lands on wrong rows when scrolled (reachable since Exp 23).
  **Fixed:** `position_to_viewport_pin`.
- **Required — the test was vacuous** (fresh surface → viewport==active).
  **Fixed:** added a scroll-into-scrollback-then-drag case asserting the
  _history_ substring (the case that distinguishes viewport from active).
- **Optional — gate must be the compound `mouse_report_context().is_some()`**,
  not the coarse always-true `self.mouse_reporting`. **Fixed.**
- **Optional — shift override is conditional** (`!shift_capture`) +
  **reporting+no-shift press should clear+reset** the selection. **Folded in**
  (shift-capture nuance deferred-with-note; clear+reset added).
- **Nit — borrow:** clone the worker `Arc` before the `with_termio_mut` closure.
  **Noted.**

**Re-review: APPROVED.** Confirmed the viewport-pin path is real + correct
(`terminal.viewport_pin` exists; `SelectionGesturePress/Drag` +
`drag_select_cells` take absolute page-list pins with no viewport/active tag, so
a viewport-resolved pin is sound; `autoscroll_tick` already uses `viewport_pin`
in the scrolled context), the scroll-then-drag test is feasible +
discriminating, and the gate + clear/reset match upstream. Implementation notes
folded in:

- **Borrow:** `TermioWorker` is not `Clone`; use Rust-2021 disjoint borrows —
  bind `let worker = self.termio_worker.as_ref()` and compute the viewport pin
  **inside** the `with_termio_mut` closure from `termio.terminal_mut()` (don't
  call a `&self` method that re-locks), capturing only `self.gesture`.
- **Test text accessor:** `selection_string_map` is `pub(in crate::terminal)`
  (unreachable from lib.rs); assert via
  `terminal.selection_format(TerminalSelectionFormat::Plain, true, false, None)`
  (`terminal.rs:2225`, `pub(crate)`; `None` reads the active selection).
- **Test geometry:** `pos_to_cell` returns `None` when
  `size.width_px/height_px == 0`; the test must `set_size` (nonzero) before
  driving `mouse_pos`.
- **`time_ns: None`** is valid + faithful for cell-drag (the repeat path only
  runs when `click_count > 0`); word/line (Exp 26+) will need a real monotonic
  clock sampled in the Surface (the FFI carries no timestamp).
  Shift-while-reporting selection is out of scope this experiment.

## Result

_(to be added after the run.)_

## Conclusion

_(to be added after the run.)_
