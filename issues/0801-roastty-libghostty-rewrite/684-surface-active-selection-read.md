+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5"
reasoning = "medium"
+++

# Experiment 684: Surface Active Selection Read

## Description

Experiment 683 added explicit-selection surface text readback through
`roastty_surface_read_text(surface, selection, result)`. Upstream Ghostty also
exposes `ghostty_surface_has_selection(surface)` and
`ghostty_surface_read_selection(surface, result)`, which query and read the
surface's active user selection.

Roastty already supports active selections on terminals and can set/read them
through the terminal ABI. Surface terminals currently live behind an attached
`TermioWorker`, so this experiment adds the surface active-selection query and
read APIs against the worker terminal only. It does not implement frontend mouse
selection routing, quicklook/word selection metadata, viewport pixel metadata,
or any UI integration; tests will seed the worker terminal selection directly.

## Changes

- `roastty/include/roastty.h`
  - Add `ROASTTY_API bool roastty_surface_has_selection(roastty_surface_t);`
  - Add
    `ROASTTY_API bool roastty_surface_read_selection(roastty_surface_t, roastty_text_s*);`
    next to the existing surface text read APIs.
- `roastty/src/lib.rs`
  - Factor the Experiment 683 selection-format/allocation path enough that both
    explicit selection reads and active selection reads use the same contract:
    - plain formatting;
    - `unwrap = true`;
    - `trim = false`;
    - `text_len + 1` allocation with trailing NUL sentinel;
    - unavailable metadata defaults of `tl_px_x = -1`, `tl_px_y = -1`,
      `offset_start = 0`, and `offset_len = 0`.
  - Implement `roastty_surface_has_selection(surface)`:
    - null, detached, or no-worker surfaces return `false`;
    - worker-backed surfaces return whether the worker terminal has an active
      selection.
  - Implement `roastty_surface_read_selection(surface, result)`:
    - null result returns `false`;
    - null, detached, or no-worker surfaces return `false` after writing an
      empty result;
    - worker-backed surfaces with no active selection return `false` after
      writing an empty result;
    - worker-backed surfaces with an active selection return owned text using
      the same text ownership and metadata contract as
      `roastty_surface_read_text`;
    - callers must free a previous successful `roastty_text_s` before reusing
      the same result struct.
  - Keep `roastty_surface_read_text` explicit-selection behavior unchanged.
  - Add tests:
    - `roastty_surface_has_selection` returns false for null, detached,
      no-worker, and no-selection surfaces;
    - `roastty_surface_has_selection` returns true after seeding the worker
      terminal active selection;
    - `roastty_surface_read_selection` validates null result and empty-result
      failure paths;
    - active selection read returns the expected plain text;
    - active selection read uses the sentinel allocation and unavailable
      metadata defaults;
    - returned active-selection text remains owned across a surface tick;
    - clearing the worker active selection makes has/read return false again.
- `roastty/tests/abi_harness.c`
  - Exercise null/no-worker `roastty_surface_has_selection` and
    `roastty_surface_read_selection` through the public C header.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/684-surface-active-selection-read.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `git diff --check`

## Design Review

**Result:** Approved.

Codex found no design blockers. It approved the scope as coherent and narrow:
add only `roastty_surface_has_selection` and `roastty_surface_read_selection`
over the attached `TermioWorker` terminal's active selection, while leaving
frontend mouse selection routing, quicklook metadata, viewport pixel metadata,
and UI integration out of scope.

Codex also confirmed the ABI and ownership plan is consistent with Experiment
683: reuse `roastty_text_s`, preserve the `text_len + 1` allocation with a
trailing NUL sentinel, require `roastty_surface_free_text`, and document that
callers must free a previous successful result before reusing the same struct.
The planned verification covers null, detached, no-worker, no-selection,
positive active-selection, sentinel/default metadata, lifetime, clearing, and C
header exposure paths.
