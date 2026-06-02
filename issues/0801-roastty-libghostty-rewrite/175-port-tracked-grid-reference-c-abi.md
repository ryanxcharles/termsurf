# Experiment 175: Port Tracked Grid Reference C ABI

## Description

Port the owned tracked-grid-reference ABI. Experiment 172 added borrowed
`roastty_grid_ref_s` snapshots, Experiment 173 added selection APIs that consume
grid refs, and Experiment 174 added gesture-owned tracked anchors. The next
coherent slice is upstream's public tracked grid-ref object:

- `vendor/ghostty/src/terminal/c/grid_ref_tracked.zig`

A tracked grid ref owns a tracked `PageList` pin. The pin follows page-list
mutation while its owning screen remains alive, can be snapshotted back into a
borrowed `roastty_grid_ref_s`, and reports `ROASTTY_NO_VALUE` after reset,
screen destruction, or terminal free.

This experiment must use Roastty names only in public symbols:

- `roastty_tracked_grid_ref_t`
- `roastty_tracked_grid_ref_*`

Upstream names may appear only in this issue document as source citations.

## Changes

1. Re-read upstream source:
   - `vendor/ghostty/src/terminal/c/grid_ref_tracked.zig`;
   - `vendor/ghostty/src/terminal/c/terminal.zig` for `grid_ref_track`;
   - `vendor/ghostty/src/terminal/c/grid_ref.zig` for snapshot layout;
   - current Roastty grid-ref, tracked-pin, selection, and selection-gesture
     code.

2. Add a terminal-owned tracked-ref registry in `roastty/src/lib.rs`.
   - Extend the private `Terminal` wrapper, not the internal terminal core, with
     a collection of live tracked-ref wrapper pointers.
   - On `roastty_terminal_free`, detach every live tracked ref from the terminal
     before freeing the terminal, so later tracked-ref calls return
     `ROASTTY_NO_VALUE` instead of dereferencing a stale terminal handle.
   - On `roastty_tracked_grid_ref_free`, remove the ref from the terminal
     registry if still attached.
   - Do not claim stale non-null `roastty_terminal_t` handles are safe in
     general. The safety guarantee is specifically: a tracked-ref handle remains
     safe after its owning terminal is freed because terminal free detaches it.

3. Add a private Rust tracked-ref wrapper.
   - Store:
     - owning terminal wrapper pointer, nullable after detach;
     - screen key;
     - screen generation;
     - screen owner id;
     - tracked pin pointer.
   - Validity requires:
     - terminal still attached;
     - referenced screen still exists;
     - screen generation still matches;
     - screen owner id still matches;
     - tracked pin is not garbage.
   - Cleanup uses screen key + owner id:
     - if the original screen object still owns the tracked pin, untrack it even
       if generation changed;
     - if the screen was destroyed/recreated and owner id changed, do not
       untrack through the new screen.
   - Reuse the screen identity helpers added in Experiment 174; extend them only
     if a tracked-ref-specific helper is necessary.

4. Add public C ABI in `roastty/include/roastty.h` and `roastty/src/lib.rs`:

   ```c
   typedef void* roastty_tracked_grid_ref_t;

   ROASTTY_API roastty_result_e roastty_terminal_grid_ref_track(
       roastty_terminal_t,
       roastty_point_s,
       roastty_tracked_grid_ref_t*);

   ROASTTY_API void roastty_tracked_grid_ref_free(
       roastty_tracked_grid_ref_t);

   ROASTTY_API bool roastty_tracked_grid_ref_has_value(
       roastty_tracked_grid_ref_t);

   ROASTTY_API roastty_result_e roastty_tracked_grid_ref_snapshot(
       roastty_tracked_grid_ref_t,
       roastty_grid_ref_s*);

   ROASTTY_API roastty_result_e roastty_tracked_grid_ref_point(
       roastty_tracked_grid_ref_t,
       roastty_point_tag_e,
       roastty_point_coordinate_s*);

   ROASTTY_API roastty_result_e roastty_tracked_grid_ref_set(
       roastty_tracked_grid_ref_t,
       roastty_terminal_t,
       roastty_point_s);
   ```

   Incoming enum values must be accepted as raw `int`/`c_int` in Rust and
   checked before conversion.

