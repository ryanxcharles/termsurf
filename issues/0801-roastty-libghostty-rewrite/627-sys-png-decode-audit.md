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
- no-ghostty grep on any touched issue docs — clean.
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
