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

# Experiment 687: Surface Key Translation Mods

## Description

Experiment 686 added conservative surface inherited-config support. Another
small upstream surface entry point that Roastty lacks is
`ghostty_surface_key_translation_mods(surface, mods)`, which filters keyboard
modifier bits for key translation. Upstream uses this primarily for
`macos-option-as-alt`: when option should behave as Alt, the translated modifier
set removes the Alt bit before text translation while the original modifier set
is still sent to `surface_key`.

Roastty already has the same modifier bit layout internally in
`input::key_mods::Mods::int()` and the same translation helper. Roastty does not
yet implement full surface key dispatch, keybind parsing, `macos-option-as-alt`
configuration, or keyboard-layout detection at the surface boundary. This
experiment adds only the C ABI modifier bitmask and surface translation query
using the current default `OptionAsAlt::False`, which means no Alt filtering
yet. It does not implement key event dispatch, binding detection, key
translation through `KeyEncoder`, keybind trigger state, or config/layout-driven
`macos-option-as-alt`.

## Changes

- `roastty/include/roastty.h`
  - Add `roastty_input_mods_e` with the upstream-compatible modifier bit values:
    - `ROASTTY_MODS_NONE = 0`
    - `ROASTTY_MODS_SHIFT = 1 << 0`
    - `ROASTTY_MODS_CTRL = 1 << 1`
    - `ROASTTY_MODS_ALT = 1 << 2`
    - `ROASTTY_MODS_SUPER = 1 << 3`
    - `ROASTTY_MODS_CAPS = 1 << 4`
    - `ROASTTY_MODS_NUM = 1 << 5`
    - `ROASTTY_MODS_SHIFT_RIGHT = 1 << 6`
    - `ROASTTY_MODS_CTRL_RIGHT = 1 << 7`
    - `ROASTTY_MODS_ALT_RIGHT = 1 << 8`
    - `ROASTTY_MODS_SUPER_RIGHT = 1 << 9`
  - Add
    `ROASTTY_API roastty_input_mods_e roastty_surface_key_translation_mods(roastty_surface_t, roastty_input_mods_e);`
    next to `roastty_surface_set_color_scheme`.
- `roastty/src/lib.rs`
  - Add raw-bitmask conversion helpers between `roastty_input_mods_e` and
    `key_mods::Mods`, preserving unknown bits by dropping them from the returned
    ABI value.
  - Implement `roastty_surface_key_translation_mods(surface, mods)`:
    - null, detached, and live surfaces currently all use `OptionAsAlt::False`
      because surface config/layout option-as-alt policy is not implemented yet;
    - valid known bits round-trip unchanged under the current default policy;
    - unknown input bits are ignored in the output.
  - Add tests:
    - bit constants match upstream values and internal `Mods::int()` layout;
    - known bits round-trip unchanged for null and live surfaces under the
      current default policy;
    - right-side modifier bits round-trip with their base modifier bits;
    - unknown bits are dropped.
- `roastty/tests/abi_harness.c`
  - Assert `roastty_input_mods_e` constants.
  - Exercise null and live `roastty_surface_key_translation_mods` calls through
    the public C header.

## Verification

- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/687-surface-key-translation-mods.md`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `cargo test -p roastty surface`
- `cargo test -p roastty key`
- `cargo test -p roastty --test abi_harness`
- `git diff --check`

## Design Review

**Result:** Approved.

Codex found no design blockers. It approved using `OptionAsAlt::False` as an
incremental slice because the limitation is explicit and config/layout-driven
`macos-option-as-alt` is out of scope. The result is weaker than upstream, which
consults surface config or keyboard-layout detection, but it matches Roastty's
currently implemented default policy.

Codex also approved the narrow ABI scope: add the upstream-compatible modifier
bitmask enum, add the surface query, round-trip known bits under the current
policy, and drop unknown bits. The planned verification covers constants,
internal layout, null/live calls, right-side bits, and unknown-bit dropping.

## Result

**Result:** Pass.

Roastty now exposes an upstream-compatible `roastty_input_mods_e` bitmask and
`roastty_surface_key_translation_mods(surface, mods)` in the public C ABI. The
Rust implementation converts raw ABI bits into `input::key_mods::Mods`, applies
the existing translation helper with the current default `OptionAsAlt::False`
policy, and converts the result back to the public bitmask.

Known modifier bits round-trip for null and live surfaces under the current
policy, right-side modifier bits preserve the internal `Mods::int()` layout, and
unknown input bits are dropped from the returned ABI value. The C ABI harness
asserts the public constants and exercises null/live calls through `roastty.h`.

Verification passed:

- `cargo fmt -p roastty`
- `cargo test -p roastty surface`
- `cargo test -p roastty key`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Conclusion

The surface-level key translation modifier query is now present as a small,
tested ABI slice. It intentionally does not implement full key dispatch, binding
detection, keybind trigger state, or config/layout-driven `macos-option-as-alt`;
those remain separate work because the current Roastty surface boundary does not
yet carry that policy.

## Completion Review

**Result:** Approved after provenance update.

Codex found no code, ABI, regression, or missing-test blockers. It confirmed the
header constants match the intended upstream-compatible bit values, the exported
function applies the current `OptionAsAlt::False` policy, known bits round-trip,
and unknown bits are dropped by reconstructing the return value from
`Mods::int()`.

Codex initially blocked the result commit only because result-review provenance,
this completion-review section, and the final README agent tuple were not
recorded yet. Those workflow records are now present.
