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

# Experiment 100: Port Terminal Tabstops Formatter Extra

## Description

Port the terminal formatter `tabstops` extra.

Experiment 99 completed the first post-screen terminal formatter extra:
scrolling-region state. Upstream Ghostty's next post-screen extra is tabstops.
Roastty already has an upstream-equivalent private `Tabstops` bitset module, but
`Terminal` does not yet own tabstop state and `TerminalFormatter` cannot yet
serialize that state.

This experiment attaches private tabstop state to `Terminal` and emits it
through the opt-in formatter extra only. It must not add VT parser support for
HTS/TBC, runtime tab key movement, resize behavior, public API, public ABI,
input behavior, render behavior, app behavior, or UI behavior.

This is Experiment 100. Earlier issue-process examples used two-digit experiment
filenames through `99`; this issue has exceeded that count, so this experiment
uses a three-digit filename while preserving the same one-file,
linked-from-README process.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for:
     - `TerminalFormatter.Extra.tabstops`;
     - `CSI 3 g` clear-all output;
     - cursor column positioning output;
     - HTS output;
     - post-screen ordering;
     - post-screen terminal extra pin-map behavior.
   - Use `vendor/ghostty/src/terminal/Tabstops.zig` and existing
     `roastty/src/terminal/tabstops.rs` for:
     - 0-indexed tabstop storage;
     - default interval behavior;
     - resize/capacity behavior already ported in earlier experiments.
   - Do not modify `vendor/ghostty/`.

2. Add private terminal tabstop state.
   - Add `tabstops: tabstops::Tabstops` to `Terminal`.
   - Initialize it with the terminal column count and upstream default
     interval 8.
   - Convert `TabstopError` to `PageListAllocError` or otherwise add a narrow
     private init path that preserves existing
     `Terminal::init(...) -> Result<Self, PageListAllocError>` without widening
     the public test-facing signature.
   - Add `#[cfg(test)] pub(super)` helpers on `Terminal` to:
     - clear all tabstops;
     - set a specific column;
     - clear a specific column deterministically;
     - inspect a specific column.
   - Existing `Tabstops::unset()` matches upstream and toggles the bit with XOR.
     Do not expose that ambiguity through formatter test helpers. The terminal
     helper should check `get()` before toggling so "clear column" is
     deterministic.
   - Keep the state private. Do not expose public API or ABI.

3. Extend `TerminalFormatterExtra`.
   - Add `tabstops: bool`.
   - Extend `none()`.
   - Add a `.tabstops(bool)` builder.
   - Keep `TerminalFormatter::init()` defaulting to no extras.

4. Emit tabstops after screen content.
   - Only VT output emits tabstop bytes.
   - Plain and HTML ignore the tabstops extra.
   - Emit `CSI 3 g` first to clear all existing tabstops:

     ```text
     \x1b[3g
     ```

   - For each configured tabstop column in ascending 0-indexed column order,
     emit:

     ```text
     \x1b[{column + 1}G\x1bH
     ```

     where `CSI {column + 1} G` moves to the 1-indexed column and `ESC H` is
     HTS.

   - Preserve upstream ordering:
     `palette -> modes -> content -> screen extras -> scrolling region -> tabstops`.
   - If no tabstops are configured, still emit `CSI 3 g` when the tabstops extra
     is enabled, matching upstream's explicit clear-all behavior.

5. Preserve post-screen pin-map semantics.
   - Tabstop bytes are generated terminal-state bytes appended after screen
     formatter output and any prior post-screen terminal extras.
   - Map appended tabstop bytes to the last existing pin when output already has
     content, screen extras, palette bytes, mode bytes, or scrolling-region
     bytes.
   - If the formatter emits only tabstop bytes, map them to active screen
     top-left.
   - Pin maps must remain byte-indexed.

