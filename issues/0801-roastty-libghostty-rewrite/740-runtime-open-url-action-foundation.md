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

# Experiment 740: Runtime Open URL Action Foundation

## Description

Experiments 734 through 739 completed copy and paste behavior for the selection,
screen, and scrollback write-file targets. The remaining write-file action is
`open`, which upstream Ghostty implements by dispatching an `open_url` runtime
action carrying an open kind and URL bytes, then falling back to an OS opener if
the app runtime does not handle it.

Roastty does not yet expose `open_url` in its runtime action ABI. This
experiment adds the ABI foundation and a surface helper for forwarding open-url
requests to the app runtime. It deliberately does not implement
`write_*_file:open` or an OS fallback opener yet.

## Changes

- `roastty/include/roastty.h`
  - Add `ROASTTY_ACTION_OPEN_URL = 54`, matching upstream Ghostty's action tag.
  - Add `roastty_action_open_url_kind_e` with upstream-compatible values:
    `UNKNOWN = 0`, `TEXT = 1`, and `HTML = 2`.
  - Document the `ROASTTY_ACTION_OPEN_URL` storage layout:
    - `storage[0] = roastty_action_open_url_kind_e`
    - `storage[1] = borrowed const char* URL pointer valid only during `action_cb`
    - `storage[2] = URL byte length`
    - `storage[3]` through `storage[7]` are zeroed

- `roastty/src/lib.rs`
  - Add Rust constants for the open-url action tag and kind values.
  - Add a `Surface::perform_open_url_result(kind, url_bytes)` helper that
    forwards `ROASTTY_ACTION_OPEN_URL` to the existing runtime action callback.
  - Preserve borrowed payload semantics: the URL pointer is valid only during
    the callback, and the payload is pointer plus length rather than a required
    NUL-terminated string.
  - Return `false` for detached surfaces, missing apps, missing callbacks, or
    when the callback returns `false`.
  - Do not add parser support for `write_*_file:open` in this experiment.

- `roastty/tests/abi_harness.c`
  - Assert the new action tag and kind enum values match upstream.

- Tests in `roastty/src/lib.rs`
  - Cover open-url constants matching upstream values.
  - Cover forwarding unknown/text/html kinds to the runtime action callback with
    surface target, source app, action tag, kind, pointer, and byte length.
  - Assert unused storage slots `storage[3]` through `storage[7]` are zeroed.
  - Cover non-NUL-terminated URL bytes by decoding through pointer plus length
    inside the test callback record.
  - Cover callback return value propagation.
  - Cover false paths for detached surfaces, missing apps, and missing action
    callbacks.
  - Keep existing binding-action and app-runtime action tests passing.

## Verification

Run:

- `cargo fmt -p roastty`
- `cargo test -p roastty open_url -- --nocapture --test-threads=1`
- `cargo test -p roastty binding_action -- --nocapture --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo fmt -p roastty -- --check`
- `git diff --check`

## Design Review

Codex reviewed the Experiment 740 design and found one real ABI precision gap:
the new `ROASTTY_ACTION_OPEN_URL` storage layout must explicitly document and
test that unused slots `storage[3]` through `storage[7]` are zeroed. The plan
now requires both the documentation and test assertion.

The review otherwise approved the scope: add only the open-url runtime
ABI/helper, use borrowed pointer plus byte length rather than NUL termination,
preserve surface-target runtime callback semantics, and defer
`write_*_file:open` plus OS fallback behavior. The review also required
recording `[review.design]` frontmatter, this review section, and the README
tuple before the plan commit; those records are now present.
