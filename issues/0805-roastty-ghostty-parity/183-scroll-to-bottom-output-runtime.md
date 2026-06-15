# Experiment 183: Scroll-to-bottom output runtime

## Description

After Experiment 182, `RUNTIME-008B2B2B2B2B` contains only
`scroll-to-bottom.output`. Pinned Ghostty implements this renderer-visible
runtime behavior in `vendor/ghostty/src/renderer/generic.zig`: each render pass
checks `state.terminal.screens.active.pages.getBottomRight(.screen)` when
`scroll_to_bottom_on_output` is enabled and synchronized output mode is not
active, compares the bottom-right pin's node pointer and `y` value against the
last rendered bottom marker, and calls `state.terminal.scrollViewport(.bottom)`
before rebuilding render state if new output moved the active bottom.

Roastty already has equivalent terminal primitives: active/screen bottom-right
grid refs, viewport scrolling, and live presentation from
`Surface::present_live`. This experiment will port that exact renderer-time
behavior into Roastty, prove the `scroll-to-bottom.output` config gate, and
split it out of the residual renderer row.

## Changes

- Add renderer-present state to `Surface` for the last output bottom marker,
  matching Ghostty's `last_bottom_node`/`last_bottom_y` intent.
- Add a small `Surface` helper that runs before live frame rendering:
  - if terminal synchronized output mode (`DEC 2026`) is active, it does not
    scroll or advance the stored bottom marker, matching Ghostty's early render
    skip before the scroll-to-bottom check;
  - if `active_config().scroll_to_bottom.output` is false, it does not scroll;
  - if the bottom marker is unavailable, it does not scroll;
  - if the current active/screen bottom marker matches the last marker, it does
    not scroll;
  - otherwise it updates the marker and scrolls the terminal viewport to bottom.
- Expose only the minimal terminal accessor needed to read the active/screen
  bottom-right marker, using the same `TerminalGridRef` identity already used by
  embedded grid-ref APIs.
- Add focused Rust tests that prove:
  - disabled `scroll-to-bottom.output` preserves a viewport scrolled into
    history after output;
  - enabled `scroll-to-bottom.output` does not scroll or advance the marker
    while synchronized output mode is active, then scrolls after that mode is
    disabled and a later render observes the pending bottom marker;
  - enabled `scroll-to-bottom.output` scrolls back to the active bottom after
    output changes the bottom marker;
  - a second helper call without new output does not repeatedly scroll;
  - the marker comparison tracks node pointer plus `y`, matching pinned
    Ghostty's behavior rather than content text.
- Add
  `issues/0805-roastty-ghostty-parity/scroll_to_bottom_output_runtime_parity.py`
  to statically check the pinned Ghostty anchors, Roastty implementation/tests,
  and generated inventory split.
- Update `config_runtime_inventory.py` to split:
  - `RUNTIME-008B2B2B2B2B4`: **Oracle complete** for `scroll-to-bottom.output`
    renderer-time viewport behavior;
  - `RUNTIME-008B2B2B2B2B`: removed or narrowed depending on whether inspection
    finds any renderer-visible residual remains after this slice.
- Regenerate `config-runtime-inventory.md` and `config-matrix.md`, updating
  CFG-223 counts.
- Update affected parity guards that currently use `scroll-to-bottom.output` as
  the residual renderer sentinel.

If implementation shows that this was the final renderer residual, the remaining
`RUNTIME-008B2B2B2B2B` gap should be removed from the expected manifest rather
than left as an empty gap.

## Verification

Pass criteria:

- `cargo test --manifest-path roastty/Cargo.toml scroll_to_bottom_output -- --test-threads=1`
  passes and proves the enabled/disabled marker-based runtime behavior.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/scroll_to_bottom_output_runtime_parity.py`
  passes and fails if the pinned Ghostty anchors, Roastty helper/tests, or
  inventory row drift.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/renderer_visual_residual_audit.py`
  passes with `scroll-to-bottom.output` removed from the residual.
- `PYTHONDONTWRITEBYTECODE=1 python3 issues/0805-roastty-ghostty-parity/config_runtime_inventory.py --output issues/0805-roastty-ghostty-parity/config-runtime-inventory.md --matrix issues/0805-roastty-ghostty-parity/config-matrix.md`
  regenerates the inventory/matrix without drift.
- Affected prior guards that referenced the renderer residual are updated and
  pass.
- `python3 -m py_compile issues/0805-roastty-ghostty-parity/scroll_to_bottom_output_runtime_parity.py issues/0805-roastty-ghostty-parity/config_runtime_inventory.py issues/0805-roastty-ghostty-parity/renderer_visual_residual_audit.py`
  passes.
- `cargo fmt --manifest-path roastty/Cargo.toml` is run after Rust edits, and
  `cargo fmt --manifest-path roastty/Cargo.toml --check` passes.
- `prettier --check issues/0805-roastty-ghostty-parity/README.md issues/0805-roastty-ghostty-parity/183-scroll-to-bottom-output-runtime.md issues/0805-roastty-ghostty-parity/config-runtime-inventory.md issues/0805-roastty-ghostty-parity/config-matrix.md`
  passes after formatting.
- `git diff --check` passes.

Failure criteria:

- The helper scrolls on every render even when the bottom marker did not change.
- The disabled config path scrolls the viewport on output.
- The synchronized output path scrolls the viewport or advances the marker while
  synchronized output mode is active.
- The test only proves parser/default behavior and does not mutate a terminal
  viewport after output.
- The experiment removes the residual renderer gap while another unproven
  renderer-visible behavior remains in `RUNTIME-008B2B2B2B2B`.

## Design Review

Fresh-context Codex adversarial review:

- Initial verdict: **Changes required**.
- Required finding: the first design omitted Ghostty's synchronized-output
  render skip, so the planned helper could scroll or advance its marker while
  terminal synchronized output mode was active.
- Fix: added the synchronized-output no-op condition, required proof that the
  marker does not advance while synchronized output is active, and added the
  corresponding failure criterion.
- Re-review verdict: **Approved**. The reviewer confirmed the design now matches
  pinned Ghostty's ordering: synchronized output returns early before the
  scroll-to-bottom output check, and Roastty has the corresponding
  `SynchronizedOutput` mode anchor.