6. Add upstream-equivalent tests.
   - Add TerminalFormatter tests for:
     - default output does not emit tabstop bytes;
     - default pin maps remain unchanged when the stored tabstop state is
       non-default but `TerminalFormatterExtra::none()` is used;
     - the default interval-8 tabstops emit `CSI 3 g` followed by ascending
       `CSI {column + 1} G` + `HTS` sequences;
     - custom tabstops at columns 4, 14, and 29 emit as columns 5, 15, and 30;
     - no configured tabstops emits only `CSI 3 g`;
     - tabstop bytes emit after content;
     - tabstop bytes emit after forwarded screen extras;
     - tabstop bytes emit after scrolling-region bytes when both extras are
       enabled;
     - palette, modes, content, screen extras, scrolling region, and tabstops
       combine with ordering
       `palette -> modes -> content -> screen extras -> scrolling region -> tabstops`;
     - plain and HTML ignore the extra;
     - `Content::None` can emit only tabstop bytes for VT;
     - pin maps are byte-indexed;
     - post-screen tabstop bytes map to the last existing pin when one exists;
     - content plus both `scrolling_region` and `tabstops` enabled in
       `format_with_pin_map()` keeps the combined post-screen suffix
       byte-indexed and maps all scrolling-region and tabstop suffix bytes to
       the final pre-suffix pin;
     - post-screen tabstop bytes map to top-left when no prior bytes exist.
   - Keep existing tabstops, modes, TerminalFormatter, ScreenFormatter, PageList
     formatter, and PageList tests passing.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty tabstops
     cargo test -p roastty terminal_formatter
     cargo test -p roastty modes
     cargo test -p roastty screen_formatter
     cargo test -p roastty styled_pin_map
     cargo test -p roastty pin_map
     cargo test -p roastty page_string
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix all real design findings before implementation.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real result findings before proceeding.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - names and visibility of terminal tabstop state;
     - initialization behavior and default interval;
     - exact clear-all and HTS sequence shapes;
     - plain/HTML no-op behavior;
     - ordering relative to palette, modes, content, forwarded screen extras,
       and scrolling region;
     - pin-map behavior for post-screen generated bytes;
     - why parser/runtime mutation, resize behavior, public API, and ABI remain
       deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `Terminal` owns private tabstop state initialized with upstream interval-8
  defaults;
- `TerminalFormatterExtra` has an opt-in tabstops flag;
- default TerminalFormatter output and pin maps remain unchanged;
- VT tabstop output emits `CSI 3 g`, then configured tabstops in ascending
  column order with 1-indexed `CSI {column + 1} G` and `ESC H`;
- empty tabstop state emits only `CSI 3 g` when the extra is enabled;
- tabstop bytes emit after screen content, forwarded screen extras, and
  scrolling-region bytes;
- palette, modes, content, screen extras, scrolling region, and tabstops can
  combine with ordering
  `palette -> modes -> content -> screen extras -> scrolling region -> tabstops`;
- plain and HTML output ignore the tabstops extra;
- generated tabstop bytes are byte-indexed in pin maps and map to the last
  existing pin, or top-left when there is no prior output;
- no VT parser/runtime tabstop mutation, tab-key movement behavior, resize
  behavior, public API, public ABI, app behavior, renderer behavior, PTY
  behavior, clipboard behavior, or UI behavior is added;
