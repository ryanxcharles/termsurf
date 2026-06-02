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

# Experiment 223: Port Renderer Cursor Style

## Description

Port Ghostty's `renderer/cursor.zig` into Roastty as a new `renderer::cursor`
module. This is the next slice after the offscreen Metal `cell_text`
cursor-color read-back work (Experiments 216–222), whose conclusion explicitly
deferred "cursor shape rendering" to a later experiment. It is the first
experiment implemented by Claude (Opus 4.8, high) under the new agent
arrangement; Codex reviews both gates.

Upstream `renderer/cursor.zig` is a small, pure-logic module with two public
pieces:

- a renderer-side `Style` enum (`block`, `block_hollow`, `bar`, `underline`,
  `lock`) — a superset of the terminal cursor styles, plus `from_terminal` that
  maps a `terminal.CursorStyle` to a renderer `Style`;
- a `style(state, opts)` function that returns the cursor `Style` to draw, or
  `null` when no cursor should be drawn, using a fixed **priority order** of
  conditions.

The priority order in `style()` is the behavior that matters and must be ported
exactly:

1. cursor not in the viewport → `null`;
2. preedit → always `block` (even if otherwise not visible);
3. password input → `lock`;
4. cursor not visible (terminal mode) → `null`;
5. not focused → `block_hollow`;
6. blinking and blink state not visible → `null`;
7. otherwise → `from_terminal(visual_style)`.

This fits the issue's risk-based sizing rule: one coherent surface (a new
`renderer/cursor.rs`), predictable tests (four upstream tests port directly plus
direct-constructed render states for the two branches not yet reachable from a
real terminal), one novel mechanism (the priority-ordered style selection), and
localized failure.

### Mapping to Roastty's render state

Upstream reads
`state.cursor.{viewport, password_input, visible, blinking, visual_style}` from
`terminal.RenderState`. Roastty's render state is the crate-root
`RenderStateScalar` (in `roastty/src/lib.rs`), which already carries the exact
fields needed:

| Upstream `state.cursor.*` | Roastty `RenderStateScalar.*`            |
| ------------------------- | ---------------------------------------- |
| `viewport == null`        | `cursor_viewport: Option<...>` is `None` |
| `password_input`          | `cursor_password_input: bool`            |
| `visible`                 | `cursor_visible: bool`                   |
| `blinking`                | `cursor_blinking: bool`                  |
| `visual_style`            | `cursor_visual_style: c_int`             |

`cursor_visual_style` is stored as a `c_int` (an ABI divergence introduced when
the render state C ABI was ported). The integer encoding already has a single
source of truth in `lib.rs`:

- `ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BAR = 0`
- `ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK = 1`
- `ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_UNDERLINE = 2`
- `ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_BLOCK_HOLLOW = 3`

`renderer::cursor::style()` maps that integer to the renderer `Style` using
these existing constants (no new encoding). `RenderStateScalar` is private to
the crate root, so a descendant `renderer::cursor` module can read it directly.

### Scope and deliberate limits

- **Reachable vs. unreachable branches.** `cursor_password_input` is currently
  hardcoded `false` in `render_state_from_terminal`, and viewport scroll-away
  nulling is not yet modeled (Roastty's `cursor_viewport` is `None` only when
  the cursor column/row is out of the grid bounds, not when the viewport is
  scrolled away from the cursor). The `lock` (password) branch and the
  `viewport == None` branch are therefore **logic-complete but not reachable
  from a real terminal yet**. They are still ported and tested by constructing a
  `RenderStateScalar` directly. A note records that wiring `password_input` and
  scroll-away viewport nulling into the real render state are future
  experiments.
- This experiment does **not** render the cursor geometry/quads, blink timing,
  selection/reverse-video render ordering, or any C ABI. It ports only the pure
  `style()` decision and the `Style`/`from_terminal` types.

## Changes

1. Create `roastty/src/renderer/cursor.rs`.
   - Define `pub(crate) enum Style { Block, BlockHollow, Bar, Underline, Lock }`
     (derive `Debug, Clone, Copy, PartialEq, Eq`).
   - Define
     `pub(crate) struct StyleOptions { pub preedit: bool, pub focused: bool, pub blink_visible: bool }`
     with a `Default` that is all-`false`, matching upstream `StyleOptions`
     defaults.
   - Implement `Style::from_terminal(terminal::cursor::VisualStyle) -> Style`,
     mapping `Bar→Bar`, `Block→Block`, `BlockHollow→BlockHollow`,
     `Underline→Underline`. (Roastty's `VisualStyle` is the same superset as
     upstream `terminal.CursorStyle`, including `BlockHollow`.)
   - Implement a private mapping from the stored `cursor_visual_style: c_int` to
     `Style`, reusing the `ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_*` constants
     from `lib.rs` as the single source of truth. An unrecognized integer must
     not panic in non-test builds; fall back to `Style::Block` (the default
     cursor) and assert/`debug_assert` the value is known so a regression is
     caught in tests.
   - Implement
     `style(state: &crate::RenderStateScalar, opts: StyleOptions) -> Option<Style>`
     reproducing the upstream priority order exactly (the seven steps above, in
     that order).