5. Define behavior:
   - `roastty_terminal_grid_ref_track`:
     - null terminal or null out pointer returns `ROASTTY_INVALID_VALUE`;
     - writes null to `*out` before validation/allocation;
     - invalid point returns `ROASTTY_INVALID_VALUE`;
     - allocation failure returns `ROASTTY_OUT_OF_MEMORY`;
     - success registers the tracked ref with the terminal wrapper and writes a
       non-null handle.
   - `roastty_tracked_grid_ref_free`:
     - null is a no-op;
     - unregisters from the terminal wrapper if still attached;
     - untracks its pin only from the original still-owned screen;
     - frees the tracked-ref wrapper exactly once.
   - `roastty_tracked_grid_ref_has_value`:
     - null returns `false`;
     - detached terminal, destroyed/recreated screen, generation mismatch, owner
       mismatch, or garbage pin returns `false`;
     - valid live pin returns `true`.
   - `roastty_tracked_grid_ref_snapshot`:
     - null ref returns `ROASTTY_INVALID_VALUE`;
     - no value returns `ROASTTY_NO_VALUE`;
     - null output is allowed and returns `ROASTTY_SUCCESS` when the ref has a
       value, matching upstream's "query validity without writing" shape;
     - non-null output writes a borrowed `roastty_grid_ref_s` snapshot.
   - `roastty_tracked_grid_ref_point`:
     - null ref returns `ROASTTY_INVALID_VALUE`;
     - invalid raw point tag returns `ROASTTY_INVALID_VALUE`;
     - no value returns `ROASTTY_NO_VALUE`;
     - null output is allowed and returns `ROASTTY_SUCCESS` when the ref can be
       represented in the requested coordinate space;
     - if the tracked pin is valid but not representable in the requested
       coordinate space, return `ROASTTY_NO_VALUE`.
   - `roastty_tracked_grid_ref_set`:
     - null ref or null terminal returns `ROASTTY_INVALID_VALUE`;
     - validate in this exact order:
       1. validate the tracked-ref handle itself;
       2. check whether the tracked ref is still attached to a terminal;
       3. compare the raw provided terminal handle to the stored raw owner
          handle;
       4. only after that comparison succeeds, dereference or convert the
          terminal wrapper;
     - terminal must equal the tracked ref's original terminal while attached;
     - detached tracked refs cannot be reattached to a stale or new terminal and
       return `ROASTTY_INVALID_VALUE`;
     - if the owning terminal was already freed and the caller passes the stale
       original `roastty_terminal_t` value back into `set`, return
       `ROASTTY_INVALID_VALUE` without dereferencing that stale terminal handle;
     - invalid point returns `ROASTTY_INVALID_VALUE`;
     - allocation failure returns `ROASTTY_OUT_OF_MEMORY`;
     - on success, track the new pin first, then untrack the old pin, then
       update screen key/generation/owner id/pin atomically.

6. Add Rust tests in `roastty/src/lib.rs` for:
   - ABI discriminants and exported function null handling;
   - `track` writes null on invalid input;
   - snapshot after terminal scroll preserves the original tracked cell;
   - `has_value`, `snapshot`, and `point` return no value after terminal reset;
   - alternate-screen destroy/recreate invalidates a tracked ref and does not
     untrack stale pins through the new alternate screen;
   - terminal free detaches tracked refs; post-free `has_value` is false,
     snapshot/point return `ROASTTY_NO_VALUE`, and tracked-ref free is safe;
   - `set` updates an attached tracked ref to a new active point;
   - `set` rejects null/mismatched/detached terminals;
   - `set` after terminal free with the stale original terminal handle returns
     `ROASTTY_INVALID_VALUE` without dereferencing the stale handle;
   - `snapshot(NULL out)` and `point(NULL out)` return success when the tracked
     ref has a value.

7. Add C harness coverage in `roastty/tests/abi_harness.c` for:
   - compile/link coverage for every new export;
   - lifecycle track/free;
   - snapshot after scroll;
   - point conversion;
   - no-value after reset;
   - safe free after terminal free;
   - `set` to a new point;
   - null output behavior for snapshot/point.

