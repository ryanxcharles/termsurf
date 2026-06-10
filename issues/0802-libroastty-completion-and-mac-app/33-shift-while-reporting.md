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

# Experiment 33: Phase C — shift overrides mouse-reporting for selection

## Description

When a TUI enables mouse-reporting, the mouse goes to the program — but
**holding shift lets the user select text** anyway (the standard override).
roastty currently routes _all_ reporting button/motion events to reports +
selection-clear (Exp 32), so **shift-drag-select inside a mouse-mode TUI doesn't
work**. Upstream (`Surface.zig:3882`, `mouseShiftCapture`) overrides the report
when `mods.shift and !shift_capture`: the selection runs and **no report is
sent**.

`shift_capture` (`mouseShiftCapture`) is: config `mouse-shift-capture`
(`Never`→false, `Always`→true) **OR** the terminal's `XTSHIFTESCAPE` flag
(`CSI > 1 s` → `terminal.flags.mouse_shift_capture`, `Some(true/false)`) **OR**
the config default (`.false`/`.true`). roastty already models the **terminal
flag** (`Action::MouseShiftCapture`, `terminal.rs:2933`) but does **not**
surface the **config** value to the `Surface` (the App copies only some config
fields).

## Approach

Implement `shift_capture` **flag-first**, deferring the config-plumbing nuance
(a documented sub-deferral — the config isn't surfaced to the Surface, and
adding it is a separate App-plumbing change):

1. **`Terminal::mouse_shift_capture_flag() -> Option<bool>`** (`pub(crate)` —
   the existing `mouse_shift_capture_for_tests` is `#[cfg(test)] pub(super)`,
   unreachable from `lib.rs`; add a new `pub(crate)` accessor wrapping
   `self.flags.mouse_shift_capture`).
2. **`Surface::mouse_shift_capture() -> bool`**:
   `match flag { Some(v) => v, None => false }` — i.e. the **default-config
   (`.false`)** behavior: a terminal `CSI > 1 s` toggles capture, otherwise
   shift is **not** captured (so shift overrides). _(Config
   `Never`/`Always`/`True` is deferred — noted.)_
