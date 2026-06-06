+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.result]
agent = "codex"
session = "019e9a9a-ee48-7ec2-bb17-ea152a97b42d"
verdict = "approved_with_nits"
+++

# Experiment 643: Tmux Layout Parser

## Description

Port the tmux layout parser from Ghostty into Roastty.

Experiment 642 added the standalone tmux control-mode parser, but layout-change
notifications still expose layout strings as raw text. Upstream's next isolated
tmux unit is `terminal/tmux/layout.zig`, which parses those strings into a tree
of pane and split nodes and verifies tmux's four-character checksum prefix.

This experiment should add only that parser/checksum slice. It must not wire the
layout tree into the viewer, DCS entry, PTY handling, App/Surface integration,
or Roastty's existing terminal `split_tree` module.

## Changes

1. Extend `roastty/src/terminal/tmux.rs` with:
   - `Layout` carrying `width`, `height`, `x`, `y`, and `content`;
   - `LayoutContent::{Pane(usize), Horizontal(Vec<Layout>), Vertical(Vec<Layout>)}`;
   - `LayoutParseError::{SyntaxError, ChecksumMismatch}`;
   - `Layout::parse` for raw tmux layout strings;
   - `Layout::parse_with_checksum` for `XXXX,layout` strings;
   - `LayoutChecksum::calculate` and four-character lowercase hexadecimal
     formatting.
2. Mirror upstream parser behavior from
   `vendor/ghostty/src/terminal/tmux/layout.zig`:
   - parse `WxH,X,Y,ID` pane leaves;
   - parse `{...}` horizontal split children and `[...]` vertical split
     children;
   - support nested layouts and comma-separated sibling nodes;
   - reject malformed numeric fields, missing delimiters, mismatched brackets,
     trailing data, and checksum errors.
   - match upstream checksum-wrapper error semantics exactly:
     `parse_with_checksum` returns `SyntaxError` only when the wrapper is fewer
     than five bytes or byte 4 is not a comma; otherwise any prefix that does
     not equal the calculated lowercase checksum is `ChecksumMismatch`,
     including uppercase or non-hex-looking prefixes.
3. Use owned `Vec<Layout>` children in Rust instead of allocator-backed slices.
4. Add focused tests mirroring upstream `layout.zig` parse and checksum cases,
   including:
   - simple pane, offset pane, large values, horizontal split, vertical split,
     three-pane split, nested horizontal-in-vertical, nested
     vertical-in-horizontal, and deeply nested layout;
   - empty/missing/non-numeric fields, unclosed and mismatched brackets,
     trailing data, missing `x`, and missing content delimiter;
   - valid checksum with the known `f8f9` horizontal split case, checksum
     mismatch, too-short wrapper using `bb62`, missing comma, zero padding,
     wraparound, deterministic checksum, and known checksum examples.
5. Keep the README's overall `tmux` checklist item unchecked, refining it after
   the result to say the control and layout parsers are done while output,
   viewer, DCS entry, PTY, and App integration remain missing.
6. Update this experiment file with result and review records.

## Verification

- `cargo test -p roastty terminal::tmux`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/643-tmux-layout-parser.md`
- compare/read the Rust parser against:
  - `vendor/ghostty/src/terminal/tmux.zig`
  - `vendor/ghostty/src/terminal/tmux/layout.zig`
  - `vendor/ghostty/src/terminal/tmux/viewer.zig` layout usage sites
- `git diff --check`

Pass = Roastty has a tested standalone tmux layout/checksum parser matching
upstream's tree and checksum behavior, while the README keeps the overall tmux
item open for output formatting, viewer state, DCS entry, PTY, and App/Surface
integration.

Fail = the parser accepts malformed layouts upstream rejects, rejects valid
upstream layouts, overfits tests without the recursive tree shape, or overclaims
completion of the wider tmux subsystem.

## Design Review

Initial Codex design review session `019e9a9a-ee48-7ec2-bb17-ea152a97b42d`
requested revisions:

- clarify checksum-wrapper error semantics so Rust does not add stricter hex
  validation than upstream;
- spell out the `LayoutContent` enum payload shapes;
- explicitly list representative upstream layout and checksum cases, including
  the known checksum examples and nested/malformed layouts.

The plan was revised to address those findings.

Follow-up review in the same session approved the revised design for
implementation. The reviewer confirmed that the checksum-wrapper semantics now
match upstream, the enum payload shapes are explicit, and the planned tests
cover the key parser and checksum cases.

## Result

**Result:** Pass

Roastty now has a standalone tmux layout and checksum parser folded into
`roastty/src/terminal/tmux.rs`.

The parser ports the isolated behavior from
`vendor/ghostty/src/terminal/tmux/layout.zig`:

- `Layout` nodes carry width, height, x/y offsets, and `LayoutContent`;
- `LayoutContent` represents pane leaves plus horizontal and vertical child
  vectors;
- raw `WxH,X,Y,ID` pane layouts parse into leaf nodes;
- `{...}` horizontal splits and `[...]` vertical splits parse recursively;
- comma-separated sibling nodes and nested horizontal/vertical trees are
  preserved;
- malformed numbers, missing delimiters, missing content, unclosed or mismatched
  brackets, and trailing data return `SyntaxError`;
- `parse_with_checksum` accepts `XXXX,layout` wrappers, returns `SyntaxError`
  only for wrappers shorter than five bytes or missing the comma at byte 4, and
  returns `ChecksumMismatch` for any non-matching prefix after that;
- `LayoutChecksum` uses the upstream rotate-right/add byte algorithm and emits
  lowercase, zero-padded four-character hexadecimal checksums.

The Rust implementation owns child layouts with `Vec<Layout>`, rather than using
allocator-backed slices. It remains standalone: layout strings from tmux control
notifications are not yet converted in the viewer path because the viewer,
output formatter, DCS entry, PTY handling, and App/Surface integration are still
future tmux work.

Verification passed:

- `cargo test -p roastty terminal::tmux` — 47 passed
- `cargo fmt -p roastty` — passed
- `cargo fmt -p roastty -- --check` — passed
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/643-tmux-layout-parser.md`
  — passed
- `git diff --check` — passed

Source comparison was performed against:

- `vendor/ghostty/src/terminal/tmux.zig`
- `vendor/ghostty/src/terminal/tmux/layout.zig`
- `vendor/ghostty/src/terminal/tmux/viewer.zig` layout usage sites

Completion review in Codex session `019e9a9a-ee48-7ec2-bb17-ea152a97b42d`
approved the result with no blocking findings. The reviewer agreed that the Rust
parser matches vendored `layout.zig` behavior for recursive layout parsing,
checksum calculation, and checksum-wrapper error classification, and that the
wider tmux checklist remains open correctly. The optional test nits were fixed
before the result commit by adding the upstream different-inputs checksum test
and the known `bb62` tmux layout checksum case.

## Conclusion

The tmux layout/checksum parser foundation is complete. The overall
terminal-core `tmux` checklist item remains open because output command
formatting, DCS entry, viewer state, PTY read/write integration, and App/Surface
wiring are still missing.
