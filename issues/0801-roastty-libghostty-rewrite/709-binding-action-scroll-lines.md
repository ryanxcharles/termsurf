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

# Experiment 709: Binding Action Scroll Lines

## Description

Experiment 708 added `scroll_page_up` and `scroll_page_down` binding-action
support by applying row-delta viewport movement. Upstream Ghostty's
`performBindingAction` also supports `scroll_page_lines:<i16>`, which applies
the signed line count directly to the viewport:

- positive values scroll downwards;
- negative values scroll upwards;
- `+N` is accepted by Zig's decimal integer parser;
- empty, whitespace, malformed, or out-of-range parameters are invalid;
- the action returns `true` when performed on an attached surface.

Roastty already has the row-delta helper used by Experiment 708. This experiment
adds only the signed line-count parser and the `scroll_page_lines:<i16>`
binding-action path.

This does not implement `clear_screen`, `scroll_to_row`, `scroll_to_selection`,
fractional page scrolling, prompt jumps, search actions, clipboard actions,
cursor-key actions, full keybind storage/lookup, or app-scoped actions.

## Changes

- `roastty/src/lib.rs`
  - Add a small ASCII decimal `i16` parser that mirrors upstream
    `std.fmt.parseInt(i16, value, 10)` for this action: accept optional leading
    `+` or `-`, require at least one digit, reject whitespace/trailing bytes,
    and reject values outside the `i16` range.
  - Extend the internal parsed binding-action enum with `ScrollPageLines(i16)`.
  - Extend `parse_binding_action` to accept `scroll_page_lines:<i16>` and reject
    missing, empty, malformed, whitespace, extra-colon, and out-of-range
    parameters.
  - Add/use a surface helper that locks the active termio worker, applies the
    parsed signed row delta to the terminal viewport, and requests a render.
  - Treat a zero line count as a consumed no-op, matching a zero-delta
    interpretation.
  - Return `true` for attached parsed line-scroll actions, even when no termio
    worker exists, matching action-consumed semantics.
  - Return `false` for null or detached surfaces.
  - Keep split, close, `text:`, `csi:`, `esc:`, `reset`, top/bottom scroll, and
    page up/down semantics unchanged.

- `roastty/tests/abi_harness.c`
  - Add C ABI smoke coverage that malformed line-scroll forms are rejected and
    representative negative, positive, and explicit-plus forms can be invoked.

- Tests in `roastty/src/lib.rs`
  - Cover invalid forms returning false: missing parameter, empty parameter,
    whitespace, malformed bytes, extra colon, and values outside the `i16`
    range.
  - Cover null and detached surfaces returning false.
  - Cover attached no-worker surfaces returning true without side effects.
  - Cover worker-backed `scroll_page_lines:-N` moving the viewport up by exactly
    `N` rows when scrollback exists.
  - Cover worker-backed `scroll_page_lines:+N` and `scroll_page_lines:N` moving
    the viewport down by exactly `N` rows.
  - Cover `scroll_page_lines:0` returning true without moving the viewport.
  - Re-run existing binding-action tests to prove previous action semantics did
    not change.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty binding_action -- --nocapture`
- `cargo test -p roastty scroll_page_lines -- --nocapture`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the Experiment 709 design and approved it technically. The review
confirmed that the scope is upstream-compatible: `scroll_page_lines:i16` is a
focused signed row-delta slice and preserves upstream direct viewport movement
semantics while excluding fractional, row, selection, and prompt actions.

The review also confirmed that a small ASCII `i16` parser matching
`std.fmt.parseInt(i16, value, 10)` plus reuse of
`Terminal::scroll_selection_gesture_viewport(delta)` is feasible. The proposed
tests were accepted as sufficient for malformed parameters, signed forms,
null/detached/no-worker behavior, exact worker-backed movement, zero no-op, ABI
smoke coverage, and prior-action regression coverage.

The only required fix before plan commit was workflow provenance: replacing the
pending design-review metadata, adding this design-review section, and updating
the README provenance tuple to `Codex/Codex/-`.

## Result

**Result:** Pass

Implemented `scroll_page_lines:<i16>` binding-action support for attached
surfaces. `parse_binding_action` now accepts signed decimal `i16` parameters
with optional leading `+` or `-`, rejects missing, empty, whitespace, malformed,
extra-colon, and out-of-range parameters, and stores the parsed value as
`ScrollPageLines(i16)`.

Dispatch returns `false` for null or detached surfaces, returns `true` for
attached surfaces, and routes worker-backed surfaces through the existing
terminal row-delta viewport helper. A zero line count consumes the action
without moving the viewport.

The Rust tests cover invalid forms, signed negative/positive/explicit-plus
forms, null/detached surfaces, attached no-worker surfaces, exact worker-backed
movement by signed line count, zero no-op behavior, and unchanged binding-action
behavior around previous actions. The C ABI harness now rejects representative
malformed/out-of-range line-scroll forms and accepts negative, positive,
explicit-plus, and zero forms.

Verification:

- `cargo fmt -p roastty` passed.
- `cargo test -p roastty binding_action -- --nocapture` passed: 35 tests.
- `cargo test -p roastty scroll_page_lines -- --nocapture` passed: 4 tests.
- `cargo test -p roastty --test abi_harness` passed.
- `cargo fmt -p roastty -- --check` passed.
- `git diff --check` passed.

## Conclusion

The line-scroll slice now follows upstream's signed integer parsing and direct
row-delta viewport movement semantics. The remaining viewport binding-action
work can continue with fractional page scrolling, explicit row/selection
scrolling, prompt jumps, or the higher-risk clear-screen action.

## Completion Review

Codex reviewed the completed Experiment 709 diff and found no code correctness
blockers. The review confirmed that the parser accepts optional leading `+` or
`-`, requires digits, rejects whitespace/trailing bytes/extra colon/malformed
input, enforces `i16` bounds including `-32768`, and dispatches the signed value
directly through the existing viewport row-delta helper.

The review also confirmed that the tests cover invalid forms, out-of-range
values, signed negative/positive/explicit-plus forms, no-worker, null/detached,
exact worker-backed movement, zero no-op behavior, ABI smoke coverage, and
prior-action regression coverage.

The only required fix was workflow provenance: replacing the pending
result-review metadata, adding this completion-review note, and updating the
README provenance tuple to `Codex/Codex/Codex`.
