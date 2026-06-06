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

# Experiment 714: Binding Action Jump To Prompt

## Description

Experiment 713 added core `clear_screen` binding-action support. Upstream
Ghostty's `performBindingAction` also supports `jump_to_prompt:<i16>`, which
scrolls the viewport forward or backward by semantic prompt markers:

- the action requires a signed integer parameter;
- `+N` is accepted by Zig's decimal `i16` parser;
- missing, empty, whitespace, malformed, extra-colon, or out-of-range values are
  invalid;
- zero is a consumed no-op on attached worker-backed surfaces;
- negative values search upward from one row above the viewport top;
- positive values search downward from one row below the viewport top;
- when moving downward from a prompt or prompt continuation row, continuation
  rows are skipped so the action lands on the next prompt;
- if fewer prompts exist than requested, the viewport lands on the furthest
  prompt found in that direction;
- if the target prompt is in the active area, the viewport becomes active;
- otherwise the viewport is pinned to the target prompt row;
- the action requires shell integration prompt markers, so no-worker surfaces
  return `false`.
- attached worker-backed surfaces with no prompt markers consume the action and
  leave the viewport unchanged.

Roastty already has semantic prompt parsing and prompt iterators. This
experiment ports the prompt-delta scroll operation and wires
`jump_to_prompt:<i16>` through the binding-action parser and surface helper.

This does not implement full keybind storage/lookup, keybinding dispatch,
frontend selection routing, search actions, clipboard actions, cursor-key
actions, write-file actions, or app-scoped actions.

## Changes

- `roastty/src/terminal/page_list.rs`
  - Extend the internal `Scroll` enum with a prompt-delta variant or add an
    equivalent `scroll_delta_prompt(delta: isize)` helper.
  - Port upstream `PageList.scrollPrompt(delta)` using the existing local
    `PromptIterator`, viewport top-left lookup, `pin_up`/`pin_down`,
    continuation-row skip logic for positive deltas, and active-area clamping.
  - Treat `delta == 0` as a no-op.
  - Reuse existing viewport validity checks after scrolling.

- `roastty/src/terminal/screen.rs`
  - Add `scroll_delta_prompt(delta: isize)` forwarding to the page-list helper.

- `roastty/src/terminal/terminal.rs`
  - Add `Terminal::scroll_viewport_to_prompt(delta: isize)` forwarding to the
    active screen helper.

- `roastty/src/lib.rs`
  - Reuse the existing `parse_i16_ascii` parser for `jump_to_prompt:<i16>`.
  - Extend the internal parsed binding-action enum with `JumpToPrompt(i16)`.
  - Extend `parse_binding_action` to accept `jump_to_prompt:<i16>` and reject
    missing, empty, whitespace, malformed, extra-colon, and out-of-range
    parameters.
  - Add/use a surface helper that locks the active termio worker, calls the
    terminal helper, requests a render, and returns `true`.
  - Return `false` for null, detached, no-worker, and malformed-parameter cases.
  - Keep split, close, `text:`, `csi:`, `esc:`, `reset`, `clear_screen`, and
    scroll action semantics unchanged.

- `roastty/tests/abi_harness.c`
  - Add C ABI smoke coverage that malformed prompt-jump forms are rejected and
    representative negative, positive, explicit-plus, and zero forms return
    `false` without crashing on the no-worker harness surface.

- Tests in `roastty/src/lib.rs`
  - Cover invalid forms returning false: missing parameter, empty parameter,
    whitespace, malformed bytes, extra colon, and values outside the `i16`
    range.
  - Cover null, detached, and no-worker surfaces returning false.
  - Cover worker-backed `jump_to_prompt:0` returning true without moving.
  - Cover worker-backed nonzero prompt jumps on terminals with no prompt markers
    returning true without moving.
  - Cover worker-backed negative prompt jumps moving upward to the previous
    prompt and clamping to the furthest prompt when the requested count exceeds
    available prompts.
  - Cover worker-backed positive prompt jumps moving downward to the next prompt
    and active-area clamping when the target prompt is active.
  - Cover positive jumps skipping prompt continuation rows.
  - Re-run existing binding-action tests to prove previous action semantics did
    not change.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty jump_to_prompt -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the Experiment 714 design and accepted the parser and prompt
iterator approach: reusing `parse_i16_ascii` gives optional `+`/`-`, strict
digits, whitespace rejection, and range checks; the planned viewport algorithm
covers upward/downward search starts, positive continuation skipping,
furthest-prompt clamping, and active-area clamping.

The review raised two technical blockers before plan commit. First, the C ABI
harness expectations for valid no-worker prompt jumps were conditional and had
to be made explicit. The plan now states that `jump_to_prompt:-1`,
`jump_to_prompt:+1`, `jump_to_prompt:1`, and `jump_to_prompt:0` return `false`
without crashing on the no-worker harness surface. Second, worker-backed
terminals with zero prompt markers needed defined behavior. The plan now treats
them as consumed no-ops: attached worker-backed surfaces return `true` and leave
the viewport unchanged.

The review noted that always requesting a render for consumed zero/no-prompt
no-ops may be unnecessary but is not a correctness issue; the result should be
clear about the chosen behavior. The remaining required workflow fix before plan
commit was provenance: adding the design-review frontmatter, recording this
review section, and updating the README provenance tuple to `Codex/Codex/-`.
Result-review provenance will be added only after implementation and completion
review.

Codex re-reviewed the updated design and found no remaining blockers. The
re-review confirmed that valid no-worker prompt jumps are explicitly `false`,
worker-backed no-prompt terminals are specified as consumed no-ops with no
viewport movement, and design-review provenance plus this review section are
recorded. The design is approved for the plan commit.
