+++
[implementer]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 144: Port OSC 133 Semantic Prompts

## Description

Experiment 143 finished OSC 3008 context-signal parsing. The next coherent OSC
slice is OSC 133 semantic prompts.

Ghostty implements this in:

- `vendor/ghostty/src/terminal/osc/parsers/semantic_prompt.zig`
- `vendor/ghostty/src/terminal/Terminal.zig` around `semanticPrompt`
- `vendor/ghostty/src/terminal/Screen.zig` around `cursorSetSemanticContent`

Unlike OSC 3008, semantic prompts already have real runtime storage in Roastty:
page rows have `SemanticPrompt`, cells have `SemanticContent`, and `PageList`
already contains prompt iterators, semantic selection helpers, prompt-click
movement helpers, and semantic output selection helpers. This experiment should
therefore port both parser recognition and the first runtime marking behavior.

The goal is to make common shell-integration sequences mark prompt, input, and
output cells correctly as text is printed. Advanced consumers of that metadata
remain out of scope unless they are already implemented in `PageList`.

## Changes

1. Add a semantic-prompt terminal module.

   Add `roastty/src/terminal/semantic_prompt.rs` and register it from
   `roastty/src/terminal/mod.rs`.

   Include:
   - `SemanticPrompt<'a> { action, options }`;
   - `Action` values matching Ghostty:
     - `fresh_line` (`L`);
     - `fresh_line_new_prompt` (`A`);
     - `new_command` (`N`);
     - `prompt_start` (`P`);
     - `end_prompt_start_input` (`B`);
     - `end_prompt_start_input_terminate_eol` (`I`);
     - `end_input_start_output` (`C`);
     - `end_command` (`D`);
   - `Click::{Line, Multiple, ConservativeVertical, SmartVertical}`;
   - `PromptKind::{Initial, Right, Continuation, Secondary}`;
   - `Redraw::{True, False, Last}`;
   - lazy option readers for `aid`, `cl`, `k`, `err`, `redraw`, `special_key`,
     `click_events`, `cmdline`, `cmdline_url`, and the special `exit_code`
     field.

   Match Ghostty's option semantics:
   - options are raw bytes after the action separator;
   - options are semicolon-separated;
   - unknown options and bare fields without `=` are skipped;
   - the first matching key wins, even if its value is malformed;
   - key matching is exact and case-sensitive;
   - `aid`, `err`, `cmdline`, and `cmdline_url` return raw bytes and may be
     empty;
   - `cl`, `k`, `redraw`, `special_key`, and `click_events` return `None` for
     unknown or malformed values;
   - `exit_code` parses the first option field as decimal `i32`, including
     normal negative values, and returns `None` on empty, malformed, negative
     overflow, or positive overflow.

   Do not implement command-line decoding in this experiment. Ghostty's
   `writeCommandLine` depends on shell-style and URL decoding helpers; parser
   parity can expose raw `cmdline` and `cmdline_url` bytes first, and decoding
   can be a later coherent string-decoding slice.

2. Parse OSC 133.

   In `roastty/src/terminal/osc.rs`, add:

   ```rust
   Command::SemanticPrompt { value: semantic_prompt::SemanticPrompt<'a> }
   ```

   Parse after `133;`:
   - `L` is valid only with no additional bytes;
   - `A`, `B`, `I`, `C`, `D`, `N`, and `P` accept either a single action byte or
     `action;options`;
   - `Aextra`, `Bextra`, `Iextra`, `Cextra`, `Dextra`, `Nextra`, and `Pextra`
     reject;
   - missing `133;`, empty data, unknown actions, or malformed action tails
     reject.

   Keep OSC 133 on the fixed-buffer capture path. It must not become part of the
   growable OSC families.

3. Dispatch OSC 133 through the stream layer.

   Extend `OscAction` and the stream test harness so valid OSC 133 actions reach
   the handler.

   Add stream tests for:
   - each valid action;
   - action options preserved as raw bytes;
   - invalid actions consumed without dispatch or print leakage;
   - oversized OSC 133 consumed without dispatch or print leakage.

