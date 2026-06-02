# Experiment 203: Port Kitty Terminal Render Placement ABI

## Description

Experiment 202 added the terminal-side foundation for Kitty Unicode virtual
placements: Roastty can now scan visible terminal cells, decode placeholder
runs, and return internal `VirtualPlacement` records. That is still not enough
for an app renderer. The existing public Kitty graphics ABI is storage-backed:
`roastty_kitty_graphics_get(... PLACEMENT_ITERATOR ...)` iterates placements
stored in `ImageStorage`. That path can report non-virtual, pinned placements,
but it cannot report the concrete visible placements created by Unicode
placeholder cells.

This experiment ports the next coherent render boundary slice: a terminal-scoped
Kitty render placement iterator. The iterator snapshots the active terminal's
currently renderable Kitty placements, including:

- existing storage-backed, pinned placements from Experiments 196-197;
- visible Unicode virtual placements decoded from terminal cells by
  Experiment 202.

This experiment is ABI and geometry only. It does not add Metal rendering, GPU
upload, image compositing, texture lifetime management, or Swift app drawing. It
also does not replace the storage-backed placement iterator; that iterator
remains useful for storage inspection. The new iterator is the renderer-facing
surface because virtual placements are terminal-scoped, not storage-scoped.

Use upstream behavior as the reference:

- `vendor/ghostty/src/terminal/c/kitty_graphics.zig`
- `vendor/ghostty/src/terminal/kitty/graphics_render.zig`
- `vendor/ghostty/src/terminal/kitty/graphics_unicode.zig`
- `vendor/ghostty/src/terminal/kitty/graphics_storage.zig`
- `vendor/ghostty/src/terminal/Terminal.zig`

All public names must use Roastty naming.

## Changes

1. Add public terminal render placement ABI types in
   `roastty/include/roastty.h`.

   Add a new opaque iterator handle:

   ```c
   typedef void* roastty_kitty_graphics_render_placement_iterator_t;
   ```

   Add a data enum for one selected render placement:

   ```c
   typedef enum {
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_INVALID = 0,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_IMAGE_ID = 1,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_PLACEMENT_ID = 2,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_IS_VIRTUAL = 3,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_VIRTUAL_ROW = 4,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_VIRTUAL_COL = 5,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_SOURCE_X = 6,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_SOURCE_Y = 7,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_SOURCE_WIDTH = 8,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_SOURCE_HEIGHT = 9,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_GRID_COLS = 10,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_GRID_ROWS = 11,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_VIEWPORT_COL = 12,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_VIEWPORT_ROW = 13,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_X_OFFSET = 14,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_Y_OFFSET = 15,
     ROASTTY_KITTY_GRAPHICS_RENDER_PLACEMENT_DATA_Z = 16,
   } roastty_kitty_graphics_render_placement_data_e;
   ```

   Add a render-placement-specific info struct. Do not reuse
   `roastty_kitty_graphics_placement_render_info_s` for this new ABI because
   that older storage-backed struct lacks destination pixel offsets.

   ```c
   typedef struct {
     size_t size;
     uint32_t image_id;
     uint32_t placement_id;
     bool is_virtual;
     uint32_t x_offset;
     uint32_t y_offset;
     uint32_t pixel_width;
     uint32_t pixel_height;
     uint32_t grid_cols;
     uint32_t grid_rows;
     int32_t viewport_col;
     int32_t viewport_row;
     bool viewport_visible;
     uint32_t source_x;
     uint32_t source_y;
     uint32_t source_width;
     uint32_t source_height;
     int32_t z;
   } roastty_kitty_graphics_render_placement_info_s;
   ```

   Reuse `roastty_kitty_placement_layer_e` for layer filtering. Do not duplicate
   layer enum values.

   Add functions:

   ```c
   roastty_result_e roastty_kitty_graphics_render_placement_iterator_new(
       roastty_kitty_graphics_render_placement_iterator_t*);

   void roastty_kitty_graphics_render_placement_iterator_free(
       roastty_kitty_graphics_render_placement_iterator_t);

   roastty_result_e roastty_kitty_graphics_render_placement_iterator_set(
       roastty_kitty_graphics_render_placement_iterator_t,
       roastty_kitty_graphics_placement_iterator_option_e,
       const void*);

   roastty_result_e roastty_kitty_graphics_render_placement_iterator_update(
       roastty_kitty_graphics_render_placement_iterator_t,
       roastty_terminal_t);

   bool roastty_kitty_graphics_render_placement_next(
       roastty_kitty_graphics_render_placement_iterator_t);

   roastty_result_e roastty_kitty_graphics_render_placement_get(
       roastty_kitty_graphics_render_placement_iterator_t,
       roastty_kitty_graphics_render_placement_data_e,
       void*);

   roastty_result_e roastty_kitty_graphics_render_placement_get_multi(
       roastty_kitty_graphics_render_placement_iterator_t,
       size_t,
       const roastty_kitty_graphics_render_placement_data_e*,
       void**,
       size_t*);

   roastty_kitty_graphics_image_t roastty_kitty_graphics_render_placement_image(
       roastty_kitty_graphics_render_placement_iterator_t);

   roastty_result_e roastty_kitty_graphics_render_placement_render_info(
       roastty_kitty_graphics_render_placement_iterator_t,
       roastty_kitty_graphics_render_placement_info_s*);
   ```

   The image accessor returns the same owned image snapshot handle type used by
   storage-backed placements. Callers free it with
   `roastty_kitty_graphics_image_free`. The image comes from the iterator's
   update-time snapshot, not from a later terminal lookup.