3. **`mouse_button`** (restructure Exp 32's branch): compute
   `shift_override = reporting && self.mouse.mods.shift && !self.mouse_shift_capture()`.
   - `if !reporting || shift_override` → the selection path (Left:
     press/drag-extend/release) — so shift+click/drag while reporting selects.
   - `else` (reporting, no override) → `selection_clear_and_reset()` (Exp 32,
     unchanged).
   - Suppress the report **and return not-consumed** on override (upstream
     returns `false` after the override path, `Surface.zig:4124`):
     `if shift_override { false } else { self.dispatch_mouse_report(...) }`.
4. **`mouse_pos`**: extend the drag gate to allow the override, and suppress the
   motion report **only when a button is held** (upstream
   `Surface.zig:4586-4607` gates the override-`break :report` on a pressed
   button via a `state != .release` loop — "so movement reports are not
   affected"; bare shift-motion in any-event mode 1003 must still report):
   `let shift_override = reporting && shift && !capture; if left_pressed && (!reporting || shift_override) { selection_drag() }`
   then
   `let suppress = shift_override && self.any_mouse_button_pressed(); if !suppress { dispatch_mouse_report(Motion, …) }`
   (`any_mouse_button_pressed()` exists at `lib.rs:4359`).

`mouse_shift_capture()` is only evaluated when `reporting && shift`
(short-circuit), so the extra worker read is rare. **Only `libroastty`**
(`lib.rs` + one `terminal.rs` accessor). No app change.

## Verification

1. **Headless regression test** (deterministic): enter mouse-reporting
   (`\x1b[?1000h`); hold **shift** (mods bit 1) + press at a cell + `mouse_pos`
   drag to another cell → assert an **active selection** spanning them (despite
   reporting) and that **no mouse report was queued** (the override suppressed
   it — assert via the worker's queued-write being empty, or that the selection
   exists). Controls: (a) **no shift** while reporting → no selection, the
   report path (Exp 32 clear) runs; (b) with **`CSI > 1 s`** fed
   (`mouse_shift_capture` flag → `Some(true)`, proven at `terminal.rs:6237`) →
   shift does **not** override (no selection; report/clear runs); (c) **mode
   1003 (`\x1b[?1003h`), shift held, NO button, bare motion** → a motion report
   IS still emitted (the suppression is button-gated — the Required fidelity
   fix). Fails pre-fix (shift-drag while reporting makes no selection), passes
   after. `cargo test -p roastty` (full) green.
2. **No regression:** not-reporting selection (Exp 25/27/28/30) unchanged
   (`shift_override` is false when not reporting); reporting-without-shift clear
   (Exp 32) unchanged.
3. **Live confirmation** (screen unlocked — check `CGSSessionScreenIsLocked`):
   in a mouse-mode TUI (or after `printf '\033[?1000h'`), shift-drag → text
   highlights. If locked, record Partial-pending- live like Exp 29/30; the
   headless test proves the core logic.
4. Faithful to upstream `Surface.zig:3882`/`mouseShiftCapture` (config nuance
   deferred + cited).

**Pass** = shift overrides reporting for selection (+ suppresses the report),
the headless test (shift-drag selects; no-shift/flag-capture don't) passes, and
the suite is green; **live-confirmed**.

**Partial** = headless-proven + suite green, but the live shift-drag is
screen-blocked (locked) — OR the config (`Never`/`Always`/`True`) plumbing is
the documented remaining sub-deferral.

**Fail** = shift can't be made to override reporting from the core handlers
(documented).

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: CHANGES REQUIRED → addressed (re-review below).** It
traced the override end-to-end and **confirmed the core is sound + faithful**:
the override produces a real selection (shift+press →
`should_shift_extend()==false` (click_count 0) → `selection_press`, then
shift-drag → `selection_drag`; neither gates on reporting); report suppression +
routing to the selection branch (not the clear) match `Surface.zig:3879-3911`;
the **flag-only `match flag { Some(v)=>v, None=>false }` exactly reproduces
upstream's config-`.false` path** and the config deferral is **honest**
(`roastty_app_update_config` copies only `confirm_close_surface`/keybinds, so
the Surface genuinely can't read `mouse_shift_capture`); the test is feasible
(`set_surface_worker_mouse_mode(..,1000,true)`, `ROASTTY_MODS_SHIFT` bit 1,
`last_reported_cell.is_none()` as the no-report proxy,
`surface_worker_active_selection`, `\x1b[>1s`→flag). One **Required** + two
Optional folded in:

- **Required — the motion-report suppression must be button-gated.** Upstream
  (`Surface.zig:4586-4607`) only suppresses the override motion report when a
  button is held (the `state != .release` loop), so **bare shift-motion in mode
  1003 must still report**; the original `if !shift_override` dropped them (a
  live regression). **Fixed:**
  `suppress = shift_override && any_mouse_button_pressed()`; added a mode-1003
  control test.
- **Optional — `mouse_button` returns `false` (not consumed)** on the override
  branch (upstream `Surface.zig:4124`). **Fixed.**
- **Optional — the accessor must be `pub(crate)`** (the `_for_tests` one is
  `pub(super)`, unreachable). **Fixed** (explicit in step 1).

**Re-review: APPROVED.** Confirmed the button-gated suppression is faithful
(`any_mouse_button_pressed()` == upstream's `state != .release` loop; mode 1003
`button_code None=>3` still emits bare motion), the override-return-`false` is
correct (non-override paths unchanged), the mode-1003 control test is feasible
(precedent `set_surface_worker_mouse_mode(..,1003,true)` + bare `mouse_pos` →
`last_reported_cell == Some` at `lib.rs:29455`), and borrows are clean
(sequential `with_termio`/ `with_termio_mut`, no nesting; `shift_override`
short-circuits the capture read). Optional folded in: bind
`let reporting = self.mouse_report_context().is_some()` (the compound check, not
the coarse `self.mouse_reporting` flag which defaults true).

## Result

**Result:** Partial — shift-while-reporting is wired + fully headless-proven (4
cases, suite green); the **live shift-drag confirmation is pending a locked
screen** (environment), to re-confirm on unlock. (The config
`Never`/`Always`/`True` plumbing is a documented sub-deferral.)

### Change (only `libroastty`)

- `Terminal::mouse_shift_capture_flag()` (`pub(crate)`) exposes the
  `XTSHIFTESCAPE` flag.
- `Surface::mouse_shift_capture()` — flag-first (`Some(v)=>v, None=>false`);
  config nuance deferred.
- `mouse_button`/`mouse_pos`:
  `shift_override = reporting && mods.shift && !mouse_shift_capture()`. When
  override: run the **selection** (not the clear), and **suppress the report**
  (`mouse_button` returns `false`; `mouse_pos` suppresses the motion report
  **only while a button is held** — `any_mouse_button_pressed()` — so bare
  shift-motion in mode 1003 still reports).

### Verification

- **Headless regression test** `shift_overrides_mouse_reporting_for_selection`
  (`lib.rs`), 4 cases: (1) shift-drag while reporting → an **active
  selection** + the press/drag/release **suppress the report**
  (`last_reported_cell` stays `None`); (a) no-shift press while reporting → no
  selection (clear path); (c) **mode 1003, shift, no button** → bare motion
  **still reports** (the button-gated suppression — the Required fidelity fix);
  (b) **`CSI > 1 s`** (XTSHIFTESCAPE) → shift does **not** override. Fails
  pre-fix (shift-drag while reporting made no selection), passes after.
- **Full `cargo test -p roastty`:** lib **4416 passed**, 0 failures — no
  regression to the not-reporting selection path (Exp 25/27/28/30) or the
  reporting clear (Exp 32) or the mouse-report-mode tests.
- **Live shift-drag — blocked (locked screen):**
  `CGSSessionScreenIsLocked: true`; the visible "shift-drag selects in a
  mouse-mode TUI" awaits the unlock.

## Conclusion

Shift now overrides mouse-reporting for selection (faithful to upstream
`Surface.zig:3882`/ `mouseShiftCapture`, flag-first), with the report correctly
suppressed only while a button is held. **Mouse selection is now
feature-complete to upstream fidelity** (cell-drag + autoscroll, word, line,
shift-extend, shift-while-reporting) + clipboard. The remaining items — the
config-`mouse-shift-capture` plumbing (a sub-deferral), CVDisplayLink vsync,
DPI-change rebuild — are live/perf follow-ups. The live re-confirmations (Exp 29
CJK, Exp 30 shift-click, Exp 33 shift-drag) + closing 802 await the screen
unlock.

## Result Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED (no findings).** It attacked the most damaging
axes and each held: the test **passes + is load-bearing across all 4 cases**
((1) would fail pre-fix on both asserts — Exp 32's clear makes `has_sel` false +
always dispatches; the `last_reported_cell` proxy is sound
(`dispatch_mouse_report` passes `last_cell` to the encoder); (c) the mode-1003
bare-motion would fail under the pre-fix unconditional suppression — the
button-gating fix; `buttons.fill(None)` is legitimate fixture isolation, not a
mask; (b) isolates the XTSHIFTESCAPE flag); **no regression** (full lib **4416
passed, 0 failed**; `mouse_button` returning `false` on override broke no
caller); the **accessor is `pub(crate)` and NOT `#[cfg(test)]`-gated**,
`build`/`fmt --check` clean; **upstream-faithful** (verified
`Surface.zig:3905-3912` shift-override+`return false`,
`cursorPosCallback:4586-4607` button-gated `break :report`,
`mouseShiftCapture:3689-3713` flag-first `.false` path;
`any_mouse_button_pressed()` == upstream's `state != .release` loop; no report
leaks); **Partial honest** (locked-screen blocker real; the config deferral
genuinely unreadable — the App copies only select fields). Scope clean
(libroastty + one accessor, no new "ghostty" literals).
