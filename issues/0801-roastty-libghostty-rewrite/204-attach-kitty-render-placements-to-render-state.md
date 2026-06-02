# Experiment 204: Attach Kitty Render Placements to Render State

## Description

Experiment 203 added a terminal-scoped Kitty render placement iterator. That was
the right diagnostic and ABI boundary for proving the combined pinned +
Unicode-placeholder placement logic, but it still requires an app renderer to
query two separate frame surfaces:

- `roastty_render_state_update(...)` for terminal rows, cells, colors, cursor,
  selection, and other render-state data;
- `roastty_kitty_graphics_render_placement_iterator_update(...)` for Kitty image
  placements.

Upstream Ghostty's renderer does not treat Kitty graphics as an unrelated
terminal-inspection path. The renderer builds its per-frame image placement list
from the terminal while preparing renderer state:

- `vendor/ghostty/src/renderer/image.zig::kittyRequiresUpdate`
- `vendor/ghostty/src/renderer/image.zig::kittyUpdate`
- `vendor/ghostty/src/renderer/image.zig::prepKittyPlacement`
- `vendor/ghostty/src/renderer/image.zig::prepKittyVirtualPlacement`
- `vendor/ghostty/src/terminal/kitty/graphics_render.zig`
- `vendor/ghostty/src/terminal/kitty/graphics_unicode.zig`

Roastty does not have the Metal renderer yet, but it does have a public
`render_state` snapshot ABI. This experiment ports the next coherent renderer
boundary slice: make Kitty render placements available from the same update-time
`roastty_render_state_t` snapshot that already owns rows, cells, colors, and
cursor data.

This is still not a drawing experiment. Do not add Metal, Swift, texture upload,
image cache management, compositor batching, or app rendering. The goal is one
stable frame snapshot surface that a future renderer can consume without
re-reading terminal state.

All public names must use Roastty naming.

## Changes

1. Extend the render-state snapshot in `roastty/src/lib.rs`.

   Add Kitty render placements to `RenderStateScalar`:

   ```rust
   kitty_render_placements: Vec<KittyGraphicsRenderPlacementSnapshot>,
   ```

   Populate this field inside `render_state_from_terminal(...)` using the same
   internal snapshot builder added in Experiment 203. Do not duplicate placement
   geometry logic. Factor the Experiment 203 update path into a helper that both
   the standalone Kitty render placement iterator and render-state update can
   call.

   The render-state copy must preserve the same update-time guarantees from
   Experiment 203:
   - selected-entry getters never re-read terminal state;
   - image handles come from update-time image snapshots;
   - copied pin locations are resolved during update and no stale `Pin` pointer
     is dereferenced after update;
   - virtual placements use the deterministic id-zero matching rule from
     Experiment 203;
   - layer filtering is not applied while building the render-state snapshot.

2. Add a render-state data selector in `roastty/include/roastty.h`.

   Add one new enum value at the end of `roastty_render_state_data_e`:

   ```c
   ROASTTY_RENDER_STATE_DATA_KITTY_RENDER_PLACEMENT_ITERATOR = 18,
   ```

   Do not renumber existing render-state data values.

   When
   `roastty_render_state_get(state, ROASTTY_RENDER_STATE_DATA_KITTY_RENDER_PLACEMENT_ITERATOR, out)`
   is called:
   - `out` points to a `roastty_kitty_graphics_render_placement_iterator_t`;
   - the iterator must already have been created by
     `roastty_kitty_graphics_render_placement_iterator_new`;
   - the call binds the iterator to a copy of the render state's
     `kitty_render_placements`;
   - selection resets to the start;
   - the iterator's current layer filter is preserved and applied by
     `roastty_kitty_graphics_render_placement_next`, matching Experiment 203's
     standalone iterator behavior.

   This mirrors the existing row iterator pattern:
   `ROASTTY_RENDER_STATE_DATA_ROW_ITERATOR` binds a separately-created iterator
   to the render-state row snapshot.

   The iterator must own its bound snapshot after this call. It must remain
   valid after the source `roastty_render_state_t` is updated again or freed,
   matching the existing render-state row iterator clone pattern.

3. Keep the standalone terminal-scoped Kitty render placement iterator.

   Do not remove or deprecate the functions added by Experiment 203. They remain
   useful for focused Kitty graphics tests and direct terminal inspection.

   The standalone update path and the render-state path must share the same
   internal snapshot builder so their results are byte-for-byte equivalent for a
   terminal at the same state.

4. Update C ABI layout checks in `roastty/tests/abi_harness.c`.

   Add assertions for:
   - the new `ROASTTY_RENDER_STATE_DATA_KITTY_RENDER_PLACEMENT_ITERATOR` value;
   - binding a created Kitty render placement iterator through
     `roastty_render_state_get`;
   - invalid binding cases: null state, null output, null iterator handle, and
     output pointing at an uninitialized/null iterator.