2. Add internal render placement snapshots in `roastty/src/lib.rs`.

   Add an internal enum that distinguishes the two renderable sources:
   - pinned storage placement: resolved render geometry plus image snapshot;
   - visible virtual placement: resolved render geometry plus image snapshot.

   Snapshot all renderer-facing geometry during
   `roastty_kitty_graphics_render_placement_iterator_update`. After update,
   selected-entry getters, image access, and render-info access must not re-read
   terminal state and must not dereference copied
   `PlacementLocation::Pin(NonNull<Pin>)` pointers. This is stricter than the
   older storage iterator: the render iterator is a frame snapshot.

   For virtual placements, match the decoded `(image_id, placement_id)` against
   `ImageStorage` using a `PlacementKey` with `PlacementId::External` when the
   decoded placement id is nonzero and the stored placement is virtual. Non-
   virtual stored placements must not satisfy a virtual placeholder match.

   If the decoded placement id is zero, match the first virtual placement for
   that image using a deterministic Roastty helper:
   - collect all placements for the decoded image id where
     `placement.location == PlacementLocation::Virtual`;
   - sort by placement key, with `PlacementId::Internal(n)` before
     `PlacementId::External(n)`, and numeric ids ascending within each kind;
   - select the first sorted entry.

   This mirrors upstream's "first virtual placement for this image" behavior
   while avoiding `HashMap` iteration-order nondeterminism. If no stored virtual
   placement or image exists, skip that decoded placeholder run instead of
   returning a partially renderable item.

   The virtual snapshot must combine:
   - image id and placement id from decoded cell/style data;
   - image bytes and image metadata from the matched image;
   - placement columns, rows, z, offsets, and source defaults from the matched
     stored virtual placement;
   - viewport row/col from the decoded placeholder run's visible terminal
     position;
   - virtual source row/col and run width from the decoded placeholder run.

3. Define ordering and filtering.

   On update, build a stable snapshot in renderer discovery order:
   - include pinned, non-virtual storage placements that are currently visible;
   - include visible virtual placements decoded from the terminal viewport;
   - return entries in deterministic order.

   Do not apply the layer filter during update. Snapshot all renderable entries,
   store the current layer filter on the iterator, and apply that filter during
   `roastty_kitty_graphics_render_placement_next`, matching the existing
   storage-backed placement iterator. Changing the layer filter with
   `roastty_kitty_graphics_render_placement_iterator_set` resets selection but
   does not require another terminal update to broaden or narrow the existing
   snapshot.

   Deterministic order for this experiment:
   - pinned entries are sorted by placement key, not by `HashMap` iteration
     order: `image_id` ascending, then `PlacementId::Internal(n)` before
     `PlacementId::External(n)`, then numeric placement id ascending;
   - virtual entries keep Experiment 202's top-to-bottom, left-to-right visible
     order;
   - the final combined list is sorted by effective `z` ascending, then source
     group (`pinned` before `virtual`), then discovery order.

   This gives renderers a stable order without pretending to implement final GPU
   compositor batching.

