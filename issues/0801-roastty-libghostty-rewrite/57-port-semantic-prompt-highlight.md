# Experiment 57: Port Semantic Prompt Highlight

## Description

Port the first slice of upstream `PageList.highlightSemanticContent`: the
untracked highlight value and the `SemanticContent::Prompt` branch.

Upstream semantic highlighting returns an untracked start/end pin range for a
semantic content type within the zone owned by a prompt. Experiment 56 added the
prompt iterator, and Experiments 54-55 added row/cell iteration, so Roastty now
has the traversal primitives needed to start porting this behavior.

The whole upstream function has three content branches: prompt, input, and
output. This experiment intentionally ports only the prompt branch so the tests
and behavior stay small enough to review. Input and output highlighting must
remain follow-up experiments.

This is still PageList-only semantic traversal work. It must not port renderer
highlight flattening/tracking, search selection, diagrams, parser behavior,
renderer delivery, app behavior, or public ABI.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/PageList.zig` for:
     - `highlightSemanticContent`;
     - the `.prompt` switch branch;
     - the prompt-zone end calculation using the next prompt or screen bottom;
     - upstream tests named `PageList highlightSemanticContent prompt...`.
   - Use `vendor/ghostty/src/terminal/highlight.zig` for the shape of
     `highlight.Untracked`.
   - Do not modify `vendor/ghostty/`.

2. Add a private untracked highlight value type.
   - Add an internal `UntrackedHighlight` or similarly named private type.
   - Store:
     - `start: Pin`;
     - `end: Pin`.
   - Keep it private/internal for now.
   - Do not add tracked highlights, flattened highlights, render-state
     highlights, search highlights, or public ABI.

3. Add prompt-zone end calculation.
   - Given a prompt-start `Pin`, find the end of that prompt's semantic zone:
     - use `prompt_iterator_from_pin(Direction::RightDown, at, None)` or an
       equivalent helper;
     - assert or otherwise validate that the first prompt returned is the
       starting prompt row;
     - if a next prompt exists, the zone ends at the last column of the row
       immediately before that next prompt;
     - if no next prompt exists, the zone ends at `get_bottom_right(Screen)`.
   - Preserve upstream behavior where the caller is expected to pass the first
     row of a prompt. Do not add broad caller-recovery behavior in this
     experiment.

4. Add prompt semantic highlighting.
   - Add a private helper equivalent to the `.prompt` branch of upstream
     `highlightSemanticContent`.
   - Input:
     - a prompt-start `Pin`.
   - Output:
     - `UntrackedHighlight`, or `Option<UntrackedHighlight>` only if that makes
       later branch integration cleaner.
     - If `Option` is used, the prompt branch must always return `Some` once it
       is called. Upstream's prompt branch always returns a highlight; `None` is
       only a possible outcome for other semantic content branches.
   - Initialize:
     - `start` to the prompt row with `x = 0`;
     - `end` to the provided prompt pin.
   - Iterate cells from the provided prompt pin's `x` to the prompt-zone end
     with `cell_iterator_from_pin(Direction::RightDown, at, Some(end))`.
     - Do not normalize the scan start to `x = 0`.
     - The returned highlight start is normalized to `x = 0`, but the cell scan
       begins at the caller's provided `at.x`, matching upstream.
   - Extend `end` through cells whose `semantic_content` is `Prompt` or `Input`.
   - Stop at the first `Output` cell.
   - Match upstream's prompt behavior: prompt highlighting includes prompt cells
     and input cells, but not output cells.

5. Keep scope narrow.
   - Do not implement input highlighting.
   - Do not implement output highlighting.
   - Do not add a broad public `highlight_semantic_content` switch unless the
     unsupported branches are explicitly unavailable internally and tests cannot
     accidentally treat them as complete.
   - Do not add renderer, parser, selection, search, diagram, app, ABI, or
     public API work.

6. Add tests.
   - Port the upstream prompt-focused cases:
     - prompt cells followed by input cells highlights through the input cells;
     - prompt cells followed by output cells stops before output;
     - multiline prompt/input highlight extends across rows;
     - prompt-only row highlights the prompt cells;
     - prompt with no next prompt highlights to the end of the screen zone, but
       still stops at output for the prompt branch.
   - Add a nonzero-`at.x` test:
     - put an `Output` cell before `at.x`;
     - put prompt/input cells from `at.x` onward;
     - verify the returned highlight start is `x = 0`, but the scan does not
       stop on the earlier output cell.
   - Add a cross-page prompt-zone test where the next prompt is on a later page
     and the zone end is the row before that next prompt.
   - Add a test where no next prompt exists and the zone end is screen bottom.
   - Convert highlight start/end pins back to expected screen points with
     `point_from_pin`.
   - Verify start/end pins are not tracked and no tracked-pin side effects are
     introduced.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - untracked highlight shape;
     - prompt-zone end behavior;
     - prompt branch behavior;
     - tests added;
     - verification command output summary;
     - independent result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- the untracked highlight value stores start/end pins without tracking them;
- prompt-zone end calculation matches upstream for both next-prompt and
  no-next-prompt cases;
- prompt highlighting includes prompt and input cells;
- prompt highlighting stops before output cells;
- prompt highlighting always returns a highlight once called for a prompt row;
- prompt highlighting scans from the provided pin's `x` while returning a start
  pin normalized to `x = 0`;
- prompt highlighting works across rows and pages;
- input and output highlight branches remain unimplemented and clearly deferred;
- no renderer highlight flattening/tracking, search selection, diagram, parser,
  renderer, app, public ABI, resize/reflow, selection, or search work is
  introduced;
- `cargo fmt`, targeted PageList tests, and full `cargo test -p roastty` pass;
- an independent agent reviews the experiment design and completed result and
  approves them, or all real findings are fixed.

The experiment is partial if:

- prompt highlighting works on single-page cases, but cross-page prompt-zone
  ending needs a follow-up experiment.

The experiment fails if:

- prompt highlighting includes output cells;
- prompt highlighting omits input cells;
- prompt highlighting returns `None` for a valid prompt-row call;
- prompt highlighting scans from `x = 0` instead of the provided pin's `x`;
- prompt-zone end calculation ignores the next prompt;
- the highlight pins are accidentally tracked or mutate PageList state;
- input/output branches are presented as complete;
- the implementation expands into renderer highlights, search selection, diagram
  output, parser, renderer, app, ABI, resize/reflow, selection, or search work;
- tests or formatting fail.
