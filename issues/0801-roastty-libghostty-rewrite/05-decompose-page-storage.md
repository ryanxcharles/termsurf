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

# Experiment 5: Decompose Page Storage Port

## Description

Analyze Ghostty's terminal page storage stack and choose the next implementation
slice for Roastty.

Experiments 3 and 4 ported small foundations (`Tabstops` and `size`). The next
major area is `page.zig`, but it is not a single safe implementation step:
`page.zig` is large, pointer-heavy, layout-sensitive, and tightly coupled to
bitmap allocation, offset hash maps, styles, hyperlinks, graphemes, rows, cells,
and page-list behavior. This experiment should decompose that area before any
more code is ported.

The output should be a concrete page-storage roadmap: dependencies, risk
classification, test groups, unsafe boundaries, and the next implementation
experiment.

## Changes

1. Inspect the page-storage source set.
   - Use `vendor/ghostty/` as source of truth.
   - Inspect at least:
     - `vendor/ghostty/src/terminal/page.zig`
     - `vendor/ghostty/src/terminal/PageList.zig`
     - `vendor/ghostty/src/terminal/bitmap_allocator.zig`
     - `vendor/ghostty/src/terminal/hash_map.zig`
     - `vendor/ghostty/src/terminal/ref_counted_set.zig`
     - `vendor/ghostty/src/terminal/style.zig`
     - `vendor/ghostty/src/terminal/hyperlink.zig`
     - `vendor/ghostty/src/terminal/color.zig`
     - `vendor/ghostty/src/terminal/kitty.zig`
   - Do not modify `vendor/ghostty/`.

2. Build a direct import inventory.
   - For both `page.zig` and `PageList.zig`, enumerate every direct import and
     classify it as:
     - required for the next slice;
     - deferred;
     - replaced by Rust standard/library behavior;
     - omitted because Roastty is macOS-only or because the path is not needed.
   - The inventory must explicitly cover:
     - `terminal_options` / `slow_runtime_safety`;
     - Kitty graphics gates;
     - `fastmem` copy semantics;
     - `quirks.inlineAssert`;
     - `tripwire` failure hooks;
     - intrusive list dependencies;
     - `point`;
     - `highlight`;
     - PageList-only dependencies.

3. Classify `page.zig`.
   - Break it into coherent implementation areas:
     - page-aligned allocation and layout;
     - `Capacity`, `Size`, and layout calculations;
     - `Row` and `Cell` packed storage;
     - basic row/cell access;
     - grapheme allocation and lookup;
     - style set integration;
     - hyperlink set/map integration;
     - clone/cloneFrom/partial-row copy;
     - move/erase behavior;
     - integrity checking;
     - exact row capacity calculation.
   - For each area, record:
     - upstream functions/types;
     - dependencies;
     - unsafe requirement, if any;
     - tests that prove behavior;
     - whether it can be implemented now or requires a prerequisite port.

4. Classify dependency modules.
   - Determine which dependency should be ported before `Page` itself.
   - Evaluate at least:
     - `bitmap_allocator.zig`
     - `hash_map.zig`
     - `ref_counted_set.zig`
     - minimal `color`, `style`, `hyperlink`, and `kitty` types needed by `Page`
   - For each dependency, record:
     - whether it is required for the first useful `Page` slice;
     - whether it is safe Rust, unsafe Rust, or mixed;
     - upstream tests available;
     - expected implementation size.

5. Define the unsafe boundary for page storage.
   - Decide whether Roastty should keep Ghostty's contiguous page backing memory
     model for `Page`, or stage through safe Rust containers first.
   - If the contiguous model is required, record which modules own unsafe
     pointer arithmetic and which APIs remain safe.
   - The unsafe boundary plan must explicitly name:
     - ownership and deallocation invariants;
     - zeroed page allocation assumptions;
     - packed `Row` / `Cell` layout assertions;
     - aliasing rules for copied and moved cells;
     - which functions/modules may contain pointer arithmetic;
     - which functions/modules must remain safe wrappers over those internals.
   - Do not implement the unsafe boundary in this experiment.

