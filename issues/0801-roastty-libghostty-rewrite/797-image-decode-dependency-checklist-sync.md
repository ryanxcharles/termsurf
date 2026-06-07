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

# Experiment 797: Image Decode Dependency Checklist Sync

## Description

Issue 801's dependency checklist still says `wuffs` / `libpng` / `zlib` are not
started. That is stale for the current Kitty graphics path: zlib-deflate
payloads are decompressed with `flate2`, and PNG decode is wired through the
`sys_decode_png` callback abstraction with direct, file, temporary-file,
shared-memory, malformed, and oversized-output test coverage.

This experiment updates the checklist wording only. It does not mark the row
complete because Roastty still has not selected or implemented a bundled Rust
PNG decoder replacement for the Ghostty `wuffs`/`libpng` path; PNG decode
depends on the host callback being installed.

## Changes

- `issues/0801-roastty-libghostty-rewrite/README.md`
  - Update the `wuffs` / `libpng` / `zlib` dependency row from "not started" to
    scoped partial wording that names `flate2` zlib-deflate handling and the PNG
    decode callback path.
  - Keep the row unchecked and explicitly leave bundled PNG decoder selection /
    replacement open.
  - Add the Experiment 797 index entry.
- `issues/0801-roastty-libghostty-rewrite/797-image-decode-dependency-checklist-sync.md`
  - Record verification evidence and review results.

## Verification

- Inspect:
  - `roastty/Cargo.toml`
  - `roastty/src/terminal/kitty/graphics_image.rs`
  - `roastty/src/terminal/kitty/graphics_exec.rs`
  - `roastty/src/lib.rs`
- Run:
  - `cargo test -p roastty zlib -- --nocapture --test-threads=1`
  - `cargo test -p roastty png -- --nocapture --test-threads=1`
- Run markdown formatting:
  - `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/797-image-decode-dependency-checklist-sync.md`
- Run:
  - `git diff --check`

The experiment passes if the dependency checklist stops saying image
decode/inflate work is not started while still keeping the row unchecked and
leaving the bundled PNG decoder replacement open. It is Partial if only the
zlib-deflate wording can be corrected. It fails if the original "not started"
wording remains accurate.

## Design Review

Codex reviewed the design and found no blocking findings. The review approved
the scoped partial wording because the dependency row remains unchecked, zlib
inflate and PNG callback coverage are named without claiming a bundled PNG
decoder replacement, and the verification plan covers the relevant dependency
and Kitty graphics surfaces.

## Result

**Result:** Pass

The image decode dependency row no longer says `wuffs` / `libpng` / `zlib` work
is not started. The README now records the existing partial coverage:

- `roastty/Cargo.toml` depends on `flate2`.
- Kitty graphics zlib-deflate transmissions are decompressed in
  `roastty/src/terminal/kitty/graphics_image.rs`.
- PNG decode is wired through `sys_decode_png` / `SysDecodePngCallback`, and the
  Kitty graphics path handles direct, file, temporary-file, shared-memory,
  malformed callback output, deferred no-decoder, and oversized-output cases.

The row remains unchecked because Roastty still has no bundled Rust PNG decoder
replacement selected or implemented; PNG decode depends on the host callback.

Verification:

- Inspected:
  - `roastty/Cargo.toml`
  - `roastty/src/terminal/kitty/graphics_image.rs`
  - `roastty/src/terminal/kitty/graphics_exec.rs`
  - `roastty/src/lib.rs`
- `cargo test -p roastty zlib -- --nocapture --test-threads=1` — 5 passed
- `cargo test -p roastty png -- --nocapture --test-threads=1` — 7 passed
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/797-image-decode-dependency-checklist-sync.md`
  — passed
- `git diff --check` — passed

## Conclusion

The image decode dependency is partial, not untouched. Zlib inflate is active in
the Kitty graphics path, and PNG decode is ABI-plumbed through the host
callback, but a bundled decoder choice/replacement remains open.

## Completion Review

Codex reviewed the completed experiment and found no blocking findings. The
review approved the result because the row remains unchecked and partial, PNG
decode is still documented as host-callback based, bundled decoder selection
remains open, and the verification evidence records the zlib/png filters,
Prettier, and `git diff --check`.
