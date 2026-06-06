+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5"
reasoning = "medium"
+++

# Experiment 716: Binding Action Adjust Selection

## Description

Experiment 715 added `select_all` binding-action support. Upstream Ghostty's
`performBindingAction` also supports `adjust_selection:<direction>`, which
mutates the active selection endpoint and scrolls that adjusted endpoint into
view.

Roastty already has the core terminal selection-adjustment behavior:

- `Terminal::active_selection()` returns the active selection;
- `Terminal::selection_adjust(selection, adjustment)` adjusts a supplied
  selection and returns the adjusted selection;
- `Terminal::set_selection(Some(selection))` installs the adjusted selection;
- `roastty_terminal_selection_adjust` exposes terminal-level C ABI coverage.

This experiment wires the existing terminal behavior into
`roastty_surface_binding_action("adjust_selection:<direction>")` and adds the
surface/terminal viewport logic needed to keep the adjusted endpoint visible.

This does not implement copy/paste actions, search actions, write-file actions,
keybind storage/lookup, frontend selection routing, or clipboard integration.

## Changes

- `roastty/src/lib.rs`
  - Extend the internal parsed binding-action enum with
    `AdjustSelection(TerminalSelectionAdjustment)`.
  - Add a parser for Ghostty's adjustment names:
    - `left`
    - `right`
    - `up`
    - `down`
    - `page_up`
    - `page_down`
    - `home`
    - `end`
    - `beginning_of_line`
    - `end_of_line`
  - Extend `parse_binding_action` to accept `adjust_selection:<direction>` and
    reject missing, empty, unknown, whitespace-padded, or extra-colon
    parameters.
  - Add a surface helper that:
    - returns `false` for null, detached, and no-worker surfaces;
    - returns `false` when the worker-backed terminal has no active selection,
      matching upstream fall-through behavior;
    - adjusts the existing active selection with `Terminal::selection_adjust`;
    - installs the adjusted selection with `Terminal::set_selection`;
    - scrolls the adjusted selection end point into view;
    - requests a render and returns `true` after a successful adjustment.
  - Keep split, close, `text:`, `csi:`, `esc:`, `reset`, `clear_screen`, scroll,
    prompt-jump, and select-all action semantics unchanged.

- `roastty/src/terminal/screen.rs`
  - Add a helper that scrolls a supplied selection endpoint into view using the
    same rule as upstream Ghostty:
    - if the endpoint is already between viewport top-left and bottom-right, do
      not move the viewport;
    - if the endpoint is above the viewport, scroll to the endpoint pin;
    - if the endpoint is below the viewport, scroll to `endpoint - (rows - 1)`
      where possible so the endpoint lands on the bottom visible row.
    - if `rows <= 1` or walking up `rows - 1` rows from the endpoint cannot
      produce a valid pin, scroll to the endpoint pin instead and let the
      existing viewport clamping preserve integrity.

- `roastty/src/terminal/terminal.rs`
  - Add a terminal-level forwarding helper for the endpoint-scroll behavior.

- `roastty/tests/abi_harness.c`
  - Add C ABI smoke coverage that malformed `adjust_selection` forms are
    rejected.
  - Add no-worker coverage that representative valid adjustment forms return
    `false` without crashing.

- Tests in `roastty/src/lib.rs`
  - Cover parser false paths for missing, empty, unknown, whitespace-padded, and
    extra-colon `adjust_selection` forms.
  - Cover null, detached, no-worker, and no-active-selection surfaces returning
    `false`.
  - Cover all valid parser forms in a no-worker table proving they parse and
    return `false` without crashing: `left`, `right`, `up`, `down`, `page_up`,
    `page_down`, `home`, `end`, `beginning_of_line`, and `end_of_line`.
  - Cover worker-backed adjustments for representative horizontal, vertical,
    page, home/end, and beginning/end-of-line directions by comparing the active
    selection after the binding action to `Terminal::selection_adjust`.
  - Cover upward and downward endpoint scroll behavior.
  - Cover the below-viewport fallback for a one-row viewport or otherwise
    invalid upward adjustment.
  - Cover the already-visible endpoint case leaving the viewport unchanged while
    still requesting render.
  - Re-run existing binding-action tests to prove previous action semantics did
    not change.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty adjust_selection -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Result