6. Classify tests.
   - Assign every `page.zig` test to a named test group.
   - For PageList and dependency-module tests, mark each test group as:
     - required for the next implementation slice;
     - deferred until a later slice;
     - not applicable to Roastty with a reason.
   - The result must identify which tests prove the chosen next implementation
     slice and which tests will remain red/deferred after that slice.

7. Choose the next implementation experiment.
   - The result must name exactly one next implementation slice.
   - The next slice should be small enough to implement and review in one
     experiment.
   - It should make page storage more real, not just add unrelated terminal
     helpers.
   - It should include clear tests from upstream or direct equivalents.

8. Verify the diagnostic-only boundary.
   - Before recording the result, run:

     ```bash
     git status --short
     ```

   - Expected changed files are limited to Issue 801 documentation and
     gitignored review logs under `logs/`.
   - This experiment must not modify `roastty/`, `vendor/ghostty/`,
     `Cargo.toml`, `Cargo.lock`, scripts, build configuration, or source code.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include these tables:
     - `Page Storage Areas`
     - `Direct Import Inventory`
     - `Dependency Port Order`
     - `Unsafe Boundary Plan`
     - `Page Test Groups`
     - `Next Implementation Slice`
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- the result cites concrete upstream files and tests;
- each major `page.zig` behavior area is classified;
- required dependency modules are ordered;
- unsafe page-storage boundaries are explicit;
- the next implementation experiment is exactly one named slice;
- the diagnostic-only boundary is preserved;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- the broad decomposition is useful, but one dependency relationship remains too
  uncertain to choose the next implementation slice safely.

The experiment fails if:

- it starts porting code instead of decomposing the page-storage work;
- it leaves the unsafe boundary vague;
- it recommends a broad or multi-subsystem next implementation experiment;
- it ignores the upstream tests.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or running the audit.

## Result

**Result:** Pass

The page-storage audit confirms that `Page` should not be the next direct port.
`page.zig` is a composition point for several lower-level storage structures,
and those structures need to be ported first so `Page` can be tested in smaller
behavioral slices.

The next implementation slice should be `terminal/bitmap_allocator.zig`.

### Page Storage Areas

