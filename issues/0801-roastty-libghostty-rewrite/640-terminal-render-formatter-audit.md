+++
[implementer]
agent = "codex"
model = "gpt-5"
reasoning = "high"
+++

# Experiment 640: Terminal Render And Formatter Audit

## Description

Audit the Issue 801 terminal checklist line for `formatter` / terminal `render`,
`ScreenSet`, and `stream_terminal`.

The current checklist groups several different upstream subsystems into one
partial item. Current code suggests the grouped state is mixed:

- formatter behavior exists in `page_list.rs`, `screen.rs`, `terminal.rs`, and
  the formatter C ABI;
- renderer-facing row snapshots and shaper run options exist, but not as the
  full upstream `terminal/render.zig::RenderState`;
- upstream `ScreenSet.zig` behavior is folded into Roastty's `TerminalScreens`
  fields in `terminal.rs`;
- upstream `stream_terminal.zig` is folded into Roastty's stream dispatch in
  `terminal.rs` and `stream.rs`, while the higher-level `termio` integration is
  still outside this terminal-core slice.

This experiment should verify those claims against vendored Ghostty and update
the checklist wording so it separates completed formatter/stream pieces from the
remaining render-state or architecture work. It is intended as a
documentation-only audit unless the verification uncovers a small missing test
that should be added immediately.

## Audit Targets

1. `vendor/ghostty/src/terminal/formatter.zig` vs.
   `vendor/ghostty/src/terminal/c/formatter.zig` and
   `roastty/src/terminal/page_list.rs`, `screen.rs`, `terminal.rs`, and
   formatter C ABI tests:
   - plain, VT, and HTML formatting;
   - selection content and no-content paths;
   - codepoint, point, and pin maps;
   - terminal/screen extras for cursor, style, protection, kitty keyboard,
     hyperlink, charsets, and palette.
2. `vendor/ghostty/src/terminal/render.zig` and
   `vendor/ghostty/src/terminal/c/render.zig` vs. Roastty renderer-facing
   snapshots and C ABI:
   - `Terminal::render_rows_snapshot`;
   - `Screen::render_rows_snapshot`;
   - `PageList::render_rows_snapshot`;
   - `shape_run_options`;
   - C ABI render-state row/cell snapshot accessors.
3. `vendor/ghostty/src/terminal/ScreenSet.zig` vs. Roastty's folded
   `TerminalScreens` implementation in `terminal.rs`:
   - primary/alternate initialization;
   - active-screen switching;
   - alternate removal and generation invalidation;
   - active screen formatting and stream behavior.
4. `vendor/ghostty/src/terminal/stream_terminal.zig` vs. Roastty
   `terminal.rs`/`stream.rs`:
   - parser action dispatch into terminal state;
   - effects that are intentionally represented through callbacks or separate C
     ABI surfaces;
   - boundaries that remain in App/Surface/IO instead of terminal-core.

## Changes

1. Update `issues/0801-roastty-libghostty-rewrite/README.md`:
   - if verification supports it, check completed formatter and folded stream /
     ScreenSet sub-pieces and leave the remaining render-state work open;
   - otherwise refine the open item to name the specific missing behavior.
2. If the audit uncovers a small missing test that should be added immediately,
   update the relevant `roastty/src/terminal/*.rs` or `roastty/src/lib.rs` test
   module.
3. Update this experiment file with the result and review records.

## Verification

- `cargo test -p roastty terminal_formatter`
- `cargo test -p roastty screen_formatter`
- `cargo test -p roastty page_list::tests::page_string`
- `cargo test -p roastty page_list::tests::codepoint_map`
- `cargo test -p roastty page_list::tests::point_map`
- `cargo test -p roastty page_list::tests::pin_map`
- `cargo test -p roastty formatter_c_abi`
- `cargo test -p roastty terminal::page_list::tests::shape_run_options`
- `cargo test -p roastty terminal::terminal::tests::shape_run_options_threads_screen_state`
- `cargo test -p roastty render_state_c_abi`
- `cargo test -p roastty render_state_row_c_abi`
- `cargo test -p roastty render_state_row_cells_c_abi`
- `cargo test -p roastty terminal_stream`
- `cargo test -p roastty terminal_stream_alt_screen`
- `cargo test -p roastty tracked_grid_ref_returns_no_value_after_reset_and_alternate_recreate`
- compare/read audited Rust files against:
  - `vendor/ghostty/src/terminal/formatter.zig`
  - `vendor/ghostty/src/terminal/c/formatter.zig`
  - `vendor/ghostty/src/terminal/render.zig`
  - `vendor/ghostty/src/terminal/c/render.zig`
  - `vendor/ghostty/src/terminal/ScreenSet.zig`
  - `vendor/ghostty/src/terminal/stream_terminal.zig`
- `cargo fmt -p roastty` if Rust tests are added
- `cargo fmt -p roastty -- --check`
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/640-terminal-render-formatter-audit.md`
- `git diff --name-only` shows only issue docs unless the audit uncovers a small
  missing test
- `git diff --check`

Pass = the checklist accurately splits the mixed formatter/render/ScreenSet/
stream-terminal state, completed items are checked only with direct test
evidence, and remaining work is named precisely.

Fail = the audit relies on vague coverage, marks unverified render-state
behavior complete, or discovers a behavioral gap that needs a dedicated
implementation experiment before the checklist can be refined.

## Design Review

Codex design review session `019e9a9a-ee48-7ec2-bb17-ea152a97b42d` initially
requested revisions:

- replace the nonexistent `render_rows_snapshot` filter with real tests or make
  that path source-audit-only;
- add upstream `terminal/c/formatter.zig` and `terminal/c/render.zig` to the C
  ABI source comparison;
- broaden stream verification beyond the alternate-screen subset or narrow the
  stream claim;
- replace the broad `shape_run_options` filter with precise page-list and
  terminal filters.

The plan was revised to address those findings.

Follow-up review in the same session approved the design for the plan commit
with no blocking findings. The reviewer confirmed that the replacement render
verification matches real test prefixes, the upstream C surfaces are included,
and the scope avoids overclaiming full render-state parity.
