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

# Experiment 805: Sentry Envelope Foundation

## Description

Port the local Sentry envelope parse/serialize foundation from upstream
`crash/sentry_envelope.zig` into Roastty.

Experiment 804 added the local crash-report directory/listing support. The next
useful crash-reporting slice is the envelope representation that Sentry capture
will persist into that directory. This experiment should still avoid native
Sentry SDK initialization, crash callbacks, report upload, CLI commands, and
frontend flows.

The upstream envelope parser is intentionally incomplete: it parses envelope
headers as JSON, preserves encoded items, supports known/unknown item types,
handles item payloads with explicit `length` or line-delimited payloads, decodes
attachments when requested, and can serialize the envelope back to the wire
format.

## Changes

- `roastty/Cargo.toml`
  - Add `serde_json = "1.0"` for envelope header JSON parsing. This crate is
    already present in the workspace lock via Wezboard.
- `roastty/src/crash.rs`
  - Add `Envelope`, `EnvelopeItem`, `EncodedItem`, `Attachment`, and `ItemType`
    types.
  - Add `Envelope::parse(&[u8]) -> Result<Envelope, EnvelopeError>` that parses:
    - a one-line JSON envelope header;
    - zero or more item headers;
    - item `type` as known or `Unknown`;
    - item `length` as an unsigned integer when present;
    - exact-length payloads, accepting EOF immediately after the payload and
      rejecting only a present non-newline delimiter byte;
    - line-delimited payloads when `length` is absent;
    - empty trailing lines as end-of-envelope.
  - Add `Envelope::serialize()` that emits minified one-line JSON headers and
    payloads in item order.
  - Add attachment decoding that validates required string `filename` and
    optional string `attachment_type`.
  - Add tests ported from upstream for empty envelopes, sessions, multiple
    items, no-length payloads, trailing newlines, exact-length EOF after
    payload, attachment decode, unknown item types, and serialization.
  - Add negative tests for malformed top-level headers, item headers, missing
    item `type`, malformed `length`, short exact-length payloads, non-newline
    bytes after exact-length payloads, missing/non-string attachment filenames,
    and non-string `attachment_type`.
- `issues/0801-roastty-libghostty-rewrite/README.md`
  - After implementation, update the `sentry` dependency and supporting
    subsystem rows to partial wording that names envelope parsing/serialization
    while keeping SDK initialization, crash callbacks, persistence transport,
    upload, CLI commands, and frontend flows open.

## Verification

- Inspect:
  - `vendor/ghostty/src/crash/sentry_envelope.zig`
  - `vendor/ghostty/src/crash/sentry.zig`
  - `vendor/ghostty/src/crash/dir.zig`
- Run:
  - `cargo fmt -p roastty`
  - `cargo test -p roastty envelope -- --nocapture --test-threads=1`
  - `cargo test -p roastty crash -- --nocapture --test-threads=1`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/805-sentry-envelope-foundation.md`
- Run:
  - `git diff --check`

The experiment passes if Roastty has tested Sentry envelope parse/serialize and
attachment decode behavior while the checklist rows remain partial. It is
Partial if parsing lands but serialization or attachment decode needs follow-up.
It fails if envelope handling cannot be cleanly separated from native Sentry SDK
capture.

## Design Review

Codex reviewed the design and found three blocking verification gaps. First, the
original wording could have implemented stricter-than-upstream exact-length
payload handling: upstream accepts EOF immediately after an exact-length payload
and only rejects a present non-newline delimiter byte. Second, the plan claimed
unknown item type support without a test. Third, the plan claimed attachment
decode validation without negative tests for invalid attachment headers. The
design now requires exact-length EOF and bad-delimiter tests, an unknown-type
test, and negative attachment decode tests. Codex re-reviewed the corrected
design and approved it with no blocking findings. The approval confirmed that
`serde_json` is acceptable, the verification now covers the earlier gaps, and
the scope remains limited to partial envelope parsing/serialization without SDK
initialization, crash callbacks, persistence transport, upload, CLI commands, or
frontend flows.