4. Add semantic cursor state to `Screen`.

   Extend `ScreenCursor` with:
   - current semantic content, default `Output`;
   - whether input semantic content should clear to end-of-line.

   Add a `Screen` helper equivalent to Ghostty's `cursorSetSemanticContent`:
   - `Output` sets the cursor semantic content to output and disables
     clear-to-end-of-line behavior;
   - `Input` sets the cursor semantic content to input and records explicit vs
     end-of-line termination;
   - `Prompt(kind)` sets the cursor semantic content to prompt and marks the
     active row as `SemanticPrompt::Prompt` for `Initial`/`Right` and
     `SemanticPrompt::PromptContinuation` for `Continuation`/`Secondary`.

   Printing must write the cursor semantic content into the target cell. Do not
   mark untouched blank cells to the right of the cursor. Ghostty's
   clear-to-end-of-line model is cursor-state behavior, not bulk cell marking:
   `I` sets input semantic content with a clear-EOL flag, and the next explicit
   newline resets cursor semantic content to output instead of marking the next
   row as a prompt continuation.

   Linefeed and soft-wrap behavior must match Ghostty:
   - on explicit newline/linefeed, if `semantic_content_clear_eol` is set, reset
     cursor semantic content to output;
   - otherwise, if cursor semantic content is prompt or input, mark the new row
     as `SemanticPrompt::PromptContinuation`;
   - on pending-wrap soft wrap, if cursor semantic content is prompt or input,
     mark the new row as `SemanticPrompt::PromptContinuation`.

5. Add terminal runtime handling for OSC 133.

   In `TerminalStreamHandler::osc`, handle semantic prompts:
   - `L`: perform Ghostty's fresh-line behavior: if the cursor is already at the
     applicable left margin, do nothing; otherwise carriage-return and index.
     Honor the current left margin the same way Ghostty does.
   - `A`: perform `L`, then set cursor semantic content to prompt with
     `k`/`PromptKind` defaulting to `Initial`.
   - `N`: treat as `A` using the same options. Do not implement nested command
     tracking.
   - `P`: set prompt semantic content with `k` defaulting to `Initial`.
   - `B`: set input semantic content with explicit termination.
   - `I`: set input semantic content with end-of-line termination.
   - `C`: set output semantic content. If the current row is marked as a prompt
     and the cursor is at column zero, clear that row prompt mark to match
     Ghostty's fish-shell heuristic.
   - `D`: set output semantic content.

   Do not add public C ABI, app/surface event delivery, prompt-click event
   emission, prompt-redraw-on-resize behavior, redraw state storage,
   command-line decoding, command tracking, or UI highlighting behavior in this
   experiment.

6. Keep existing semantic helper behavior intact.

   `PageList` already has semantic prompt iterators, semantic selection, prompt
   click movement, and output selection helpers. Do not refactor those helpers
   in this experiment except for the minimum test accessors needed to verify
   runtime cell/row markings.

## Verification

Run formatting and tests:

```bash
cargo fmt
cargo test -p roastty semantic_prompt
cargo test -p roastty osc
cargo test -p roastty terminal_stream_osc133
cargo test -p roastty page_list_prompt
cargo test -p roastty
```

Add parser/option tests:

- all actions parse from their exact action bytes;
- `L` rejects any extra bytes;
- the other actions reject extra bytes unless the second byte is `;`;
- raw options are preserved exactly, including non-UTF-8 bytes;
- `aid`, `err`, `cmdline`, and `cmdline_url` return raw bytes, including empty
  values;
- `cl` accepts `line`, `m`, `v`, and `w` only;
- `k` accepts `i`, `r`, `c`, and `s` only;
- `redraw` accepts `0`, `1`, and `last` only;
- `special_key` and `click_events` accept one-byte `0`/`1` only;
- `exit_code` parses the first option field as decimal `i32`, including normal
  negative values;
- duplicate matching keys stop at the first matching key, even if malformed;
- missing `133;`, empty data, unknown actions, malformed tails, and oversized
  payloads reject without dispatch.