4. Implement render-info for render placements.

   `roastty_kitty_graphics_render_placement_render_info` fills the new
   `roastty_kitty_graphics_render_placement_info_s`.

   For pinned placements, reuse the same geometry logic as
   `roastty_kitty_graphics_placement_render_info` during iterator update, then
   read from the render placement snapshot instead of a storage iterator or live
   terminal. Include stored destination `x_offset` and `y_offset` in the
   snapshot.

   For virtual placements, port the renderer-independent geometry math from
   `graphics_unicode.zig::Placement.renderPlacement`:
   - calculate the virtual placement grid from the matched stored virtual
     placement: stored `rows`/`columns` when nonzero, otherwise ceil image
     height/width by cell height/width;
   - scale the full image into that placement grid while preserving aspect
     ratio;
   - use decoded virtual `row`, `col`, `width`, and `height` to slice the scaled
     image into the source rectangle for the visible placeholder run;
   - compute destination pixel `x_offset`, `y_offset`, `pixel_width`, and
     `pixel_height`, including aspect-ratio padding/clipping;
   - when the sliced source rectangle has no positive width or height, skip the
     virtual placement during iterator update;
   - round output values the same way upstream does for render placements;
   - set `grid_cols` and `grid_rows` to the decoded visible run width/height,
     currently height 1;
   - set `viewport_col` and `viewport_row` to the decoded placeholder run's
     visible terminal position;
   - set `viewport_visible = true` for entries produced by the visible iterator;
   - avoid tracked pins entirely for virtual placements.

   Validate `out->size` before writing and do not partially mutate undersized
   structs.

5. Preserve storage-backed ABI behavior.

   Do not change the existing `roastty_kitty_graphics_placement_iterator_*`
   functions except for shared helper refactoring that leaves their observable
   behavior and tests unchanged. The old iterator remains storage-backed and may
   still expose virtual placement definitions as storage records. The new render
   iterator is the only ABI that exposes concrete visible Unicode virtual
   placements.

6. Add Rust tests in `roastty/src/lib.rs`.

   Add a `kitty_graphics_render_placement_c_abi` test group covering:
   - empty terminal update returns no render placements;
   - a direct transmitted pinned image appears through the render iterator;
   - storage-backed render info matches the existing storage iterator render
     info for the same pinned placement, plus stored x/y offsets from the pinned
     display command;
   - a stored virtual placement alone does not appear until placeholder cells
     are printed;
   - placeholder cells without a matching image or stored virtual placement are
     skipped;
   - virtual placeholder matching by explicit external placement id;
   - virtual placeholder id zero selects the deterministic first stored virtual
     placement for that image;
   - non-virtual stored placements are ignored by virtual placeholder matching;
   - a matching virtual placeholder appears with decoded image id, placement id,
     virtual row/col, viewport row/col, grid width, grid height, source rect,
     destination pixel size, destination offsets, and z;
   - virtual geometry for nonzero row/col, multi-cell run width, aspect-ratio
     padding/clipping, and out-of-grid/no-op cases matches upstream-derived
     expectations;
   - layer filtering includes and excludes both pinned and virtual placements;
   - changing the layer filter after update can broaden the same snapshot again
     without another update;
   - deterministic ordering covers two same-z pinned placements plus one virtual
     placement and does not depend on `HashMap` iteration order;
   - update snapshots are stable if the terminal later mutates, including after
     placement deletion, image deletion, scrollback changes, and placeholder
     overwrites;
   - invalid handles, null output pointers, invalid enum values, and undersized
     render-info structs return `ROASTTY_INVALID_VALUE`;
   - iterators with no selected entry return `ROASTTY_NO_VALUE` for selected
     entry accessors.

7. Extend the C ABI harness in `roastty/tests/abi_harness.c`.

   Add a smoke test that:
   - creates and frees the new iterator;
   - updates it from a terminal;
   - iterates at least one pinned render placement;
   - calls `get`, `get_multi`, `render_placement_image`, and
     `render_placement_render_info`;
   - checks the new render-info struct size, alignment, offsets, and enum values
     for new public ABI additions.

