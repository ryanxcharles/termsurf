+++
[implementer]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 13: Port Page Clone for Text and Graphemes

## Description

Port whole-Page cloning for the currently implemented Page storage: rows, cells,
and graphemes.

Upstream Ghostty's next Page tests after grapheme append/lookup/clear are:

- `Page clone`
- `Page clone graphemes`

Those tests exercise `Page.clone()` / `Page.cloneBuf()` and rely on the central
Page storage invariant: everything inside the Page allocation is addressed by
offset, so a byte-for-byte copy of the backing allocation is enough to preserve
row/cell data and managed-memory maps in the clone.

This experiment should port that whole-page clone invariant for the storage
already implemented in Roastty. Do not port style clone, hyperlink clone,
`cloneFrom`, `cloneRowFrom`, partial-row copy, exact row capacity, reflow, or
integrity checking in this experiment.

## Changes

1. Inspect upstream source.
   - Use `vendor/ghostty/src/terminal/page.zig` as source of truth.
   - Re-read:
     - `Page.clone`
     - `Page.cloneBuf`
     - upstream tests `Page clone` and `Page clone graphemes`
   - Read later clone-related tests for future context only:
     - `Page clone styles`
     - `Page cloneFrom`
     - `Page cloneFrom graphemes`
     - `Page cloneFrom frees dst graphemes`
     - `Page cloneRowFrom ...`
   - Do not modify `vendor/ghostty/`.

2. Add whole-page clone support.
   - Add `Page::clone_page(&self) -> Result<Page, PageAllocError>` or a
     similarly named internal method.
   - Allocate a new `PageMemory` with exactly `self.memory.len()`.
   - Copy `self.memory` bytes into the new memory.
   - Copy all offset-valued Page metadata by value:
     - `rows`
     - `cells`
     - `dirty`
     - `size`
     - `capacity`
     - `layout`
     - `grapheme_alloc`
     - `grapheme_map`
   - The clone's `PageMemory` must own the new mapping and must free it exactly
     once on drop.
   - Do not share backing memory between original and clone.

3. Add clone-buffer support only if it is useful.
   - Upstream has `cloneBuf(buf)`.
   - Roastty may skip a public/internal clone-buffer API in this experiment if
     no current caller needs it.
   - If clone-buffer is added, it must:
     - require a caller-provided page-aligned buffer at least as large as the
       source memory;
     - copy into that buffer;
     - preserve offset metadata exactly like `clone_page`.
   - Do not broaden `PageMemory` ownership rules just to mimic `cloneBuf` if a
     simple owned clone is sufficient for the upstream tests being ported.

4. Preserve the offset-copy invariant.
   - Do not rebuild rows, cells, grapheme allocator state, or grapheme maps by
     walking the data structure.
   - Do not translate offsets after copying. Offsets stay valid because they are
     relative to the new Page backing pointer.
   - Add tests that mutate the source after cloning and prove the clone is
     unchanged.
   - Add tests that clear/free graphemes in the source after cloning and prove
     the clone still owns its independent copied grapheme storage.

5. Preserve scope.
   - Do not port:
     - style clone behavior;
     - hyperlink clone behavior;
     - `cloneFrom`;
     - `cloneRowFrom`;
     - `clonePartialRowFrom`;
     - exact row capacity;
     - integrity checking.
   - If the clone implementation encounters non-default style or hyperlink
     markers in future tests, that belongs to a later experiment after those
     storage systems are ported.

6. Translate tests.
   - Port upstream `Page clone`.
   - Port upstream `Page clone graphemes`.
   - Add Rust-specific tests for:
     - clone has a different backing pointer from the source;
     - clone has the same backing length and capacity metadata as the source;
     - source text mutation after clone does not affect the clone;
     - source grapheme clear/free after clone does not affect the clone;
     - dropping source before clone still leaves clone readable;
     - dropping clone does not affect source.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - clone API added;
     - whether clone-buffer was added or deferred;
     - memory ownership/copy strategy;
     - upstream tests ported;
     - deferred clone tests and why;
     - verification command output summary.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `Page` can clone its currently implemented storage into a new independent Page
  allocation;
- rows, cells, and grapheme map/allocator state survive the clone;
- original and clone do not share backing memory;
- mutating text or clearing graphemes in the source after clone does not affect
  the clone;
- dropping either Page does not invalidate the other;
- upstream `Page clone` and `Page clone graphemes` tests are ported and pass;
- no style/hyperlink/cloneFrom/row-copy/exact-capacity behavior is introduced;
- `cargo fmt`, targeted Page tests, and full `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- text clone works, but grapheme-backed clone reveals a missing offset-map or
  bitmap-allocator copy invariant that needs one focused prerequisite fix.

The experiment fails if:

- the clone shares backing memory with the source;
- offsets are incorrectly rebased or rewritten;
- grapheme data in the clone points into the source allocation;
- source mutation or drop affects the clone;
- it drifts into style, hyperlink, `cloneFrom`, or row-copy behavior.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or implementing the
slice.

## Result

**Result:** Pass

Experiment 13 ported owned whole-Page cloning for the Page storage implemented
so far: rows, cells, and graphemes.

The implementation added:

- `Page::clone_page(&self) -> Result<Page, PageAllocError>`

### Clone Strategy

`clone_page` follows Ghostty's offset-copy invariant:

1. allocate a fresh `PageMemory` with the same byte length as the source;
2. copy the source backing memory byte-for-byte into the new mapping;
3. copy offset-valued Page metadata by value:
   - `rows`
   - `cells`
   - `dirty`
   - `size`
   - `capacity`
   - `layout`
   - `grapheme_alloc`
   - `grapheme_map`

No offsets are rewritten. They remain valid because they are relative to the new
Page backing pointer.

The clone owns a separate mmap allocation and frees it independently.

### Clone Buffer

The upstream `cloneBuf` API was not ported in this experiment. No current
Roastty caller needs caller-provided clone buffers, and the upstream tests
selected for this slice are satisfied by owned `clone_page`.

### Tests Ported

The upstream tests ported are:

- `Page clone`
- `Page clone graphemes`

Additional Roastty tests cover:

- clone backing pointer differs from source backing pointer;
- clone backing length and capacity match the source;
- mutating source text after clone does not affect clone;
- clearing/freeing source graphemes after clone does not affect clone;
- dropping the source before reading the clone leaves the clone readable;
- dropping the clone does not affect the source.

Deferred upstream clone tests are:

- `Page clone styles`
- `Page cloneFrom`
- `Page cloneFrom shrink columns`
- `Page cloneFrom partial`
- `Page cloneFrom hyperlinks exact capacity`
- `Page cloneFrom graphemes`
- `Page cloneFrom frees dst graphemes`
- `Page cloneRowFrom ...`

Those require style storage, hyperlink storage, `cloneFrom`, partial row copy,
or exact-capacity behavior that is intentionally outside this experiment.

### Verification

The required verification passed:

```bash
cargo fmt
cargo test -p roastty terminal::page
cargo test -p roastty
```

Observed results:

- `terminal::page`: 50 passed
- full `roastty` suite: 136 Rust unit tests passed, C ABI harness passed, doc
  tests passed

## Conclusion

Roastty now preserves Ghostty's whole-Page offset-copy invariant for rows,
cells, and graphemes. The next Page work can move to either the next managed
storage dependency, such as styles, or the next clone-related operation that can
be scoped without styles/hyperlinks.
