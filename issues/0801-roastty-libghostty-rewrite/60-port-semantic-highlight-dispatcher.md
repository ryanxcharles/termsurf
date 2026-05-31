# Experiment 60: Port Semantic Highlight Dispatcher

## Description

Port the top-level shape of upstream `PageList.highlightSemanticContent` now
that Experiments 57, 58, and 59 have ported the prompt, input, and output branch
logic.

The upstream method takes a prompt-zone `Pin` and a `SemanticContent` value,
computes the semantic prompt zone, and dispatches to the branch for prompt,
input, or output. Roastty already has the shared prompt-zone helper and three
branch helpers. This experiment should add the single private dispatcher that
composes those pieces and gives future highlight work one PageList entrypoint to
call.

This experiment is intentionally not the renderer/app integration step. It
should not move highlight rendering into the UI, expose public ABI, add tracked
or flattened highlights, add selection/search behavior, or change parser
behavior.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/PageList.zig` for:
     - `highlightSemanticContent`;
     - the call signature and `SemanticContent` switch;
     - existing prompt/input/output tests.
   - Use the existing Roastty branch helpers:
     - `highlight_semantic_prompt`;
     - `highlight_semantic_input`;
     - `highlight_semantic_output`.
   - Do not modify `vendor/ghostty/`.

2. Add the dispatcher.
   - Add a private helper such as:

     ```rust
     fn highlight_semantic_content(
         &self,
         at: Pin,
         content: SemanticContent,
     ) -> Option<UntrackedHighlight>
     ```

   - Dispatch exactly by `SemanticContent`:
     - `Prompt` -> `highlight_semantic_prompt(at)`;
     - `Input` -> `highlight_semantic_input(at)`;
     - `Output` -> `highlight_semantic_output(at)`.
   - Do not duplicate branch logic inside the dispatcher.
   - Keep prompt-zone calculation inside the existing branch helpers for this
     experiment. Do not refactor shared state unless the implementation proves a
     real duplication problem.

3. Keep API shape narrow.
   - Keep the dispatcher private to `PageList` for now.
   - Do not create `terminal/highlight.rs` in this experiment.
   - Do not move `UntrackedHighlight` out of `page_list.rs` yet.
   - Do not add tracked highlights, flattened highlights, selection, search,
     renderer, app, parser, ABI, resize/reflow, or public API work.

4. Add tests.
   - Add dispatcher-focused tests that prove:
     - `SemanticContent::Prompt` returns the same range as the prompt branch;
     - `SemanticContent::Input` returns the same range as the input branch;
     - `SemanticContent::Output` returns the same range as the output branch;
     - `SemanticContent::Input` returns `None` when no input appears before
       output;
     - `SemanticContent::Output` returns `None` when no text-bearing output
       exists;
     - the dispatcher scans from the provided `at.x` by using a nonzero `at`
       case for at least one branch.
   - Prefer small fixtures that exercise the dispatcher rather than duplicating
     every branch-specific test from Experiments 57-59.
   - Convert highlight start/end pins back to expected screen points with
     `point_from_pin`.

5. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

6. Independent review.
   - Before implementation, get an independent agent review of this experiment
     design.
   - After implementation and verification, get an independent result review.
   - Fix all real findings before proceeding.

7. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - dispatcher behavior;
     - tests added;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `highlight_semantic_content` dispatches `Prompt`, `Input`, and `Output` to the
  corresponding branch helpers;
- the dispatcher returns branch `None` results unchanged;
- branch-specific behavior from Experiments 57-59 still passes;
- the dispatcher respects the caller-provided `at.x`;
- no branch logic is duplicated inside the dispatcher;
- no renderer highlight flattening/tracking, search selection, diagram, parser,
  renderer, app, public ABI, resize/reflow, selection, or search work is
  introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- independent design and result reviews approve the experiment, or all real
  findings are fixed before proceeding.

The experiment is partial if:

- the dispatcher works but the implementation reveals that `UntrackedHighlight`
  must first move into a dedicated highlight module before future consumers can
  call it cleanly.

The experiment fails if:

- the dispatcher changes prompt, input, or output branch behavior;
- the dispatcher recomputes or partially reimplements branch logic in a way that
  can drift from the already-tested helpers;
- branch `None` results are converted into highlights;
- the dispatcher scans from `x = 0` instead of the provided pin's `x`;
- the implementation expands into renderer highlights, search selection, diagram
  output, parser, renderer, app, ABI, resize/reflow, selection, or search work;
- tests or formatting fail.

## Result

**Result:** Pass

Experiment 60 added the private
`PageList::highlight_semantic_content(at, content)` dispatcher. It matches the
upstream `highlightSemanticContent` switch shape by dispatching
`SemanticContent::Prompt`, `SemanticContent::Input`, and
`SemanticContent::Output` directly to the branch helpers ported in Experiments
57-59.

The dispatcher does not duplicate branch logic and does not expose renderer,
app, public API, ABI, tracked highlight, flattened highlight, parser, selection,
or search behavior.

Tests added:

- dispatching `Prompt`, `Input`, and `Output` returns the expected branch
  ranges;
- input and output `None` results pass through unchanged;
- dispatcher calls respect a nonzero caller-provided `at.x`.

Verification:

```bash
cargo fmt
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

Results:

- `cargo test -p roastty terminal::page_list`: 263 passed, 0 failed.
- `cargo test -p roastty`: 544 unit tests passed, ABI harness 1 passed,
  doctests 0.

Independent result review approved the experiment as Pass with no required
findings. The reviewer confirmed that the dispatcher matches the experiment
scope and upstream shape, delegates directly to the existing branch helpers, has
sufficient focused test coverage, and does not drift into renderer, app, public
API, ABI, or highlight-module work.

## Conclusion

Roastty now has the complete private PageList semantic-highlight entrypoint
shape for prompt, input, and output content. The next highlight-related step
should move toward upstream's highlight data model, likely by designing a
dedicated `terminal/highlight.rs` module for untracked/tracked/flattened
highlight structures instead of continuing to grow ad hoc highlight types inside
`page_list.rs`.
