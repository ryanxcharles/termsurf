+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
model = "default"
reasoning = "medium"

[review.result]
agent = "codex"
model = "default"
reasoning = "medium"
+++

# Experiment 780: Clipboard Paste Request State

## Description

Implement real active clipboard request state for paste-from-clipboard actions.

The current Roastty paste path starts a `read_clipboard_cb`, but passes the
surface pointer itself as the request state.
`roastty_surface_complete_clipboard_request` then accepts that same surface
pointer when `confirmed` is true. This matches the no-active-request foundation
but does not provide the stable per-request pointer or confirmation-preserving
request behavior that upstream Ghostty's embedded runtime uses.

This experiment ports the paste-request subset of that behavior. It does not add
OSC 52 read/write request initiation; those are separate request types and can
land in later experiments when the terminal OSC path is ready.

## Changes

- `roastty/src/lib.rs`
  - Add a Roastty-owned clipboard request state object for active paste
    requests.
  - Store active request pointers on `Surface` so completion validates that the
    state belongs to that surface.
  - Make `paste_from_clipboard` allocate a stable request state before calling
    `read_clipboard_cb`, register it before invoking the callback, pass that
    request pointer to the callback, and destroy it after a refused callback
    only if it is still active. This handles synchronous callback completion.
  - Free any still-active clipboard request states when the surface is
    destroyed.
  - Make `roastty_surface_complete_clipboard_request` consume only active
    request pointers for that surface.
  - For paste requests:
    - confirmed text, safe or unsafe, writes to the child PTY and destroys the
      request;
    - empty text destroys the request without writing;
    - unconfirmed safe text writes to the child PTY and destroys the request;
    - unconfirmed unsafe text containing newline bytes calls
      `confirm_read_clipboard_cb` with `ROASTTY_CLIPBOARD_REQUEST_PASTE` and
      preserves the request for a later confirmed completion;
    - unconfirmed unsafe text without a confirmation callback destroys the
      request without writing.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - Update checklist wording only if the implementation and tests prove the
    paste-request allocation/handling subset is complete.
  - Use scoped checklist wording: paste clipboard request state done, OSC 52
    request allocation/handling still missing.

## Verification

- Inspect upstream reference:
  - `vendor/ghostty/src/apprt/embedded.zig` clipboard request allocation and
    completion.
  - `vendor/ghostty/src/Surface.zig` paste completion behavior.
- Run focused tests:
  - `cargo test -p roastty surface_binding_action_paste_from_clipboard -- --nocapture --test-threads=1`
  - `cargo test -p roastty surface_complete_clipboard_request -- --nocapture --test-threads=1`
  - `cargo test -p roastty clipboard_request -- --nocapture --test-threads=1`
- New or updated assertions must cover:
  - read callback receives a non-surface stable request pointer;
  - callback refusal cleans up the request;
  - synchronous callback completion before `read_clipboard_cb` returns does not
    double-free or resurrect a request;
  - double completion is ignored;
  - cross-surface state is ignored;
  - unconfirmed safe paste writes;
  - unconfirmed unsafe paste calls `confirm_read_clipboard_cb` and preserves the
    same state;
  - later confirmed completion writes and consumes the preserved request;
  - confirmed unsafe paste writes without confirmation;
  - unconfirmed unsafe paste without a confirmation callback drops the request
    without writing;
  - surface destruction frees abandoned request states.
- Run:
  - `cargo fmt -p roastty`
  - `cargo fmt -p roastty -- --check`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/780-clipboard-paste-request-state.md`
- Run:
  - `git diff --check`

The experiment passes if paste-from-clipboard uses stable active request
pointers, completion validates and consumes those requests, unsafe unconfirmed
paste data routes through the confirmation callback while preserving the
request, safe/confirmed paste data reaches the child PTY, stale or cross-surface
states are ignored, and all focused tests pass. It is Partial if only part of
the request lifecycle can be implemented without overclaiming. It fails if the
current C ABI shape cannot safely represent active clipboard requests.

## Design Review

Codex reviewed the initial design and found five issues: confirmed unsafe paste
behavior was under-specified, callback reentrancy around `read_clipboard_cb`
refusal was not covered, the README checklist update risked overclaiming beyond
paste requests, pending request cleanup on surface destruction was not
specified, and the verification section did not name the important edge-case
assertions.

The design was updated to cover confirmed unsafe paste, register requests before
calling the read callback, destroy refused requests only if still active, free
abandoned requests on surface destruction, use paste-scoped checklist wording
that leaves OSC 52 request allocation/handling missing, and require explicit
tests for stale/cross-surface state, double completion, confirmation
preservation, confirmed unsafe paste, callback refusal, synchronous completion,
and abandoned request cleanup. Codex reviewed the revision, found no blockers,
and approved the Experiment 780 plan commit.

## Result

**Result:** Pass

`Surface` now owns active paste clipboard request state:

- `paste_from_clipboard` allocates a stable request pointer before calling
  `read_clipboard_cb`.
- Refused callbacks clean up the request only if it is still active, so
  synchronous completion during the callback is stable.
- `roastty_surface_complete_clipboard_request` validates that the request state
  belongs to the target surface before consuming it.
- Empty paste text consumes the request without writing.
- Safe unconfirmed paste and confirmed paste write to the child PTY and consume
  the request.
- Unsafe unconfirmed paste calls `confirm_read_clipboard_cb` with
  `ROASTTY_CLIPBOARD_REQUEST_PASTE` and preserves the request for a later
  confirmed completion.
- Unsafe unconfirmed paste without a confirmation callback drops the request
  without writing.
- Surface destruction frees abandoned request states.

The README checklist update is scoped: paste clipboard request state is done,
while OSC 52 request allocation/handling remains missing.

Verification passed:

- `cargo test -p roastty surface_binding_action_paste_from_clipboard -- --nocapture --test-threads=1`
  - 4 tests passed, finished in 7.28s.
- `cargo test -p roastty surface_complete_clipboard_request -- --nocapture --test-threads=1`
  - 10 tests passed, finished in 33.85s.
- `cargo test -p roastty clipboard_request -- --nocapture --test-threads=1`
  - 12 tests passed, finished in 42.31s.
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/780-clipboard-paste-request-state.md`
- `git diff --check`

## Conclusion

Paste clipboard requests now have real Roastty-owned request state and
confirmation-preserving completion behavior. The remaining clipboard request gap
is OSC 52 request allocation/handling, which should be handled separately when
the terminal OSC request path is ready.

## Completion Review

Codex reviewed the completed result and initially found three issues: the README
checklist wording could read as if frontend/lifecycle work was done, active
requests were not consumed if the app or worker disappeared before completion,
and the preserved confirmation path did not prove that later confirmed
completion wrote to the child PTY on the same request.

The implementation and docs were updated so the checklist keeps only paste
request state in the done clause, active requests are consumed after validating
state even if the app or worker is unavailable, and the unsafe confirmation test
verifies later confirmed completion writes the expected PTY bytes for the same
preserved request. Codex reviewed the revised diff, found no blockers, and
approved the Experiment 780 result commit.
