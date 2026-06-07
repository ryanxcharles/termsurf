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

# Experiment 786: Kitty Clipboard Read Replies

## Description

Experiment 785 completed OSC 52 write handling, but OSC 5522 Kitty clipboard
events still stop at `Surface::handle_terminal_clipboard_event` and are silently
ignored. The retained parser already preserves metadata, payload, and the OSC
terminator, so the next narrow slice is to make read requests observable to
terminal applications.

This experiment implements Kitty clipboard `type=read` replies for the existing
plain-text runtime clipboard ABI. It decodes the canonical read payload, accepts
requests that include `text/plain`, supports the MIME availability query payload
`.` with the known `text/plain` type, and reuses the current clipboard read
policy, confirmation flow, and PTY write path from OSC 52 reads. Full arbitrary
MIME clipboard support, passwords, write transactions, write aliases, and
multipart/chunked write state are explicitly deferred.

## Changes

- `roastty/src/terminal/mod.rs`
  - Expose the existing Kitty clipboard option parser to the crate so the
    surface layer can classify retained OSC 5522 events without duplicating the
    parser.
- `roastty/src/terminal/clipboard.rs`
  - Widen the visibility of `KittyClipboard`, `Location`, `Operation`, `Status`,
    and their option accessors only as far as the crate needs.
  - Keep existing parser semantics and tests unchanged.
- `roastty/src/lib.rs`
  - Extend `Surface::handle_terminal_clipboard_event` so Kitty clipboard events
    enter a dedicated handler instead of being dropped.
  - Handle `type=read` requests for the standard clipboard and for `loc=primary`
    when the runtime reports selection clipboard support.
  - Decode Kitty read payloads as base64. Accept payloads whose decoded
    space-separated MIME list contains `text/plain`.
  - Treat a decoded payload of `.` as a MIME availability query and answer with
    the only MIME type the current runtime clipboard ABI can expose:
    `text/plain`. This path must not ask for clipboard-read permission because
    it does not read clipboard contents.
  - Preserve the request terminator in all OSC 5522 replies.
  - Preserve the request `id` option on every reply after stripping characters
    outside the protocol's valid id set `[a-zA-Z0-9-_+.]`. Omit the `id` option
    when no valid characters remain.
  - Map standard reads to `ROASTTY_CLIPBOARD_STANDARD`; map `loc=primary` reads
    to `ROASTTY_CLIPBOARD_SELECTION` only when selection support exists.
  - Ignore Kitty read requests when the surface has no app, no termio worker,
    `clipboard-read = deny`, no runtime read callback, or no supported clipboard
    target.
  - For unsupported primary-selection reads, invalid/missing operation values,
    non-read operations, or unsupported Kitty read payload forms, send an OSC
    5522 error reply instead of silently doing nothing.
  - Add a Kitty clipboard read request kind to the active clipboard request
    state so completion can encode protocol-specific replies.
  - On successful read completion, write this sequence to the PTY:
    - `OSC 5522;type=read[:id=<id>]:status=OK ST`
    - `OSC 5522;type=read[:id=<id>]:status=DATA:mime=<base64 text/plain>;<base64 text> ST`
    - `OSC 5522;type=read[:id=<id>]:status=DONE ST`
  - On denied confirmation completion, write
    `OSC 5522;type=read[:id=<id>]:status=EPERM ST`.
  - Use `status=ENOSYS` for unsupported clipboard targets or unsupported read
    forms, and keep `status=EPERM` for read-policy or user-denied permission.
  - Do not implement `type=write`, `type=wdata`, or `type=walias`; reply with
    `status=ENOSYS` for those operations so callers receive an explicit
    unsupported response.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - Add the experiment index entry.
  - Narrow the surface lifecycle checklist item from broad Kitty clipboard
    handling to the remaining Kitty write/multipart handling after this
    experiment passes.

## Verification

- Inspect references:
  - Kitty clipboard protocol documentation for OSC 5522 read replies and error
    statuses.
  - `vendor/ghostty/src/terminal/osc/parsers/kitty_clipboard_protocol.zig`
    parser behavior.
  - `vendor/ghostty/src/terminal/stream.zig` upstream's currently unimplemented
    Kitty clipboard callback.
- Run focused tests:
  - `cargo test -p roastty kitty_clipboard -- --nocapture --test-threads=1`
  - `cargo test -p roastty osc52_clipboard -- --nocapture --test-threads=1`
  - `cargo test -p roastty clipboard_read -- --nocapture --test-threads=1`
- New or updated Kitty clipboard assertions must cover:
  - `type=read` starts a runtime read request for the standard clipboard;
  - `type=read:loc=primary` starts a selection read request when supported;
  - `type=read:loc=primary` receives `ENOSYS` when selection support is absent;
  - read payloads decode as base64 MIME lists and only requests including
    `text/plain` reach the runtime read callback;
  - read payload `Lg==` decodes to `.`, returns the known `text/plain` MIME
    availability data without a runtime read or permission prompt, and still
    preserves the original terminator;
  - completed standard reads emit `OK`, `DATA` with `text/plain`, and `DONE`;
  - completed reads use BEL when the request used BEL and ST when the request
    used ST;
  - valid request ids are copied to every reply;
  - invalid id characters are stripped before replies, and an id with no valid
    characters is omitted;
  - `clipboard-read = deny` emits `EPERM` and does not allocate an active
    request;
  - user-denied confirmation emits `EPERM`;
  - missing `type`, invalid `type`, `type=write`, `type=wdata`, and
    `type=walias` emit explicit unsupported errors;
  - invalid base64 read payloads and MIME lists that do not include `text/plain`
    emit `ENOSYS` rather than being mistaken for plain-text reads;
  - existing OSC 52 read/write tests still pass.
- Run:
  - `cargo fmt -p roastty`
  - `cargo fmt -p roastty -- --check`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/786-kitty-clipboard-read-replies.md`
- Run:
  - `git diff --check`

The experiment passes if Kitty clipboard read requests reach the existing
runtime read callback, successful completion writes spec-shaped plain-text OSC
5522 replies to the PTY with the original terminator, permission and unsupported
cases produce explicit status replies, MIME availability queries return
`text/plain` without reading the clipboard, valid request ids are preserved
after sanitization, OSC 52 behavior remains unchanged, and all focused tests
pass. It is Partial if only standard clipboard reads can be proven. It fails if
the current runtime clipboard ABI cannot safely produce a minimal Kitty read
reply without adding broader MIME clipboard support first.

## Design Review

The first Codex design review found two blocking protocol issues:

- the initial plan rejected the canonical Kitty `type=read;<base64 MIME list>`
  request form instead of accepting requests that include `text/plain`;
- the initial plan omitted `id` propagation, which Kitty requires for
  multiplexer support.

The design now decodes read payloads, accepts MIME lists containing
`text/plain`, supports the `.` availability query with the current plain-text
MIME surface, and requires sanitized `id` propagation on every reply.

Re-review approved the revised design with no blocking findings.
