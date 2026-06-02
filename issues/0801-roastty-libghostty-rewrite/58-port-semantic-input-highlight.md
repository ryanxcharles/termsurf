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

# Experiment 58: Port Semantic Input Highlight

## Description

Port the second slice of upstream `PageList.highlightSemanticContent`: the
`SemanticContent::Input` branch.

Experiment 57 added the untracked highlight value, prompt-zone end calculation,
and the prompt branch. The upstream input branch is different enough to deserve
its own experiment: it can return `null`, it skips prompt cells before and
inside the input region, it starts at the first input cell, extends through
later input cells, and stops before output.

This experiment should add input semantic highlighting without also porting the
output branch, renderer highlight flattening/tracking, search selection,
diagrams, parser behavior, renderer delivery, app behavior, or public ABI.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/PageList.zig` for:
     - `highlightSemanticContent`;
     - the `.input` switch branch;
     - the shared prompt-zone end calculation;
     - upstream tests named `PageList highlightSemanticContent input...`.
   - Use Experiment 57's `UntrackedHighlight` and prompt-zone helper.
   - Do not modify `vendor/ghostty/`.

2. Add input semantic highlighting.
   - Add a private helper equivalent to the `.input` branch of upstream
     `highlightSemanticContent`.
   - Input:
     - a prompt-start `Pin`.
   - Output:
     - `Option<UntrackedHighlight>`.
   - Use the same prompt-zone end calculation added in Experiment 57.
   - Iterate cells from the provided prompt pin's `x` to the prompt-zone end
     with `cell_iterator_from_pin(Direction::RightDown, at, Some(end))`.
   - Find the start:
     - skip `SemanticContent::Prompt`;
     - on `SemanticContent::Input`, set both `start` and `end` to that pin;
     - on `SemanticContent::Output` before any input, return `None`;
     - if no input is found by the zone end, return `None`.
   - Find the end:
     - ignore nested `SemanticContent::Prompt` cells for end advancement,
       matching upstream continuation-prompt behavior;
     - nested prompt cells do not terminate scanning and may lie inside the
       returned contiguous start/end range if later input extends past them;
     - extend `end` through `SemanticContent::Input`;
     - stop before `SemanticContent::Output`.

3. Keep API shape narrow.
   - Prefer a private helper such as `highlight_semantic_input`.
   - Do not expose a complete `highlight_semantic_content` switch unless the
     unimplemented output branch is impossible to call accidentally.
   - Do not implement output highlighting in this experiment.
   - Do not add renderer, parser, selection, search, diagram, app, ABI, or
     public API work.

4. Add tests.
   - Port upstream input-focused cases:
     - basic input on one row starts at the first input cell and ends at the
       last input cell;
     - input followed by output stops before output;
     - multiline input with nested prompt/continuation cells spans across those
       nested prompt cells and extends through later input;
     - no input before output returns `None`;
     - no following prompt scans through the screen-bottom prompt zone, but the
       returned highlight ends at the last input cell before output or zone end,
       not automatically at screen bottom;
     - prompt-only content returns `None`.
   - Add a nonzero-`at.x` test:
     - put output before `at.x`;
     - put input at and after `at.x`;
     - verify the earlier output cell does not cause `None`.
   - Add a cross-page input-zone test where input spans from one page into the
     next page before a later prompt.
   - Convert highlight start/end pins back to expected screen points with
     `point_from_pin`.
   - Verify no tracked-pin side effects are introduced.

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
     - input branch behavior;
     - null behavior;
     - nested prompt behavior;
     - tests added;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- input highlighting starts at the first input cell in the prompt zone;
- input highlighting extends through later input cells;
- nested prompt cells after input begins do not advance `end` and do not stop
  scanning, but may lie inside the returned contiguous start/end range;
- output before input returns `None`;
- no input by the prompt-zone end returns `None`;
- prompt-only content returns `None`;
- input highlighting stops before output;
- input highlighting scans from the provided pin's `x`;
- input highlighting works across rows and pages;
- output highlighting remains unimplemented and clearly deferred;
- no renderer highlight flattening/tracking, search selection, diagram, parser,
  renderer, app, public ABI, resize/reflow, selection, or search work is
  introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- an independent agent reviews the experiment design and completed result and
  approves them, or all real findings are fixed.

The experiment is partial if:

- input highlighting works on single-page cases, but cross-page or nested-prompt
  behavior needs a follow-up experiment.

The experiment fails if:

- input highlighting includes output cells;
- input highlighting omits later input cells after nested prompt cells;
- input highlighting treats nested prompt cells as a hard gap outside the
  returned contiguous range;
- input highlighting returns a highlight when no input exists;
- output before input is ignored incorrectly;
- input highlighting scans from `x = 0` instead of the provided pin's `x`;
- output branch support is presented as complete;
- the implementation expands into renderer highlights, search selection, diagram
  output, parser, renderer, app, ABI, resize/reflow, selection, or search work;
- tests or formatting fail.

## Result

**Result:** Pass

Experiment 58 ported the upstream `SemanticContent::Input` branch as a private
`PageList::highlight_semantic_input` helper. The helper uses the prompt-zone end
calculation from Experiment 57, scans from the caller-provided `Pin`, returns
`None` when output appears before any input, starts on the first input cell,
extends through later input cells, ignores nested prompt cells while scanning,
and stops before output.

The implementation deliberately remains narrow: output highlighting, renderer
highlight flattening/tracking, search selection, diagrams, parser behavior,
renderer delivery, app behavior, public ABI, resize/reflow, selection, and
search work are still deferred.

Tests added:

- basic single-row input highlighting;
- input followed by output stops before output;
- multiline input with nested prompt/continuation cells;
- output before input returns `None`;
- no following prompt scans to the bottom prompt zone but ends at the last input
  cell;
- prompt-only content returns `None`;
- nonzero `at.x` starts scanning from the provided pin;
- cross-page input-zone highlighting;
- returned highlight pins are untracked.

Verification:

```bash
cargo fmt
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

Results:

- `cargo test -p roastty terminal::page_list`: 250 passed, 0 failed.
- `cargo test -p roastty`: 531 unit tests passed, ABI harness 1 passed,
  doctests 0.

Independent result review approved the experiment as Pass with no required
findings. The reviewer confirmed that the implementation matches upstream's
two-phase input branch, preserves the nonzero-`at.x` requirement, covers the
required cases, and does not expand beyond the intended scope.

## Conclusion

Roastty now has the prompt and input semantic highlight branches from upstream
`PageList.highlightSemanticContent`. The remaining semantic branch is output
highlighting; that should be designed as the next experiment instead of mixing
it into this completed slice.
