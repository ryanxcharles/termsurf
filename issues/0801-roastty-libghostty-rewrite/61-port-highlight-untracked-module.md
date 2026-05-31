# Experiment 61: Port Highlight Untracked Module

## Description

Create the first Roastty `terminal/highlight.rs` module by porting upstream
`highlight.Untracked` and moving the ad hoc `UntrackedHighlight` struct out of
`page_list.rs`.

Experiments 57-60 intentionally kept highlight values local to PageList while
the semantic branches were being ported. Upstream Ghostty has a dedicated
`terminal/highlight.zig` module for untracked, tracked, and flattened
highlights. Roastty should now start matching that shape, but the first slice
should stay narrow: only port the untracked highlight type that current PageList
code already uses. Tracked and flattened highlights depend on broader Screen,
tracked-pin lifecycle, page-chunk, and allocation decisions that deserve their
own experiments.

This experiment should be a structure extraction and naming alignment, not a
behavior change.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/highlight.zig` for:
     - module purpose comments;
     - `Untracked`;
     - `Untracked.eql`;
     - the relationship between untracked, tracked, and flattened highlights.
   - Use `vendor/ghostty/src/terminal/PageList.zig` only to confirm
     `highlight.Untracked` is the return type of `highlightSemanticContent`.
   - Do not modify `vendor/ghostty/`.

2. Add `roastty/src/terminal/highlight.rs`.
   - Add a module comment adapted to Roastty naming.
   - Add:

     ```rust
     #[derive(Debug, Clone, Copy, PartialEq, Eq)]
     pub(super) struct Untracked {
         pub(super) start: Pin,
         pub(super) end: Pin,
     }
     ```

   - Use `super::page_list::Pin`.
   - Rely on derived `PartialEq`/`Eq` for upstream `eql` behavior unless a
     manual method is needed by existing code.
   - Do not add `Tracked`, `Flattened`, `Chunk`, `track`, `deinit`, allocator
     behavior, or clone behavior in this experiment.

3. Wire the module.
   - Add `mod highlight;` to `roastty/src/terminal/mod.rs` with the same
     `#[allow(dead_code)]` style as the existing terminal modules.
   - Make `page_list::Pin` visible to sibling terminal modules with the
     narrowest needed visibility, likely `pub(super)`.
   - In `page_list.rs`, import `super::highlight`.
   - Replace local `UntrackedHighlight` with `highlight::Untracked`.
   - Keep all semantic highlight behavior unchanged.

4. Keep API shape narrow.
   - Do not make highlight types public outside `terminal`.
   - Do not expose public ABI or app/renderer APIs.
   - Do not move PageList semantic helper methods out of `page_list.rs`.
   - Do not add tracked or flattened highlight support yet.
   - Do not add selection, search, renderer, parser, app, resize/reflow, or
     diagram work.

5. Add or update tests.
   - Update existing semantic highlight tests to compile against
     `highlight::Untracked`.
   - Add a small unit test for `highlight::Untracked` equality if that is useful
     after moving the type; otherwise existing tests using derived equality and
     start/end point conversion are enough.
   - Existing PageList semantic highlight tests must continue proving behavior
     is unchanged.

6. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

7. Independent review.
   - Before implementation, get an independent agent review of this experiment
     design.
   - After implementation and verification, get an independent result review.
   - Fix all real findings before proceeding.

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - module/type changes;
     - visibility changes;
     - confirmation that behavior did not change;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `roastty/src/terminal/highlight.rs` exists and owns `Untracked`;
- `PageList` semantic highlight helpers return `highlight::Untracked`;
- `page_list.rs` no longer defines the ad hoc `UntrackedHighlight`;
- `Pin` visibility is only widened enough for sibling terminal modules to use
  the highlight type;
- semantic prompt/input/output behavior and dispatcher behavior remain
  unchanged;
- tracked and flattened highlight support remain explicitly deferred;
- no renderer highlight flattening/tracking, search selection, diagram, parser,
  renderer, app, public ABI, resize/reflow, selection, or search work is
  introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- independent design and result reviews approve the experiment, or all real
  findings are fixed before proceeding.

The experiment is partial if:

- the module extraction works, but a clean `Untracked` type requires one
  additional visibility or ownership cleanup before future tracked/flattened
  work can build on it.

The experiment fails if:

- semantic highlight behavior changes;
- `Pin` is made unnecessarily public outside `terminal`;
- tracked or flattened highlights are presented as implemented;
- renderer/app/public ABI behavior is introduced;
- tests or formatting fail.
