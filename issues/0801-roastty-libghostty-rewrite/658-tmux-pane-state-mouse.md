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

# Experiment 658: Tmux Pane State Mouse Modes

## Description

Experiment 657 restored pane-state cursor visibility, cursor blinking, and the
non-mouse terminal mode subset. Upstream Ghostty restores tmux mouse event and
mouse format modes from the same pane-state payload immediately after those core
mode writes.

This experiment applies the six parsed tmux mouse flags to each tracked pane
terminal. The important part is the upstream-compatible mapping: tmux's field
names do not line up one-for-one with Roastty's mode names. The restore path
must map `mouse_all_flag` to `MouseEventAny`, `mouse_any_flag` to
`MouseEventButton`, `mouse_button_flag` to `MouseEventNormal`,
`mouse_standard_flag` to `MouseEventX10`, `mouse_utf8_flag` to
`MouseFormatUtf8`, and `mouse_sgr_flag` to `MouseFormatSgr`.

Scroll region, tab stops, alternate saved cursor restoration, live pane output,
PTY writes, and App integration remain out of scope.

## Changes

- `roastty/src/terminal/terminal.rs`
  - Add a narrow tmux-facing helper that applies mouse event and mouse format
    pane-state booleans directly to `ModeState`.
  - Keep the helper independent of `TmuxPaneState`; terminal core should accept
    explicit booleans rather than depending on tmux parser structures.
  - Do not route restoration through normal terminal mode execution paths. This
    mirrors upstream's direct `t.modes.set(...)` behavior and avoids unrelated
    side effects during state reconstruction.
  - Leave Roastty's runtime `flags.mouse_event` and `flags.mouse_format` caches
    unchanged in this experiment. Upstream's tmux viewer restores only the mode
    bits here; deciding whether Roastty needs cache synchronization for future
    App mouse forwarding belongs with the live input/output integration work.
- `roastty/src/terminal/tmux.rs`
  - Call the mouse-mode restoration helper after the cursor and non-mouse mode
    pane-state restoration added by Experiments 656 and 657.
  - Preserve existing behavior for malformed pane-state output, stale pane IDs,
    and command-queue continuation.
  - Update the pane-state fixture helper so all six mouse fields are explicit
    arguments instead of hardcoding `mouse_any_flag`, `mouse_button_flag`, or
    `mouse_utf8_flag`.
  - Prefer a named test fixture for mouse flags over six loose positional
    booleans so test calls cannot silently swap similarly named fields.
- Tests in `roastty/src/terminal/tmux.rs`
  - Verify pane-state mouse flags set all six event/format modes to true and
    then clear all six modes to false.
  - Verify the upstream field-name mapping with one-hot assertions for all six
    fields: each fixture enables exactly one tmux mouse flag and asserts that
    exactly the corresponding Roastty mode changes.
  - Verify stale pane IDs do not apply mouse modes while a later valid pane
    state line still does.
  - Keep malformed pane-state output and command-queue continuation coverage in
    the tmux pane-state test set.
  - Keep scroll region, tab stops, and alternate saved cursor behavior unchanged
    and out of scope.

## Design Review

**Result:** Not approved on first review.

Codex confirmed the upstream mouse field mapping and the narrow scope, but found
two design gaps. First, six positional fixture booleans could still mask swapped
field wiring, especially if tests only used all-true/all-false cases. The design
now requires a named mouse-flag fixture and one-hot assertions for all six tmux
mouse fields.

Second, Codex noted that Roastty has runtime `flags.mouse_event` and
`flags.mouse_format` caches used by mouse encoding, while upstream's tmux viewer
only restores `ModeState` bits in this path. The design now explicitly leaves
those runtime caches unchanged for upstream parity and defers any cache
synchronization decision to future live input/output integration work.

**Re-review result:** Technically approved, pending provenance fix.

Codex confirmed the revised design resolved the previous technical findings: it
requires one-hot coverage for all six mouse fields, avoids positional fixture
masking with a named mouse fixture, and explicitly documents the runtime cache
boundary. The only remaining blocker was missing issue-local provenance
frontmatter and a matching README experiment index tag; both were added before
the plan commit.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/658-tmux-pane-state-mouse.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty terminal::tmux`
- `git diff --check`

## Result

**Result:** Pass.

Roastty now applies the tmux pane-state mouse mode subset after the cursor and
non-mouse mode restoration added by Experiments 656 and 657. The restore path
maps tmux's six mouse fields to Roastty mode bits using the upstream Ghostty
mapping:

- `mouse_all_flag` -> `MouseEventAny`
- `mouse_any_flag` -> `MouseEventButton`
- `mouse_button_flag` -> `MouseEventNormal`
- `mouse_standard_flag` -> `MouseEventX10`
- `mouse_utf8_flag` -> `MouseFormatUtf8`
- `mouse_sgr_flag` -> `MouseFormatSgr`

The fixture helper now carries mouse fields through a named `TmuxPaneMouseFlags`
struct, so tests no longer hide three mouse fields behind hardcoded values. The
tests prove all six flags set and clear, each one-hot field maps to exactly one
expected Roastty mode, stale pane IDs do not apply mouse modes, malformed
pane-state output still defuncts the viewer, and successful pane-state handling
still emits the next queued command.

The implementation intentionally leaves `flags.mouse_event` and
`flags.mouse_format` unchanged. That matches upstream's tmux pane-state restore
path, which writes the mode bits directly; future live input/output integration
can decide whether Roastty needs an additional cache synchronization step for
mouse forwarding.

Verification passed:

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/658-tmux-pane-state-mouse.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty terminal::tmux` — 127 passed, 0 failed
- `git diff --check`

## Conclusion

Pane-state restoration now covers cursor state, non-mouse terminal modes, and
the tmux mouse event/format mode subset. The next pane-state experiments should
restore the remaining upstream fields: alternate saved cursor position, scroll
region, and tab stops, before moving on to live output, PTY writes, and App
integration.

## Completion Review

**Result:** Approved.

Codex found no blocking issues. It confirmed that
`Terminal::apply_tmux_mouse_mode_state` writes the six tmux mouse booleans
directly to `ModeState`, uses the approved upstream mapping, and intentionally
does not touch Roastty's runtime mouse caches. It also confirmed that
`TmuxViewer` calls the helper after cursor and non-mouse mode restoration, the
fixture field order matches the `list-panes` format, and the named mouse fixture
avoids the previous positional masking risk.

Codex judged the test coverage sufficient for this slice: all-true/all-false
coverage, one-hot mapping for all six fields, stale-pane behavior, and explicit
runtime-cache non-synchronization checks. It also confirmed that the recorded
result, README status update, and conclusion accurately describe the
implementation and remaining pane-state work.
