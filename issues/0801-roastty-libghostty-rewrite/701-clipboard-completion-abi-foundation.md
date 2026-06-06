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

# Experiment 701: Clipboard Completion ABI Foundation

## Description

Upstream Ghostty's embedded C ABI includes clipboard runtime callbacks and a
surface completion API:

- `ghostty_runtime_read_clipboard_cb`;
- `ghostty_runtime_confirm_read_clipboard_cb`;
- `ghostty_runtime_write_clipboard_cb`;
- `ghostty_surface_complete_clipboard_request(surface, text, state, confirmed)`.

Roastty already exposes runtime clipboard callback fields and clipboard content
types, but it does not expose `roastty_surface_complete_clipboard_request`. Its
`roastty_clipboard_request_e` values also do not match upstream: Roastty
currently names standard/selection clipboards, while upstream request types are
paste, OSC 52 read, and OSC 52 write. This leaves the Swift/frontend side
without the ABI needed to complete read/confirm clipboard requests, and the
request enum cannot faithfully describe upstream request intent.

The full upstream path requires request allocation/lifetime, terminal paste
insertion, OSC 52 replies, write confirmation, clipboard access config, paste
protection, bracketed paste integration, and callbacks initiated by terminal
actions. Roastty does not have that clipboard request machinery yet.

This experiment corrects the ABI shape and adds an explicit no-active-request
foundation:

- align `roastty_clipboard_request_e` with upstream request values;
- add `roastty_surface_complete_clipboard_request`;
- make completion a no-op until Roastty has request state to complete;
- test that null/detached/default calls are safe and that runtime callback ABI
  signatures still compile and link.

This does not implement clipboard reads, writes, paste insertion, OSC 52
responses, confirmation prompts, request allocation, or request invalidation.

## Changes

- `roastty/include/roastty.h`
  - Change `roastty_clipboard_request_e` to upstream-shaped values:
    - `ROASTTY_CLIPBOARD_REQUEST_PASTE = 0`;
    - `ROASTTY_CLIPBOARD_REQUEST_OSC_52_READ = 1`;
    - `ROASTTY_CLIPBOARD_REQUEST_OSC_52_WRITE = 2`.
  - Add
    `roastty_surface_complete_clipboard_request(roastty_surface_t, const char*, void*, bool)`.

- `roastty/src/lib.rs`
  - Add matching request constants and layout tests.
  - Add `roastty_surface_complete_clipboard_request`:
    - tolerate null surfaces, null text, null request state, detached surfaces,
      and arbitrary confirmation flags;
    - do not mutate terminal input/output while there is no active request
      state;
    - preserve all existing runtime callback storage.
  - Keep runtime clipboard callback fields unchanged except for the corrected
    request enum values.

- `roastty/tests/abi_harness.c`
  - Add compile/link smoke coverage for the corrected request enum values and
    the new completion function on null and default surfaces.

- Tests in `roastty/src/lib.rs`
  - Cover clipboard enum values matching upstream.
  - Cover null/default/detached completion no-ops.
  - Cover completion not dirtying or waking a surface while no request exists.
  - Cover callback signatures still record request kinds using the corrected
    enum values.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty clipboard -- --nocapture`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the staged Experiment 701 design and approved it with no blocking
findings. The review confirmed that correcting `roastty_clipboard_request_e` to
request-intent values is necessary because clipboard identity remains in
`roastty_clipboard_e`, and that adding
`roastty_surface_complete_clipboard_request` as an explicit no-active-request
no-op is honest progress before request allocation and completion behavior
exist. The review accepted the callback ABI compatibility and proposed Rust/C
harness coverage.

## Result

**Result:** Pass

Implemented the clipboard completion ABI foundation:

- Corrected `roastty_clipboard_request_e` to upstream request-intent values:
  paste, OSC 52 read, and OSC 52 write.
- Added `roastty_surface_complete_clipboard_request`.
- Implemented completion as a no-op for null surfaces, null text, null state,
  detached surfaces, and default surfaces while no request state exists.
- Preserved runtime clipboard callback storage and existing callback signatures.
- Added Rust tests for enum values and no-op side effects.
- Added C harness smoke coverage for the corrected enum values and completion
  symbol.

Verification passed:

- `cargo fmt -p roastty`
- `cargo test -p roastty clipboard -- --nocapture`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Conclusion

Roastty now has the upstream-shaped clipboard completion C boundary and request
enum semantics. Real clipboard behavior remains for later experiments:
allocating request state, invoking runtime read/confirm/write callbacks from
terminal actions, inserting paste data, replying to OSC 52 reads, and
invalidating completed requests.

## Completion Review

Codex reviewed the staged completed Experiment 701 result. The review found no
code correctness blockers: corrected request enum values match upstream,
clipboard identity remains separate,
`roastty_surface_complete_clipboard_request` is safe for
null/default/detached/no-state calls, no dirty/wakeup side effects are covered,
callback field shapes remain compatible, and the C harness covers the corrected
enum values plus the new completion symbol.

The review initially blocked the result commit only because completion-review
provenance had not yet been recorded. This section, the `[review.result]`
frontmatter, and the README tuple update resolve that workflow finding.