- `cargo fmt`, targeted formatter tests, tabstops tests, modes tests, PageList
  formatter tests, PageList tests, and full `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- terminal tabstop serialization cannot be represented honestly without first
  changing the `Tabstops` port or terminal sizing model, and that prerequisite
  is identified precisely.

The experiment fails if:

- default TerminalFormatter output changes;
- tabstop bytes emit without explicit `TerminalFormatter::with_extra()`;
- HTML or plain output emits tabstop bytes;
- tabstops emit before content, screen extras, or scrolling-region bytes;
- tabstop output skips `CSI 3 g`;
- configured tabstops are emitted out of ascending order or with zero-indexed
  column values;
- generated tabstop pin maps become character-indexed, shorter than output
  bytes, or map to top-left when prior content pins exist;
- runtime parser, public API, ABI, tab-key movement, resize, render, or UI
  behavior is added.

## Design Review

Codex reviewed this design before implementation.

Review artifacts:

- Prompt: `logs/codex-review/20260601-001550-548589-prompt.md`
- Result: `logs/codex-review/20260601-001550-548589-last-message.md`

Codex approved the three-digit Experiment 100 filename and overall scope, with
two required design fixes:

- clarify that the terminal test helper must deterministically clear a tabstop,
  because upstream-compatible `Tabstops::unset()` toggles rather than
  idempotently clearing;
- add an explicit `format_with_pin_map()` test where content, scrolling region,
  and tabstops all emit, proving the combined post-screen suffix remains
  byte-indexed and maps to the final pre-suffix pin.

Both findings were applied before implementation.

## Result

**Result:** Pass

Implemented private terminal tabstop state and the opt-in terminal formatter
tabstops extra.

Code changes:

- `Terminal` now owns private `tabstops: tabstops::Tabstops` state.
- Tabstops initialize from the terminal column count with upstream default
  interval 8.
- `TabstopError::OutOfMemory` is converted to `PageListAllocError::PageAlloc`
  inside `Terminal::init`, preserving the existing `Terminal::init` result type.
- Test-only terminal helpers can clear all tabstops, set a tabstop,
  deterministically clear a tabstop, and inspect a tabstop.
- The deterministic clear helper checks `get()` before calling the
  upstream-compatible toggling `Tabstops::unset()`.
- `TerminalFormatterExtra` now has an opt-in `tabstops: bool` flag and
  `.tabstops(bool)` builder.

Formatter behavior:

- Default `TerminalFormatter::init()` still uses
  `TerminalFormatterExtra::none()` and emits no tabstop bytes without explicit
  opt-in.
- VT output emits `CSI 3 g` first:

  ```text
  \x1b[3g
  ```

- For each configured tabstop in ascending 0-indexed column order, VT output
  emits:

  ```text
  \x1b[{column + 1}G\x1bH
  ```

  The cursor-positioning column is 1-indexed, and `ESC H` is HTS.

- An empty tabstop state still emits `CSI 3 g`, matching upstream's explicit
  clear-all behavior.
- Plain and HTML output ignore the tabstops extra.
- When combined with palette, modes, screen extras, and scrolling region, VT
  ordering is
  `palette -> modes -> content -> screen extras -> scrolling region -> tabstops`.

Pin-map behavior:

- Generated tabstop bytes are byte-indexed.
- Tabstop bytes are appended after screen formatter output and any earlier
  post-screen terminal suffixes.
- Appended tabstop bytes map to the last existing pin when prior output exists.
- If the formatter emits only tabstop bytes, they map to active-screen top-left.
- Tests cover content plus both scrolling region and tabstops, proving the
  combined post-screen suffix remains byte-indexed and maps to the final
  pre-suffix content pin.

Deferred by design:

- VT parser/runtime HTS/TBC mutation.
- Runtime tab key movement behavior.
- Resize behavior.
- Public API and public ABI.
- App behavior, renderer behavior, PTY behavior, clipboard behavior, and UI
  behavior.

Verification run:

```text
cargo fmt
cargo test -p roastty tabstops
cargo test -p roastty terminal_formatter
cargo test -p roastty modes
cargo test -p roastty screen_formatter
cargo test -p roastty styled_pin_map
cargo test -p roastty pin_map
cargo test -p roastty page_string
cargo test -p roastty terminal::page_list
cargo test -p roastty
```

Results:

- `tabstops`: 17 passed.
- `terminal_formatter`: 58 passed.
- `modes`: 20 passed.
- `screen_formatter`: 55 passed.
- `styled_pin_map`: 9 passed.
- `pin_map`: 62 passed.
- `page_string`: 12 passed.
- `terminal::page_list`: 524 passed.
- full `cargo test -p roastty`: 948 unit tests passed, ABI harness passed, doc
  tests passed.

Codex reviewed the completed result before commit.

Result review artifacts:

- Prompt: `logs/codex-review/20260601-002206-731890-prompt.md`
- Result: `logs/codex-review/20260601-002206-731890-last-message.md`

Codex found no required changes. It confirmed the upstream-equivalent `CSI 3 g`
and `HTS` sequence shapes, private tabstop state initialization, suffix
ordering, pin-map coverage, default behavior preservation, plain/HTML no-op
behavior, and result language.

## Conclusion

Roastty can now serialize stored tabstop state through the terminal formatter,
preserving upstream clear-all and HTS sequence behavior while keeping runtime
mutation and public surfaces deferred. The terminal formatter now covers
palette, modes, screen extras, scrolling region, and tabstops in the same
relative order as upstream.