| Area                    | Upstream code                                                                                       | Dependencies                                                                             | Unsafe need                                                                                 | Tests                                                                                      |
| ----------------------- | --------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| Page-aligned allocation | `PageAlloc`, `Page.init`, `Page.deinit`                                                             | macOS `mmap`/`munmap`, `Layout`, `OffsetBuf`                                             | Yes: page-aligned zeroed backing memory and raw slice ownership                             | `Page init`                                                                                |
| Capacity and layout     | `std_capacity`, `Size`, `Capacity`, `Capacity.adjust`, `Capacity.maxCols`, `Page.layout`            | `Row`, `Cell`, `StyleSet.Layout`, `BitmapAllocator.Layout`, `Hyperlink.Map/Layout`       | Low directly, but layout must match packed storage                                          | `Page.layout can take a maxed capacity`, all `Page capacity...`, all `Capacity maxCols...` |
| Packed row/cell storage | `Row`, `Cell`, nested enums/unions, `Cell.init`, helpers                                            | `Offset<Cell>`, `StyleId`, color RGB, Kitty placeholder constants                        | Yes for exact packed layout/bitcasts if represented as raw `u64`; safe wrapper API required | `Cell is zero by default`, later row/cell and resize tests                                 |
| Basic row/cell access   | `getRow`, `getRowAndCell`, `clearCells`, row dirty flags                                            | `Row`, `Cell`, `Offset`                                                                  | Yes internally: offset-derived typed pointers                                               | `Page read and write cells`                                                                |
| Grapheme storage        | `GraphemeAlloc`, `GraphemeMap`, `appendGrapheme`, `lookupGrapheme`, `clearGrapheme`, `moveGrapheme` | `BitmapAllocator`, `AutoOffsetHashMap`, `OffsetSlice`                                    | Mixed: allocator and offset map pointer work                                                | `Page appendGrapheme...`, `Page clearGrapheme...`, grapheme clone/move/integrity tests     |
| Style storage           | `StyleSet`, `StyleId`, style reference counts                                                       | `style.zig`, `ref_counted_set.zig`, `color.zig`, `sgr.zig`                               | Mixed: ref-counted set in page memory                                                       | `Page clone styles`, style integrity and exact-row-capacity tests                          |
| Hyperlink storage       | `hyperlink.Set`, `hyperlink.Map`, `insertHyperlink`, `lookupHyperlink`, `clearHyperlink`            | `hyperlink.zig`, `hash_map.zig`, `ref_counted_set.zig`, `BitmapAllocator` string storage | Mixed: offset slices into page memory, ref-counted entries                                  | hyperlink clone, exact capacity, split preservation tests                                  |
| Clone/copy operations   | `clone`, `cloneFrom`, `clonePartialRowFrom`                                                         | all page memory categories                                                               | Yes: preserve aliasing/refcount behavior during copy and rollback                           | all `Page clone...` and `Page cloneRowFrom...` tests                                       |
| Move/erase operations   | `moveCells`, `clearCells`, `clearManagedMemory`                                                     | graphemes, hyperlinks, styles                                                            | Yes: source/destination overlap and metadata remapping                                      | `Page moveCells...`, later PageList erase tests                                            |
| Integrity checking      | `verifyIntegrity`                                                                                   | all metadata maps and refcounts                                                          | No new unsafe, but depends on safe views over unsafe storage                                | all `Page verifyIntegrity...` tests                                                        |
| Exact row capacity      | `exactRowCapacity`                                                                                  | styles, graphemes, hyperlinks, clone behavior                                            | No new unsafe, but requires complete metadata model                                         | all `Page exactRowCapacity...` tests                                                       |

### Direct Import Inventory

| Import                      | Used by                     | Classification                       | Notes                                                                                                                                              |
| --------------------------- | --------------------------- | ------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------- |
| `std`                       | `page.zig`, `PageList.zig`  | Required                             | Rust std equivalents.                                                                                                                              |
| `builtin`                   | both                        | Omit/defer                           | Non-macOS branches are omitted. Test-mode differences must become Rust `cfg(test)` decisions.                                                      |
| `terminal_options`          | both                        | Deferred                             | `slow_runtime_safety` affects integrity-check gating; Kitty graphics affects placeholder behavior. Keep fields/layout stable, defer feature gates. |
| `std.mem.Allocator`         | both                        | Replaced                             | Rust ownership/fallible allocation per Experiment 2; no broad allocator abstraction unless a module proves it necessary.                           |
| `ArenaAllocator`            | `page.zig` integrity checks | Deferred                             | Only needed when porting `verifyIntegrity`.                                                                                                        |
| `quirks.inlineAssert`       | both                        | Replaced                             | Use `assert!` / `debug_assert!` case by case; behavior tests should catch invariant changes.                                                       |
| `std.testing`               | `page.zig`                  | Replaced                             | Rust unit tests.                                                                                                                                   |
| `std.posix`                 | `page.zig`                  | Required later                       | macOS `mmap`/`munmap` for page backing memory.                                                                                                     |
| `std.os.windows`            | `page.zig`                  | Omitted                              | Roastty is macOS-only.                                                                                                                             |
| `fastmem`                   | both                        | Replaced                             | Use `copy_from_slice`, `ptr::copy`, or `ptr::copy_nonoverlapping` depending on overlap semantics. Must be decided per callsite.                    |
| `color.zig`                 | both                        | Required later                       | Needed for `Cell.RGB`, style colors, and PageList highlight behavior.                                                                              |
| `hyperlink.zig`             | `page.zig`                  | Required after allocator/map ports   | Depends on hash map/ref-counted set/page memory.                                                                                                   |
| `kitty.zig`                 | both                        | Deferred                             | Placeholder layout fields must stay; graphics feature behavior can wait for resize/reflow slices.                                                  |
| `style.zig` / `sgr.zig`     | both                        | Required after allocator/map ports   | Needed for style set and cell flags.                                                                                                               |
| `size.zig`                  | both                        | Done                                 | Ported in Experiment 4.                                                                                                                            |
| `bitmap_allocator.zig`      | `page.zig`                  | Required next                        | Direct dependency for grapheme and string storage; isolated enough for one experiment.                                                             |
| `hash_map.zig`              | `page.zig`, hyperlink       | Required before full metadata maps   | Larger than bitmap allocator; port after bitmap allocator.                                                                                         |
| `ref_counted_set.zig`       | style/hyperlink             | Required before style/hyperlink sets | Depends on offset storage and hash behavior.                                                                                                       |
| `tripwire`                  | `PageList.zig`              | Deferred/test-only                   | Used for failure injection; Rust should add local hooks only where needed.                                                                         |
| `IntrusiveDoublyLinkedList` | `PageList.zig`              | Deferred                             | Needed for PageList nodes, not first Page slice.                                                                                                   |
| `highlight.zig`             | `PageList.zig`              | Deferred                             | Needed for semantic highlighting tests, not page storage foundations.                                                                              |
| `point.zig`                 | `PageList.zig`              | Deferred                             | Needed for PageList pins/coordinates, not first Page slice.                                                                                        |
| `page.zig`                  | `PageList.zig`              | Deferred                             | PageList waits until Page has basic storage and clone/erase behavior.                                                                              |

