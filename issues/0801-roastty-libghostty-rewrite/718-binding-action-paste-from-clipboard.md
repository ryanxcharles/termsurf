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

# Experiment 718: Binding Action Paste From Clipboard

## Description

Experiment 717 added standard-clipboard copy support. Upstream Ghostty's
`performBindingAction` also supports two adjacent paste request actions:

- `paste_from_clipboard`
- `paste_from_selection`

Roastty already exposes the runtime clipboard read callback, the selection
clipboard capability flag, and `roastty_surface_complete_clipboard_request`.
Today the completion function is a no-op. This experiment turns those pieces
into a narrow paste request slice: binding actions start a runtime read request,
and completion pastes provided text into the surface's child PTY using the same
paste encoder as direct text paste.

This does not implement OSC 52 reads/writes, unsafe-paste classification,
clipboard request allocation tables, or asynchronous request cancellation. The
completion API remains paste-only in this experiment, but it requires a
confirmed completion and a matching request state pointer before it writes to
the PTY.

## Changes

- `roastty/src/lib.rs`
  - Add `ROASTTY_CLIPBOARD_SELECTION = 1` beside the existing standard clipboard
    constant.
  - Extend the internal parsed binding-action enum with
    `PasteFromClipboard(c_int)`.
  - Extend `parse_binding_action` to accept:
    - `paste_from_clipboard` with no parameter;
    - `paste_from_selection` with no parameter.
  - Reject any parameter on either paste action.
  - Add a surface helper that:
    - returns `false` for null and detached surfaces;
    - returns `false` for no-worker surfaces because paste completion needs a
      target PTY;
    - returns `false` when the runtime has no `read_clipboard_cb`;
    - returns `false` for `paste_from_selection` when the runtime does not
      advertise `supports_selection_clipboard`;
    - invokes `read_clipboard_cb(userdata, clipboard, state)` with
      `ROASTTY_CLIPBOARD_STANDARD` for `paste_from_clipboard` and
      `ROASTTY_CLIPBOARD_SELECTION` for `paste_from_selection`;
    - passes the surface handle as the opaque state pointer; runtimes must treat
      this pointer as a borrowed request token and must not retain it after the
      surface can be freed;
    - returns the runtime callback result.
  - Update `roastty_surface_complete_clipboard_request` so non-null completion
    text feeds into the existing `Surface::text` paste path only when:
    - the surface handle is valid and attached to an app;
    - the state pointer equals the surface handle;
    - the surface has a worker;
    - `confirmed = true`;
    - the text pointer is non-null.
  - Treat null text, null/mismatched state, detached surfaces, no-worker
    surfaces, and `confirmed = false` as no-ops.
  - Keep existing copy, split, close, text/CSI/ESC, reset, clear-screen, scroll,
    prompt-jump, select-all, and adjust-selection semantics unchanged.

- `roastty/tests/abi_harness.c`
  - Add C ABI smoke coverage that parameterized paste action forms are rejected.
  - Add no-callback coverage that bare paste actions return `false` without
    crashing.

- Tests in `roastty/src/lib.rs`
  - Cover parser false paths for parameterized paste action forms.
  - Cover null, detached, no-callback, and unsupported selection-clipboard
    surfaces returning `false`.
  - Cover no-worker surfaces returning `false` even when a read callback is
    installed.
  - Cover standard and selection paste actions invoking `read_clipboard_cb` with
    the expected clipboard id, userdata, surface state pointer, and callback
    result.
  - Cover `roastty_surface_complete_clipboard_request` queueing completed text
    through the paste encoder into a worker-backed surface.
  - Cover completion no-ops for null text, null/mismatched state, detached
    surfaces, no-worker surfaces, and `confirmed = false`.
  - Re-run existing binding-action and clipboard-completion tests to prove
    previous action semantics did not change.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty paste_from_clipboard -- --nocapture --test-threads=1`
- `cargo test -p roastty complete_clipboard_request -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the initial Experiment 718 design and found the paste-through
`Surface::text` approach coherent once completion validity is established. The
review raised three design blockers:

- no-worker paste action semantics were unspecified;
- passing the surface handle as callback state was unsafe without a lifetime and
  validation rule;
- completion was too permissive when `confirmed` and `state` were ignored.

The design now makes no-worker paste actions return `false`, treats the state
pointer as a borrowed request token equal to the surface handle, validates
`state == surface` during completion, and only pastes when `confirmed = true`.
Null text, null/mismatched state, detached surfaces, no-worker surfaces, and
unconfirmed completions are no-ops.

The review also raised the normal workflow provenance requirement. Design-review
frontmatter and this section are now present, and the README provenance tuple
will be updated to `Codex/Codex/-` before the plan commit. Result-review
provenance will be added only after implementation and completion review.

Codex re-reviewed the revised design and found no remaining blockers. The review
approved the explicit no-worker `false` behavior, borrowed state-token rule,
`state == surface` validation, `confirmed = true` completion requirement,
completion no-op cases, and test plan.

## Result

**Result:** Pass

Implemented `paste_from_clipboard` and `paste_from_selection` binding-action
parsing and dispatch. Worker-backed, attached surfaces now start runtime
clipboard read requests with `ROASTTY_CLIPBOARD_STANDARD` or
`ROASTTY_CLIPBOARD_SELECTION`, pass the surface handle as the borrowed state
token, honor the runtime callback result, and return `false` for no-worker,
detached, no-callback, and unsupported selection-clipboard cases.

`roastty_surface_complete_clipboard_request` now feeds confirmed clipboard text
through the existing paste encoder into the child PTY when the state token
matches the surface. Null text, null/mismatched state, detached surfaces,
no-worker surfaces, and `confirmed = false` remain no-ops.

The C ABI harness now covers parameterized paste action rejection and valid
no-worker paste actions returning `false`.

Verification:

- `cargo fmt -p roastty`
- `cargo test -p roastty paste_from_clipboard -- --nocapture --test-threads=1` —
  2 passed
- `cargo test -p roastty complete_clipboard_request -- --nocapture --test-threads=1`
  — 5 passed
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1` — 66
  passed on rerun; an earlier run hit the same transient existing PTY text test
  timing failure seen in Experiment 717, and that text test passed when rerun
  directly
- `cargo test -p roastty --test abi_harness` — 1 passed
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Conclusion

Roastty now has the first paste request binding-action slice: binding actions
can start standard or selection clipboard reads, and confirmed completions paste
into a worker-backed surface. Remaining clipboard work includes request table
ownership, asynchronous cancellation, unsafe-paste prompt behavior, OSC 52
read/write completion, and richer runtime documentation for the borrowed state
token.

## Completion Review

Codex reviewed the completed Experiment 718 implementation and result record.
The review found one workflow blocker: result-review provenance was not yet
recorded in the experiment frontmatter or README tuple. This file now includes
`[review.result]`, and the README provenance tuple has been updated to
`Codex/Codex/Codex`.

The review found no code blockers. It approved parser behavior for the bare
paste actions and parameter rejection, false-path handling for
null/detached/no-worker/no-callback/unsupported-selection surfaces, callback
clipboard ids and state token, callback-result propagation, completion
validation, completion routing through `Surface::text`, and the Rust and C ABI
test coverage.

The review noted one non-blocking risk: the state token remains a borrowed raw
surface handle. The implementation validates `state == surface`, and the result
records request-table ownership and runtime documentation as remaining work.