8. Preserve scope:
   - Do not add cell/row/style/grapheme/hyperlink extraction APIs in this
     experiment.
   - Do not add renderer integration, selection painting, or app/surface event
     plumbing.
   - Do not expose internal `PageList::Node` or dereference opaque node pointers
     from C-provided snapshots before membership is proven.
   - Do not add `ghostty_*` symbols or compatibility aliases.

## Verification

Run:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/page_list.rs roastty/src/terminal/screen.rs roastty/src/terminal/terminal.rs
cargo test -p roastty tracked_grid_ref
cargo test -p roastty terminal_grid_ref
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

Pass criteria:

- tracked refs follow page-list mutation while their owning screen remains live;
- tracked refs report no value after reset, alternate destroy/recreate, and
  terminal free;
- terminal free detaches tracked refs without requiring caller ordering;
- cleanup untracks only from the original still-owned screen object;
- `set` updates attached refs atomically;
- C header, Rust ABI tests, C harness, full `roastty` tests, no-Ghostty check,
  and `git diff --check` all pass;
- Codex reviews and approves both the design and the completed result.

Partial criteria:

- the tracked-ref object works for live terminals, but terminal-free detachment
  needs a different registry shape before it can be safely exposed.

Failure criteria:

- a tracked ref can dereference a freed terminal;
- terminal free leaves tracked refs with stale terminal pointers;
- cleanup untracks a stale pointer through a destroyed/recreated screen object;
- `set` loses the old tracked pin before successfully tracking the replacement;
- C enum values are represented as Rust enums before validation;
- null output behavior silently diverges from this experiment's specified
  snapshot/point behavior;
- the implementation expands into cell/row/style/grapheme/hyperlink extraction
  or renderer/app plumbing.

## Review

This design must be reviewed with the Codex review skill before implementation.
All real findings must be fixed, and the design must be re-reviewed until Codex
approves it.

After implementation and result recording, the completed result must also be
reviewed with Codex and approved before the result commit.

## Result

**Result:** Pass

Implemented the owned tracked-grid-reference ABI with Roastty-only public names:

- `roastty_tracked_grid_ref_t`;
- `roastty_terminal_grid_ref_track`;
- `roastty_tracked_grid_ref_free`;
- `roastty_tracked_grid_ref_has_value`;
- `roastty_tracked_grid_ref_snapshot`;
- `roastty_tracked_grid_ref_point`;
- `roastty_tracked_grid_ref_set`.

The implementation adds a terminal-wrapper registry for live tracked refs. When
`roastty_terminal_free` runs, it detaches every live tracked ref before dropping
the terminal so post-terminal-free calls return `ROASTTY_NO_VALUE` or
`ROASTTY_INVALID_VALUE` without dereferencing the stale terminal handle.

Core terminal helpers now track a `PageList` pin with screen key, screen
generation, and screen owner id. Valid tracked refs follow page-list mutation
while their owning screen remains live. Reset, alternate destruction/recreation,
terminal free, garbage pins, generation mismatch, and owner mismatch all produce
no value. Cleanup untracks only through the original still-owned screen object.

`roastty_tracked_grid_ref_set` tracks the replacement pin before untracking the
old pin, and validates the raw terminal handle before dereferencing the terminal
wrapper. Detached refs cannot be reattached to a stale or different terminal.

Verification passed:

```bash
cargo fmt -- roastty/src/lib.rs roastty/src/terminal/page_list.rs roastty/src/terminal/screen.rs roastty/src/terminal/terminal.rs
cargo test -p roastty tracked_grid_ref
cargo test -p roastty terminal_grid_ref
cargo test -p roastty c_harness_links_against_roastty_header_and_roastty_dylib
cargo test -p roastty terminal_stream
cargo test -p roastty
if rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c; then exit 1; else exit 0; fi
git diff --check
```

The final full `roastty` run passed with 1847 Rust tests, the C ABI harness, and
doc tests.

## Conclusion

Experiment 175 completes the tracked grid-ref C ABI slice. Roastty now has both
borrowed grid-ref snapshots and owned tracked refs that can safely outlive page
mutation and terminal free. The next experiment can move to the next coherent
libroastty parity gap instead of adding more grid-ref ownership plumbing.