### Dependency Port Order

| Order | Module/slice                                         | Why                                                                                                                                    | Expected Rust shape                                                                                                                                                 |
| ----- | ---------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1     | `bitmap_allocator.zig`                               | Direct `Page` dependency, uses existing `Offset`/`OffsetBuf`, has strong tests, and is small enough for one implementation experiment. | Mixed safe/unsafe. Generic `BitmapAllocator<const CHUNK_SIZE: usize>`, layout struct, raw offset-backed allocation methods returning slices behind unsafe boundary. |
| 2     | Minimal `color` + `sgr` + `style::Style` value types | Needed before `Cell`/style tests can compile.                                                                                          | Mostly safe enums/structs; layout tests only where packed values matter.                                                                                            |
| 3     | `Row`/`Cell` packed value port                       | Needed before basic `Page.layout` and row/cell access.                                                                                 | Prefer raw `u64` storage with safe accessors if Rust bitfield layout would be fragile.                                                                              |
| 4     | Minimal `Page.layout`, `Capacity`, `Size`            | Can be tested before full allocation/metadata operations.                                                                              | Safe arithmetic with checked conversions and layout assertions.                                                                                                     |
| 5     | `hash_map.zig` offset map                            | Needed for grapheme and hyperlink maps.                                                                                                | Likely mixed; offset-backed map plus upstream hash-map tests.                                                                                                       |
| 6     | `ref_counted_set.zig`                                | Needed for style/hyperlink sets.                                                                                                       | Mixed; preserve reserved ID 0, Robin Hood probing, refcount semantics.                                                                                              |
| 7     | Basic `Page` init/row/cell access                    | First useful `Page` slice.                                                                                                             | Contiguous page backing memory plus safe row/cell wrappers.                                                                                                         |
| 8     | Grapheme storage                                     | Builds on bitmap allocator + hash map.                                                                                                 | Mixed; offset slices and map updates.                                                                                                                               |
| 9     | Style/hyperlink storage                              | Builds on ref-counted set, string allocator, hash map.                                                                                 | Mixed; refcount and rollback-heavy.                                                                                                                                 |
| 10    | Clone/move/integrity/exact capacity                  | Full page behavior.                                                                                                                    | Many small experiments, each tied to upstream test groups.                                                                                                          |
| 11    | `PageList` foundations                               | Only after `Page` storage behavior is stable.                                                                                          | Intrusive/indexed list decision needed separately.                                                                                                                  |

