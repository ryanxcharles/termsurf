+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 389: route the row passes through selected_colors

## Description

The row passes (`rebuild_bg_row`, `rebuild_row`) currently branch on the
`is_selected` **bool** and call `selection_colors` directly for the selection
case, while `selected_colors` (Experiment 388 — the full `Selected`-state
dispatcher, including the search arms) is exercised only by tests. This
experiment makes `selected_colors` the **live** color path: a new
`selected_state` derives the per-cell `Selected` enum (currently `Selection` or
`False`, since the search highlight ranges are not yet plumbed), and both passes
route through `selected_colors(state, …)`, falling back to `cell_colors` for
`False`. Behavior is unchanged (only `Selection`/`False` are produced), but the
production path now matches upstream's enum dispatch, so wiring search later
changes only `selected_state`.

## Upstream behavior

In `rebuildCells` (`renderer/generic.zig`), there is **one** per-cell `selected`
enum, computed once, and the background/foreground switches dispatch on it
(`.false` → the base colors; `.selection`/`.search`/`.search_selected` → the
selected arms). roastty's `selected_colors` (Experiment 388) already mirrors
that switch; this experiment routes the passes through it (rather than the
`is_selected`-bool + direct `selection_colors`), so the structure matches
upstream's single `selected` dispatch.

## Rust mapping (`roastty/src/renderer/cell.rs`)

A `selected_state` derives the enum from the selection bounds (search deferred),
and the passes dispatch through `selected_colors`:

```rust
/// The per-cell [`Selected`] state for a rebuild. Selection takes precedence;
/// the search-highlight states (`Search`/`SearchSelected`) are deferred (their
/// per-row ranges are not yet plumbed), so this yields `Selection` or `False`.
fn selected_state(selection: Option<[u16; 2]>, x: u16, wide: Wide) -> Selected {
    if is_selected(selection, x, wide) {
        Selected::Selection
    } else {
        Selected::False
    }
}
```

`rebuild_bg_row` — the color comes from `selected_colors` (falling back to
`cell_colors` for `False`), and the opaque branch keys on `state != False`:

```rust
let x = u16::try_from(col).expect("viewport column fits u16");
let state = selected_state(selection, x, cell.wide);
let colors = selected_colors(
    state, cell.style, default_fg, default_bg, palette, bold, selection_config,
)
.unwrap_or_else(|| {
    cell_colors(cell.style, cell.codepoint, default_fg, default_bg, palette, bold)
});
let has_explicit_bg = !matches!(cell.style.bg_color, Color::None);
let selected = state != Selected::False;
let bg_alpha = if selected || cell.style.flags.inverse || has_explicit_bg {
    alpha
} else {
    0
};
let rgb = colors.bg.unwrap_or(default_bg);
*contents.bg_cell_mut(row, col) = CellBg([rgb.r, rgb.g, rgb.b, bg_alpha]);
```

`rebuild_row`'s `fg_colors` builder — the foreground from `selected_colors`
(falling back to `cell_colors`):

```rust
let state = selected_state(selection, x, cell.wide);
let fg = selected_colors(
    state, cell.style, default_fg, default_bg, palette, bold, selection_config,
)
.map(|c| c.fg)
.unwrap_or_else(|| {
    cell_colors(cell.style, cell.codepoint, default_fg, default_bg, palette, bold).fg
});
```

No function signatures change (both passes already take `selection` and
`selection_config`), so no call-site churn. After this, the passes no longer
call `selection_colors` directly — it is reached only via `selected_colors`.

## Scope / faithfulness notes

- **Ported (bridged)**: the single per-cell `Selected` dispatch in the row
  passes — both passes derive one `selected_state` and route the color through
  `selected_colors`, matching upstream's one `selected` enum feeding the
  background and foreground switches.
- **Faithful**: `selected_state` yields `Selection` exactly when `is_selected`
  (the Experiment-386 bounds predicate) is true, else `False` (the search states
  are deferred — their ranges are not yet available, so upstream's later
  highlight branch has no input). The color for a `Selection` cell is
  `selected_colors(Selection, …)` =
  `selection_colors(…, config.background, config.foreground)` — identical to the
  previous direct call; `False` falls back to `cell_colors` (covering twist
  intact). The `bg_alpha` opaque branch keys on `state != False` (equivalent to
  the previous `selected` bool). So behavior is unchanged; only the structure
  now matches upstream's dispatch.
- **Faithful adaptation**: roastty keeps the bg/fg in separate passes, so each
  derives `selected_state` and calls `selected_colors` for its half (upstream
  computes the state once for both); the result is identical. `selected_colors`
  returns `Option`, so the `False` fallback to `cell_colors` is an
  `unwrap_or_else`.