**Result:** Pass

Roastty now accepts `adjust_selection:<direction>` as a surface binding action
for all ten Ghostty direction names: `left`, `right`, `up`, `down`, `page_up`,
`page_down`, `home`, `end`, `beginning_of_line`, and `end_of_line`.
Parameterized malformed forms, missing parameters, whitespace-padded names,
extra-colon forms, and unknown directions are rejected.

Attached worker-backed surfaces now reuse the existing terminal selection
adjustment implementation. If no active selection exists, the action returns
`false` and does not request render. If an active selection exists, the action
adjusts it, installs the adjusted selection, scrolls the adjusted endpoint into
view, requests render, and returns `true`. Boundary no-op adjustments with an
active selection are still consumed as `true`, preserve the selection, and
request render, matching upstream fall-through semantics.

Endpoint scrolling follows the upstream rule. Already-visible endpoints leave
the viewport unchanged. Endpoints above the viewport scroll to the endpoint.
Endpoints below the viewport scroll so the endpoint lands on the bottom visible
row. The one-row fallback scrolls to the endpoint row and relies on existing
viewport clamping.

The C ABI harness now smoke-tests malformed `adjust_selection` forms and valid
no-worker adjustment forms.

Verification run:

- `cargo fmt -p roastty`
- `cargo test -p roastty adjust_selection -- --nocapture --test-threads=1` — 7
  passed
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1` — 62
  passed
- `cargo test -p roastty --test abi_harness` — 1 passed
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Conclusion

Experiment 716 completes the selection-adjustment binding-action path using the
existing terminal selection machinery plus a new endpoint-specific viewport
scroll helper. Remaining binding-action gaps are now outside local selection
mutation: clipboard actions, search actions, write-file actions, cursor-key
actions, and keybind storage/dispatch.

## Design Review

Codex reviewed the Experiment 716 design and found the scope otherwise matches
upstream behavior: return `false` when there is no active selection, adjust and
install the active selection endpoint, scroll that endpoint into view, request
render, and return `true` on success.

The review raised two technical blockers before plan commit. First, parser
coverage needed to be explicit for every accepted direction rather than only
representative groups. The plan now requires a no-worker table covering all ten
valid direction names. Second, the endpoint-scroll fallback for below-viewport
targets needed a deterministic rule. The plan now states that `rows <= 1` or
failed `endpoint - (rows - 1)` movement falls back to scrolling to the endpoint
pin, relying on existing viewport clamping for integrity.

The review also raised the normal workflow provenance requirement. Design-review
frontmatter and this review section are now present, and the README provenance
tuple will be updated to `Codex/Codex/-` before the plan commit. Result-review
provenance will be added only after implementation and completion review.

## Completion Review

Codex reviewed the completed Experiment 716 diff and first found one real
semantic issue: active-selection boundary/no-op adjustments could return `false`
if `Terminal::selection_adjust` returned `None`, while upstream only falls
through when there is no active selection. The implementation now preserves the
existing selection with `unwrap_or(selection)`, scrolls the endpoint, returns
`true`, and lets the surface request render. A focused test now covers this
boundary no-op behavior.

Codex re-reviewed the corrected diff and found no remaining code blockers. The
review confirmed that parser coverage includes all ten direction names,
malformed forms are rejected, no-selection returns `false` without render,
successful adjustments install the expected selection, boundary no-op
adjustments preserve selection while consuming and rendering, and endpoint
scrolling covers above, below, visible, and one-row fallback behavior.

The only remaining required finding was workflow provenance: the result-review
frontmatter, this completion-review section, and the README provenance tuple
needed to be recorded before the result commit. Those fields are now present.
The review noted that the result lists `cargo fmt -p roastty`; that command was
run before the focused test command.
