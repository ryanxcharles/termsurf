+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 642: Tmux Control Parser

## Description

Port the tmux control-mode parser foundation from Ghostty into Roastty.

The terminal checklist still marks `tmux` control mode as missing. Upstream
splits that subsystem into several pieces:

- `terminal/tmux/control.zig` parses tmux control-mode notifications;
- `terminal/tmux/layout.zig` parses pane layout trees;
- `terminal/tmux/output.zig` formats commands sent back to tmux;
- `terminal/tmux/viewer.zig` owns the high-level viewer state machine;
- `terminal/dcs.zig` recognizes the `DCS 1000 p` entry sequence and
  `termio/stream_handler.zig` wires the viewer to the PTY and terminal stream.

This experiment should implement only the first slice: the standalone
control-mode parser and notification types. It is the smallest faithful unit
with direct upstream tests and it does not require committing to the later
viewer, layout, or PTY integration shape yet.

## Changes

1. Add `roastty/src/terminal/tmux.rs` and expose it from
   `roastty/src/terminal/mod.rs`.
2. Implement a `ControlParser` with the same behavioral surface as
   `vendor/ghostty/src/terminal/tmux/control.zig`:
   - idle, notification, block, and broken states;
   - one-megabyte default buffer limit and broken-state drop behavior;
   - `%begin` / `%end` / `%error` block handling with exact guard-line shape
     validation, without matching terminator metadata against the prior
     `%begin`;
   - `%output`, `%session-changed`, `%sessions-changed`, `%layout-change`,
     `%window-add`, `%window-renamed`, `%window-pane-changed`,
     `%client-detached`, and `%client-session-changed` notifications;
   - unknown or malformed notifications returning to idle without emitting a
     notification;
   - non-`%` idle input emitting an exit notification and entering the broken
     state.
3. Use Rust string parsing or `regex` only where it keeps the upstream matching
   semantics clear. The parser should own emitted notification data rather than
   returning borrowed slices into an internal buffer.
4. Represent upstream's `enter` notification boundary deliberately. Because DCS
   entry remains excluded, either include an un-emitted `Enter` variant for
   future DCS wiring or defer it explicitly in the result.
5. Add focused tests mirroring the upstream `control.zig` cases.
6. Leave `terminal/dcs.rs`'s current `DCS 1000 p` ignore behavior unchanged in
   this experiment. DCS entry, layout parsing, output formatting, viewer state,
   PTY writes, and Surface/App integration remain future tmux experiments.
7. Update `issues/0801-roastty-libghostty-rewrite/README.md` after the result:
   - keep the overall terminal-core `tmux` checklist item unchecked;
   - refine the line to say the control parser is done, while DCS entry, layout,
     output, viewer, PTY, and App integration remain missing;
   - update the Experiment 642 index status.
8. Update this experiment file with the result and review records.

## Verification

- `cargo test -p roastty terminal::tmux`
- `cargo test -p roastty terminal::dcs`
- `cargo test -p roastty terminal::terminal::tests::terminal_stream_dcs_command_tmux_and_unknown_are_ignored`
- `cargo fmt -p roastty`
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/642-tmux-control-parser.md`
- compare/read the Rust parser against:
  - `vendor/ghostty/src/terminal/tmux.zig`
  - `vendor/ghostty/src/terminal/tmux/control.zig`
  - `vendor/ghostty/src/terminal/dcs.zig`
- `git diff --check`

Pass = Roastty has a tested standalone tmux control-mode parser matching
upstream's notification/block/broken-state behavior, while the README keeps the
overall `tmux` checklist item open because DCS entry, layout, output, viewer,
PTY, and app integration are still missing.

Fail = the parser diverges from upstream control-mode behavior, depends on later
viewer integration to be testable, or overclaims completion of the wider tmux
subsystem.

## Design Review

Initial Codex design review session `019e9a9a-ee48-7ec2-bb17-ea152a97b42d`
requested revisions:

- clarify that block terminator validation only checks the exact `%end` /
  `%error` guard-line shape and does not compare terminator metadata to the
  prior `%begin`;
- add the README checklist update to the planned changes;
- add the existing terminal stream DCS-ignore regression to verification;
- make the upstream `enter` notification boundary explicit while DCS entry
  remains excluded.

The plan was revised to address those findings.

Follow-up review in the same session approved the revised design with no
blocking findings. The reviewer confirmed that the guard-line validation
boundary, README checklist update, DCS-ignore regression, and `enter`
notification boundary were all addressed.
