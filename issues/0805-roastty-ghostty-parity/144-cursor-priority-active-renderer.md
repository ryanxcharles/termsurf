# Experiment 144: Cursor Priority Active Renderer

## Description

`RUNTIME-008B2B` still owns password/preedit cursor-style priority through the
active renderer path. This is narrower than GUI cursor pixels: pinned Ghostty's
priority rule lives in `renderer/cursor.zig`, and Roastty already has a faithful
value-level port in `roastty/src/renderer/cursor.rs`. The remaining gap is that
`FrameRenderState::from_terminal_with_cursor_options` currently derives cursor
style through its own reduced helper, which handles viewport presence, terminal
visibility, focus, and blink visibility, but does not route the active frame
path through the ported priority helper.

Pinned Ghostty priority is:

- no cursor when the cursor is outside the viewport;
- preedit forces a block cursor before cursor visibility, password lock,
  unfocused hollowing, or blink gating;
- password input forces the lock cursor before terminal visibility, unfocused
  hollowing, or blink gating;
- hidden cursor suppresses normal cursors after preedit/password checks;
- unfocused windows force the hollow block;
- hidden blink suppresses focused blinking cursors;
- otherwise the terminal visual cursor style is used.

This experiment will wire the active frame renderer through that same priority
function, add focused active-path tests for preedit and password ordering, and
split `RUNTIME-008B2B`:

- `RUNTIME-008B2B1`: **Oracle complete** for password/preedit cursor-style
  priority through the active frame renderer path.
- `RUNTIME-008B2B2`: **Gap** for remaining renderer-visible effects: background
  blur, real compositor opacity, window padding layout pixels, GUI cursor
  pixels, custom shader output, and broader GUI/pixel parity.

This experiment will not claim pixel parity for rendered cursor shapes, actual
password-prompt detection from a shell, IME text rendering beyond the already
tested preedit overlay path, background blur, compositor opacity, window padding
pixels, custom shader output, or broader GUI parity.

## Changes

- `roastty/src/renderer/frame_renderer.rs`
  - Add a `preedit` flag to `FrameCursorOptions`.
  - Ensure every active frame rendering path that receives
    `preedit: Option<Preedit>` derives the cursor priority option from
    `preedit.is_some()` while preserving any focused/blink options supplied by
    the caller.
  - Replace the reduced `cursor_style_from_terminal` logic with construction of
    a render-state scalar and a call into `renderer::cursor::style`.
  - Preserve existing non-password/non-preedit behavior: viewport gating, hidden
    cursor behavior, unfocused hollow cursor, blink suppression, and terminal
    visual style mapping.
  - Add focused active-frame tests proving:
    - preedit forces a block cursor even when the terminal cursor is hidden,
      unfocused, and blink-hidden;
    - password input forces a lock cursor even when the terminal cursor is
      hidden and blink-hidden;
    - preedit takes priority over password input;
    - no viewport still suppresses preedit/password cursor output;
    - a real active renderer method called with `Some(Preedit)` and a hidden
      terminal cursor produces a block cursor through the frame rebuild input;
    - existing focused/unfocused/blink/default cursor tests still pass through
      the shared priority helper.
- `issues/0805-roastty-ghostty-parity/cursor_priority_runtime_parity.py`
  - Add a guard that checks pinned Ghostty's `renderer/cursor.zig` priority
    markers, Roastty's shared cursor priority helper, the active frame renderer
    call into that helper, the new active-path tests, and the inventory split.
- `issues/0805-roastty-ghostty-parity/cursor_renderer_runtime_parity.py`
  - Update the previous cursor renderer guard so it no longer expects
    password/preedit priority to remain inside `RUNTIME-008B2B`.
- `issues/0805-roastty-ghostty-parity/config_runtime_inventory.py`
  - Split `RUNTIME-008B2B` into `RUNTIME-008B2B1` and `RUNTIME-008B2B2`.
- `issues/0805-roastty-ghostty-parity/config-runtime-inventory.md`
  - Regenerate from the inventory script.
- `issues/0805-roastty-ghostty-parity/config-matrix.md`
  - Regenerate CFG-223 summary. It must remain `Gap`.
