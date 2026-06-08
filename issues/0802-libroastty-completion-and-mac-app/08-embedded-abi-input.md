+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"
+++

# Experiment 8: Embedded ABI — the input type surface (tranche 1 of the 56-symbol worklist)

## Description

Exp 7 mapped the real embedded-ABI gap: **56 missing `roastty_*` symbols** + the
Exp-6 by-value layout divergences. That's too big for one experiment, so it's
split into coherent tranches (input, action, config). This is **tranche 1 —
input**: make `libroastty` expose the embedded **by-value** input ABI the app
uses, byte-faithful to `ghostty.h`.

**Scope (the input subset of the worklist):**

- **By-value struct:** `roastty_input_key_s` (7 fields:
  `input_action_e action; input_mods_e mods; input_mods_e consumed_mods; uint32_t keycode; const char* text; uint32_t unshifted_codepoint; bool composing`)
  — currently `roastty.h` exposes only the opaque `roastty_key_event_t` handle.
  (Exp-6 divergence #1.)
- **Enums** (match upstream names + values): `input_action_e`
  (RELEASE/PRESS/REPEAT), `input_key_e` (`UNIDENTIFIED` + **176** key
  constants), `input_mouse_button_e`, `input_mouse_state_e`,
  `input_mouse_momentum_e`. `input_mods_e` already matches.
- **By-value functions:** `surface_key(surface, input_key_s) bool`,
  `app_key(app, input_key_s) bool` (a missing fn),
  `surface_key_is_binding(surface, input_key_s, binding_flags_e*) bool`.

**Leverage (verified by the design review):** `libroastty`'s internal `Key` enum
(`roastty/src/input/key.rs`) **already matches `ghostty.h`'s
`ghostty_input_key_e` value-for-value** (all 176 positions;
`KeyAction{Release=0,Press=1,Repeat=2}` matches `Action`), the
`roastty_input_key_s` 7-field layout is **byte-identical** to upstream, and the
internal reuse path is clean (`roastty_surface_key` calls `surface.key(event)` /
`surface.key_is_binding` on a `&mut KeyEvent` — not entangled with the opaque
handle's lifecycle). So **no internal value-mapping is needed** — this is a
**header-exposure + signature-change** task, not new key logic or ~300 new
constants.

**The signature problem (must be handled, not hand-waved):**
`roastty_surface_key` and `roastty_surface_key_is_binding` **already exist**
taking the **opaque** `roastty_key_event_t`, with **~67 test call sites** in
`lib.rs` building events via `key_event_new`/`set_*`. Two `#[no_mangle]` fns
can't share a name, so the by-value versions **replace** the opaque ones, and
**migrating those 67 test call sites to the by-value `input_key_s` form is
in-scope** (that's the bulk of the work + the risk). The opaque `key_event_*`
builder getters/setters may remain (they operate on the handle, not on
`surface_key`).

## Approach

1. **Expose the enums** in `roastty.h` with values **from `ghostty.h`** (the
   authoritative ABI oracle — **not** `vendor/ghostty/src/input/key.zig`, which
   is stale here: key.zig has 175 `Key` fields and omits `fn`, while
   `ghostty.h`'s `ghostty_input_key_e` and roastty's internal `Key` both have
   **176** with `Fn` at index 146). The values **already match** roastty's
   internal `Key`/`KeyAction` positionally, so this is pure header exposure (C
   names `ROASTTY_KEY_*`, `ROASTTY_ACTION_*`, mouse enums) — **no mapping
   table**.
2. **Add `roastty_input_key_s`** to `roastty.h` + a Rust `#[repr(C)]` struct,
   byte-faithful (the review confirmed the 7-field layout is identical to
   upstream).
3. **Replace the opaque `surface_key`/`surface_key_is_binding` with by-value,
   add `app_key`.** Change
   `roastty_surface_key`/`roastty_surface_key_is_binding` to take
   `roastty_input_key_s` by value and add `roastty_app_key(app, input_key_s)`;
   each builds a local `KeyEvent` from the struct and calls the existing
   `surface.key(...)` / `surface.key_is_binding(...)` / app-key path. **`text`
   is a NUL-terminated C string** — read via `strlen` (null → empty),
   **UTF-8-validate** to match the existing `set_utf8(ptr,len)` semantics.
4. **Migrate the ~67 opaque-handle test call sites** in `lib.rs`
   (`key_event_new`/`set_*` → build a `roastty_input_key_s` literal and pass by
   value); add a small test helper to keep them readable. (The opaque
   `key_event_*` getters/setters stay for now.)
5. **Keep `roastty.h` ↔ Rust exports in sync by hand** (no cbindgen); rebuild
   `RoasttyKit`
   - the app.
6. **`cargo test -p roastty`** (bounded runner) green after the migration.

## Changes / Deliverables

- `roastty/include/roastty.h` — the input enums (values byte-faithful to
  `ghostty.h`), `roastty_input_key_s`, the by-value fn decls.
- `roastty/src/lib.rs` (+ `roastty/src/input/…` as needed) — the `#[repr(C)]`
  `roastty_input_key_s`, the by-value
  `surface_key`/`app_key`/`surface_key_is_binding`, and any internal
  `Key`/`Action`↔C-value mapping.
- A small **ABI test** (Rust) asserting `size_of`/`offset_of` of
  `roastty_input_key_s` matches the upstream layout, and the enum values match
  (a few representative constants).
- Result: the input symbols resolve in the app build; `cargo test -p roastty`
  green.

## Verification

1. **Layout parity:** a Rust test asserts `roastty_input_key_s` field offsets +
   size match `ghostty_input_key_s` (7 fields, the documented order), and
   representative enum values (`PRESS=1`, a few `input_key_e` constants,
   `MODS_SHIFT=1<<0`) equal upstream.
2. **`cargo test -p roastty`** (bounded runner, Central-stamped) — green (no
   key/input regression).
3. **Authoritative "all input symbols resolved" check:** every input worklist
   symbol (`input_key_s`, `input_action_e`, `input_key_e`,
   `input_mouse_button_e`/`_state_e`/ `_momentum_e`, the by-value
   `surface_key`/`app_key`/`surface_key_is_binding`) is present in `roastty.h`
   and `lib.rs` (a grep/diff of the input subset of the worklist → empty).
   Rebuilding the app should advance past the input symbols, but compiler error
   **ordering is non-deterministic**, so the build is treated as **directional**
   confirmation, not the gate.

**Pass** = the input enums + `input_key_s` + the by-value
`surface_key`/`app_key`/ `surface_key_is_binding` are in
`roastty.h`/`libroastty`, the **layout/value Rust test passes**, the **~67 test
call sites are migrated and `cargo test -p roastty` is green**, and the input
subset of the worklist is empty (no input symbol still missing).

**Partial** = the input symbols resolve and tests pass, but a layout/value
mismatch is found that needs a follow-up, or the by-value `surface_key` can't
fully reuse the internal path (documented).

**Fail** = the internal key representation can't be mapped to the upstream enum
values without a deeper rework (documented as the real blocker).

## Design Review

**Reviewer:** `adversarial-reviewer` subagent (Claude Opus, fresh context,
read-only). **Verdict: CHANGES REQUIRED → addressed.** It **independently
verified the load-bearing assumption holds**: roastty's internal `Key`
(`input/key.rs`) matches `ghostty.h`'s `ghostty_input_key_e` value-for-value
(all 176; `Fn` at 146), `KeyAction` matches `Action`, the `roastty_input_key_s`
7-field layout is byte-identical, and the reuse path
(`surface.key`/`surface.key_is_binding` on `&mut KeyEvent`) is clean. Findings,
addressed:

- **Required — signature collision + test breakage.** `roastty_surface_key`/
  `roastty_surface_key_is_binding` already exist taking the **opaque**
  `RoasttyKeyEvent`, with **~67 test call sites**. Two `#[no_mangle]` fns can't
  share a name, and "the opaque builder may remain" was self-contradictory.
  **Fixed:** the design now **replaces** those signatures with by-value and
  makes **migrating the ~67 test call sites in-scope** (the bulk of the work);
  the opaque getters/setters stay.
- **Required — wrong verification oracle.** The design cited `input/key.zig`
  (stale: 175, no `fn`); roastty's `Key` matches `ghostty.h` (176, `Fn`@146), so
  verifying against key.zig would falsely flag a mismatch at `Fn` and corrupt 30
  discriminants. **Fixed:** the oracle is now `ghostty.h`; values already match
  → **no mapping**.
- **Optional — `text` semantics.** **Fixed:** by-value `text` is NUL-terminated
  (`strlen`), null → empty, UTF-8-validated to match `set_utf8`.
- **Optional — soft pass signal.** **Fixed:** authoritative check is the
  layout/value Rust test + the worklist diff; the build is directional (error
  ordering non-deterministic).
- **Nit — "~300 constants" overstated.** **Fixed:** it's **176**, already
  present internally — a header-exposure task.

## Result

_(to be added after the run.)_

## Conclusion

_(to be added after the run.)_
