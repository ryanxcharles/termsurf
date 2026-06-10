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

# Experiment 34: Phase C — plumb the `mouse-shift-capture` config into `shiftCapture`

## Description

Exp 33 implemented `Surface::mouse_shift_capture()` **flag-first** and deferred
the config (`mouse-shift-capture = Never/Always/True`) because the App didn't
surface the value. This experiment plumbs the config through so
`mouse_shift_capture()` is the **full** upstream `mouseShiftCapture`
(`Surface.zig:3689-3713`): config `Never`→`false`, `Always`→`true`; otherwise
the terminal's `XTSHIFTESCAPE` flag (`Some(v)`→`v`); otherwise the config
default (`False`→`false`, `True`→`true`).

The parsed config already has the value
(`config::Config.mouse_shift_capture: MouseShiftCapture`, `config/mod.rs:48`,
default `False`); only the App→Surface surfacing is missing.

## Approach

1. **`App` gains `mouse_shift_capture: config::MouseShiftCapture`**
   (`lib.rs:1834`), set in `roastty_app_new` from
   `config.parsed.mouse_shift_capture` (default `False` when no config), and
   refreshed in `roastty_app_update_config`
   (`app.mouse_shift_capture = config.parsed.mouse_shift_capture`, mirroring
   `confirm_close_surface`).
2. **`Surface::mouse_shift_capture()`** delegates to the **existing tested
   helper** `MouseShiftCapture::capture_shift(terminal_request: Option<bool>)`
   (`config/mod.rs:3955`, covered by the truth-table test at
   `config/mod.rs:4736`) — no inlined second copy of the upstream decision:
   ```rust
   let config = app_from_handle(self.app).map(|a| a.mouse_shift_capture).unwrap_or(config::MouseShiftCapture::False);
   let flag = self.termio_worker.as_ref().and_then(|w| w.with_termio(|t| t.terminal().mouse_shift_capture_flag()));
   config.capture_shift(flag)
   ```
   (`MouseShiftCapture` is `Copy`; the `&mut App` borrow is dropped after
   copying the value, then the worker read for the flag. `flag` is `None` both
   for no-worker and XTSHIFTESCAPE-unset — both correctly fall through to the
   config default inside `capture_shift`.)

Behavior is **unchanged for the default config** (`False` + no flag → `false`,
i.e. shift overrides — the Exp-33 result), so all Exp-33 tests still pass;
`Never`/`Always`/`True` now take effect. **Only `libroastty`** (`lib.rs`). No
app change.

## Verification

1. **Headless regression test:** set the App's `mouse_shift_capture` directly
   (`app_from_handle(app) .unwrap().mouse_shift_capture = …`) and assert
   `Surface::mouse_shift_capture()` (or the observable effect — shift-press
   while reporting selects vs clears) for each variant:
   - `Never` → `false` (shift overrides → shift-drag-while-reporting selects);
   - `Always` → `true` (shift does **not** override → no selection, even with no
     `XTSHIFTESCAPE`);
   - `False` + `XTSHIFTESCAPE Some(true)` (`CSI > 1 s`) → `true` (flag wins);
   - `True` + no flag → `true`. Reuse the Exp-33 reporting harness
     (`set_surface_worker_mouse_mode(.,1000,true)`, `ROASTTY_MODS_SHIFT`). Fails
     pre-fix (config ignored — `Always`/`Never` had no effect), passes after.
     `cargo test -p roastty` (full) green.
2. **No regression:** the Exp-33 test (default config `False`) still passes
   (default behavior unchanged); `roastty_app_new`/`update_config` still set the
   other App fields.
3. **No live confirmation needed** — a config-driven logic completion; the
   headless model assertion per variant is the proof. (Completes fully while the
   screen is locked.)
4. Faithful to upstream `mouseShiftCapture` (`Surface.zig:3689-3713`) — cite
   each arm.

**Pass** = the config is plumbed into the full `shiftCapture` logic, the
per-variant headless test passes, and the suite is green (default behavior
unchanged). Fully headless — no Partial-pending-live.

**Partial** = the plumbing works but a variant needs more (unlikely — four
explicit arms).

