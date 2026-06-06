+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"

[review.design]
agent = "codex"
session = "019e9ad0-cc79-7040-a54f-3245185e6401"
verdict = "approved"

[review.result]
agent = "codex"
session = "019e9ad0-cc79-7040-a54f-3245185e6401"
verdict = "approved"
+++

# Experiment 646: Tmux Command Formatting

## Description

Port the tmux viewer command string formatting layer.

Experiment 644 added output variable parsing and `#{variable}` format-string
helpers. Upstream's next narrow viewer-adjacent slice is the `Command` union and
`Format` constants in `terminal/tmux/viewer.zig`: they produce concrete tmux
commands such as `list-windows`, `list-panes`, `display-message`, and
`capture-pane`.

This experiment should add only the command-formatting data types and tests. It
must not build the viewer state machine, command queue, PTY write path, DCS
notification consumer, or App/Surface integration.

## Changes

1. Extend `roastty/src/terminal/tmux.rs` with:
   - `TmuxScreenKey::{Primary, Alternate}` for capture-pane formatting;
   - `TmuxCapturePane { id, screen_key }`;
   - `TmuxCommand::{ListWindows, PaneHistory, PaneVisible, PaneState, TmuxVersion, User(String)}`;
   - reusable `LIST_WINDOWS_VARIABLES`, `LIST_PANES_VARIABLES`,
     `TMUX_VERSION_VARIABLES`, and delimiter constants;
   - `TmuxCommand::format_command` returning the exact command string. Built-in
     commands are newline-terminated; `User(String)` is exact passthrough, so
     callers must include a newline if they need one.
2. Port upstream command strings from
   `vendor/ghostty/src/terminal/tmux/viewer.zig`:
   - `list-windows -F '{format}'\n`;
   - `capture-pane -p -e -q {optional -a }-S - -E -1 -t %{id}\n`;
   - `capture-pane -p -e -q {optional -a }-t %{id}\n`;
   - `list-panes -F '{format}'\n`;
   - `display-message -p '{format}'\n`;
   - user command passthrough.
3. Reuse `format_output_variables` for the upstream `Format.list_windows`,
   `Format.list_panes`, and `Format.tmux_version` variable lists and delimiters.
4. Add tests for every command variant, primary/alternate capture-pane
   formatting, built-in command newline termination, exact `-t %42` pane target
   formatting, exact `list-windows` / `list-panes` / `display-message` strings,
   exact format variable lists, and user passthrough without implicit newline.
5. Keep the README's overall `tmux` checklist item unchecked, refining it after
   the result to say command formatting is done while viewer state, PTY, and App
   integration remain missing.
6. Update this experiment file with result and review records.

## Verification

- `cargo test -p roastty terminal::tmux`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/646-tmux-command-formatting.md`
- compare/read the Rust command formatter against:
  - `vendor/ghostty/src/terminal/tmux/viewer.zig` `Command` and `Format`
    sections
  - `vendor/ghostty/src/terminal/tmux/output.zig`
- `git diff --check`

Pass = Roastty has tested standalone tmux command string formatting matching
upstream command shapes and format variable lists, while the README keeps viewer
state, PTY, and App/Surface integration open.

Fail = command strings differ from upstream, capture-pane alternate-screen flags
are wrong, format variable lists drift, built-in commands are not
newline-terminated, or the experiment overclaims viewer/runtime integration.

## Design Review

Initial Codex design review session `019e9ad0-cc79-7040-a54f-3245185e6401`
requested revisions:

- clarify that built-in commands are newline-terminated while `User(String)` is
  exact passthrough;
- state that capture-pane targets include the tmux `%` prefix, for example
  `-t %42`;
- name reusable Rust format variable/delimiter constants;
- add explicit expected string assertions for list-windows, list-panes, and
  display-message.

The plan was revised to address those findings.

Follow-up review in the same session approved the revised design for
implementation. The reviewer confirmed that built-in command newline behavior,
user passthrough, `%` pane targets, reusable format constants, and exact command
string tests are now specified. The only nit was to make the fail criterion say
built-in commands, not all commands, are newline-terminated; that wording was
updated before the plan commit.

## Result

**Result:** Pass

Roastty now has standalone tmux command string formatting in
`roastty/src/terminal/tmux.rs`.

The implementation ports the command-formatting slice from
`vendor/ghostty/src/terminal/tmux/viewer.zig`:

- `TmuxScreenKey` distinguishes primary and alternate capture targets;
- `TmuxCapturePane` carries pane ID and screen key;
- `TmuxCommand` covers list windows, pane history, pane visible, pane state,
  tmux version, and user passthrough commands;
- reusable variable-list and delimiter constants mirror upstream
  `Format.list_windows`, `Format.list_panes`, and `Format.tmux_version`;
- built-in commands are newline-terminated;
- user commands are returned exactly as provided;
- capture-pane targets include the tmux `%` pane prefix, e.g. `-t %42`;
- alternate capture-pane commands include `-a`.

Verification passed:

- `cargo test -p roastty terminal::tmux` — 68 passed
- `cargo fmt -p roastty` — passed
- `cargo fmt -p roastty -- --check` — passed
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/646-tmux-command-formatting.md`
  — passed
- `git diff --check` — passed

Source comparison was performed against:

- `vendor/ghostty/src/terminal/tmux/viewer.zig` `Command` and `Format` sections
- `vendor/ghostty/src/terminal/tmux/output.zig`

Completion review in Codex session `019e9ad0-cc79-7040-a54f-3245185e6401`
approved the code behavior and scope. The reviewer confirmed that Rust command
formatting matches upstream `Command.formatCommand`, that built-ins include
trailing newlines, user commands are exact passthrough, `capture-pane` uses
`-t %42`, alternate captures include `-a`, and the format variable constants
match upstream ordering and delimiters. The only blocking issue was missing
review provenance metadata, fixed before the result commit.

## Conclusion

Tmux command string formatting is complete. The overall terminal-core `tmux`
checklist item remains open because viewer state, command queue/runtime
coordination, PTY read/write integration, and App/Surface wiring are still
missing.
