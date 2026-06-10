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

# Experiment 28: Phase C — drag-selection autoscroll past the edge

## Description

Exp 25 wired drag selection, but a drag that goes **past the top/bottom edge**
doesn't scroll to extend the selection into off-screen content — a common,
expected behavior (select more than one screen). The gesture machinery is
complete: `SelectionGesture::drag` already sets `self.autoscroll` (`Up` when the
drag `y <= 1`, `Down` when `y > screen_height - 1`, `selection_gesture.rs`), and
`autoscroll_tick` scrolls the viewport by ±1 and re-drags to the edge cell. But
**nothing calls `autoscroll_tick`** — so the autoscroll state is computed and
never acted on. Upstream drives it on a timer while the button is held past the
edge.

## Approach

1. **`Surface::selection_autoscroll_tick`** (`lib.rs`): when the left button is
   held and the gesture's `autoscroll()` is `Up`/`Down`, compute the **clamped**
   viewport cell at the current mouse position — `geometry.pos_to_cell(pos)`
   directly (it clamps row/col to the grid), **not** `position_to_cell` (which
   returns `None` past the edge via `pos_out_of_viewport`) — plus
   `selection_geometry`, then call `gesture.autoscroll_tick(...)` and apply the
   returned selection, mark dirty. No-op when `autoscroll == None`, no left
   button, **or `mouse_report_context().is_some()`** (symmetry with
   `selection_drag`'s gate — else a program enabling reporting mid-drag leaves a
   retained `autoscroll` scrolling until button-up; the gesture also self-guards
   on `click_count == 0`). **Borrow:** mirror `selection_drag`'s
   read-then-mutate split exactly — read the cell/geometry in one `with_termio`,
   mutate in a separate `with_termio_mut`; **never nest** them
   (`with_termio_mut` is a non-reentrant `Mutex::lock` → self-deadlock on the
   main thread).
2. **Drive it from the present loop.** `start_present_driver` (Exp 19) already
   ticks ~16ms on the main thread (`tick_termio` + `present_live`); add
   `surface.selection_autoscroll_tick()` to that tick **before** the
   `if surface.dirty` present check (so the scrolled row presents the same
   frame, not 16ms late — the tick sets `dirty`). So while a drag is held past
   the edge, the viewport scrolls ~1 row/tick and the selection extends — and
   stops the moment the button releases (the release sets
   `buttons[Left]=Release` → `left_button_pressed()` false, **and**
   `gesture.release` sets `autoscroll = None`) or the mouse returns inside
   (autoscroll → None on the next `drag`).

Faithful to upstream's held-past-edge autoscroll. **Only `libroastty`**
(`lib.rs`: the tick method + the present-driver hook). No app change.

## Verification

1. **Headless regression test:** fill past the screen; drag from a cell **up
   past the top edge** (`mouse_pos` with `y <= 1` so `drag` sets
   `autoscroll = Up`); then call `selection_autoscroll_tick()` a few times
   (simulating the present ticks); assert the **viewport scrolled up into
   history** (a previously-off-screen row is now selected / visible via
   `render_rows_snapshot` or the selection text grew to include history). A
   control: with the mouse **inside** the viewport (no autoscroll), the tick is
   a no-op (selection unchanged). Asserts via
   `active_selection()`/`selection_format`. Fails pre-fix (tick never scrolls —
   there's no tick), passes after. `cargo test -p roastty` (full) green,
   deterministic (no wall-clock dependence — the test calls the tick directly).
2. **No regression:** the present-driver hook is a no-op unless a drag
   autoscroll is active (guarded by `autoscroll()`/left-button/`click_count`),
   so normal rendering + the Exp-25/27 selection tests are unaffected.
3. **Live confirmation** (screen unlocked — check `CGSSessionScreenIsLocked`):
   launch with content past one screen; drag from mid-screen up to the **top
   edge and hold** (a `drag.swift` variant that pauses with the button down at
   the edge); the viewport **auto-scrolls into history** and the selection
   extends. App + descendant tree killed (0 dangling); shots out-of-repo.
4. Faithful to upstream autoscroll (cite).

**Pass** = `autoscroll_tick` is wired (tick method + present-driver hook), the
headless test (drag past edge → viewport scrolls + selection extends) passes,
the suite is green, and the live app auto-scrolls a held past-edge drag.

**Partial** = the tick method + headless test pass, but the live hold-at-edge
can't be driven from the harness (documented; the headless proves the logic).

**Fail** = autoscroll can't be driven from the present loop (documented).

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED.** It traced the full path and **refuted the
runaway-after-release risk** (two independent guards: Release sets
`buttons[Left]=Release` before any branch → `left_button_pressed()` false, and
`gesture.release` sets `autoscroll = None`; the serial `dispatch2` main queue
means no tick interleaves the FFI release), **confirmed the present-driver hook
is safe** (main thread, ~16ms, `platform_tag == 1` only so tests/abi_harness
with tag 0 never start the driver; `tick_termio` returns before the new call —
no nested lock), **the clamped cell is correct** (`pos_to_cell` clamps to row 0
past the top / `rows-1` past the bottom — exactly the edge cell
`autoscroll_tick` re-pins after scrolling; `pos_to_cell` over `position_to_cell`
is right since past-edge is when a cell is still needed), **direction correct**
(Up→`-1`→toward history), and **the test is sound + deterministic**. Three
Optional/Nit, folded in: mirror `selection_drag`'s read-then-mutate split (never
nest `with_termio_mut` → deadlock); add the `mouse_report_context().is_none()`
gate for symmetry; place the tick before the dirty check (present same frame).

