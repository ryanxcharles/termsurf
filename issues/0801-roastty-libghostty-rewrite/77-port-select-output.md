# Experiment 77: Port Select Output

## Description

Port upstream `Screen.selectOutput` into Roastty's current PageList-centered
terminal model.

Experiments 57-60 added semantic prompt/input/output highlight helpers.
Experiments 73-76 added the selection value and selection primitives needed
around them. Upstream `selectOutput` is now a small bridge: given a pin on
output content, find the command output block associated with the surrounding
semantic prompt state and return it as an untracked non-rectangular selection.

This experiment should add only the PageList-local select-output calculation and
tests. It must not add `Screen`, `Terminal`, `ScreenFormatter`,
`selectionString`, string-map support, `LineIterator`, gesture state, public
ABI, renderer, parser, app, platform input, mouse event behavior, clipboard
behavior, or UI wiring.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/Screen.zig` for:
     - `selectOutput`;
     - `test "Screen: selectOutput"`.
   - Use existing Roastty code:
     - `roastty/src/terminal/page_list.rs`;
     - `roastty/src/terminal/selection.rs`;
     - existing `highlight_semantic_output`, `highlight_semantic_content`,
       `prompt_iterator_from_pin`, and semantic fixture helpers.
   - Do not modify `vendor/ghostty/`.

2. Add a private PageList helper.
   - Preferred shape:

     ```rust
     fn select_output(&self, pin: Pin) -> Option<selection::Selection>
     ```

   - Match upstream behavior:
     - return `None` unless the clicked pin is valid, non-garbage, and its cell
       semantic content is `SemanticContent::Output`;
     - search left/up for the prior semantic prompt row using the existing
       prompt iterator;
     - if a prior prompt exists, call the existing semantic-output highlight
       helper from that prompt and return its bounds as an untracked
       non-rectangular selection;
     - if no prior prompt exists, search right/down for the next semantic prompt
       row. In that fallback case, select from top-left screen through the row
       immediately before that next prompt, then trim the end backward to the
       last cell with text, matching upstream's "output before first prompt"
       behavior;
     - return `None` if no prompt boundary exists or if the resulting output
       range has no text.
   - Preserve screen-coordinate behavior. Do not silently switch to active-only
     coordinates.

3. Add upstream-equivalent tests.
   - Port the upstream `Screen: selectOutput` scenarios as pin-bound tests:
     - first output block before the first prompt exercises the no-prior-prompt
       fallback and selects `(0, 0)..(6, 1)`;
     - second output block after `prompt2/input2` selects the `output2` span at
       `(0, 4)..(6, 7)`;
     - third output block after `$ input3` selects `(0, 9)..(6, 11)`;
     - clicking prompt cells returns `None`;
     - clicking input cells returns `None`.
   - Because Roastty does not yet have `selectionString` or parser-backed
     `testWriteString`, verify selection bounds, not copied text. Construct the
     semantic fixtures directly with existing PageList test helpers.
   - Add Roastty-specific coverage:
     - invalid and garbage pins return `None`;
     - output before the first prompt selects from the screen top through the
       row before the next prompt and trims trailing unwritten cells;
     - output with no prompt boundary returns `None`;
     - returned selections are untracked and non-rectangular;
     - a scrollback/history fixture proves screen-coordinate output selection is
       not limited to active coordinates.

4. Keep scope narrow.
   - Do not add selection string extraction. This experiment proves selected
     `Pin` bounds, not copied command text.
   - Do not rewrite the semantic highlight helpers unless a real bug is found
     and documented in the result.
   - Do not add public API or C ABI exposure.
   - Do not add gesture, keyboard shortcut, clipboard, app, terminal, or UI
     behavior.

5. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty select_output
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

6. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real findings before proceeding.

7. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - helper name and location;
     - upstream test coverage;
     - Roastty-specific edge tests;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has a private PageList helper equivalent to upstream `selectOutput`;
- prompt and input clicks return `None`;
- output clicks return the expected command-output selection bounds;
- output before the first prompt follows upstream's top-of-screen fallback and
  trims trailing unwritten cells;
- invalid or garbage pins return `None`;
- screen-coordinate selection includes scrollback/history rows when present;
- returned selections are untracked and non-rectangular;
- no `Screen`, `Terminal`, `ScreenFormatter`, `selectionString`, string-map
  support, `LineIterator`, gesture state, public ABI, renderer, parser, app,
  platform input, mouse event behavior, clipboard behavior, or UI wiring is
  added;
- `cargo fmt`, targeted select-output tests, PageList tests, and full
  `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- select-output works for ordinary prompt-bounded output, but the
  before-first-prompt or scrollback case exposes a missing PageList fixture that
  must be split into its own prerequisite experiment.

