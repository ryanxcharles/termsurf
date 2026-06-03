+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 354: the run iterator's content hash

## Description

Each `TextRun` (Experiment 353) carries a **position-independent content hash**
used as a shaping-cache key. `RunIterator.next()` builds it by hashing, per
codepoint, the `(codepoint, cluster)` pair (clusters are **relative to the run
start**, making the hash position-independent), then the run's cell count and
the run's font index. This experiment ports that hash computation as a pure
function in `font/run.rs`, independent of the cell-walking loop that feeds it.

## Upstream behavior (`shaper/run.zig`)

```zig
var hasher = Hasher.init(0);                 // Wyhash, seed 0
// …per cell, via addCodepoint(hasher, cp, cluster):
fn addCodepoint(self, hasher, cp, cluster) !void {
    autoHash(hasher, cp);                    // codepoint first
    autoHash(hasher, cluster);               // then the (run-relative) cluster
    try self.hooks.addCodepoint(cp, cluster);
}
// …after the cell loop:
autoHash(&hasher, j - self.i);               // the run's cell count
autoHash(&hasher, current_font);             // the run's font index
const hash = hasher.final();
```

`cluster` is `j - self.i` (relative to the run start), so two runs with
identical content at different row positions hash the same — enabling cache
reuse. The hash mixes the `(cp, cluster)` sequence, the cell count, and the font
index.

## Rust mapping (`roastty/src/font/run.rs`)

```rust
use std::hash::{Hash, Hasher};
use crate::font::shape::Codepoint;

/// The position-independent content hash of a run, a shaping-cache key. Hashes
/// each codepoint's `(codepoint, cluster)` (clusters are run-relative, so the
/// hash is position-independent), then the run's `cell_count` and `font_index`.
/// Faithful port of `RunIterator.next()`'s hash construction.
///
/// (Like `Descriptor::hashcode`, the concrete hasher is roastty's deterministic
/// `DefaultHasher` rather than upstream's Wyhash — the value is internal, only the
/// content, order, and determinism matter.)
pub(crate) fn run_hash(codepoints: &[Codepoint], cell_count: u16, font_index: Index) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for cp in codepoints {
        cp.codepoint.hash(&mut h);   // codepoint first…
        cp.cluster.hash(&mut h);     // …then the run-relative cluster
    }
    cell_count.hash(&mut h);         // the run's cell count
    font_index.int().hash(&mut h);   // the run's font index (packed u16)
    h.finish()
}
```

## Scope / faithfulness notes

- **Ported**: `RunIterator.next()`'s content-hash construction — the
  per-codepoint `(codepoint, cluster)` mixing (codepoint then cluster), then the
  cell count, then the font index.
- **Faithful**: the hashed **content and order** match upstream (`cp`→`cluster`
  per codepoint, then `cell_count`, then `font_index`); position-independence is
  inherent (the caller passes run-relative clusters); `font_index` is mixed via
  its packed `int()` (the analog of `autoHash`-ing the `Collection.Index`).
- **Faithful analog**: the concrete hasher is `DefaultHasher` (SipHash), not
  upstream's Wyhash — the hash value is an internal cache key, so only
  determinism and content matter, matching the precedent set by
  `Descriptor::hashcode`.
- **Deferred** (to `next()`): producing the `(codepoint, cluster)` stream and
  the cell count from the cell-walking loop. (Consumed by tests now;
  `#![allow(dead_code)]` covers the not-yet-wired path.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/font/run.rs`: add `run_hash`; import `Codepoint` and the `Hash`/
   `Hasher` traits.
2. Tests (in `run.rs`):
   - `run_hash_deterministic`: the same `(codepoints, cell_count, font_index)`
     hashes to the same value across calls.
   - `run_hash_distinguishes`: changing any input — a codepoint, a cluster, the
     cell count, or the font index — changes the hash (each a distinct assertion
     against a baseline).
   - `run_hash_position_independent`: a run hashed from run-relative clusters
     `[0, 1, 2]` equals the same content regardless of the row position it came
     from (the function only sees the relative clusters), and differs from a run
     with absolute-looking clusters `[5, 6, 7]` — demonstrating the
     caller-supplied relativity drives the key.
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty run_hash
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `run_hash` mixes the `(codepoint, cluster)` sequence, the cell count, and the
  font index in upstream's order, deterministically;
- the deterministic, distinguishes, and position-independent tests pass, and the
  existing tests still pass;
- the cell-walking `next()` stays deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the hashed content or order diverges from upstream,
or any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and found **no Required
findings**. It confirmed: the `run_hash` inputs and order match upstream — each
`Codepoint` contributes `codepoint` first then the run-relative `cluster`,
followed by the run cell count and then the font index — covering the same data
upstream hashes (style is **not** separately hashed there, and neither is the
offset, so no input is missing); using `DefaultHasher` instead of Wyhash is a
sound roastty analog for an internal cache key (consistent with
`Descriptor::hashcode` — the exact value won't match upstream, but ordering,
determinism, and distinction are what matter); `font_index.int()` is the right
representation (the packed `u16` analog of upstream's packed
`Collection.Index`); and the position-independence framing is correct (the
helper hashes the clusters it is given, and the future `next()` must pass
run-relative clusters `j - self.i`). Deferring the cell-walking producer is
clean.

Review artifacts:

- Prompt: `logs/codex-review/20260603-150801-863867-prompt.md` (design)
- Result: `logs/codex-review/20260603-150801-863867-last-message.md` (design)