### Unsafe Boundary Plan

| Boundary                   | Decision                                                                                                                                                                                                                                    |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Contiguous backing memory  | Keep Ghostty's model. Page metadata stores offsets into a single page-aligned allocation; replacing it with unrelated Rust containers would make `Page`, `PageList`, clone, compact, and exact capacity behavior diverge.                   |
| Ownership/deallocation     | `Page` must own exactly one page-aligned zeroed backing allocation. The allocation is freed exactly once by `Drop`; cloned pages own distinct backing memory unless an operation explicitly copies within the same page.                    |
| Zeroed allocation          | Preserve Ghostty's assumption that page memory starts zeroed. On macOS this should use `mmap` with anonymous/private pages or an equivalent zeroed aligned allocation wrapper.                                                              |
| Pointer arithmetic         | Only `terminal::size`, future `terminal::bitmap_allocator`, and future `terminal::page` memory-view helpers may contain offset-to-pointer arithmetic. Higher-level page operations should call safe wrappers.                               |
| Packed `Row`/`Cell` layout | Require tests for `size_of == 8`, `align_of`, zero-cell bit pattern, default empty cell, and C-value/bitcast equivalents. Avoid exposing Rust bitfields unless layout is proven.                                                            |
| Aliasing during copy/move  | Any same-page move must explicitly choose overlapping or non-overlapping copy semantics. `moveCells` requires metadata remapping tests for graphemes/hyperlinks before unsafe copying is accepted.                                          |
| Safe wrappers              | Public/internal APIs should expose `RowRef`, `RowMut`, `CellRef`, or raw-pointer-returning helpers only where necessary. Creating Rust references from offsets must stay inside small unsafe functions with documented validity invariants. |
| Failure rollback           | Any operation that allocates metadata during clone/copy/insert must build temporary state before committing or have a test proving rollback leaves page state consistent.                                                                   |

### Page Test Groups

| Group                           | `page.zig` tests                                                                                                                                                                                                                                                                                                                                                                                                        |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Layout/capacity                 | `Page.layout can take a maxed capacity`; `Page capacity adjust cols down`; `Page capacity adjust cols down to 1`; `Page capacity adjust cols up`; `Page capacity adjust cols sweep`; `Page capacity adjust cols too high`; `Capacity maxCols basic`; `Capacity maxCols preserves total size`; `Capacity maxCols with 1 row exactly`                                                                                     |
| Cell/row basics                 | `Cell is zero by default`; `Page init`; `Page read and write cells`                                                                                                                                                                                                                                                                                                                                                     |
| Graphemes                       | `Page appendGrapheme small`; `Page appendGrapheme larger than chunk`; `Page clearGrapheme not all cells`; `Page clone graphemes`; `Page cloneFrom graphemes`; `Page cloneFrom frees dst graphemes`; `Page moveCells graphemes`; `Page verifyIntegrity graphemes good`; `Page verifyIntegrity grapheme row not marked`; `Page exactRowCapacity grapheme_bytes`; `Page exactRowCapacity grapheme_bytes larger than chunk` |
| Styles                          | `Page clone styles`; `Page verifyIntegrity styles good`; `Page verifyIntegrity styles ref count mismatch`; `Page exactRowCapacity styles`; `Page exactRowCapacity single style clone`; `Page exactRowCapacity styles max single row`                                                                                                                                                                                    |
| Hyperlinks                      | `Page cloneFrom hyperlinks exact capacity`; `Page cloneRowFrom partial hyperlink in same page copy`; `Page cloneRowFrom partial hyperlink in same page omit`; `Page exactRowCapacity hyperlinks`; `Page exactRowCapacity single hyperlink clone`; `Page exactRowCapacity hyperlink map capacity for many cells`                                                                                                         |
| Clone/copy                      | `Page clone`; `Page cloneFrom`; `Page cloneFrom shrink columns`; `Page cloneFrom partial`; `Page cloneRowFrom partial`; `Page cloneRowFrom partial grapheme in non-copied source region`; `Page cloneRowFrom partial grapheme in non-copied dest region`                                                                                                                                                                |
| Move/erase/integrity edge cases | `Page moveCells text-only`; `Page verifyIntegrity zero rows`; `Page verifyIntegrity zero cols`; `Page exactRowCapacity empty rows`                                                                                                                                                                                                                                                                                      |

