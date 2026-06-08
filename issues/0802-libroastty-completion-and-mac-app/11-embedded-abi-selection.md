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

# Experiment 11: Embedded ABI — the selection/point layout divergence (Exp-6 #3)

## Description

The last identified embedded-ABI blocker before the app compiles. After Exp 8–10
resolved all 56 missing symbols, the app build reaches **14 errors in
`SurfaceView_AppKit.swift`**: it constructs `roastty_point_s(tag:coord:x:y:)` +
`ROASTTY_POINT_COORD_*` and a
`roastty_selection_s(top_left:bottom_right:rectangle:)`, but roastty's
`point_s`/`selection_s` have a **completely different layout** (801's pull-model
scaffolding).

**The divergence (Exp-6 #3):**

| type            | upstream (app expects)                             | roastty current                      |
| --------------- | -------------------------------------------------- | ------------------------------------ |
| `point_tag_e`   | `ACTIVE, VIEWPORT, SCREEN, SURFACE`                | `… SCREEN, HISTORY` (idx 3 differs)  |
| `point_coord_e` | `EXACT, TOP_LEFT, BOTTOM_RIGHT`                    | **absent**                           |
| `point_s`       | `{tag, coord, x, y}` (flat, 16 B)                  | `{tag, value_u}` (grid tagged-union) |
| `selection_s`   | `{point_s top_left, bottom_right; bool rectangle}` | size-prefixed, grid refs, gestures   |

**The complication (confirmed by recon):** the embedded
`roastty_surface_read_text(surface, selection_s, text_s*)` — the app-facing
function — takes `RoasttySelection` **by value**, but that same grid-based
`RoasttySelection`/`RoasttyPoint` is **also** consumed by the roastty-internal
formatter/granular path: `write_selection` (lib.rs:7655), `read_selection`
(7676), the `RoasttyFormatterTerminalOptions.selection` field (8087), and the
granular point accessors (1462, 1498). So the two selection concepts — the
**embedded** one (upstream layout, app-facing) and the **granular** one (roastty
pull-model scaffolding) — currently collide on one type name. The function
_signatures_ already match upstream (`read_text(surface, selection_s, text_s*)`,
`read_selection(surface, text_s*)`, `quicklook_word(surface, text_s*)`); only
the `selection_s`/`point_s` _layout_ is wrong.

## Approach

Separate the two concepts; make the **embedded** one byte-faithful and rewire
the one app-facing consumer (`read_text`):

1. **Rename the grid/pull-model types** so they stop occupying the embedded
   names: `roastty_point_s`→`roastty_grid_point_s`,
   `roastty_selection_s`→`roastty_grid_selection_s` (+
   `roastty_point_value_u`→`roastty_grid_point_value_u`, `point_coordinate_s` as
   needed) in `roastty.h`; `RoasttyPoint`→`RoasttyGridPoint`,
   `RoasttySelection`→`RoasttyGridSelection` in `lib.rs`, updating the
   formatter/granular consumers (`write_selection`, `read_selection`, the
   formatter options field, the granular accessors, the gesture geometry). This
   is a mechanical rename of a roastty-internal type — no behavior change.
2. **Define the embedded types** byte-faithful to upstream. A **new** enum
   `point_coord_e` with `EXACT`/`TOP_LEFT`/`BOTTOM_RIGHT` (0/1/2) — distinct
   from the existing grid `roastty_point_coordinate_s` (`{u16 x; u32 y}`), which
   is renamed to a `grid_` name in step 1 and must NOT be conflated.
   `roastty_point_s` becomes
   `{point_tag_e tag; point_coord_e coord; u32 x; u32 y}`; `roastty_selection_s`
   becomes `{point_s top_left; point_s bottom_right; bool rectangle}`. Add Rust
   `#[repr(C)]` mirrors. **`point_tag_e` is left unchanged** — keep `HISTORY` at
   idx 3: the review confirmed the real embedded impl
   (`vendor/ghostty/src/apprt/embedded.zig:1315`, `terminal/point.zig:31`) uses
   `history`, and `ghostty.h`'s `GHOSTTY_POINT_SURFACE` is an out-of-sync header
   artifact; roastty's `ROASTTY_POINT_HISTORY` + `TerminalPointTag::History`
   already match upstream, and the app references neither name (only
   `SCREEN`/`VIEWPORT`). Do not touch `TerminalPointTag`/`point_tag_from_raw`.
3. **Rewire `roastty_surface_read_text`** to take the **embedded**
   `roastty_selection_s` by value and build the internal `TerminalSelection` via
   a **new conversion** — the existing `read_selection` (lib.rs:7676)
   reconstructs page-node grid refs from the grid struct and **cannot be
   reused** (the embedded `point_s` carries no node). Each embedded `point_s`
   resolves to a terminal pin by `(tag, coord)`: `EXACT`→`page_list.pin{x,y}` on
   the tagged screen; `TOP_LEFT`→`get_top_left(tag)`;
   `BOTTOM_RIGHT`→`get_bottom_right(tag)` (matching `embedded.zig:1344-1366`).
   roastty already has `get_top_left`/`get_bottom_right`
   (`terminal/page_list.rs:2294,2330`) — wire them through a new point resolver.
   Without this, the app's whole-screen read (`SCREEN` +
   `TOP_LEFT`/`BOTTOM_RIGHT`, `x=y=0`) collapses to cell (0,0).