- Existing CFG-223 static guards that hard-code current runtime row counts or
  the remaining renderer visual gap row
  - Update expected counts after the split.
  - Update references from `RUNTIME-008B2B` to `RUNTIME-008B2B2` where they mean
    the remaining renderer visual gap.
- `issues/0805-roastty-ghostty-parity/README.md`
  - Add the experiment link and update Learnings after the result.

## Verification

Pass criteria:

- The active frame cursor path calls the shared `renderer::cursor::style`
  priority helper rather than carrying a reduced duplicate of the priority
  logic.
- Active frame rendering methods that already receive `preedit: Option<Preedit>`
  derive cursor priority from `preedit.is_some()` on the real render path, not
  only from manually constructed test options.
- Preedit forces an active frame block cursor even when the terminal cursor is
  hidden, focus is false, and blink visibility is false.
- Password input forces an active frame lock cursor even when the terminal
  cursor is hidden and blink visibility is false.
- Preedit beats password input in the active frame path.
- A cursor outside the viewport still suppresses both preedit and password
  active frame cursor output.
- Existing active cursor overlay, OSC 12 cursor color, block-uniform,
  unfocused-hollow, blink, hidden-cursor, sprite, lock fallback, and
  list-routing tests continue to pass.
- `RUNTIME-008B2B1` is Oracle complete and cites shared-helper plus active-frame
  test evidence.
- `RUNTIME-008B2B2` remains `Gap` for the remaining renderer-visible GUI/pixel
  effects.
- `CFG-223` remains `Gap`.

Commands:

```bash
cargo test --manifest-path roastty/Cargo.toml cursor_priority_active_renderer
cargo test --manifest-path roastty/Cargo.toml render_state_derives_visible_block_cursor_overlay
cargo test --manifest-path roastty/Cargo.toml render_state_cursor_color_comes_from_osc12
cargo test --manifest-path roastty/Cargo.toml render_state_block_sets_uniform_underline_does_not
cargo test --manifest-path roastty/Cargo.toml cursor_blink_render_state
cargo test --manifest-path roastty/Cargo.toml render_state_hidden_cursor_has_no_overlay_or_uniform
cargo test --manifest-path roastty/Cargo.toml add_cursor
cargo test --manifest-path roastty/Cargo.toml set_cursor
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/cursor_priority_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/cursor_renderer_runtime_parity.py
PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md
for f in issues/0805-roastty-ghostty-parity/*_runtime_parity.py; do PYTHONDONTWRITEBYTECODE=1 python3 "$f" >/tmp/$(basename "$f").out || { echo FAIL:$f; cat /tmp/$(basename "$f").out; exit 1; }; done; echo all_runtime_parity_guards=pass
cargo fmt --manifest-path roastty/Cargo.toml
cargo fmt --manifest-path roastty/Cargo.toml --check
prettier --write --prose-wrap always --print-width 80 issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/144-cursor-priority-active-renderer.md
git diff --check
```

Fail criteria:

- The active frame renderer keeps a duplicate priority implementation instead of
  calling the shared Ghostty-port cursor priority helper.
- Preedit does not override hidden cursor, focus, blink, or password state in
  the active frame path.
- Password input does not override hidden cursor or blink state in the active
  frame path.
- No-viewport behavior is weakened by preedit or password handling.
- The experiment promotes GUI cursor pixels, password-prompt detection, IME text
  rendering, background blur, compositor opacity, padding pixels, custom shader
  output, or broader GUI/pixel parity from the remaining gap.
- CFG-223 is marked complete.

## Design Review

**Reviewer:** Codex adversarial subagent with fresh context.

**Initial verdict:** Changes required.

The reviewer found one required issue: the design could allow implementation to
pass tests through manually constructed `FrameCursorOptions` without proving
that the real active renderer path derives cursor priority from the
`preedit: Option<Preedit>` argument already passed to rendering methods.

**Fix:** Updated the design to require every active frame rendering path that
receives `preedit: Option<Preedit>` to derive cursor priority from
`preedit.is_some()` while preserving focused/blink options. Also added a
required real active renderer method test that calls the renderer with
`Some(Preedit)` and a hidden terminal cursor and proves a block cursor through
the frame rebuild input.

**Final verdict:** Approved.

The reviewer confirmed the prior finding was resolved and no new required
findings were introduced.