Add stream tests:

- valid OSC 133 actions dispatch;
- valid options are preserved as raw bytes;
- invalid OSC 133 forms do not dispatch and do not leak bytes into printed
  output;
- oversized OSC 133 does not dispatch or leak bytes.

Add terminal runtime tests:

- `OSC 133;L` fresh-lines only when the cursor is not already at the applicable
  left margin;
- fresh-line tests cover cursor at column zero, at the current left margin, and
  left of the current left margin but not zero, matching Ghostty's left-margin
  calculation;
- `OSC 133;A` fresh-lines, marks the row as a prompt, and marks subsequent
  printed cells as prompt semantic content;
- `OSC 133;N` behaves like `A`;
- `OSC 133;P;k=c` and `OSC 133;P;k=s` mark prompt-continuation rows;
- `OSC 133;B` marks subsequent printed cells as input;
- `OSC 133;I` marks subsequent printed cells as input, does not bulk-mark
  untouched blank cells, and resets cursor semantic content to output after the
  next explicit newline instead of marking the next row as a prompt
  continuation;
- `OSC 133;C` marks subsequent printed cells as output and clears a prompt row
  mark when emitted at column zero;
- `OSC 133;D` restores output semantic content;
- prompt/input semantic content marks soft-wrapped rows as prompt continuations;
- prompt/input semantic content marks explicit-newline rows as prompt
  continuations, except when the cursor is in `I` clear-EOL input mode;
- normal printing without OSC 133 remains output;
- semantic markers preserve existing text, style, hyperlinks, title, PWD, color
  state, modes, and PTY responses except for the intended fresh-line cursor/row
  effects.

Add regression tests for existing `PageList` semantic helpers when possible:

- prompt iterator sees rows marked by runtime OSC 133;
- semantic selection boundaries can observe runtime-marked prompt/input/output
  cells without manual test-only setup.

## Pass Criteria

- OSC 133 parser behavior matches Ghostty's accepted/rejected action forms and
  lazy option-reader semantics.
- OSC 133 remains fixed-buffered and oversized forms reject without dispatch or
  print leakage.
- Stream dispatch delivers semantic prompt actions to the terminal runtime.
- Terminal runtime writes prompt/input/output semantic content and prompt row
  markers matching Ghostty's core behavior for `L`, `A`, `N`, `P`, `B`, `I`,
  `C`, and `D`.
- Existing `PageList` semantic prompt helpers can consume runtime-produced
  metadata.
- Existing OSC behavior and the full `roastty` suite continue to pass.
- No public ABI, app/surface delivery, prompt-click event emission,
  prompt-redraw-on-resize, redraw state storage, command tracking, command-line
  decoding, or UI highlighting behavior is added.

## Failure Criteria

- Parser accepts malformed action tails such as `Cextra` or `L;aid=x`.
- Parser requires UTF-8 for options or string option readers.
- Option readers skip past a malformed duplicate matching key to a later valid
  duplicate.
- Numeric option parsing accepts malformed values or overflows.
- Invalid or oversized OSC 133 forms dispatch actions or leak bytes into display
  output.
- `I` clear-EOL behavior bulk-marks untouched blank cells instead of resetting
  cursor semantic content to output on explicit newline.
- Prompt/input linefeeds or soft wraps fail to mark continuation rows, or
  clear-EOL input linefeeds incorrectly mark continuation rows.
- Runtime semantic marking changes unrelated terminal state.
- Runtime prompt/input/output markings are only test-only state and are not
  written into actual row/cell metadata.
- The experiment refactors `PageList` semantic helpers unrelated to the runtime
  integration.
- The experiment adds public ABI, app/surface events, prompt-click event
  emission, prompt-redraw-on-resize, redraw state storage, command tracking,
  command-line decoding, or UI highlighting.

## Design Review

Codex reviewed the initial design and found real issues:

- `I` clear-EOL behavior was incorrectly described as blank-cell marking instead
  of cursor semantic state that resets to output on explicit newline;