4. **Tests:** Rust `offset_of`/`size_of` + C `_Static_assert` (`point_s`=16,
   `selection_s`=2·16+1→padded 36, `point_coord_e` 0/1/2, `point_tag_e`
   unchanged) **and a behavioral test**: a `SCREEN` selection with
   `TOP_LEFT`..`BOTTOM_RIGHT` returns the full screen text (not one cell) — the
   layout/compile checks alone would not catch the coord-resolution bug. Note
   the **rename blast radius** (~60+ `RoasttySelection`/`RoasttyPoint` refs
   incl. the grid layout test asserting size 64, the pull-model FFI, formatter
   options, gesture geometry) for the result review to confirm every renamed
   site still asserts the grid layout.
5. **`cargo test` green**, then rebuild the app — it should compile past
   selection (revealing the next divergence, if any, or linking).

**Resolved by the design review:** (a) the grid-rename is the right call —
`read_text` is the **only** embedded function taking a selection by value; every
other consumer is 801/803 pull-model and correctly keeps the grid type, so the
separation is genuinely safe. (b) `point_tag_e` stays `HISTORY` (the real
upstream impl uses `history`; `SURFACE` in `ghostty.h` is an out-of-sync
artifact).

## Verification

1. **Header parses clean**, no duplicate constants, `_Static_assert`s pass.
2. **`cargo test -p roastty --lib`** green (the rename + the read_text rewire
   don't regress the formatter/granular selection tests).
3. **The app build compiles past the `selection_s`/`point_s` errors** (the 14
   errors gone); the first remaining error (if any) is recorded as the next
   divergence / or it links.

**Pass** = the embedded `point_s`/`selection_s`/`point_coord_e` are
byte-faithful, the grid types are cleanly separated, `read_text` takes the
embedded selection **and resolves `coord` correctly** (the behavioral
full-screen test passes, not just layout/compile), `cargo test` green, and the
app compiles past selection.

**Partial** = selection resolves + tests green, but the app surfaces a further
divergence (documented as the next experiment).

**Fail** = the embedded selection can't be reconciled with the terminal
selection model without deeper work (documented).

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: CHANGES REQUIRED → addressed.** It confirmed the
grid-vs-embedded split is real (`read_text` is the only embedded consumer of a
by-value selection; all other consumers are 801/803 pull-model and keep the grid
type), the selection-only scope is honest (the app touches selection in one
file), and the byte-faithfulness bar is present. Two Required + two minor
findings, all folded in:

- **Required — keep `HISTORY`, not `SURFACE`.** The _real_ embedded impl
  (`apprt/embedded.zig:1315`, `terminal/point.zig:31`) uses `history` at idx 3;
  `ghostty.h`'s `GHOSTTY_POINT_SURFACE` is an out-of-sync header artifact.
  roastty's `ROASTTY_POINT_HISTORY` + `TerminalPointTag::History` already match
  upstream and the app uses neither name — so leave `point_tag_e` unchanged
  (changing it would _introduce_ a divergence). Removed the SURFACE directive.
- **Required — the `read_text` rewire can't reuse the grid read path** and a
  compile/layout check would pass while returning wrong text:
  `grid_ref(tag, coord)` resolves coords as EXACT only, so the app's
  whole-screen read (`SCREEN` + `TOP_LEFT`/`BOTTOM_RIGHT`, x=y=0) collapses to
  cell (0,0). The plan now specifies a new `(tag, coord)`→pin resolver wiring
  the existing `get_top_left`/`get_bottom_right`/`pin` (matching
  `embedded.zig:1344`), plus a **behavioral full-screen test**.
- **Optional/Nit:** the rename blast radius is ~60+ refs (incl. the grid layout
  test asserting size 64) — noted for the result review; and the existing grid
  `roastty_point_coordinate_s` (`{u16 x; u32 y}`) must not be conflated with the
  new `point_coord_e` enum (both coexist).

## Result

**Result:** Pass — the embedded `point_s`/`selection_s`/`point_coord_e` are
byte-faithful, the grid pull-model types are cleanly separated, `read_text`
resolves `coord` correctly (the crux behavioral test passes), `cargo test` is
green, and **the app compiles past the selection/point errors** (the 14 errors
are gone).

### What landed

- **Grid rename (mechanical, no behavior change):**
  `RoasttySelection`→`RoasttyGridSelection`, `RoasttyPoint`→`RoasttyGridPoint`,
  `RoasttyPointValue`→`RoasttyGridPointValue`,
  `RoasttyPointCoordinate`→`RoasttyGridPointCoordinate` (~110 Rust refs) + the C
  header
  `roastty_grid_point_s`/`_selection_s`/`_point_value_u`/`_point_coordinate_s`
  (32 refs) — word-boundary so the gesture types + `point_tag_e` (shared) are
  untouched.
- **Embedded types** byte-faithful: new `roastty_point_coord_e`
  (EXACT/TOP_LEFT/BOTTOM_RIGHT), `roastty_point_s` `{tag, coord, x, y}` (16 B),
  `roastty_selection_s` `{top_left, bottom_right, rectangle}` (36 B); C
  `_Static_assert`s + Rust `offset_of` test agree. `point_tag_e` kept `HISTORY`
  (matches the real embedded impl).
- **The coord resolver (the real work):**
  `Terminal::resolve_embedded_selection` + `EmbeddedPointCoord`, wiring
  `screen.grid_ref` (EXACT) / `get_top_left` / `get_bottom_right` (now
  `pub(in crate::terminal)`) → `GridRef` → `TerminalGridRef`. `read_text` takes
  the embedded selection, resolves both endpoints inside the terminal context.
- **Tests:** 11 read_text test sites migrated to the embedded selection (incl.
  the invalid-selection test switched from a corrupt grid `size` to an invalid
  tag), the layout test, and the **crux behavioral test** (SCREEN +
  TOP_LEFT..BOTTOM_RIGHT returns the full screen, not cell (0,0)).

### Verification

- **`cargo test -p roastty --lib`: 4398 passed, 0 failed** (4396 + the 2 new
  tests) — the rename + the read_text rewire caused no regression.
- **App rebuild: 0 `point_s`/`selection_s`/`POINT_COORD` errors** — compiles
  past selection.

## Conclusion

The selection/point divergence is closed and the app build advances again. The
app-build-driven loop now surfaces the **next divergences** (80 errors, all in
the app's action/target glue), which split into clear follow-ups:

1. **`roastty_target_s` is missing its `target` union member (51 errors)** — the
   app reads `target.target.surface`, but roastty's `target_s` is flat. This is
   the **same tagged-union pattern as Exp 9's `action_u`**: define
   `roastty_target_u {surface}` + change `target_s` to `{tag, target_u target}`,
   byte-faithful.
2. **`roastty_action_tag_e` is incomplete (~25 missing `ROASTTY_ACTION_*`
   constants: `MOUSE_SHAPE`, `RENDERER_HEALTH`, `SIZE_LIMIT`,
   `PRESENT_TERMINAL`, `KEY_TABLE`, `PROGRESS_REPORT`, …)** — Exp 9 added the
   `action_u` types but roastty's tag enum has gaps vs upstream's 64. (The Exp-9
   type-gap check counted `roastty_action_*` _types_, not the `ROASTTY_ACTION_*`
   _constants_ — hence missed here, caught now at the app compile.)
3. **A handful of misc:** `AppDelegate:579` passes `input_key_s` where a
   `key_event_t` handle is expected (a key-event API shape), and a few app enums
   (`floating`/`normal`) + a `DispatchWorkItem` closure.

**Next (Exp 12):** the `target_s` tagged union + completing `action_tag_e` to
upstream's 64 (the two big clusters), then re-attempt the compile to surface the
misc tail.

## Result Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: CHANGES REQUIRED → addressed.** It confirmed the
byte-faithfulness, the resolver tag/coord mapping, the grid rename
(behavior-preserving; grid layout test still asserts size 64; gesture types +
`point_tag_e` untouched), the test integrity (the crux test genuinely uses coord
1/2 and would fail on a (0,0) collapse), and that the Exp-12 deferral is honest.
It found **two real correctness bugs** (so "Pass" wasn't yet earned), both
fixed:

- **Required — use-after-free across two lock holds.** `read_text` resolved the
  selection in one `with_termio` (capturing raw page-node refs) then read in a
  _second_ — and the worker thread can prune scrollback pages between them.
  Upstream holds the mutex **once**. **Fixed:**
  `try_surface_embedded_selection_text` resolves + formats under a **single**
  `with_termio` hold, so the resolved `TerminalSelection` never outlives the
  lock.
- **Required — the EXACT arm dropped upstream's bounds clamp.** Upstream clamps
  `x→min(x, cols-1)`, `y→min(y, rows-1)` (`embedded.zig`), so an out-of-edge
  exact point (routine for pixel→cell mouse drags) yields the edge cell; roastty
  returned `None`→false. **Fixed:** the EXACT arm clamps to
  `screen.cols()/rows()` (verified = upstream `pages.rows` = active rows) before
  resolving; `embedded_point_spec` uses a saturating cast. Added
  `surface_read_text_clamps_out_of_bounds_exact_coords`, and corrected
  `metadata_clamps_partially_visible_selection` to the faithful clamped result
  (the SCREEN selection clamps to scrollback-top rows 0..2, off-viewport →
  `(-1, -1, 0, 0)` metadata + `"aaa\nbbb\nccc"`).

Re-verified: **`cargo test -p roastty --lib` green** after both fixes.
