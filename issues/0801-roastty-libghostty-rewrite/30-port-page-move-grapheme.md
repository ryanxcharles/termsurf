# Experiment 30: Port Page Move Grapheme

## Description

Port upstream `Page.moveGrapheme` to Roastty as an explicit internal Page
primitive.

Roastty already has lower-level grapheme-map movement for `move_cells` and
`swap_cells`, but upstream exposes `moveGrapheme` as a distinct Page operation:
it moves the grapheme map entry from one cell to another without allocating and
without changing either cell's content tag. That warning is important: callers
must update cell tags and row flags themselves, and the integrity checker should
catch misuse.

This experiment should formalize that primitive and add tests for the exact
warning semantics. It should not rewrite cell move/swap behavior unless a small
local refactor is necessary to route through the new primitive with unchanged
behavior.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/page.zig` as the source of truth for:
     - `Page.moveGrapheme`;
     - `Page.lookupGrapheme`;
     - `Page.clearGrapheme`;
     - related grapheme integrity behavior.
   - Preserve upstream semantics:
     - source cell must have grapheme data;
     - destination cell must not have grapheme data;
     - the operation does not allocate;
     - the operation moves only the grapheme map entry;
     - it does not change source or destination cell content tags;
     - callers remain responsible for cell tags and row flags.
   - Do not modify `vendor/ghostty/`.

2. Add an explicit internal move-grapheme method.
   - Add an internal method shaped like:

     ```rust
     fn move_grapheme_at(
         &mut self,
         src_x: usize,
         src_y: usize,
         dst_x: usize,
         dst_y: usize,
     )
     ```

   - Keep the existing offset-level helper, or rename/refactor it to:

     ```rust
     fn move_grapheme_at_offset(
         &mut self,
         src_offset: Offset<Cell>,
         dst_offset: Offset<Cell>,
     )
     ```

   - The coordinate wrapper should derive offsets from checked cell positions
     and call the offset helper.
   - The offset helper should assert the source cell has grapheme data and the
     destination cell does not.
   - The offset helper should fetch/remove the source map entry and insert it at
     the destination with no allocation.
   - Do not update either cell's `ContentTag`.
   - Do not update row grapheme flags.

3. Preserve existing move/swap behavior.
   - Existing `move_cells` and `swap_cells` behavior must not change.
   - If those paths are routed through the new offset helper, all existing
     move/swap tests must still pass.
   - Do not add automatic `verify_integrity()` calls to mutation paths.

4. Add focused tests.
   - Direct map movement:
     - create a source cell with grapheme data and a plain destination cell;
     - call `move_grapheme_at`;
     - verify lookup no longer returns data at the source;
     - verify lookup returns the same codepoints at the destination;
     - verify grapheme count and used bytes are unchanged.
   - Warning semantics:
     - after `move_grapheme_at`, verify the source cell is still marked as a
       grapheme cell and the destination cell is still plain;
     - verify `verify_integrity()` reports `MissingGraphemeData` while the
       source cell tag still points at moved-away grapheme data;
     - manually clear the source cell content tag and verify
       `verify_integrity()` then reports `UnmarkedGraphemeCell` while the
       destination cell has map data but no grapheme tag;
     - manually fix the destination content tag and row grapheme flag;
     - verify `verify_integrity()` passes.
   - Preconditions:
     - moving from a source without grapheme data panics;
     - moving to a destination that already has grapheme data panics.
   - Cross-row movement:
     - move grapheme data across rows;
     - verify the caller must set the destination row grapheme flag for
       integrity to pass after fixing the destination cell tag;
     - verify stale `source_row.grapheme = true` is not rejected by
       `verify_integrity()`, matching upstream's lower-bound row-flag check, but
       document that callers should still clean it up for accurate metadata.
   - Existing behavior:
     - existing move/swap/clear/clone grapheme tests still pass.

5. Preserve scope.
   - Do not implement:
     - parser/screen lifecycle;
     - public ABI or app-facing APIs;
     - automatic integrity checks after mutation;
     - broader grapheme storage redesign.
   - Do not change grapheme allocator or map layout semantics.

6. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

7. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - move-grapheme API added;
     - whether existing move/swap paths were routed through it;
     - warning semantics and integrity behavior;
     - tests added;
     - verification command output summary.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `move_grapheme_at` moves grapheme map data without allocation;
- grapheme count and used bytes are unchanged by the move;
- source and destination cell content tags are not changed by the primitive;
- integrity fails until the caller fixes content tags/row flags, then passes;
- source-missing and destination-already-has-grapheme preconditions panic;
- existing move, swap, clear, clone, set, append, and integrity tests do not
  regress;
- `cargo fmt`, targeted Page tests, and full `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- the primitive works, but existing helper naming needs a small follow-up;
- one warning-semantics test needs to be deferred because current integrity
  behavior lacks the exact needed error.

The experiment fails if:

- the method allocates or frees grapheme data;
- the method changes cell content tags or row flags;
- moving corrupts map entries or loses codepoints;
- existing `move_cells` or `swap_cells` behavior regresses;
- the implementation expands into parser/screen lifecycle, public ABI, or
  unrelated behavior.

## Result

**Result:** Pass

Implemented explicit internal `Page::move_grapheme_at` and
`Page::move_grapheme_at_offset` in `roastty/src/terminal/page.rs`.

The implementation moves only the grapheme map entry from source cell offset to
destination cell offset. It does not allocate, free, or copy grapheme codepoint
storage, and it does not change either cell's content tag or either row's
grapheme flag. Existing `move_cells` and one-sided `swap_cells` grapheme paths
now route through the explicit offset helper with unchanged behavior.

Added focused tests covering:

- direct map movement without allocation;
- unchanged grapheme count and used bytes;
- source and destination content tags intentionally left unchanged;
- concrete integrity failure sequence after raw movement: `MissingGraphemeData`,
  then `UnmarkedGraphemeCell`, then success after caller repairs tags and row
  flags;
- source-without-grapheme and destination-with-grapheme precondition panics;
- cross-row movement requiring the destination row grapheme flag for integrity;
- stale source row grapheme flags being tolerated by integrity, matching
  upstream's lower-bound row-flag semantics.

Verification passed:

```bash
cargo fmt
cargo test -p roastty terminal::page
cargo test -p roastty
```

The targeted Page suite reported 166 passing tests. The full `roastty` suite
reported 275 unit tests, the ABI harness, and doc tests passing.

## Conclusion

Roastty now has the upstream explicit Page primitive for moving grapheme data
between cells without touching visible cell tags or row metadata. The tests
capture the deliberately sharp edge from upstream: callers own the tag/flag
repair, and integrity detects the important invalid states while tolerating
stale source row grapheme flags.