The experiment fails if:

- select-output cannot be implemented without adding Screen, Terminal,
  formatter, ABI, renderer, parser, app, or platform input behavior;
- prompt or input cells are selected as output;
- output selection uses active-only coordinates;
- output ranges with no text are selected instead of returning `None`;
- tests or formatting fail.

## Design Review

Codex reviewed the initial design and found two real blockers: the
upstream-equivalent tests needed exact pin bounds for the first two output
blocks, and the first output block needed to be labeled as the no-prior-prompt
fallback rather than "before the second prompt."

The design now requires exact upstream-equivalent bounds:

- first output block: `(0, 0)..(6, 1)`;
- second output block: `(0, 4)..(6, 7)`;
- third output block: `(0, 9)..(6, 11)`.

Follow-up Codex review approved the updated design with no remaining blockers.

## Result

**Result:** Pass

Implemented private PageList output selection in
`roastty/src/terminal/page_list.rs`:

- `PageList::select_output()` validates the clicked pin and requires
  `SemanticContent::Output`.
- When a prior semantic prompt exists, it delegates to the existing semantic
  output highlight path and returns the output bounds as an untracked
  non-rectangular `Selection`.
- When no prior prompt exists, it matches upstream's fallback by finding the
  next prompt, selecting from top-left screen through the row before that
  prompt, and trimming the end backward to the last text cell.
- Invalid pins, garbage pins, prompt cells, input cells, output with no prompt
  boundary, and output ranges with no text return `None`.

The implementation stays private to PageList. It does not add `Screen`,
`Terminal`, `ScreenFormatter`, `selectionString`, string-map support,
`LineIterator`, gesture state, public ABI, renderer, parser, app, platform
input, mouse behavior, clipboard behavior, or UI wiring.

The tests cover:

- upstream-equivalent first, second, and third output block bounds:
  `(0, 0)..(6, 1)`, `(0, 4)..(6, 7)`, and `(0, 9)..(6, 11)`;
- prompt and input clicks returning `None`;
- invalid and garbage pins returning `None`;
- output with no prompt boundary returning `None`;
- screen-domain behavior across scrollback/history rows;
- untracked non-rectangular selection shape.

Verification passed:

```bash
cargo fmt
cargo test -p roastty select_output
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

Observed results:

- `cargo test -p roastty select_output`: 4 passed.
- `cargo test -p roastty terminal::page_list`: 377 passed.
- `cargo test -p roastty`: 670 unit tests passed, ABI harness passed, and
  doctests passed.

Codex reviewed the implementation and found no implementation blockers. Its only
finding was to record this result and update the README status.

## Conclusion

Experiment 77 successfully ports upstream `selectOutput` into Roastty's
PageList-centered terminal layer. Roastty can now compute semantic command
output selections using the prompt iterator and semantic-output highlight
substrate, including the upstream no-prior-prompt fallback, without adding
formatter, string extraction, gesture, UI, or public API behavior.

The next experiment can continue the selection stack with another primitive or
begin the formatter/string extraction layer that will eventually make these
selection bounds copyable as text.
