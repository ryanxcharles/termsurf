# Experiment 150: Phase I — SIMD fast paths

## Description

Add Roastty's first Rust-native fast paths for the Phase I SIMD item: base64, VT
byte scanning, shared index-of, and Unicode width lookup.

Upstream Ghostty routes these through `src/simd/*.zig` and C++ helpers backed by
Highway / simdutf. Roastty does not currently have a C++ build bridge in the
`roastty` crate, so this experiment will use Rust-native accelerated crates
where they directly match the behavior and keep explicit scalar fallbacks where
the upstream helper's exact semantics do not map cleanly yet. The goal is to
turn the current scalar-only ports into a measured fast-path layer without
introducing a new C++ toolchain dependency.

This experiment is intentionally not a full Highway / simdutf port. If the
Rust-native path cannot match a surface's existing behavior exactly, or cannot
show a release-mode speedup against the current scalar code, the surface must
remain scalar or the result must be recorded as Partial instead of silently
weakening correctness or overclaiming performance.

## Changes

- `roastty/Cargo.toml` / `Cargo.lock`
  - Add `base64-simd = "0.8.0"` for SIMD-dispatched standard-base64 decoding.
  - Add a direct `memchr = "2.8"` dependency even though it is already present
    transitively, because Roastty will call it directly for its `index_of`
    equivalent and VT byte scanning.
  - Do not add a `build.rs` or C++ compilation step in this experiment.
- `roastty/src/fastmem.rs`
  - Add a small shared `index_of(input: &[u8], needle: u8) -> Option<usize>`
    helper backed by `memchr::memchr`.
  - Add tests matching upstream `simd/index_of.zig`: no match, short match, and
    larger input matches.
- `roastty/src/terminal/base64.rs`
  - Replace the scalar-only module note with a Rust-native SIMD-dispatched path
    plus scalar fallback note.
  - Try `base64_simd::STANDARD_NO_PAD` on the same safely stripped input used
    today.
  - Fall back to the existing scalar decoder only when behavior must remain
    compatible with the current Kitty/OSC leniency; the fallback must not hide
    output-buffer sizing bugs.
  - Add tests that compare the SIMD path and scalar path on padded input,
    unpadded input, empty/all-padding input, long payloads, and malformed
    padding cases the current decoder intentionally tolerates.
- `roastty/src/terminal/stream.rs`
  - Use the shared `fastmem::index_of` to skip over long ground-state runs that
    contain no ESC byte before entering the per-byte VT parser.
  - Keep the existing parser and UTF-8 replacement semantics authoritative: the
    fast path may batch ASCII print actions, but it must stop before any control
    byte, ESC byte, non-ASCII byte, or pending UTF-8 state.
  - Add stream tests for long printable ASCII runs, runs ending at ESC, runs
    ending at C0 controls, and multibyte/incomplete UTF-8 boundaries.
- `roastty/src/unicode/mod.rs`
  - Add an explicit ASCII width fast path before table lookup in `get`. This is
    a scalar hot-path shortcut, not a full replacement for upstream's Highway
    `ghostty_simd_codepoint_width` helper.
  - Preserve existing control / zero-width behavior for codepoints that the
    current table reports as width zero.
  - Add representative tests proving ASCII printable width still returns the
    same `Properties`, while C0 controls, combining marks, emoji, and out-of-
    range fallbacks keep their existing values.
- `roastty/src/terminal/*` / `roastty/src/unicode/mod.rs` tests
  - Add ignored release-mode perf probes that compare the new fast paths against
    scalar reference helpers for:
    - base64 long-payload decode;
    - byte index-of miss and late-hit cases;
    - VT printable ASCII long-run parsing;
    - ASCII width lookup.
  - Print elapsed times and ratios with `--nocapture`, and make the probes fail
    if a claimed accelerated path is not at least 1.05x faster than its scalar
    reference in release mode. Width may be reported as a scalar shortcut, but
    then the result must not call it upstream-equivalent SIMD.
- `roastty/src/lib.rs`
  - Update `ROASTTY_BUILD_INFO_SIMD` to report `true` only if this experiment
    lands actual SIMD-backed crate calls on the current build target. If the
    accepted implementation only adds scalar-compatible fast paths, leave it
    `false` and record the reason in the result.
  - Extend the ABI harness only if the value changes.

## Verification

- `cargo fmt`
- `cargo test -p roastty base64 -- --test-threads=1`
- `cargo test -p roastty fastmem -- --test-threads=1`
- `cargo test -p roastty stream -- --test-threads=1`
- `cargo test -p roastty unicode -- --test-threads=1`
- `cargo test -p roastty --test abi_harness`
- `cargo test -p roastty -- --test-threads=1`
- `cd roastty && macos/build.nu --action test`
- `cargo fmt --check`
- `git diff --check`
- `cargo tree -p roastty -i base64-simd`
- `cargo tree -p roastty -i memchr`
- `cargo test --release -p roastty simd_fast_path_perf -- --ignored --nocapture --test-threads=1`

**Pass** = Roastty has real Rust-native accelerated paths for base64 decoding
and byte index-of / VT scanning, a width lookup fast path that preserves every
tested Unicode property, release-mode perf probes show the claimed fast paths
are at least 1.05x faster than scalar references, `ROASTTY_BUILD_INFO_SIMD`
truthfully reflects the implemented acceleration, and all focused, ABI, full
Rust, hosted macOS, performance, and hygiene checks pass.

