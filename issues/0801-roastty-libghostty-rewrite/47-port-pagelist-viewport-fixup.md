# Experiment 47: Port PageList Viewport Fixup

## Description

Port upstream PageList `fixupViewport`.

Experiment 46 added PageList splitting. The next missing source-order PageList
helper is `fixupViewport`, which adjusts viewport state after rows are removed
from the PageList. Upstream erase paths call this helper after deleting rows, so
it should be ported before implementing `eraseRow`, `eraseRowBounded`,
`eraseHistory`, `eraseActive`, or `eraseRows`.

This experiment should add viewport fixup only. It must not implement erase,
bounded erase, history/active erase, row deletion, resize/reflow, scrollClear,
row/cell/prompt iterators, parser retry loops, renderer dirty-region delivery,
or public ABI.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/PageList.zig` as the source of truth for:
     - `PageList.fixupViewport`;
     - callers of `fixupViewport` in `eraseRow`, `eraseRowBounded`, and
       `eraseRows`, as future context only.
   - Do not modify `vendor/ghostty/`.

2. Add `PageList::fixup_viewport`.
   - Signature should take `removed: usize`.
   - Match upstream behavior:
     - `Viewport::Active` is unchanged.
     - `Viewport::Pin` changes to `Viewport::Active` if the viewport pin is now
       in the active area.
     - `Viewport::Pin` with a cached row offset subtracts `removed` from the
       cached offset.
     - `Viewport::Pin` with cached offset smaller than `removed` changes to
       `Viewport::Top`.
     - `Viewport::Pin` without a cached row offset remains pinned unless it
       became active.
     - `Viewport::Top` changes to `Viewport::Active` if the first page is now in
       the active area.
   - Preserve the existing `viewport_pin` and cached offset semantics used by
     `viewport_row_offset`.
   - Verify integrity in tests, not inside the helper unless doing so matches
     nearby Rust PageList helper style.

3. Add tests.
   - `Viewport::Active`:
     - call `fixup_viewport` with nonzero removed rows;
     - verify viewport remains active and cached offset remains unchanged.
   - `Viewport::Pin`, pin becomes active:
     - create scrollback across at least two pages;
     - place `viewport_pin` in or after the active top-left after simulated row
       removal;
     - call `fixup_viewport`;
     - verify viewport becomes active.
   - `Viewport::Pin`, cached offset remains above removed count:
     - set `viewport_pin_row_offset`;
     - call `fixup_viewport`;
     - verify cached offset is decremented by `removed`.
   - `Viewport::Pin`, cached offset equals removed count:
     - set `viewport_pin_row_offset` equal to `removed`;
     - call `fixup_viewport`;
     - verify viewport remains pinned and cached offset becomes `Some(0)`.
   - `Viewport::Pin`, cached offset smaller than removed count:
     - set `viewport_pin_row_offset` below `removed`;
     - call `fixup_viewport`;
     - verify viewport becomes top.
   - `Viewport::Pin`, no cached offset:
     - use a still-valid pin outside the active area;
     - call `fixup_viewport`;
     - verify viewport remains pinned and cache remains `None`.
   - `Viewport::Top`:
     - simulate a state where the first page is in the active area;
     - call `fixup_viewport`;
     - verify viewport becomes active.
   - In every test, verify PageList integrity unless the expected state is
     intentionally transient because callers have not yet adjusted row
     accounting.

4. Preserve scope.
   - Do not implement:
     - eraseRow or eraseRowBounded;
     - eraseHistory, eraseActive, or eraseRows;
     - row deletion or row shifting;
     - resize/reflow;
     - scrollClear;
     - row/cell/prompt iterators;
     - parser retry loops;
     - renderer or app integration;
     - public C ABI additions.
   - Do not add `ghostty` names except when citing upstream paths or test
     provenance.

5. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

6. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - viewport fixup behavior implemented;
     - active/top/pin behavior;
     - cached offset behavior;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- active viewport fixup is a no-op;
- pinned viewport moves to active when the pin is in active space;
- pinned viewport cached row offset is decremented by removed rows when
  possible;
- pinned viewport cached row offset equal to removed rows remains pinned at
  offset 0;
- pinned viewport moves to top when the cached offset would move before row 0;
- pinned viewport without a cached offset remains pinned unless active;
- top viewport moves to active when the first page is in the active area;
- PageList integrity remains valid in all stable test cases;
- no erase, row shifting, resize/reflow, scrollClear, iterator, parser,
  renderer, app, or ABI work is introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- an independent agent reviews the experiment design and completed result and
  approves them, or all real findings are fixed.

The experiment is partial if:

- the helper behavior is implemented, but one verification case reveals a
  pre-existing viewport cache invariant that needs a separate design before
  erase can safely use it.

The experiment fails if:

- viewport state transitions do not match upstream;
- cached row offsets are updated incorrectly;
- fixup mutates pins, page order, row counts, or unrelated PageList state;
- the implementation expands into erase or row deletion;
- tests or formatting fail.

## Result

**Result:** Pass

Implemented `PageList::fixup_viewport` in `roastty/src/terminal/page_list.rs`.

The helper now matches upstream `fixupViewport` behavior:

- active viewports are unchanged;
- pinned viewports move to active if their pin is now in the active area;
- pinned viewports with cached offsets decrement the cache by removed rows;
- cached offsets equal to the removed row count remain pinned at offset 0;
- cached offsets below the removed row count move the viewport to top;
- pinned viewports without cached offsets remain pinned unless active; and
- top viewports move to active only when the first page is in the active area.

Tests cover active no-op behavior, pinned active precedence over cached offset
handling, cached decrement/equality/below-removed cases, no-cache pinned
behavior, top-to-active behavior, and top no-op behavior.

Verification:

```bash
cargo fmt && cargo test -p roastty terminal::page_list
```

Result: 152 PageList tests passed.

```bash
cargo test -p roastty
```

Result: 433 unit tests passed, plus the ABI harness passed.

Independent result review: Codex reviewer approved recording Experiment 47 as
Pass after two branch-coverage tests were added. The reviewer specifically
confirmed strict `< removed` behavior, active precedence over cached offset
handling, top no-op coverage, and clean scope.

## Conclusion

PageList viewport fixup is now ported and tested. This gives the upcoming erase
operations the upstream viewport state transition helper they expect, without
mixing row deletion or erase behavior into this experiment.

The next experiment should continue with the next upstream PageList operation,
starting the erase-row path on top of this helper.