5. Add Rust tests in `roastty/src/lib.rs`.

   Cover at least:
   - render-state update includes pinned Kitty render placements;
   - render-state update includes visible Unicode virtual placements;
   - render-state-bound iterator preserves the iterator's layer filter and can
     broaden/narrow without another terminal update;
   - render-state-bound iterator snapshot survives later terminal mutation and
     terminal free;
   - render-state-bound iterator snapshot survives freeing the
     `roastty_render_state_t` it was bound from;
   - render-state-bound iterator output matches the standalone terminal-scoped
     iterator output for the same terminal state;
   - invalid handles and output pointers return `ROASTTY_INVALID_VALUE`.

   Tests should reuse the existing Kitty graphics setup helpers where possible.
   Do not create unrelated renderer/app test infrastructure in this experiment.

6. Verification commands.

   Run:

   ```bash
   cargo fmt -- roastty/src/lib.rs
   cargo test -p roastty kitty_graphics_render_placement_c_abi
   cargo test -p roastty render_state
   cargo test -p roastty --test abi_harness
   cargo test -p roastty
   if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
   git diff --check
   ```

   `cargo fmt` is required for Rust edits. Accept formatter output as-is. The C
   header and C harness are checked by the ABI harness and `git diff --check`;
   there is no project C formatter in this experiment.

## Non-Negotiable Invariants

- Do not expose `ghostty_*` symbols or comments in the Roastty public ABI.
- Do not modify the vendored Ghostty source.
- Do not add Metal rendering, Swift app rendering, texture upload, image cache
  management, or compositor batching.
- Do not remove Experiment 203's standalone terminal-scoped Kitty render
  placement iterator.
- Do not duplicate Kitty placement geometry logic between render state and the
  standalone iterator.
- Do not make render-state-bound iterators depend on the terminal remaining
  alive after `roastty_render_state_update`.
- Do not make render-state-bound iterators depend on the source render-state
  handle remaining alive after binding.
- Do not renumber existing C ABI enum values.

## Pass Criteria

- `roastty_render_state_update` snapshots Kitty render placements along with
  rows, cells, colors, and cursor data.
- A created Kitty render placement iterator can be bound through
  `ROASTTY_RENDER_STATE_DATA_KITTY_RENDER_PLACEMENT_ITERATOR`.
- Render-state-bound Kitty placement iteration returns the same entries as the
  standalone terminal-scoped iterator for the same terminal state.
- Layer filtering behaves the same after render-state binding as it does for
  Experiment 203's standalone iterator.
- Render-state-bound placement snapshots survive later terminal mutation and
  terminal free.
- Render-state-bound placement snapshots survive freeing the render-state handle
  they were bound from.
- Full verification passes, including the public no-`ghostty` gate.

## Failure Criteria

- The implementation re-reads terminal state from render-state-bound placement
  getters or image accessors.
- The render-state path and standalone iterator path have divergent geometry or
  ordering logic.
- The implementation adds renderer/app drawing behavior instead of keeping this
  to the render-state ABI boundary.
- Existing render-state row/cell/color/cursor tests regress.
- Existing Kitty render placement tests from Experiment 203 regress.

## Result

**Result:** Pass

Implemented the render-state Kitty placement binding as designed.

Changes made:

- added `ROASTTY_RENDER_STATE_DATA_KITTY_RENDER_PLACEMENT_ITERATOR = 18` without
  renumbering existing render-state selectors;
- added `kitty_render_placements` to the update-time `RenderStateScalar`
  snapshot;
- populated that field through the existing Experiment 203 Kitty render
  placement snapshot builder, so the standalone iterator and render-state path
  share the same geometry and ordering logic;
- taught `roastty_render_state_get` to bind a created
  `roastty_kitty_graphics_render_placement_iterator_t` to a cloned render-state
  Kitty placement snapshot;
- preserved the iterator's current layer filter while resetting selection on
  bind;
- updated the C ABI harness for the new enum value and empty-snapshot binding
  path;
- added Rust tests for pinned placements, virtual placements, layer filter
  broadening, snapshot lifetime after terminal/render-state mutation and free,
  standalone-vs-render-state equivalence, and invalid binding cases.

Verification passed:

```bash
cargo fmt -- roastty/src/lib.rs
cargo test -p roastty kitty_graphics_render_placement_c_abi
cargo test -p roastty render_state
cargo test -p roastty --test abi_harness
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

Codex result review passed with no blocking findings. It confirmed that the new
selector is appended safely, render-state binding clones the placement snapshot
into the already-created iterator, the iterator lifetime is covered, layer
filter behavior is preserved, and the experiment satisfies the design.

## Conclusion

Roastty now exposes Kitty image render placements through the same
`roastty_render_state_t` frame snapshot used for rows, cells, colors, and cursor
state. Future renderer/app work can consume one render-state boundary instead of
querying terminal rows and Kitty placements separately.

The standalone terminal-scoped Kitty render placement iterator remains in place
for focused tests and direct terminal inspection, but both paths now share the
same snapshot builder. The next experiment can move further toward renderer
consumption: either image renderer state/cache prep modeled on upstream
`renderer/image.zig`, or another missing render-state surface if inspection
shows one should precede image upload/cache work.