## Result

**Result:** Pass — a drag held past the top edge auto-scrolls into history and
extends the selection.

### Change (only `libroastty`)

- **`Surface::selection_autoscroll_tick`** (`lib.rs`): when the left button is
  held + `autoscroll()` is `Up`/`Down` + not mouse-reporting, computes the
  clamped edge cell + geometry (read-then-mutate split, no nested lock) and
  calls `gesture.autoscroll_tick(...)`, applying the returned selection +
  marking dirty.
- **Hooked into the present driver's ~16ms tick** (Exp 19), before the dirty
  check.
- **Discovered necessity:** `selection_drag` was changed to use a new
  **`position_to_cell_clamped`** (`geometry.pos_to_cell`, no out-of-viewport
  check) instead of `position_to_cell` (which returns `None` past the edge).
  Without this, dragging **above** the window — the natural autoscroll gesture —
  never _set_ `autoscroll` (only the exact top-1px in-viewport band would); the
  headless test only passed because `y=0` is in-viewport. With the clamp, a drag
  above/below clamps to the edge cell and sets the autoscroll direction. The
  tick (also clamped) then re-drags to the edge each frame.

### Verification

- **Headless regression test** `drag_autoscroll_scrolls_into_history`
  (`lib.rs`): fill 40 numbered lines; press inside the viewport, drag **above**
  the top edge (`y = -10`) → `autoscroll = Up`; tick 5× → the rendered first-row
  line number **decreases** (viewport scrolled up into history); after
  **release**, further ticks are no-ops (autoscroll stops). Deterministic (calls
  the tick directly).
- **Full `cargo test -p roastty`:** lib **4411 passed**, 0 failures — the
  Exp-25/26/27 selection tests still pass (the clamp is identical to
  `position_to_cell` for in-viewport drags).
- **Live confirmation** (screen unlocked; app + descendant tree killed, 0
  dangling): `seq 1 100`, then `draghold.swift` drags from mid-content **above**
  the top edge and **holds** 1.2s — the viewport **auto-scrolled into history**
  (`e28-after.png`: now showing lines 55–78, was ~77–100) and the **selection
  extended across the whole revealed region** (white highlight over 55–78).
  Out-of-repo.

## Conclusion

Drag-selection autoscroll works: a drag held past the edge scrolls the viewport
one row per present tick and extends the selection, stopping on release —
faithful to upstream's held-past-edge loop. Implementing it surfaced that
`selection_drag` must clamp past-edge positions (now fixed), completing the
drag-selection feature. Mouse selection is now full: cell-drag (incl. past-edge
autoscroll), double-word, triple-line, + clipboard copy. Remaining refinements:
shift-while-reporting override, the reporting clear+reset widening, CJK
ideographic wide-pitch, CVDisplayLink vsync, DPI-change rebuild,
cursor-shaping-hint viewport-gating — then close.

## Result Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED.** It attacked every axis and landed no Required
finding: the test is **load-bearing** (drags `y=-10` above the edge — with the
old `position_to_cell`, `pos_out_of_viewport` returns `None` so `selection_drag`
bails, `autoscroll` stays `None`, the tick no-ops → assert fails; the release
control genuinely proves autoscroll stops); the **clamp is an upstream-fidelity
fix not a regression** (identical to `position_to_cell` in-viewport —
Exp-25/26/27 untouched; the out-of-bounds clamp matches upstream
`renderer/size.zig:142-148` `@max(0)`/`@min(col, cols-1)` and `Surface.zig:4153`
feeds the clamped cell into the gesture unconditionally — roastty's old `None`
was the divergence); **no runaway/deadlock** (two stop guards +
`gesture.release` sets `autoscroll=None`; driver `platform_tag==1`-gated;
read/mutate non-nested, mirroring upstream `selectionScrollTick`); **live
evidence honest** (`e28-before` top=line 78, `e28-after` top=line 55 with the
55-78 region highlighted); full lib **4411 passed, 0 failed**; scope clean
(libroastty + test-only `draghold.swift`, `fmt` clean, no "ghostty" literals).
Non-blocking notes: the one-line present-driver hook is covered only by the live
screenshot (it calls the unit-tested `selection_autoscroll_tick`); a
pre-existing `unused doc comment` warning from Exp 27 — **fixed** here (the
`thread_local!` doc comment → line comment).
