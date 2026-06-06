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

# Experiment 707: Binding Action Scroll Top and Bottom

## Description

Experiments 702-706 added surface binding-action invocation for split actions,
`close_surface`, `text:`, `csi:`, `esc:`, and `reset`. Upstream Ghostty's
`performBindingAction` also supports viewport scroll actions. The smallest
useful next slice is the two parameterless viewport endpoints:

- `scroll_to_top` queues a viewport scroll to the top of the screen history;
- `scroll_to_bottom` queues a viewport scroll back to the active bottom;
- both are void actions, so colon-bearing forms such as `scroll_to_top:` and
  `scroll_to_bottom:now` are malformed;
- both return `true` when performed on an attached surface.

Roastty already has viewport machinery in `PageList::scroll(Scroll::Top)` and
`PageList::scroll(Scroll::Active)`, plus existing tests that prove those lower
level behaviors. This experiment exposes only the terminal/surface helper needed
for these two binding actions and verifies the public C ABI path.

This does not implement `clear_screen`, `scroll_to_row`, `scroll_to_selection`,
page scroll actions, fractional/line scroll actions, prompt jumps, search
actions, clipboard actions, cursor-key actions, full keybind storage/lookup, or
app-scoped actions.

## Changes

- `roastty/src/terminal/terminal.rs`
  - Add a small viewport helper, for example `scroll_viewport_top_bottom`, that
    maps top to the active screen's top viewport and bottom to the active
    viewport.
  - Keep the helper internal to the crate; do not add public C ABI terminal
    functions for this experiment.
  - Add focused terminal tests, if needed, that prove the helper moves the
    viewport top-left to the history top and back to the active bottom.

- `roastty/src/terminal/screen.rs` and `roastty/src/terminal/page_list.rs`
  - Add the minimal scoped helper path needed by `Terminal`:
    `PageList::scroll(Scroll::Top)` and `PageList::scroll(Scroll::Active)` are
    currently private to `page_list.rs`, so the implementation should expose
    small `pub(super)` wrappers rather than widening the `Scroll` enum broadly.
  - Keep the new helpers scoped inside the terminal module tree.

- `roastty/src/lib.rs`
  - Extend the internal parsed binding-action enum with top/bottom viewport
    actions.
  - Extend `parse_binding_action` to accept exact `scroll_to_top` and
    `scroll_to_bottom` forms and reject any colon-bearing parameters.
  - Add/use a surface helper that locks the active termio worker, calls the
    terminal viewport helper, and requests a render.
  - Return `true` for attached parsed scroll top/bottom actions, even when no
    termio worker exists, matching action-consumed semantics.
  - Return `false` for null or detached surfaces.
  - Keep split, close, `text:`, `csi:`, `esc:`, and `reset` semantics unchanged.

- `roastty/tests/abi_harness.c`
  - Add C ABI smoke coverage that colon-bearing top/bottom forms are rejected
    and exact `scroll_to_top` / `scroll_to_bottom` can be invoked.

- Tests in `roastty/src/lib.rs`
  - Cover `scroll_to_top:`, `scroll_to_top:now`, `scroll_to_bottom:`, and
    `scroll_to_bottom:now` returning false.
  - Cover null and detached surfaces returning false for both actions.
  - Cover attached no-worker surfaces returning true without side effects.
  - Cover worker-backed `scroll_to_top` moves the viewport to the exact history
    top when scrollback exists, using `viewport_bounds` plus point conversion to
    assert that the top-left viewport ref maps to screen coordinate `(0, 0)`.
  - Cover worker-backed `scroll_to_bottom` returns the viewport to the exact
    active bottom after `scroll_to_top`, asserting that the top-left viewport
    ref maps to the expected bottom coordinate for the terminal's total rows and
    viewport rows.
  - Re-run existing binding-action tests to prove previous action semantics did
    not change.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty binding_action -- --nocapture`
- `cargo test -p roastty scroll_to_top -- --nocapture`
- `cargo test -p roastty scroll_to_bottom -- --nocapture`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the initial Experiment 707 design and blocked plan commit until
three concrete design gaps were fixed.

First, the plan named `PageList::scroll(Scroll::Top)` and
`PageList::scroll(Scroll::Active)` as if `terminal.rs` could call them directly,
but both the `Scroll` enum and `scroll` method are private to `page_list.rs`.
The plan now requires small scoped wrappers through `page_list.rs` and
`screen.rs` instead of broad visibility changes.

Second, the original worker-backed test only required `scroll_to_top` to move
away from the active bottom, which would not prove the exact upstream endpoint.
The plan now requires exact viewport top-left assertions after both top and
bottom actions.

Third, malformed-input coverage only mentioned one empty-colon form and one
non-empty-colon form. The plan now requires both empty and non-empty colon
variants for `scroll_to_top` and `scroll_to_bottom`.

## Result

**Result:** Pass

Implemented exact `scroll_to_top` and `scroll_to_bottom` binding-action support
for attached surfaces. `parse_binding_action` now accepts only the exact
parameterless forms and rejects colon-bearing variants for both actions.
Dispatch returns `false` for null or detached surfaces, returns `true` for
attached surfaces, and routes worker-backed surfaces through a narrowly scoped
terminal viewport helper.

The terminal helper path keeps `PageList::scroll` and the private `Scroll` enum
inside `page_list.rs`. `page_list.rs` exposes only `scroll_top` and
`scroll_active` to `screen.rs`, `screen.rs` exposes matching helpers to
`terminal.rs`, and `terminal.rs` exposes the crate-local surface-facing helpers.

The Rust tests cover malformed top/bottom forms, null/detached surfaces,
attached no-worker surfaces, exact worker-backed viewport top and bottom
endpoints, and unchanged binding-action behavior around split, close, `text:`,
`csi:`, `esc:`, and `reset`. The C ABI harness now rejects colon-bearing
top/bottom forms and accepts exact `scroll_to_top` / `scroll_to_bottom`.

Verification:

- `cargo fmt -p roastty` passed.
- `cargo test -p roastty binding_action -- --nocapture` passed: 27 tests.
- `cargo test -p roastty scroll_to_top -- --nocapture` passed: 1 test.
- `cargo test -p roastty scroll_to_bottom -- --nocapture` passed: 2 tests,
  including the new endpoint test and one existing config default test.
- `cargo test -p roastty --test abi_harness` passed.
- `cargo fmt -p roastty -- --check` passed.
- `git diff --check` passed.

## Conclusion

The top/bottom viewport endpoint slice now follows upstream's void-action
parsing contract and uses Roastty's existing viewport model without widening
private page-list internals. The remaining viewport binding-action work can
build on this path for row, selection, page, fractional, line, and prompt
scrolling actions.

## Completion Review

Codex reviewed the completed Experiment 707 diff and found no implementation
blockers. The review confirmed that both actions reject colon-bearing
parameters, return `false` for null/detached surfaces, consume attached
no-worker actions, and route worker-backed surfaces through the scoped viewport
helper path.

The review also confirmed that the helper path keeps `Scroll` private inside
`page_list.rs` and only exposes narrow `scroll_top` / `scroll_active` wrappers
through `screen.rs` and `terminal.rs`. The only required fix was workflow
provenance: replacing the pending result-review metadata, adding this
completion-review note, and updating the README provenance tuple to
`Codex/Codex/Codex`.
