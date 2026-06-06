+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 627: sys PNG decode abstraction audit

## Description

Verify and close the Issue 801 checklist item for the `sys` PNG-decode
abstraction. The README still says `sys` is missing, but current Roastty source
already contains the C ABI option, callback storage, allocator bridge,
`sys_decode_png` wrapper, and Kitty graphics integration tests.

This is a verification-only experiment unless review or testing finds a real
gap. The intended result change is to update the issue checklist line:

```markdown
- [ ] `sys` (PNG-decode abstraction) — missing
```

to:

```markdown
- [x] `sys` (PNG-decode abstraction) — implemented and tested via C ABI
```

only after the focused gates prove the current implementation is complete.

## Current implementation surface

- `roastty/include/roastty.h` — exposes `roastty_sys_image_s`,
  `roastty_sys_decode_png_fn`, `ROASTTY_SYS_OPT_DECODE_PNG`, and
  `roastty_sys_set`.
- `roastty/src/lib.rs` — defines `RoasttySysImage`, `SysDecodePngCallback`,
  `SYS_STATE`, `sys_has_decode_png`, `sys_decode_png`,
  `sys_decode_png_with_limit`, the allocator bridge used by callbacks, and
  `roastty_sys_set`.
- `roastty/src/terminal/kitty/graphics_image.rs` — gates PNG file/temp/shared
  memory media when no decoder is installed, decodes PNG transmissions through
  `crate::sys_decode_png`, validates dimensions/data length, converts the image
  format to RGBA, and covers malformed callback output.
- `roastty/src/terminal/terminal.rs` — covers terminal stream integration for a
  Kitty PNG APC decoded through the sys callback.

## Verification

- `cargo test -p roastty sys_c_abi_sets_callbacks_and_userdata` — proves sys
  option discriminants, user data, callback installation, callback clearing, and
  invalid option handling.
- `cargo test -p roastty kitty_graphics_image_png` — proves direct PNG deferral
  without a decoder, direct decode through the sys callback, malformed callback
  handling, and oversized output rejection.
- `cargo test -p roastty kitty_graphics_image_non_direct_png` — proves
  non-direct PNG deferral without a decoder and file/temp/shared-memory decode
  when a callback is installed.
- `cargo test -p roastty terminal_stream_kitty_graphics_png_decodes_through_sys_callback`
  — proves terminal stream integration routes Kitty PNG APC data through the sys
  decoder and stores the decoded image.
- `cargo test -p roastty --test abi_harness` — proves the exported C header and
  dylib still link.
- `cargo test -p roastty` — full Roastty test suite stays green.
- forbidden compatibility-name grep on `roastty/include/roastty.h` and
  `roastty/src/lib.rs` — clean.
- `git diff --check` — clean.

Pass = the current source and tests prove the sys PNG-decode abstraction is
implemented for Issue 801, allowing the checklist item to be checked without new
code.

## Design Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED.

Initial review found one Required issue: the `kitty_graphics_image_png` test
filter did not cover the non-direct PNG media tests, so the plan overstated what
that gate proved. The design now narrows the direct PNG gate to direct/callback
behavior, adds a separate `kitty_graphics_image_non_direct_png` gate for
file/temp/shared-memory paths, and adds an explicit ABI harness gate for the C
ABI claim.

Follow-up review approved the verification-only approach and confirmed the gates
are broad enough to justify the checklist update if they pass.

## Result

**Result:** Pass

The current Roastty source already implements the `sys` PNG-decode abstraction
required by Issue 801:

- `roastty/include/roastty.h` exposes `roastty_sys_image_s`,
  `roastty_sys_decode_png_fn`, `ROASTTY_SYS_OPT_DECODE_PNG`, and
  `roastty_sys_set`.
- `roastty/src/lib.rs` stores sys callbacks/userdata, exposes
  `sys_has_decode_png`, routes `sys_decode_png` through the installed callback,
  supplies the callback allocator bridge, validates decoded output, frees
  oversized callback output, and implements `roastty_sys_set`.
- `roastty/src/terminal/kitty/graphics_image.rs` defers unsupported PNG input
  when no decoder is installed, decodes direct/file/temp/shared-memory PNG data
  through `sys_decode_png` when a decoder exists, validates decoded dimensions
  and data length, and converts the loaded image to RGBA.
- `roastty/src/terminal/terminal.rs` covers full terminal-stream Kitty PNG APC
  integration through the sys callback.

The README checklist line now records this as complete:

```markdown
- [x] `sys` (PNG-decode abstraction) — implemented and tested via C ABI
```

Gates (all green):

- `cargo test -p roastty sys_c_abi_sets_callbacks_and_userdata` — **1 passed / 0
  failed**.
- `cargo test -p roastty kitty_graphics_image_png` — **4 passed / 0 failed**.
- `cargo test -p roastty kitty_graphics_image_non_direct_png` — **2 passed / 0
  failed**.
- `cargo test -p roastty terminal_stream_kitty_graphics_png_decodes_through_sys_callback`
  — **1 passed / 0 failed**.
- `cargo test -p roastty --test abi_harness` — **1 passed / 0 failed**.
- `cargo test -p roastty` — **3461 passed / 0 failed** unit tests, plus **1
  passed / 0 failed** ABI harness test and **0** doc tests.
- `rg -n "\\bghostty_[A-Za-z0-9_]*\\b" roastty/include/roastty.h roastty/src/lib.rs`
  — clean.
- `git diff --check` — clean.

The original plan listed a broad no-Ghostty grep on issue docs. That is the
wrong hygiene check for this repository because Issue 801 and some test names
intentionally cite upstream Ghostty as source material. The result instead
checks the exported header and ABI surface file for forbidden `ghostty_*`
compatibility names.

## Conclusion

No new code was needed. The sys PNG-decode abstraction is implemented, tested
through the C ABI and Kitty graphics paths, and no longer belongs on the missing
terminal-behavior checklist.

## Completion Review

**Reviewer:** Codex (gpt-5.5, medium) · resumed session
`019e8f83-9029-7d43-8e82-f4c5754e14ba`

**Verdict:** APPROVED after fixing the README experiment index.

Initial completion review found one Required documentation issue: the checklist
update and experiment result were recorded, but the README experiment index
still listed Experiment 627 as `Designed`. The README now marks Experiment 627
as `Pass`.

Codex confirmed the checklist update is supported by the recorded evidence: C
ABI surface, callback install/clear/userdata tests, direct and non-direct Kitty
PNG decode tests, terminal stream integration, ABI harness, full suite,
forbidden compatibility-name grep, and `git diff --check`.
