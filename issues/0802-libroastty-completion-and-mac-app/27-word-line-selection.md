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

# Experiment 27: Phase C — double/triple-click word & line selection

## Description

Exp 25 wired single-click cell-drag selection, but passed `time_ns: None` to
`gesture.press` (`lib.rs`). The gesture's click-count machinery
(`selection_gesture.rs::press_repeat`) **requires a monotonic `time_ns`**: with
`None` it always `reset`s, so `click_count` is stuck at 1 and only the `Cell`
behavior runs. So **double-click word and triple-click line selection don't
work** — a common, expected terminal feature (the real Ghostty has it).

The machinery is otherwise complete: `press_repeat`, when two presses land
within `repeat_interval_ns` and `max_distance` at the same cell, increments
`click_count` (capped at 3) and sets `behavior = behaviors[click_count-1]` —
`[Cell, Word, Line]` (`DEFAULT_BEHAVIORS`) → `Cell` (1), `Word` (2), `Line` (3)
— and `press_selection` calls `select_word`/`select_line`. `release` preserves
`click_count` (so press→release→press detects the repeat). The only missing
input is the clock.

Upstream (`Surface.zig:3945-3952`) sets
`repeat_interval = config.mouse_interval` (= 500ms) and
`max_distance = cell.width`.

## Approach

1. **A monotonic time source on the Surface.** Add an epoch `Instant` (sampled
   in `surface_new`); `time_ns()` = `epoch.elapsed().as_nanos() as u64`
   (monotonic — `press_repeat` only compares _differences_; `u64` ns ≈ 584 yr,
   no truncation). Faithful to upstream `Surface.zig:3776`
   (`Instant.now()`/`since`). (Adds the first production `Instant` use; fine.)
2. **Feed it into `selection_press`** (`lib.rs`): pass
   `time_ns: Some(self.time_ns())`, `repeat_interval_ns: 500_000_000` (500ms,
   matching upstream `mouse_interval`), and `max_distance:` the cell width as
   `f64` (the same `selection_geometry`/`mouse_report_geometry` `cell.width`
   source `selection_drag` uses, matching upstream).
   `word_boundary_codepoints: None` (default boundaries) for now.
3. No change to `selection_drag`/`release` (drag already extends with the active
   `behavior` — a double-click-then-drag extends by words, handled by the
   existing `behavior` field).

The result: a rapid second click at the same cell → `click_count = 2` → `Word` →
`select_word`; a third → `Line` → `select_line`. **Only `libroastty`**
(`lib.rs`: the Surface epoch + the `selection_press` params). No app change.

## Verification

1. **Headless regression test:** feed a line with space-bounded words (e.g.
   `foo TARGET bar`).
   - **Double-click** (press+release+press at a cell inside `TARGET`, no drag —
     two `mouse_button` presses microseconds apart, well within 500ms) →
     `active_selection()` text == `TARGET`.
   - **Triple-click** (a third rapid press) → text == the line
     (`foo TARGET bar`). The new test covers the **positive** double/triple path
     only; the interval/distance **reset** gate is already deterministically
     proven at the gesture layer by
     `selection_gesture_repeat_distance_and_interval_reset` (no C-API time
     injection, so the >500ms path isn't drivable here — and two back-to-back
     FFI presses are always <500ms apart, which is exactly what makes the
     positive test work; noted as a mild wall-clock dependence). Asserts via
     `selection_format(Plain, …, None)`. Fails pre-fix (clicks never reach
     Word/Line), passes after. `cargo test -p roastty` (full) green.
2. **Live confirmation** (screen unlocked): launch with known text;
   **double-click** a word with the `drag.swift` driver used as a click
   (down+up, no movement, twice rapidly) — the word renders highlighted;
   triple-click → the line. App + descendant tree killed (0 dangling); shots
   out-of-repo.
3. Faithful to upstream `mouse_interval` (500ms) + `max_distance = cell.width`.

**Pass** = `time_ns` is wired, the headless double→word / triple→line test
passes, the suite is green, and the live app selects a word on double-click (and
a line on triple-click).

**Partial** = double-click word works + tested, but triple-click line or the
live capture is deferred (documented).

**Fail** = click-count detection can't be driven from the core handlers
(documented).

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: APPROVED.** It traced the gesture state machine
end-to-end and confirmed: the 2nd press sees `click_count > 0` → `press_repeat`
→ `click_count=2`, `behavior=Word` → `select_word` (and the live `release` does
**not** reset `click_count`/`click_time_ns`/anchor, so the post-release press
still validates); the existing `selection_gesture_single_double_triple_press`
already proves 1→Cell/2→Word/3→Line; `selection_press` applies the `press()`
return via `set_selection`; the epoch- `Instant` + `as_nanos as u64`
`saturating_sub` is the sound monotonic equivalent of upstream
`Instant.now()/since`; `500_000_000`ns + `max_distance=cell.width` match
upstream; and `select_word(None)` → `DEFAULT_WORD_BOUNDARIES` (incl. space)
yields `TARGET`, not the whole line. Non-blocking notes folded in: cite
`selection_gesture_repeat_distance_and_interval_reset` for the gate (no C-API
time injection); `cell.width as f64`; fixed the inaccurate `lib.rs:18600`
citation (test `SystemTime`, not production `Instant`).

## Result

_(to be added after the run.)_

## Conclusion

_(to be added after the run.)_