- **Deferred**: deriving the `Search`/`SearchSelected` states (the per-row
  search highlight ranges on `RunOptions`, and `selected_state` consulting
  them); the lock-cursor glyph + under-cursor recolor; the column-ordered
  decoration merge + link double-underline; the Metal upload. (Consumed by tests
  now.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/cell.rs`:
   - add `selected_state` (wraps `is_selected`; `Selection` or `False`);
   - `rebuild_bg_row`: derive `state = selected_state(…)`; the color is
     `selected_colors(state, …).unwrap_or_else(cell_colors)`; the opaque branch
     keys on `state != Selected::False`. Update its doc comment.
   - `rebuild_row`: the `fg_colors` builder derives `state` and uses
     `selected_colors(state, …).map(|c| c.fg).unwrap_or_else(…)`. Update its doc
     comment.
2. Tests (in `cell.rs`): a `selected_state` test — `Selection` inside the
   bounds, `False` outside and for `None` bounds, and a **spacer tail** at
   `end + 1` yielding `Selection` (its `x_compare = end`). The existing
   selection background/foreground tests (Experiments 386–387) continue to pass
   unchanged, confirming the routed path produces identical colors.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty selected_state
cargo test -p roastty rebuild
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- both passes derive a `selected_state` and route the color through
  `selected_colors` (falling back to `cell_colors` for `False`), with the opaque
  branch keying on `state != False` — matching upstream's single `selected`
  dispatch, with unchanged behavior (only `Selection`/`False` produced);
- the tests pass (`selected_state` yields `Selection`/`False`/spacer-tail
  correctly), and the existing selection tests still pass (identical colors via
  the routed path);
- the search-range derivation, the lock-cursor recolor, and the Metal upload
  stay deferred; the passes no longer call `selection_colors` directly;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the routing changes behavior (a `Selection` cell's
colors differ, the `False` fallback drops the covering twist, the opaque branch
mis-keyed), or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the refactor is behavior-preserving: `selected_state`
returns `Selection` exactly when the existing `is_selected` predicate is true,
else `False`, and producing only those two states is the right scope while the
search ranges are not yet plumbed; `selected_colors(Selection, …)` is equivalent
to the previous direct
`selection_colors(…, config.background, config.foreground)`, and `False → None`
keeps the fallback on `cell_colors`, preserving the covering/full-block twist.
It confirmed the `bg_alpha` change from the prior bool to
`state != Selected::False` is equivalent while only `Selection`/`False` are
produced, that deriving the state independently in the background and foreground
passes is fine because both use the same inputs (`selection`, `x`, `cell.wide`)
and therefore agree despite the split-pass adaptation, and that `unwrap_or_else`
is the right shape (`cell_colors` is evaluated only for `False`, not for the
selected states). It agreed the `selected_state` test plus the existing
selection bg/fg rebuild tests are sufficient to prove the refactor preserves the
routed colors and the spacer-tail behavior.

Review artifacts:

- Prompt: `logs/codex-review/20260603-202419-299355-prompt.md` (design)
- Result: `logs/codex-review/20260603-202419-299355-last-message.md` (design)

## Result

**Result:** Pass

Both row passes now route through the `selected_colors` dispatcher.

- `roastty/src/renderer/cell.rs`:
  - `selected_state(selection, x, wide) -> Selected` — wraps `is_selected`,
    yielding `Selection` (in bounds) or `False` (the search states deferred —
    their per-row ranges are not yet plumbed).
  - `rebuild_bg_row`: derives `state = selected_state(…)`; the color is
    `selected_colors(state, cell.style, default_fg, default_bg, palette, bold, selection_config).unwrap_or_else(|| cell_colors(…))`;
    the `bg_alpha` opaque branch keys on `selected = state != Selected::False`.
  - `rebuild_row`'s `fg_colors` builder: the foreground is
    `selected_colors(state, …).map(|c| c.fg).unwrap_or_else(|| cell_colors(…).fg)`;
    the faint alpha is unchanged.
  - The doc comments now describe the `selected_state` / `selected_colors` path.
    No signatures change; after this the passes no longer call
    `selection_colors` directly (it is reached only via `selected_colors`).

Test (in `cell.rs`): `selected_state_yields_selection_or_false` — `None` bounds
→ `False`; inside `[1, 3]` → `Selection` (at start and end), outside → `False`;
a spacer tail at `end + 1` → `Selection` (its `x_compare = end`), a narrow cell
at the same column → `False`.

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2846 passed, 0 failed (+1, no regressions; the 13
  existing rebuild tests pass unchanged, confirming identical routed colors).
- `cargo build -p roastty` → no warnings (`selection_colors` is still used via
  `selected_colors`).
- No-`ghostty`-name gates (font + renderer) clean; `git diff --check` clean.

## Conclusion

The row passes now mirror upstream's single `selected` dispatch: each cell's
`selected_state` drives both its background and foreground through
`selected_colors`, with `False` falling back to `cell_colors` (covering twist
intact). Behavior is identical to Experiments 386–387 (only `Selection`/`False`
are produced), but the production color path is now the enum dispatcher — so
adding search recoloring later changes only `selected_state` (and its input, the
per-row search highlight ranges).

The remaining renderer-bridge work: deriving the `Search`/`SearchSelected`
states from per-row search highlight ranges (adding them to `RunOptions`/the
shaper and having `selected_state` consult them); the lock-cursor glyph +
under-cursor text recolor; the column-ordered decoration merge + link
double-underline; and the **Metal upload** of `Contents`.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed the implementation is behavior-preserving and
matches the approved design: `selected_state` wraps `is_selected` exactly,
producing only `Selection` or `False` while the search states remain deferred;
both passes route through `selected_colors`, and the `False` path uses
`unwrap_or_else` to lazily call `cell_colors`, preserving the
covering/full-block twist only when the base path is needed; for selected cells
`selected_colors(Selected::Selection, …)` is equivalent to the prior direct
`selection_colors(…, config.background, config.foreground)` call; and the
`bg_alpha` branch keying on `state != Selected::False` is equivalent to the
previous `selected` bool for the current state set. It confirmed no signatures
changed, no call-site churn occurred, and `selection_colors` remains used via
`selected_colors` plus tests; that the new `selected_state` test covers
`Selection`/`False`/spacer-tail while the existing rebuild selection tests
passing confirms the routed colors stayed identical; and that the diff is
internal Rust only (no public C ABI/header change). Nothing needed to change
before the result commit.

Review artifacts:

- Result review: `logs/codex-review/20260603-202643-889475-last-message.md`