- prompt/input linefeed and soft-wrap continuation marking was underspecified;
- fresh-line left-margin verification needed the Ghostty edge case where the
  cursor is left of the configured margin but not at column zero;
- runtime `redraw` storage was vague and should stay out of scope while resize
  behavior is out of scope;
- `exit_code` verification needed normal negative `i32` values, not digits-only
  parsing.

The design was updated to pin clear-EOL cursor-state behavior, continuation-row
marking on linefeed and soft wrap, the fresh-line margin edge cases, parser-only
`redraw` support, and signed `i32` exit-code parsing. Codex re-reviewed the
revised design and approved it for implementation with no blocking findings.

## Result

**Result:** Pass

Implemented OSC 133 semantic prompts across parser, stream dispatch, page/cell
metadata writes, and terminal runtime behavior.

The implementation adds `roastty/src/terminal/semantic_prompt.rs` with
Ghostty-compatible action values and lazy option readers for `aid`, `cl`, `k`,
`err`, `redraw`, `special_key`, `click_events`, `cmdline`, `cmdline_url`, and
`exit_code`. Option readers preserve raw bytes for string options, match keys
case-sensitively, stop at the first matching key even when malformed, parse
strict enum/bool values, and parse signed decimal `i32` exit codes.

`roastty/src/terminal/osc.rs` now parses fixed-buffer OSC 133 actions and
rejects malformed tails such as `Aextra`, missing separators, empty data,
unknown actions, and oversized payloads. `roastty/src/terminal/stream.rs`
dispatches every OSC 133 action through the stream layer and consumes invalid
forms without print leakage.

Runtime handling now stores semantic cursor state in `Screen`, writes
prompt/input/output semantic content into real page cells during printing, and
writes prompt row markers into real `PageList` row metadata. `A` and `N`
fresh-line into prompt state, `P` marks prompt kinds, `B` and `I` mark input,
`C` and `D` restore output, `I` resets semantic state on explicit newline
without bulk-marking untouched blank cells, and prompt/input linefeeds and soft
wraps mark prompt-continuation rows.

Verification passed:

```bash
cargo fmt -- roastty/src/terminal/semantic_prompt.rs roastty/src/terminal/mod.rs roastty/src/terminal/osc.rs roastty/src/terminal/stream.rs roastty/src/terminal/page.rs roastty/src/terminal/page_list.rs roastty/src/terminal/screen.rs roastty/src/terminal/terminal.rs
cargo test -p roastty terminal_stream_osc133
cargo test -p roastty semantic_prompt
cargo test -p roastty osc
cargo test -p roastty page_list_prompt
cargo test -p roastty
```

Observed results:

- `cargo test -p roastty terminal_stream_osc133`: 7 passed
- `cargo test -p roastty semantic_prompt`: 17 passed
- `cargo test -p roastty osc`: 94 passed
- `cargo test -p roastty page_list_prompt`: 12 passed
- `cargo test -p roastty`: 1570 unit tests and 1 ABI harness test passed

## Result Review

Codex reviewed the completed implementation and found no blocking issues. It
approved the result as good enough to record as **Pass**.

The first result review noted two non-blocking test-polish items:

- stream dispatch coverage should include every OSC 133 action;
- terminal runtime should have a dedicated `OSC 133;D` assertion.

Both tests were added. After rerunning formatting and tests, Codex re-reviewed
the updated snippets and confirmed the polish items were resolved with no new
blocking issues.

## Conclusion

Roastty now recognizes and executes the core OSC 133 semantic prompt protocol
against real row/cell metadata. This connects the parser and runtime to the
semantic infrastructure already ported in `PageList`, so prompt iterators,
semantic selection helpers, and output-selection helpers can now observe
runtime-produced prompt/input/output markings instead of only manually seeded
test data.

Advanced behavior remains out of scope for later experiments: prompt-redraw on
resize, shell command-line decoding, explicit command tracking, prompt-click
event emission, app/surface delivery, public ABI, and UI highlighting.
