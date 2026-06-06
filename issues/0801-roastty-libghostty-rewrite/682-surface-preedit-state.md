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

# Experiment 682: Surface Preedit State

## Description

Experiment 681 added `roastty_surface_text(surface, ptr, len)` for paste-style
text input. The adjacent upstream embedded ABI is
`ghostty_surface_preedit(surface, ptr, len)`, which sets IME/dead-key preedit
text on the core surface and clears it when `len == 0`.

Upstream core preedit handling stores renderer preedit state, marks terminal
preedit dirty, clears selection depending on config, converts UTF-8 text into
codepoints with Unicode cell widths, and queues a render. Roastty already has a
renderer `Preedit` value type, but the issue checklist still records the full
Unicode width subsystem and live renderer integration as missing. This
experiment therefore adds a narrow ABI/state slice: accept the renamed preedit
ABI, validate and store the UTF-8 bytes on the surface, clear on zero length,
mark the surface dirty, and wake the app runtime.

This experiment does not implement renderer preedit placement, glyph rendering,
selection-clearing policy, Unicode width tables, IME frontend routing, or
render-state preedit export.

## Changes

- `roastty/include/roastty.h`
  - Add
    `ROASTTY_API void roastty_surface_preedit(roastty_surface_t, const char*, uintptr_t);`
    alongside `roastty_surface_text`.
- `roastty/src/lib.rs`
  - Add `preedit: Option<String>` to `Surface`.
  - Initialize new surfaces with no preedit.
  - Add `roastty_surface_preedit(surface, ptr, len)`.
  - Null surfaces are a no-op.
  - `len == 0` clears the stored preedit even if `ptr` is null.
  - Null pointers with nonzero length are a no-op.
  - Non-UTF-8 input clears the existing stored preedit for upstream fidelity:
    upstream clears prior preedit before UTF-8 validation and still marks
    preedit dirty.
  - Valid UTF-8 input stores an owned string.
  - Setting or clearing preedit marks the surface dirty and invokes `wakeup_cb`
    when the surface is attached to a live app with a wakeup callback.
  - Detached live surfaces update stored preedit state and dirty state without
    dereferencing the cleared app pointer or waking the app.
  - Add tests:
    - null surface is a no-op;
    - null pointer with nonzero length is a no-op;
    - valid UTF-8 stores owned preedit text;
    - source bytes may be dropped after the call;
    - a second valid preedit replaces the first;
    - valid multi-byte UTF-8 stores correctly;
    - zero length clears stored preedit;
    - invalid UTF-8 clears stored preedit;
    - invalid UTF-8 marks dirty and invokes `wakeup_cb` while attached;
    - setting and clearing preedit mark the surface dirty;
    - setting and clearing preedit invoke `wakeup_cb` while attached;
    - detached surfaces update/clear preedit and dirty state without waking.
- `roastty/tests/abi_harness.c`
  - Exercise `roastty_surface_preedit(surface, text, len)` and
    `roastty_surface_preedit(surface, NULL, 0)` through the C header on a live
    skeleton surface.
  - Exercise null-surface and null-pointer nonzero-length no-op calls to prove
    the symbol exists and is null-safe.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/682-surface-preedit-state.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `git diff --check`

## Design Review

**Result:** Approved after amendments.

Codex found one upstream-fidelity issue and two test gaps. The original plan
kept stale preedit text on invalid UTF-8, but upstream clears prior preedit
before validating new UTF-8 and marks preedit dirty. The plan now clears stored
preedit on invalid nonzero input, marks dirty, and wakes the attached app just
like a valid set or clear.

Codex also asked for tests proving a second valid preedit replaces the first and
that valid multi-byte UTF-8 is accepted. Those cases are now in the test plan,
along with explicit invalid-input dirty/wakeup expectations.

Codex otherwise approved the narrow state slice as long as renderer placement,
Unicode widths, selection clearing, and preedit export remain out of scope.

## Result

**Result:** Pass

Implemented `roastty_surface_preedit(surface, ptr, len)` in the public C header
and Rust ABI. Surfaces now store preedit as an owned `Option<String>`. New
surfaces start with no preedit, valid UTF-8 stores or replaces the current
value, zero-length calls clear the value, and invalid UTF-8 clears the current
value for upstream fidelity.

Setting, clearing, and invalid-input clearing mark the surface dirty and invoke
the app runtime `wakeup_cb` while the surface is attached to a live app.
Detached surfaces still update and dirty their stored preedit state without
dereferencing the cleared app pointer or waking the app. Null surfaces and null
pointers with nonzero length remain no-ops.

The Rust tests cover null/no-op cases, owned storage, replacement, multi-byte
UTF-8, zero-length clear, invalid UTF-8 clear, dirty state, wakeup delivery, and
detached-surface behavior. The C ABI harness now calls `roastty_surface_preedit`
through `roastty.h` on null and live skeleton surfaces, including live set,
null-pointer nonzero no-op, and zero-length clear.

Verification passed:

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/682-surface-preedit-state.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty --test abi_harness`
- `git diff --check`

## Conclusion

Roastty now accepts and stores IME/dead-key preedit text at the renamed surface
ABI boundary. Renderer preedit placement, Unicode width tables, selection
clearing, and render-state/preedit export remain future slices.

## Completion Review

**Result:** Approved after provenance update.

Codex found no code issues. It confirmed that the ABI/header addition is
correct, the implementation clears on `len == 0`, no-ops nonzero null pointers,
clears on invalid UTF-8 for upstream fidelity, and marks dirty/wakes through
`request_render()`. Detached-surface safety follows the existing null-app wakeup
path.

Codex also confirmed that the result documentation correctly scopes this to
stored preedit state and leaves renderer placement, Unicode widths, selection
clearing, and render-state/preedit export as future work. The first
completion-review pass blocked only because `[review.result]`, this
completion-review section, and the README `Codex/Codex/Codex` tuple had not yet
been recorded.