**Fail** = the config genuinely can't be surfaced to the Surface (documented).

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED.** Confirmed: the **plumbing is complete +
correct** — the four edit sites in `roastty_app_new` (tuple destructure `10722`,
map body `10729` reading `config.parsed.mouse_shift_capture`, `unwrap_or`
default `MouseShiftCapture::False` `10738`, `App {}` literal `10746`) + the
`update_config` assignment `10810`; `mouse_shift_capture` is **not** hoisted
onto the `Config` wrapper (unlike `confirm_close_surface`), so it's read as
`config.parsed.mouse_shift_capture` (exactly as planned); the other
`Config`-wrapper constructors (`10278`/`10304`) need no change (they clone
`parsed`). **Logic faithful**, **default unchanged** (`False`+no-flag → `false`,
Exp-33 identical), **borrow clean** (`app_from_handle` → a separate allocation
from the Surface; Copy value read then dropped before the worker read; stays
`&self`), **test feasible**
(`app_from_handle(app).unwrap().mouse_shift_capture = …` in-crate). One
Optional + a Nit folded in: **reuse the existing tested
`capture_shift(Option<bool>)` helper** instead of inlining the match (avoids
drift); note the no-worker→`None`→config-default case.

## Result

**Result:** Pass — the `mouse-shift-capture` config is plumbed into the full
`shiftCapture` logic (fully headless; no live needed).

### Change (only `libroastty`)

- `App` gains `mouse_shift_capture: config::MouseShiftCapture`, set in
  `roastty_app_new` (tuple + map body `config.parsed.mouse_shift_capture` +
  `unwrap_or` default `False` + the `App {}` literal) and refreshed in
  `roastty_app_update_config`.
- `Surface::mouse_shift_capture()` now reads the App config + the terminal
  `XTSHIFTESCAPE` flag and delegates to the **existing tested**
  `MouseShiftCapture::capture_shift(Option<bool>)` (no inlined logic copy).

### Verification

- **Headless regression test** `mouse_shift_capture_honors_config` (`lib.rs`):
  with no flag — `Never`→`false`, `Always`→`true`, `False`→`false`,
  `True`→`true`; with `CSI > 1 s` (flag `Some(true)`) — `False`→`true` (flag
  wins), `Never`/`Always` ignore it. Fails pre-fix (config ignored —
  `Always`/`Never` had no effect), passes after.
- **Full `cargo test -p roastty`:** lib **4417 passed**, 0 failures — the Exp-33
  test (`shift_overrides_mouse_reporting_for_selection`, default config `False`)
  still passes (default behavior unchanged), as do the existing `capture_shift`
  truth-table + XTSHIFTESCAPE-stream tests.
- **No live confirmation needed** — a config-driven logic completion; the
  per-variant model assertion is the proof. **Completes fully while the screen
  is locked.**

## Conclusion

`Surface::mouse_shift_capture()` is now the **full** upstream
`mouseShiftCapture` (config `Never`/`Always` + the `XTSHIFTESCAPE` flag + config
default), via the shared `capture_shift` helper. This closes the Exp-33 config
sub-deferral, so **shift-while-reporting is now fully faithful**. The remaining
follow-ups (CVDisplayLink vsync, DPI-change rebuild) are genuinely live/perf.
The live re-confirmations (Exp 29 CJK, Exp 30 shift-click, Exp 33 shift-drag) +
closing 802 await the screen unlock.

## Result Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED (no findings).** Verified: the test is
**load-bearing** (exercises the real App path `surface.mouse_shift_capture()` →
`app_from_handle(self.app)`; fails pre-fix at multiple asserts — `Always`/`True`
no-flag (old=false) and `Never` ignoring the flag (old=true); all six cases
asserted); **no regression** (full lib **4417 passed, 0 failed**; Exp-33
default-`False` test still passes — identical to the old `unwrap_or(false)`;
`capture_shift` truth-table + XTSHIFTESCAPE-stream tests pass); **plumbing
complete** (all five sites — App field, the four `roastty_app_new` sub-sites,
`update_config`; reads `config.parsed.mouse_shift_capture`, not the hoisted
wrapper, no stale site); **faithful** (`capture_shift` reproduces
`Surface.zig:3689-3713` arm-for-arm; borrow clean — Copy value dropped before
the worker read); scope/hygiene clean (lib.rs only, `fmt --check` 0, no new
"ghostty" literals); Pass honest (pure config-logic completion, no live needed).
