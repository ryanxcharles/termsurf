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

# Experiment 683: Surface Read Text

## Description

Experiment 682 added stored surface preedit state. The next narrow surface
readback gap is `ghostty_surface_read_text(surface, selection, result)` plus
`ghostty_surface_free_text(surface, result)`. Upstream reads arbitrary selected
terminal text into an owned `ghostty_text_s`, and callers free that owned text
through the matching surface API.

Roastty already has `roastty_selection_s`, grid-ref validation, and terminal
selection formatting. Surface terminals currently live behind the attached
`TermioWorker`, so this experiment adds explicit-selection readback from the
worker terminal. It does not implement active selection reads
(`roastty_surface_read_selection`), viewport pixel metadata, quicklook word
metadata, or frontend selection routing.

## Changes

- `roastty/include/roastty.h`
  - Add `roastty_text_s`, matching upstream layout:
    - `double tl_px_x`
    - `double tl_px_y`
    - `uint32_t offset_start`
    - `uint32_t offset_len`
    - `const char* text`
    - `uintptr_t text_len`
  - Add
    `ROASTTY_API bool roastty_surface_read_text(roastty_surface_t, roastty_selection_s, roastty_text_s*);`
  - Add
    `ROASTTY_API void roastty_surface_free_text(roastty_surface_t, roastty_text_s*);`
- `roastty/src/lib.rs`
  - Add `RoasttyText`.
  - Add a helper that writes an empty text result before attempting a read.
  - Implement `roastty_surface_read_text(surface, selection, result)`:
    - null surface returns `false`;
    - null result returns `false`;
    - detached or no-worker surfaces return `false`;
    - invalid selection/grid refs return `false`;
    - valid explicit selections format plain text from the worker terminal with
      `Plain`, `unwrap = true`, and `trim = false`;
    - allocate `text_len + 1` owned bytes for `result.text`, write a trailing
      NUL sentinel, and expose `text_len` excluding that sentinel;
    - because viewport metadata is not available yet, set `tl_px_x = -1`,
      `tl_px_y = -1`, `offset_start = 0`, and `offset_len = 0`.
  - Implement `roastty_surface_free_text(surface, result)`:
    - null result is a no-op;
    - free the full owned `text_len + 1` allocation and reset the struct to an
      empty value;
    - surface is ignored, matching upstream's surface-parameter shape while
      keeping ownership on the text result.
  - Add tests:
    - null result and null surface return false and leave no allocation;
    - no-worker and detached surfaces return false;
    - invalid selection returns false;
    - valid worker-backed selection returns expected plain text;
    - successful reads expose a trailing NUL at `result.text[result.text_len]`;
    - successful reads set unavailable metadata fields to `tl_px_x = -1`,
      `tl_px_y = -1`, `offset_start = 0`, and `offset_len = 0`;
    - returned text is owned and remains valid after another surface tick;
    - free_text resets the struct and is safe for null and empty results;
    - repeated read/free does not reuse stale pointer state;
    - callers must free a previous successful result before reusing the same
      `roastty_text_s`; this experiment does not try to recover an overwritten
      owned pointer.
- `roastty/tests/abi_harness.c`
  - Assert `roastty_text_s` size/offsets.
  - Exercise null/no-worker `roastty_surface_read_text` and
    `roastty_surface_free_text` through the C header.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/683-surface-read-text.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `git diff --check`

## Design Review

**Result:** Approved after amendments.

Codex found two upstream-fidelity/ownership gaps and two test-contract gaps. The
returned `Text.text` must follow upstream's sentinel-terminated ownership model:
allocate `text_len + 1`, write a trailing NUL, report `text_len` without the
sentinel, and free the full allocation. The plan now states that contract and
adds a sentinel assertion.

Codex also noted that upstream surface text reads use plain formatting with
`unwrap = true` and `trim = false`, so the plan now requires those exact
formatter options. The success tests now also check unavailable metadata
defaults. Finally, the plan now states that callers must free a previous
successful `roastty_text_s` before reusing the struct, and the repeated
read/free test covers the intended non-stale-pointer flow.

Codex otherwise approved the scope: explicit-selection readback from the
attached worker terminal only, with active selection, viewport pixel metadata,
quicklook metadata, and frontend selection routing left out of this slice.
