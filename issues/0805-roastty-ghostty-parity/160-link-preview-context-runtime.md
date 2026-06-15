# Experiment 160: Link preview context runtime

## Description

This experiment narrows the remaining `RUNTIME-012B2B2B2B2B`
notification/link/bell gap by proving the deterministic config predicate slice
for link previews plus the runtime slice behind link-specific context-menu
selection.

Pinned Ghostty's `Surface.zig` link handling has three separable behaviors in
this area:

- regular detected links preview only when `link-previews = true`;
- OSC 8 hyperlinks preview when `link-previews != false`;
- right-click `context-menu` selects an existing link at the cursor position and
  returns unhandled so the app can show the native context menu.

Roastty already proves generic open-url dispatch, renderer link matching,
non-link `right-click-action`, and copied macOS hover-banner plumbing. This
experiment does not claim actual native menu display, OS URL opening, or live
mouse hover/cursor UI parity. Those remain GUI gaps until a real app walkthrough
proves them.

## Changes

- Add focused Rust unit coverage in `roastty/src/lib.rs` for:
  - `link-previews` regular-link and OSC 8 preview predicate semantics;
  - right-click `context-menu` selecting a link-shaped cell range at the cursor
    position and returning unhandled;
  - right-click `context-menu` preserving an existing selection when the click
    is inside it and still returning unhandled.
- Add
  `issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py` as
  the durable Issue 805 guard for this slice.
- Split a new `RUNTIME-012B2B2B2B2B1` row out of `RUNTIME-012B2B2B2B2B` in
  `config-runtime-inventory.md`, then reduce the remaining gap row to
  `RUNTIME-012B2B2B2B2B2`.
- Update `config_runtime_inventory.py`, `config-matrix.md`, and the issue
  learnings with the new row counts and reusable finding.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml link_preview_context_runtime`
  passes.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py`
  passes.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  passes and reports the updated CFG-223 counts.

The experiment passes only if the remaining gap row still explicitly lists the
unproven actual GUI/OS behaviors: OS banner/sound delivery, actual audio/dock/
border/title effects, real app link hover/cursor UI, real app link previews, and
native context/menu link flows.

## Design Review

**Reviewer:** Helmholtz the 2nd (`019eca6f-7a0c-7721-9993-6165d8e3242f`)

**Verdict:** Approved

The reviewer found that the design is narrow enough, does not overclaim GUI
parity, uses sufficient verification for this deterministic runtime slice, and
follows the Issue 805 one-experiment-at-a-time workflow. The reviewer added a
non-blocking implementation note: the Python guard should prove the behavior
against pinned Ghostty semantics, not only Roastty internals in isolation.

## Result

**Result:** Pass

Implemented the deterministic link preview predicate/context-menu runtime slice
in `roastty/src/lib.rs`:

- right-click `context-menu` now tries link selection before word fallback;
- effective mouse modifiers are computed before entering the Termio worker lock,
  avoiding lock re-entry while preserving Ghostty's shift-capture semantics;
- OSC 8 link selection is gated by ctrl/super, matching pinned Ghostty's
  `linkAtPos` behavior;
- configured regex links honor their modifier requirements before selecting the
  matched link range;
- configured regex links now search only the clicked line with semantic prompt
  boundaries enabled, matching pinned Ghostty's `linkAtPin` search scope instead
  of scanning the whole viewport.

Added `link_preview_context_runtime_*` Rust tests proving preview predicates,
regex-link context selection, line-scoped regex matching, containing-selection
preservation, and OSC 8 ctrl/super context selection.

Added
`issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py`,
which checks pinned Ghostty source markers, Roastty implementation/test markers,
the new complete inventory row, the reduced gap row, and updated CFG-223 counts.

Verification:

- `cargo test --manifest-path roastty/Cargo.toml link_preview_context_runtime -- --test-threads=1`
  passed with 5 tests.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/link_preview_context_runtime_parity.py`
  passed.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  passed and reported `runtime_rows=68`, `oracle_complete=61`, `closed=64`,
  `incomplete=4`, `gap=4`, `cfg223=Gap`.

## Conclusion

The deterministic `link-previews` config predicates and link-specific
right-click context-menu selection runtime semantics are now proven. Actual
runtime `mouse_over_link` preview dispatch, native preview display, native
context-menu display, live hover/cursor GUI behavior, OS URL-opening flows, OS
notification delivery, and actual bell side effects remain in the reduced
`RUNTIME-012B2B2B2B2B2` gap.

## Completion Review

**Reviewer:** Galileo the 2nd (`019eca7d-368f-7b51-a925-fc5062dadcb5`)

**Initial verdict:** Blocked

The reviewer found two real issues:

- the result overclaimed runtime preview gating even though Roastty does not yet
  dispatch `ROASTTY_ACTION_MOUSE_OVER_LINK`;
- configured regex link selection searched the whole viewport, while pinned
  Ghostty's `linkAtPin` scopes regex matching to the clicked line with semantic
  prompt boundaries.

Fixes applied:

- narrowed the experiment, inventory row, README learning, and guard to claim
  only `link-previews` config predicate parity, leaving runtime
  `mouse_over_link` preview dispatch in the remaining gap;
- added `Terminal::selection_viewport_string_map` and used
  `select_line(ref_, None, true)` before regex matching in
  `regex_link_selection_at_viewport_cell`;
- added `link_preview_context_runtime_context_menu_regex_is_line_scoped` to
  prove a cross-row regex does not match from the context-menu path;
- updated `link_preview_context_runtime_parity.py` to require the line-scoped
  search markers and the reduced preview claim.

**Second reviewer:** Ampere the 2nd (`019eca83-5a01-7de0-a002-dc1ea704ee4f`)

**Final verdict:** Approved

The second reviewer found no blocking issues after the fixes. They confirmed
that the completed row is limited to link-preview config predicates plus
context-menu selection semantics, runtime `mouse_over_link` preview dispatch and
native preview display remain in the gap, regex matching now uses
`select_line(ref_, None, true)` plus `selection_viewport_string_map`, the
line-scope regression is covered, and the guard checks Ghostty markers, Roastty
markers, tests, inventory split, and CFG-223 counts.