8. Preserve formatting and review rules.

   Run:

   ```bash
   cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs roastty/src/terminal/screen.rs
   prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/203-port-kitty-terminal-render-placement-abi.md
   ```

## Verification

Run:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/terminal.rs roastty/src/terminal/screen.rs
cargo test -p roastty kitty_graphics_render_placement_c_abi
cargo test -p roastty kitty_graphics_render_info_c_abi
cargo test -p roastty terminal_stream_kitty_virtual_placeholder
cargo test -p roastty --test abi_harness
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The experiment passes when:

- the public ABI exposes a terminal-scoped render placement iterator;
- pinned storage placements remain renderable through the new iterator;
- visible Unicode virtual placeholders become concrete render placements only
  when they match an existing image and stored virtual placement;
- render-info geometry for pinned placements matches the existing ABI;
- render-info geometry for virtual placements uses decoded visible-cell position
  and stored image/placement metadata without tracked pins, including upstream-
  equivalent aspect-ratio slicing and destination offsets;
- the existing storage-backed placement iterator behavior is unchanged;
- the C ABI harness covers the new public types/functions;
- Codex approves the design before implementation and approves the completed
  result before it is recorded.

## Non-Negotiable Invariants

- Do not add GPU rendering, Metal, texture upload, Swift app drawing, or final
  compositor batching in this experiment.
- Do not remove or repurpose the existing storage-backed Kitty placement
  iterator.
- Do not expose storage-only virtual placement definitions as if they were
  concrete visible render placements.
- Do not dereference tracked-pin pointers copied into stale snapshots; render
  iterator update must resolve pinned geometry into stable snapshot values.
- Do not return virtual render placements for placeholder cells that lack a
  matching image and stored virtual placement.
- Do not simplify virtual render placement geometry to existing storage
  placement geometry; port the renderer-independent upstream slicing/offset
  math.
- Do not expose any `ghostty_*` ABI names.
- Do not skip Codex design review or Codex result review.

## Result

**Result:** Pass

Roastty now exposes a terminal-scoped Kitty render placement ABI:

- `roastty_kitty_graphics_render_placement_iterator_t` snapshots renderable
  Kitty placements from a terminal frame.
- Pinned storage-backed placements are resolved into stable update-time
  snapshots, including geometry, image data, offsets, source rectangles,
  viewport position, and z.
- Visible Unicode virtual placeholders are matched against stored virtual
  placements, converted into render placements, and exposed through the same
  iterator.
- Virtual render geometry ports the renderer-independent upstream slicing logic:
  aspect-ratio fit, source clipping, destination offsets, destination pixel
  size, no-op slices, and visible-cell viewport position.
- Layer filtering happens during `next`, so changing the layer after update can
  broaden or narrow the same snapshot without another terminal update.
- The storage-backed Kitty placement iterator remains unchanged.
- The C ABI harness now covers the new render-placement handle, data selectors,
  info struct layout, image accessor, render-info accessor, and undersized
  struct validation.

The implementation also fixed a pre-existing parallel-test collision in Kitty
image tests by replacing timestamp-derived temporary directory and shared memory
names with a process-local atomic counter.

Verification run:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/kitty/graphics_unicode.rs roastty/src/terminal/kitty/graphics_image.rs roastty/src/terminal/terminal.rs roastty/src/terminal/page_list.rs
cargo test -p roastty kitty_graphics_render_placement_c_abi
cargo test -p roastty kitty_graphics_render_info_c_abi
cargo test -p roastty terminal_stream_kitty_virtual_placeholder
cargo test -p roastty --test abi_harness
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

All commands passed.

Codex reviewed the completed implementation twice. The first review found a real
test-coverage blocker for virtual matching, aspect clipping, no-op slices, and
snapshot stability. The missing tests were added, and the second review found no
remaining correctness, regression, public ABI layout/safety, or scope issues.
Codex approved recording the experiment as Pass.

## Conclusion

Experiment 203 gives app/render code a correct terminal-scoped Kitty render
placement snapshot boundary. The next experiment can build on this ABI to move
toward the actual renderer/app consumption path for Kitty graphics, without
reopening storage-only placement iteration or Unicode placeholder decoding.