2. Wire the module from `roastty/src/renderer/mod.rs`.
   - Add `pub(crate) mod cursor;` (or `mod cursor;` if no cross-module use is
     needed yet). Keep it internal; do not expose any C ABI or public Rust API.

3. Make the needed `lib.rs` items reachable from the renderer module.
   - Ensure `RenderStateScalar`, `RenderStateCursorViewport`, the
     `ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_*` constants, and
     `render_state_from_terminal` are visible to `renderer::cursor` (they are
     crate-root-private, so descendant modules already have access; add
     `pub(crate)` only if the compiler requires it for a specific item, and no
     wider).
   - Do not change the meaning, ABI, or existing call sites of any of these.

4. Port the upstream tests into `roastty/src/renderer/cursor.rs`.
   - Terminal-driven tests, built via an `InnerTerminal` plus
     `render_state_from_terminal`, mirroring upstream test setup:
     - `cursor_default_uses_configured_style` (upstream "default uses configured
       style"): bar style + blinking; focused+blink_visible → `Bar`; unfocused →
       `BlockHollow`; focused + not blink_visible → `None`.
     - `cursor_blinking_disabled` (upstream "blinking disabled"): bar style,
       blinking off; focused → `Bar` regardless of blink_visible; unfocused →
       `BlockHollow`.
     - `cursor_explicitly_not_visible` (upstream "explicitly not visible"):
       `cursor_visible = false` → `None` in all option combinations.
     - `cursor_always_block_with_preedit` (upstream "always block with
       preedit"): preedit → `Block` in all option combinations.
   - Direct-render-state tests for the branches not reachable from the real
     terminal yet:
     - `cursor_password_input_is_lock`: a `RenderStateScalar` with
       `cursor_password_input = true` → `Lock`, and confirm it takes priority
       over `visible = false` but not over `preedit`.
     - `cursor_absent_viewport_is_none`: a `RenderStateScalar` with
       `cursor_viewport = None` → `None` for every option combination, including
       `preedit = true` (viewport check is first in the priority order).
   - Preserve upstream test intent and names closely enough that the source test
     is obvious.

5. Keep scope narrow.
   - Do not port cursor geometry, blink timing, `Overlay`, `cell.zig`,
     `generic.zig`, or any renderer presentation code.
   - Do not add or change any C ABI, header, or ABI inventory entry.
   - Do not add dependencies.

6. Format and test.
   - Run `cargo fmt` after Rust edits and accept its output.

## Verification

Run:

```bash
cargo fmt
cargo test -p roastty renderer::cursor
cargo test -p roastty renderer
cargo test -p roastty
# no-ghostty-name gates
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/renderer/cursor.rs && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `renderer::cursor` implements `Style`, `StyleOptions`, `from_terminal`, and
  `style()` with the exact upstream priority order;
- the stored `cursor_visual_style` integer maps to the renderer `Style` via the
  existing `ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_*` constants;
- all four upstream behavior tests pass, plus the two direct-render-state tests
  for the `lock` and absent-viewport branches;
- no C ABI, header, or ABI inventory changes are made;
- `cargo fmt` is accepted and `cargo test -p roastty` passes with no
  regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, and all
  real findings are fixed.

The experiment is **partial** if:

- the reachable `style()` branches are ported and tested, but a branch turns out
  to need a render-state change (e.g., real `password_input` or scroll-away
  viewport) that should be its own prerequisite experiment.

The experiment **fails** if:

- the priority order diverges from upstream;
- cursor geometry, blink timing, or render ordering is pulled in;
- any public API or ABI changes;
- the integer→`Style` mapping duplicates or contradicts the existing constants;
- targeted Roastty tests cannot pass.

## Design Review

Codex reviewed this design before implementation and found **no issues**
(findings: none; "nothing should change before implementation").

Review artifacts:

- Prompt: `logs/codex-review/20260602-065725-707851-prompt.md`
- Result: `logs/codex-review/20260602-065725-707851-last-message.md`

Codex confirmed, reading the upstream `renderer/cursor.zig` and the named
Roastty sources, that:

- the ported `style()` priority order is faithful to upstream, including
  viewport-first and preedit-before-password;
- the visibility assumption holds — a descendant `renderer::cursor` module can
  read the crate-root-private `RenderStateScalar`, its fields,
  `RenderStateCursorViewport`, the `ROASTTY_RENDER_STATE_CURSOR_VISUAL_STYLE_*`
  constants, and `render_state_from_terminal` without widening visibility; the
  "add `pub(crate)` only if required" guidance is safe and minimal;
- mapping the stored `cursor_visual_style` integer back through the existing
  constants is correct and keeps the encoding single-sourced;
  fallback-to-`Block` paired with `debug_assert` is the right non-panicking
  behavior;
- the test set is sufficient and reproducible.

Implementation note adopted from the review: build the direct-render-state tests
from the existing `render_state_default()` helper (`lib.rs:1924`) and mutate the
cursor fields, to keep test setup compact.
