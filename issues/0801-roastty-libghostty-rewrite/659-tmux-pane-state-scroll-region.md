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

# Experiment 659: Tmux Pane State Scroll Region

## Description

Experiment 658 restored the tmux pane-state mouse event and mouse format mode
subset. Upstream Ghostty's next pane-state step restores the vertical scroll
region from `scroll_region_upper` and `scroll_region_lower`, using tmux's
0-based row values.

This experiment applies that scroll-region subset to tracked pane terminals. The
restore path should set only the top and bottom row bounds from pane state; left
and right margins stay unchanged. Tab stops, alternate saved cursor restoration,
live pane output, PTY writes, and App integration remain out of scope.

## Changes

- `roastty/src/terminal/terminal.rs`
  - Add a narrow tmux-facing helper to apply pane-state vertical scroll-region
    bounds.
  - Treat tmux values as 0-based rows, matching upstream's direct assignment.
  - Preserve existing left/right margins instead of resetting them.
  - Ignore invalid parsed bounds non-fatally and leave the existing region
    unchanged. Invalid scroll-region bounds are different from malformed
    pane-state output: the line parsed successfully, so the viewer should not
    enter the defunct state.
  - Validate the full candidate `ScrollingRegion` before assignment, including
    preserved left/right margins. The candidate is invalid if `top > bottom`,
    `bottom >= rows`, or `top == bottom` on a multi-row pane.
- `roastty/src/terminal/tmux.rs`
  - Call the scroll-region helper after cursor, non-mouse mode, and mouse mode
    pane-state restoration.
  - Add test-only pane setter/accessor helpers that work with a plain
    `(top, bottom, left, right)` tuple, so tests can pre-seed horizontal margins
    and inspect the result without exposing the private `ScrollingRegion` type.
  - Preserve existing behavior for malformed pane-state output, stale pane IDs,
    and command-queue continuation.
  - Keep the pane-state fixture focused on the tmux field order; this experiment
    may add explicit scroll-region arguments while keeping mouse fields named.
- Tests in `roastty/src/terminal/tmux.rs`
  - Verify a pane state line applies a valid vertical scroll region to the
    tracked pane.
  - Verify left/right margins remain unchanged when tmux pane state restores the
    vertical region.
  - Verify stale pane IDs do not apply scroll-region changes while a later valid
    pane state line still does.
  - Verify invalid scroll-region bounds do not defunct the viewer and do not
    corrupt the existing region. Cover at least `top > bottom`,
    `bottom == rows`, and `top == bottom` on a multi-row pane.
  - Keep malformed pane-state output and command-queue continuation coverage in
    the tmux pane-state test set.

## Design Review

**Result:** Not approved on first review.

Codex approved the experiment scope but found that invalid bounds handling was
ambiguous. The design now requires invalid parsed scroll-region bounds to be
ignored non-fatally, leaving the previous region unchanged. Malformed pane-state
output still defuncts the viewer, but a valid line with unusable scroll bounds
does not.

Codex also required the exact validation contract to be documented. The helper
must validate the complete candidate region before assignment, including
preserved horizontal margins, and reject `top > bottom`, `bottom >= rows`, and
`top == bottom` on multi-row panes. The test plan now names these cases and
requires a test-only tuple setter/accessor so horizontal margin preservation is
provable.

**Re-review result:** Approved.

Codex confirmed the revised design resolves the invalid-bound ambiguity,
documents the validation contract, and includes test-only tuple setter/accessor
planning for horizontal margin preservation. It noted that implementation should
preserve the complete `ScrollingRegion` invariant, including horizontal rules,
which the design already requires by validating the full candidate region before
assignment.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/659-tmux-pane-state-scroll-region.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty terminal::tmux`
- `git diff --check`

## Result

**Result:** Pass.

Roastty now applies the tmux pane-state vertical scroll-region subset after
cursor, non-mouse mode, and mouse mode restoration. The restore path treats
`scroll_region_upper` and `scroll_region_lower` as 0-based tmux row values,
preserves existing left/right margins, validates the full candidate
`ScrollingRegion`, and ignores invalid parsed bounds without defuncting the
viewer.

The implementation added a direct tmux-facing terminal helper for scroll-region
restoration and a shared `ScrollingRegion::is_valid` predicate so normal test
setters and tmux restoration use the same invariant. The tmux tests use a
tuple-based pane accessor/setter to avoid exposing the private region type while
still proving horizontal margin preservation.

Verification covers valid restoration, left/right margin preservation, stale
pane IDs followed by a valid pane state line, and invalid parsed bounds for
`top > bottom`, `bottom == rows`, and `top == bottom` on a multi-row pane. The
existing tmux pane-state tests continue to cover malformed output defuncting the
viewer and successful command-queue continuation.

Verification passed:

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/659-tmux-pane-state-scroll-region.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty terminal::tmux` — 131 passed, 0 failed
- `git diff --check`

## Conclusion

Pane-state restoration now covers cursor state, non-mouse modes, mouse modes,
and vertical scroll regions. The remaining parsed pane-state fields are
alternate saved cursor position and tab stops; after those, the tmux work can
move toward live pane output, PTY writes, and App integration.

## Completion Review

**Result:** Approved.

Codex found no issues. It confirmed that
`Terminal::apply_tmux_scroll_region_state` treats tmux row values as 0-based,
preserves horizontal margins, validates the complete candidate with
`ScrollingRegion::is_valid`, and silently ignores invalid or overflowing bounds
without defuncting the viewer.

Codex also confirmed the tests cover valid restoration, horizontal margin
preservation, stale-pane handling with a later valid restore, and the approved
invalid cases. It judged the recorded result, conclusion, README status update,
and checklist wording accurate and sufficient for the result commit.
