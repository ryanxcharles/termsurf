+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 51: Phase E — Unicode width properties

## Description

Experiment 50 added a live `unicode-width` A/B recipe, exposing the current
Roastty gap at the app level. The underlying cause was already identified by the
architecture audit: Roastty has page/cell support for wide cells and grapheme
storage, but `Terminal::print()` still writes narrow, single-codepoint cells and
there is no Rust `unicode/` property namespace equivalent to Ghostty's
`unicode.table.get(c)`.

This experiment adds the first Rust-side Unicode property API that
`Terminal::print()` can call in the next slice. It should mirror the upstream
shape closely enough to keep the later print rewrite mechanical:

- a `unicode` module;
- a compact `Properties` value containing at least `width`,
  `width_zero_in_grapheme`, `grapheme_break`, and `emoji_vs_base`;
- a `get(codepoint: u32)` lookup, so Rust can model Ghostty's explicit
  out-of-range fallback for values beyond the Unicode scalar range;
- representative tests for the codepoints exercised by the new live
  `unicode-width` recipe.

This is intentionally not the full grapheme-break table or the full
`Terminal::print()` rewrite. This slice should include a Ghostty-shaped
`grapheme_break` property classification for representative cases because the
print rewrite needs to inspect it, but it should defer the full
`unicode.graphemeBreak(cp1, cp2, state)` state-machine/table port to the next
grapheme-clustering slice. If direct use of a Rust Unicode crate differs from
Ghostty's generated table on a representative case, add a local compatibility
override and record the gap; the final target remains Ghostty's pinned
`vendor/ghostty/src/unicode/` semantics.

## Changes

- `roastty/Cargo.toml`
  - Add any direct Unicode crate dependency needed by `roastty` instead of
    relying on transitive dependencies.
- `roastty/src/unicode/`
  - Add a `mod.rs` with a Ghostty-shaped `Properties` struct and `get` function.
  - Implement width lookup clamped to Ghostty's `[0, 2]` terminal-cell range.
  - Implement `width_zero_in_grapheme`, `grapheme_break`, and `emoji_vs_base`
    for at least the representative variation-selector / combining / emoji cases
    needed by the next print slice.
  - Use a `u32` codepoint input and mirror Ghostty's out-of-range fallback:
    width `1`, `width_zero_in_grapheme = true`, `grapheme_break = other`, and
    `emoji_vs_base = false`.
  - Keep the API internal unless another module already needs it public.
- `roastty/src/lib.rs` or module declarations
  - Register the new module.
- Tests
  - Add unit tests for ASCII fast-path width, combining marks, CJK wide
    characters, emoji used in the live recipe, VS15/VS16, box/symbol glyphs,
    representative grapheme-break classes, and out-of-range/default behavior.
  - Where practical, cite the corresponding upstream Ghostty property behavior
    from `vendor/ghostty/src/unicode/props*.zig`.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Add this experiment to the index as `Designed`.
  - After implementation, record the durable fact and next Phase-E target.

## Verification

- Run formatting:
  - `cargo fmt`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/51-unicode-width-properties.md`
- Run targeted tests:
  - `cargo test -p roastty unicode`
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run shell syntax checks to ensure the live recipe harness remains valid:
  - `bash -n scripts/roastty-app/live-ab-smoke.sh`
  - `bash -n scripts/roastty-app/live-ab-matrix.sh`
- Run `git diff --check`.
- Run `git status --short` and verify no generated artifacts or screenshots are
  in the repo.

**Pass** = Roastty has a Ghostty-shaped Unicode property lookup with
representative width/variation tests passing, the full Roastty test suite
passes, formatting and diff checks pass, and the next experiment can rewrite
`Terminal::print()` against this API.

**Partial** = the property API and targeted tests exist, but full-suite
verification is blocked by an unrelated local failure; record the exact failure
and why it is unrelated.

**Fail** = the chosen Rust-side implementation cannot be made compatible with
the representative Ghostty width/property semantics without a generated-table
port first.

## Design Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, read-only). **Initial verdict: CHANGES REQUIRED. Final verdict:
APPROVED.**

The reviewer found one Required issue: the first design under-specified
`grapheme_break`, even though upstream `Properties` includes it and
`Terminal.print` needs it for the next rewrite. Fixed by adding a Ghostty-shaped
`grapheme_break` property to this slice while explicitly deferring the full
`unicode.graphemeBreak(cp1, cp2, state)` table/state-machine port. The reviewer
also flagged an Optional ambiguity around Rust out-of-range behavior; fixed by
specifying a `u32` codepoint input and Ghostty's fallback properties for values
beyond the Unicode scalar range. Re-review approved with no remaining blockers.
