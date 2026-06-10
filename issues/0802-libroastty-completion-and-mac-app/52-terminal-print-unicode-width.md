+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
+++

# Experiment 52: Phase E — Terminal print Unicode width

## Description

Experiment 51 added the Rust-side `unicode::get(codepoint)` property facade that
matches Ghostty's `unicode.table.get(c)` shape for representative width,
grapheme, and emoji-variation cases. Roastty still does not consume that API:
`TerminalStreamHandler::print()` rejects only ASCII controls, stores
`previous_char`, and delegates every printable scalar to
`Screen::print_basic_cell`, which always writes a single narrow cell.

This experiment rewrites the print path to make the terminal model consume the
new Unicode properties. The goal is to make Roastty's cell grid behave like
Ghostty for the cases already exposed by the `unicode-width` live A/B recipe:
CJK/emoji wide cells, spacer tails, zero-width variation selectors, combining
marks, and cursor alignment. The current live recipe does not enable mode 2027;
mode 2027 grapheme accumulation must be proven by focused terminal tests in this
experiment. It should keep the implementation mechanical and faithful to
upstream `vendor/ghostty/src/terminal/Terminal.zig` without attempting to port
the entire generated Unicode table in this slice.

The full `unicode.graphemeBreak(cp1, cp2, state)` parity table can remain
representative rather than exhaustive here, but this experiment must leave
`Terminal::print()` shaped so a generated-table replacement can drop in later
without another terminal rewrite.

## Changes

- `roastty/src/unicode/mod.rs`
  - Add a small `BreakState` / `grapheme_break(previous, current, state)` API if
    needed by the print path.
  - Cover the representative grapheme rules needed by the live recipe and
    Ghostty print tests: Extend / ZWJ, emoji ZWJ sequences, variation selectors,
    Hangul L/V/T/LV/LVT, regional indicators, spacing marks, and the default
    break.
  - Keep the state and function internal and shaped like
    `vendor/ghostty/src/unicode/grapheme.zig` so the later full-table port is a
    local replacement.
- `roastty/src/terminal/screen.rs`
  - Add width-aware print helpers or extend `print_basic_cell` into a new helper
    that can write narrow cells, wide cells, spacer tails, and right-edge spacer
    heads while preserving existing margins, wraparound, insert mode, charset
    mapping, semantic prompt wrap marking, dirty marking, and scrollback growth.
  - Add helpers for appending grapheme codepoints to the previous printable cell
    through the existing `Page::append_grapheme_at` / grapheme-map storage.
  - Preserve current ASCII behavior for normal single-width text.
- `roastty/src/terminal/terminal.rs`
  - Rewrite `TerminalStreamHandler::print()` to query `crate::unicode::get()`.
  - Use width `2` for CJK and emoji cells, writing a wide head and spacer tail.
  - Respect wraparound and right-margin behavior for wide characters, including
    the "do not print if wide char would start at the right edge with wraparound
    disabled" case.
  - Respect insert mode using the printed width instead of hard-coded `1`.
  - Attach width-`0` scalars to the previous printable cell when grapheme mode
    is disabled, matching Ghostty's legacy zero-width fallback.
  - When `Mode::GraphemeCluster` is enabled, use the representative
    `unicode::grapheme_break` path to append non-breaking codepoints to the
    previous cell instead of blindly printing them as new cells.
  - Match Ghostty's two variation-selector branches for representative cases: in
    mode 2027, VS15/VS16 should only affect a previous `emoji_vs_base`; in the
    legacy zero-width fallback, VS15/VS16 should only attach to a previous
    `ExtendedPictographic` cell. Invalid variation selectors should be ignored
    rather than printed.
  - Update `previous_char` only for newly printed base characters, not for
    zero-width appended scalars that should not change REP (`CSI b`) behavior.
- Tests
  - Add focused terminal tests for CJK width, emoji width, right-edge wide
    wrapping, wide-at-right-edge with wraparound disabled, insert-mode width
    handling, combining mark attachment under mode 2027, VS16 widening, VS15
    narrowing, invalid variation selector ignore behavior, and `print_repeat`
    preserving the correct previous printable scalar.
  - Keep tests numeric-codepoint based where practical to avoid introducing
    non-ASCII source text.
- `issues/0802-libroastty-completion-and-mac-app/README.md`
  - Add this experiment to the index as `Designed`.
  - After implementation, record the new durable Phase-E behavior and the
    remaining Unicode table/grapheme parity gap.

## Verification

- Run formatting:
  - `cargo fmt -- roastty/src/unicode/mod.rs roastty/src/terminal/terminal.rs roastty/src/terminal/screen.rs`
  - `prettier --write --prose-wrap always --print-width 80 issues/0802-libroastty-completion-and-mac-app/README.md issues/0802-libroastty-completion-and-mac-app/52-terminal-print-unicode-width.md`
- Run targeted tests:
  - `cargo test -p roastty unicode`
  - `cargo test -p roastty terminal_stream_print`
  - `cargo test -p roastty terminal_stream_unicode`
- Run full Roastty tests:
  - `cargo test -p roastty`
- Run shell syntax checks:
  - `bash -n scripts/roastty-app/live-ab-smoke.sh`
  - `bash -n scripts/roastty-app/live-ab-matrix.sh`
- Run the Unicode live A/B recipe with permissive thresholds and record the
  content-region metric:
  - `scripts/roastty-app/live-ab-smoke.sh --recipe unicode-width --max-mismatch-ratio 1 --max-mean-channel-delta 255`
- Run `git diff --check`.
- Run `git status --short` and verify no generated artifacts or screenshots are
  in the repo.

**Pass** = Roastty's terminal grid prints representative wide and zero-width
cases through the new Unicode property API; focused unit tests prove mode 2027
grapheme accumulation; focused and full Roastty tests pass; the live
`unicode-width` recipe still runs and records a content metric for
width/alignment; formatting, shell syntax, diff, and status checks pass.

**Partial** = the Rust print path and focused tests work, but the live app
recipe or full suite is blocked by an unrelated local failure; record the exact
failure and why it is unrelated.

**Fail** = the current page/screen abstractions cannot support Ghostty-style
wide cells and grapheme mutation without a lower-level page rewrite first.

## Design Review

**Reviewer:** Codex-native adversarial subagent (`multi_agent_v1.spawn_agent`,
fresh context, read-only). **Initial verdict: CHANGES REQUIRED. Final verdict:
APPROVED.**

The reviewer found one Required issue: the first design overstated what the
existing live `unicode-width` recipe proves because that recipe does not enable
mode 2027. Fixed by narrowing the live A/B claim to width, zero-width, and
alignment behavior, and by requiring focused terminal tests to prove mode 2027
grapheme accumulation. The reviewer also noted an Optional ambiguity around
variation selectors; fixed by spelling out Ghostty's two branches: mode 2027
uses `emoji_vs_base`, while the legacy zero-width fallback accepts a previous
`ExtendedPictographic` cell. Re-review approved with no remaining Required
findings.
