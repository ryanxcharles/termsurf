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

# Experiment 644: Tmux Output Format Helpers

## Description

Port the standalone tmux output-format helper layer from Ghostty into Roastty.

Experiments 642 and 643 added the tmux control and layout parsers. The next
isolated upstream tmux unit is `terminal/tmux/output.zig`, which formats tmux
`#{variable}` strings and parses command output fields back into typed values.
Upstream uses Zig comptime to synthesize structs from variable lists; Rust
cannot mirror that shape directly without macro machinery. This experiment
should port the same runtime behavior with explicit Rust value types.

This experiment must not build the viewer command queue, format concrete tmux
commands, wire DCS entry, write to the PTY, or integrate with App/Surface.

## Changes

1. Extend `roastty/src/terminal/tmux.rs` with:
   - `OutputVariable` covering the upstream `output.Variable` variants;
   - `OutputValue::{Bool(bool), Number(usize), Text(String)}`;
   - `OutputParseError::{MissingEntry, ExtraEntry, FormatError}`;
   - `OutputVariable::parse_value`;
   - `parse_output_values(vars, text, delimiter: u8)` returning one
     `OutputValue` per variable in order;
   - `format_output_variables(vars, delimiter: u8)` returning tmux `#{variable}`
     format strings.
2. Preserve upstream parsing behavior:
   - boolean variables are true only for exactly `1`;
   - numeric variables parse unsigned base-10 numbers;
   - `session_id`, `window_id`, and `pane_id` require `$`, `@`, and `%` prefixes
     respectively;
   - text variables preserve the input exactly, including empty strings;
   - `parse_output_values` preserves upstream `std.mem.splitScalar` behavior:
     split on a single byte delimiter, preserve empty fields, preserve trailing
     empty fields, and never use whitespace splitting or empty-field filtering;
   - `parse_output_values` reports `MissingEntry`, `ExtraEntry`, and collapses
     any per-field parse failure to `FormatError`;
   - direct `OutputVariable::parse_value` intentionally returns
     `OutputParseError::FormatError` for all parse failures in Rust, rather than
     exposing separate numeric parse errors like Zig's direct `Variable.parse`.
3. Use snake-case Rust enum variants but emit upstream tmux variable names in
   format strings.
4. Add focused tests mirroring upstream `output.zig` parse and format cases,
   including:
   - direct boolean, numeric, prefixed ID, and text variable parsing;
   - `parse_output_values([SessionId], "")` returning `FormatError`;
   - `parse_output_values([SessionId, WindowLayout], "$1,", b',')` preserving
     the empty layout field;
   - `parse_output_values([SessionId], "$1 ", b' ')` returning `ExtraEntry`;
   - missing, extra, bad-format, alternate delimiter, and empty-layout cases;
   - format output for zero, one, multiple, comma-delimited, tab-delimited, and
     representative all-variable lists.
5. Keep the README's overall `tmux` checklist item unchecked, refining it after
   the result to say control/layout/output helpers are done while DCS entry,
   viewer, PTY, and App integration remain missing.
6. Update this experiment file with result and review records.

## Verification

- `cargo test -p roastty terminal::tmux`
- `cargo fmt -p roastty`
- `cargo fmt -p roastty -- --check`
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/644-tmux-output-format.md`
- compare/read the Rust helpers against:
  - `vendor/ghostty/src/terminal/tmux.zig`
  - `vendor/ghostty/src/terminal/tmux/output.zig`
  - `vendor/ghostty/src/terminal/tmux/viewer.zig` `Format` and `Command` usage
    sites
- `git diff --check`

Pass = Roastty has tested standalone tmux output variable parsing and format
string construction matching upstream runtime behavior, while the README keeps
the overall tmux item open for viewer commands, DCS entry, PTY, and App/Surface
integration.

Fail = the Rust value model loses upstream parse semantics, format strings emit
wrong tmux variable names or delimiters, parse-format errors differ from
upstream, or the experiment overclaims wider tmux viewer behavior.

## Design Review

Initial Codex design review session `019e9a9a-ee48-7ec2-bb17-ea152a97b42d`
requested revisions:

- specify exact `std.mem.splitScalar` parity for empty fields and trailing
  delimiters;
- define the direct `OutputVariable::parse_value` error contract separately from
  aggregate `parse_output_values`;
- keep delimiters as single bytes;
- include explicit zero-variable formatting and split-edge tests;
- keep README wording limited to output helpers, not concrete viewer commands.

The plan was revised to address those findings.

Follow-up review in the same session approved the revised design for
implementation. The reviewer confirmed that split semantics, direct parse error
contract, single-byte delimiter scope, zero-variable formatting tests, and
README wording were all resolved.

## Result

**Result:** Pass

Roastty now has standalone tmux output-format helpers in
`roastty/src/terminal/tmux.rs`.

The helper layer ports the runtime behavior from
`vendor/ghostty/src/terminal/tmux/output.zig` while replacing Zig's comptime
generated structs with explicit Rust values:

- `OutputVariable` covers upstream tmux output variables;
- `OutputValue` stores parsed booleans, numbers, and text values;
- `OutputParseError` reports missing entries, extra entries, and format errors;
- `OutputVariable::parse_value` parses direct variable values and intentionally
  collapses direct Rust parse failures to `FormatError`;
- `parse_output_values` returns one ordered `OutputValue` per requested variable
  and preserves upstream `splitScalar` behavior for empty fields and trailing
  delimiters;
- `format_output_variables` emits tmux `#{variable}` format strings with a
  single-byte ASCII delimiter.

The implementation preserves the key upstream semantics: booleans are true only
for exactly `1`, unsigned numeric fields reject malformed input, `$` / `@` / `%`
prefixes are required for session/window/pane IDs, text variables preserve empty
strings, and aggregate parsing collapses per-field parse failures to
`FormatError`.

Verification passed:

- `cargo test -p roastty terminal::tmux` — 61 passed
- `cargo fmt -p roastty` — passed
- `cargo fmt -p roastty -- --check` — passed
- `prettier --write --prose-wrap always --print-width 80 issues/0801-roastty-libghostty-rewrite/README.md issues/0801-roastty-libghostty-rewrite/644-tmux-output-format.md`
  — passed
- `git diff --check` — passed

Source comparison was performed against:

- `vendor/ghostty/src/terminal/tmux.zig`
- `vendor/ghostty/src/terminal/tmux/output.zig`
- `vendor/ghostty/src/terminal/tmux/viewer.zig` `Format` and `Command` usage
  sites

Completion review in Codex session `019e9a9a-ee48-7ec2-bb17-ea152a97b42d`
approved the result with no blocking findings. The reviewer agreed that the
helper matches upstream `output.zig` runtime behavior within the approved Rust
contract and that scope stayed limited to output helpers. The review nits were
to complete the result-review provenance tag and document the ASCII delimiter
boundary for the `String`-based formatter before the result commit.

## Conclusion

The tmux output variable parsing and format-string helper layer is complete. The
overall terminal-core `tmux` checklist item remains open because concrete viewer
command formatting, DCS entry, viewer state, PTY read/write integration, and
App/Surface wiring are still missing.