**Partial** = one or more surfaces must remain scalar to preserve behavior, but
the implemented fast paths are correct, documented, perf-checked, and the result
identifies the exact remaining SIMD bridge work; a claimed accelerated surface
is correct but only non-regressing or less than 1.05x faster in release mode; or
width remains only a scalar ASCII shortcut while base64 / index-of / VT are
accelerated.

**Fail** = the Rust-native crates cannot preserve current behavior, introduce
unacceptable dependency/build risk, or the stream/parser fast path changes VT or
UTF-8 semantics.

## Design Review

**Reviewer:** Codex-native adversarial subagent with fresh context, using the
`adversarial-review` skill's Codex path (`multi_agent_v1.spawn_agent`), not
Claude's named `adversarial-reviewer` agent.

**Status:** Approved after fixes.

**Findings:**

- **Required:** Verification did not prove the performance fast-path goal. The
  initial design listed correctness and dependency checks, but no release-mode
  benchmark, before/after comparison, or CPU-dispatch assertion that would prove
  the claimed performance work.
- **Optional:** The width plan was a scalar ASCII/table shortcut, not faithful
  to upstream's Highway `ghostty_simd_codepoint_width` helper.

**Fixes:**

- Added ignored release-mode perf probes for base64, index-of, VT printable
  ASCII parsing, and ASCII width lookup.
- Required claimed accelerated surfaces to be at least 1.05x faster than scalar
  references in release mode.
- Updated Pass / Partial criteria so correct-but-non-speedup paths are Partial,
  not Pass.
- Explicitly classified the width change as a scalar hot-path shortcut unless a
  real SIMD width path lands.

**Final verdict:** Approved.

## Result

**Result:** Partial

Roastty now has Rust-native accelerated fast paths for the three surfaces that
mapped cleanly without adding a C++ build bridge:

- base64 decoding uses `base64-simd::STANDARD_NO_PAD` on Roastty's safely
  stripped standard alphabet input, with the existing scalar decoder retained as
  the compatibility fallback for Kitty/OSC leniency cases;
- shared byte `index_of` uses a direct `memchr` dependency, matching upstream's
  `simd/index_of` surface while relying on Rust-native vectorized search on
  supported targets;
- `terminal::stream::Stream::next_slice` batches ground-state printable ASCII
  runs and uses the shared byte search to stop before ESC, while preserving the
  existing per-byte parser at controls, non-ASCII, escape states, and pending
  UTF-8 boundaries.

The Unicode width work is deliberately narrower. `unicode::get` now bypasses the
generated table for plain printable ASCII codepoints whose complete `Properties`
exactly match the generated table. It excludes ASCII emoji variation bases (`#`,
`*`, and digits) because those carry `emoji_vs_base = true`. The release-mode
perf probe measured this shortcut at 0.98x versus the table reference on this
machine, so it is a correctness-safe scalar shortcut, not a completed
upstream-equivalent SIMD width path.

`ROASTTY_BUILD_INFO_SIMD` now reports true on targets where the Rust-native
SIMD-backed crates used here have vector acceleration (`aarch64`, `x86_64`, and
`wasm32`), and the Rust/C ABI tests were updated accordingly.

## Verification

- `cargo fmt` — passed
- `cargo test -p roastty base64 -- --test-threads=1` — 20 passed, 1 ignored
- `cargo test -p roastty fastmem -- --test-threads=1` — 10 passed, 1 ignored
- `cargo test -p roastty stream -- --test-threads=1` — 860 passed, 1 ignored
- `cargo test -p roastty unicode -- --test-threads=1` — 30 passed, 1 ignored
- `cargo test -p roastty --test abi_harness` — 1 passed, with existing C
  enum-conversion warnings
- `cargo test --release -p roastty simd_fast_path_perf -- --ignored --nocapture --test-threads=1`
  — 4 passed. Measured ratios:
  - `index_of_miss`: 27.66x
  - `index_of_late_hit`: 28.46x
  - `base64_decode`: 1.72x
  - `stream_ascii`: 11.01x
  - `unicode_ascii_width`: 0.98x (reported only; this is why the result is
    Partial)
- `cargo test -p roastty -- --test-threads=1` — 4,836 unit tests passed; ABI
  harness and doc-tests passed, with existing C enum-conversion warnings and
  existing `[unknown](scope): message` noise
- `cd roastty && macos/build.nu --action test` — 211 hosted macOS tests passed
  (`TEST SUCCEEDED`), with existing SwiftLint, Swift concurrency,
  main-thread-checker, and pasteboard warning noise
- `cargo fmt --check` — passed
- `git diff --check` — passed
- `cargo tree -p roastty -i base64-simd` — direct dependency through `roastty`
- `cargo tree -p roastty -i memchr` — direct dependency through `roastty`, plus
  existing transitive paths

## Conclusion

The Rust-native SIMD/perf layer is useful but incomplete. Base64, byte search,
and VT printable ASCII parsing now have measured accelerated paths and remain
covered by compatibility tests. Width lookup still needs a real
upstream-equivalent SIMD/range-accelerated implementation if the Phase I SIMD
checklist item is to be fully closed; the current ASCII shortcut is safe but not
faster on the release probe.

## Completion Review

Codex-native adversarial subagent `Darwin` reviewed the completed experiment
with fresh context before the result commit. The reviewer inspected the
experiment file, the implementation diff from plan commit `1a593d8b62348`, the
changed source files, and the documented verification claims.

**Verdict:** Approved.

**Findings:** None.