PageList has a much larger test surface. Its groups are deferred until `Page`
has storage, metadata, clone, erase, and resize behavior:

| PageList/dependency test group                | Status                                                                                |
| --------------------------------------------- | ------------------------------------------------------------------------------------- |
| PageList initialization/grow/prune/scrollback | Deferred until Page init and row/cell access work.                                    |
| Pins, viewport, point conversion              | Deferred; depends on PageList node model and `point.zig`.                             |
| Scroll/scrollbar/prompt iterators             | Deferred; depends on PageList and semantic prompt cell data.                          |
| Highlight semantic content                    | Deferred; depends on `highlight.zig`, semantic prompt fields, and PageList iteration. |
| Erase/clone/compact/split                     | Deferred; depends on Page clone, exact capacity, grapheme/style/hyperlink metadata.   |
| Resize/reflow                                 | Deferred late; depends on nearly all Page behavior plus Kitty placeholder handling.   |
| `bitmap_allocator.zig` tests                  | Required for the next slice.                                                          |
| `hash_map.zig` tests                          | Deferred until offset map slice.                                                      |
| `ref_counted_set.zig` tests                   | Deferred until style/hyperlink set slice.                                             |

### Next Implementation Slice

| Decision           | Details                                                                                                                                                                                                                      |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Next experiment    | Port `vendor/ghostty/src/terminal/bitmap_allocator.zig` to `roastty/src/terminal/bitmap_allocator.rs`.                                                                                                                       |
| Why this slice     | It is the smallest direct `Page` storage dependency that materially advances the page model. It already depends on `terminal::size`, which is now ported, and it has a rich upstream test suite.                             |
| Scope              | Port `BitmapAllocator<const CHUNK_SIZE: usize>`, `Layout`, `bytes_required`, `layout`, allocation/free/used/capacity behavior, and `findFreeChunks`. Do not port `Page`, hash maps, style, or hyperlinks.                    |
| Tests              | Port all `findFreeChunks` tests and the `BitmapAllocator` layout/allocation/free/bytesRequired tests from `bitmap_allocator.zig`.                                                                                            |
| Unsafe expectation | Mixed. Offset-backed typed allocation will need a narrow unsafe boundary. The allocator should return slices only through functions that document base-buffer validity, chunk alignment, and allocation-lifetime invariants. |
| Pass criteria      | All bitmap allocator tests pass, `cargo test -p roastty terminal::bitmap_allocator` passes, and full `cargo test -p roastty` remains green.                                                                                  |

### Boundary Check

`git status --short` was clean before recording this result. This experiment
modified only Issue 801 documentation and did not modify `roastty/`,
`vendor/ghostty/`, workspace files, scripts, build configuration, or source
code.

### Completion Review

Codex reviewed the completed diagnostic result and found no blocking issues. It
confirmed that the decomposition satisfies the verification criteria, that the
page-storage roadmap is accurate enough to guide the next experiment, and that
`bitmap_allocator.zig` is a good next implementation slice.

## Conclusion

Experiment 5 succeeds. The page-storage work is now decomposed into dependency
and behavior slices, with an explicit unsafe boundary plan and a concrete next
implementation step.

The next experiment should port `terminal/bitmap_allocator.zig`. That moves
Issue 801 toward real page storage while keeping the blast radius small enough
for behavior-parity review.
