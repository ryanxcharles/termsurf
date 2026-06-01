# Experiment 132: Port SGR and Styled Printing

## Description

Port Select Graphic Rendition (`CSI ... m`) far enough that Roastty can parse
upstream Ghostty's SGR attributes, apply them to the active cursor style, and
write styled ASCII cells into page storage with correct style ref counts.

The style value types, style set storage, page style metadata, and formatters
already exist from earlier experiments. The missing runtime connection is:

1. CSI parsing must preserve semicolon-vs-colon separators for SGR.
2. `CSI ... m` must produce style-attribute actions.
3. Terminal execution must mutate the active screen cursor style.
4. Printing must store the active cursor style on each written cell.
5. Replacing styled cells must release old style references and maintain page
   integrity.

This experiment should be one coherent subsystem slice, not a parser-only pass.
It is acceptable to implement the SGR parser in a focused subset of files, but
success requires visible formatted output proving styled printing works end to
end.

Do not implement OSC hyperlinks, alternate screen, mouse modes, Kitty graphics,
Kitty keyboard changes, color palette mutation, public ABI, renderer APIs,
font/text shaping, non-ASCII styled graphemes, or non-macOS behavior here.

## Changes

1. Re-read upstream source of truth.
   - Use `vendor/ghostty/src/terminal/stream.zig` for `CSI ... m` routing.
   - Use `vendor/ghostty/src/terminal/sgr.zig` for attribute parsing.
   - Use `vendor/ghostty/src/terminal/Screen.zig::setAttribute` and
     `manualStyleUpdate` for cursor-style semantics.
   - Use existing Roastty `style::Style`, `style::Color`, `sgr::Underline`, page
     style storage, and formatter code rather than inventing a parallel style
     representation.
   - Do not modify `vendor/ghostty/`.

2. Extend CSI param storage to preserve separators.
   - Replace the current single `separator_seen` shape with enough metadata to
     distinguish:
     - no separator after a param;
     - semicolon separator after a param;
     - colon separator after a param.
   - Preserve existing semicolon behavior for current commands.
   - Continue rejecting colon-separated params for every existing non-SGR
     command family unless that command explicitly supports colon later.
   - Keep unsupported intermediate forms no-dispatch/no-leak as established by
     Experiment 131.
   - Keep raw C1 `0x9b` out of scope and preserve existing raw-UTF-8 behavior.
   - Add parser tests proving existing cursor, erase, mode, tab, scroll, and
     DECRQM commands do not begin accepting colon params accidentally.

3. Port SGR attribute parsing into `roastty/src/terminal/sgr.rs`.
   - Add an internal `Attribute` enum matching the relevant upstream variants:
     - unset;
     - unknown;
     - bold / reset bold;
     - faint;
     - italic / reset italic;
     - underline styles and reset underline;
     - direct RGB underline color, 256-color underline color, reset underline
       color;
     - overline / reset overline;
     - blink / reset blink;
     - inverse / reset inverse;
     - invisible / reset invisible;
     - strikethrough / reset strikethrough;
     - direct RGB foreground/background;
     - 8-color and bright 8-color foreground/background;
     - 256-color foreground/background;
     - reset foreground/background.
   - Add a no-allocation parser over finalized CSI params and separator
     metadata.
   - Match upstream behavior for:
     - empty `CSI m` and explicit `CSI 0 m` reset;
     - multi-attribute streams such as `CSI 1;3;31;44 m`;
     - semicolon direct color forms such as `CSI 38;2;1;2;3 m`;
     - semicolon indexed color forms such as `CSI 38;5;161 m`;
     - colon underline styles such as `CSI 4:3 m`;
     - colon RGB forms with optional color-space value, such as
       `CSI 38:2:1:2:3 m` and `CSI 38:2:0:1:2:3 m`;
     - direct underline RGB forms such as `CSI 58;2;1;2;3 m` and
       `CSI 58:2:0:1:2:3 m`;
     - unknown or incomplete attributes being consumed without panics and
       without leaking final `m`.
   - Preserve upstream's truncating `u16` to `u8` behavior for color components
     and 256-color indexes.
   - Explicitly pin empty and trailing separator behavior:
     - `CSI ; m` is equivalent to reset/unset;
     - `CSI 1 ; m` applies bold and then reset/unset if the finalized-param
       model represents the trailing empty param, or applies only bold if the
       current model does not represent trailing empty params. The result must
       document which behavior Roastty implements and why it matches the chosen
       parser representation;
     - repeated semicolon separators such as `CSI 1;;31m` do not panic and are
       tested against the implemented finalized-param behavior;
     - trailing colon malformed forms such as `CSI 58:4:m` are consumed as
       unknown/malformed without leaking final `m`;
     - the upstream Kakoune-style long SGR sequence with underline color as a
       late parameter is covered by a regression test.

4. Route `CSI ... m` from the stream parser.
   - Add `Action::SetAttribute { attr: sgr::Attribute }` or equivalent.
   - Dispatch one action per parsed SGR attribute, in order.
   - `CSI m` dispatches exactly one reset/unset attribute.
   - `CSI 1;31m` dispatches bold then red foreground.
   - Unsupported private/intermediate SGR forms, such as `CSI ? 1 m` or
     `CSI 1 $ m`, dispatch no action and do not leak final `m`.
   - SGR parser errors or unknown attributes do not mutate the terminal, but
     they also do not surface as stream errors.

5. Apply attributes to the active cursor style.
   - Add screen/terminal helpers equivalent to upstream `Screen.setAttribute`.
   - Mutate only the active cursor style when an SGR action executes.
   - Reset/unset restores the active cursor style to `style::Style::default()`.
   - SGR `22` clears both bold and faint.
   - SGR `21` maps to double underline, matching upstream.
   - SGR `24` clears underline but does not clear underline color.
   - SGR `39`, `49`, and `59` reset foreground, background, and underline color
     respectively.
   - Unknown attributes are ignored.
   - Applying SGR must not dirty rows, move the cursor, change pending wrap,
     change modes, append PTY responses, or change visible cells.

6. Write styled cells through page storage.
   - Add a style-aware active-cell write primitive. It must handle both
     non-default styled writes and default-style writes that replace a
     previously styled cell.
   - The existing basic write path may still be used only when the active cursor
     style is default and the target cell has no managed print state.
   - When the cursor style is non-default, write the printed cell with a
     page-owned style id, mark the row styled, and create exactly one style ref
     for that cell.
   - Be explicit about reference ownership: if `Page::add_style` already returns
     an id with one reference for the inserted style, do not call `use_style`
     again for the same cell. Tests must catch accidental double-ref behavior.
   - Replacement must be rollback-safe. Acquire/add the new style before
     mutating the destination cell or releasing the old style. Only after the
     new style id is available should the code replace the cell and release the
     old style ref. If any allocation/rehash step fails, the old cell and old
     style ref count must remain intact.
   - Map style-add failures into the existing private terminal stream error
     surface (`PageAlloc` unless a more specific current error already exists).
     Do not expose a public ABI error in this experiment.
   - If the target cell already has a style id, release the old style ref after
     the replacement succeeds.
   - If the target cell has other managed metadata that this experiment cannot
     correctly overwrite yet, return the current private
     `ManagedCellUnsupported` error rather than corrupting page state. Document
     exactly which managed metadata remains unsupported.
   - Ensure insert mode, pending wrap, wrap scroll, horizontal margins, and
     ordinary cursor advancement keep their existing behavior while using styled
     cell writes.
   - Ensure page integrity checks pass after styled print, styled overwrite,
     styled wrap, and styled scrollback movement.

7. Add tests.
   - Port or mirror upstream `sgr.zig` parser tests for:
     - reset/default;
     - all basic flags and resets;
     - underline styles;
     - 8-color, bright 8-color, 256-color, and RGB foreground/background;
     - underline color and reset;
     - semicolon and colon RGB forms;
     - invalid/incomplete forms;
     - multiple attributes in one sequence.
   - Add stream parser tests for:
     - `CSI m`;
     - `CSI 0 m`;
     - `CSI 1;3;31;44 m`;
     - `CSI 38;2;1;2;3 m`;
     - `CSI 38:2:0:1:2:3 m`;
     - `CSI 58;2;1;2;3 m`;
     - `CSI 58:2:0:1:2:3 m`;
     - `CSI 4:3 m`;
     - empty/trailing separator forms: `CSI ; m`, `CSI 1 ; m`, `CSI 1 ;; 31 m`,
       and `CSI 58:4: m`;
     - the upstream Kakoune-style underline-color sequence with at least 17
       parameters;
     - unsupported private/intermediate SGR forms;
     - non-SGR command families rejecting colon params;
     - split-feed SGR across semicolon, colon, and final `m`;
     - handler-error recovery returning the parser to ground.
   - Add terminal tests proving:
     - SGR changes active cursor style without mutating visible content;
     - printing after SGR creates styled cells visible in VT and HTML formatter
       output;
     - reset makes later printed cells default styled;
     - overwriting a styled cell with another styled cell releases the old style
       ref, creates exactly one new style ref, and preserves page integrity;
     - overwriting a styled cell with default style releases the old style ref,
       leaves the replacement cell at `style::DEFAULT_ID`, and preserves page
       integrity;
     - simulated style-add failure leaves the old styled cell and old ref count
       intact, if the current test harness can inject that failure; if not,
       record the missing failure-injection hook and cover the non-failure
       ordering with direct ref-count assertions;
     - styled printing works with insert mode, pending wrap, wrap scroll, and
       horizontal margins;
     - SGR does not append PTY responses;
     - full `cargo test -p roastty` still passes.

8. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::sgr
     cargo test -p roastty stream_csi_sgr
     cargo test -p roastty terminal_stream_sgr
     cargo test -p roastty terminal_formatter
     cargo test -p roastty terminal::page
     cargo test -p roastty terminal::page_list
     cargo test -p roastty terminal::terminal
     cargo test -p roastty stream
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

9. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Fix all real design findings before implementation.
   - Record the design-review outcome in this experiment file before
     implementation.
   - Commit the approved design before implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real result findings before proceeding.
   - Commit the approved result separately from the design commit.

10. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Include:
      - exact SGR parser behavior;
      - exact separator metadata behavior;
      - exact cursor-style behavior;
      - exact styled-cell storage/ref-count behavior;
      - any still-unsupported managed overwrite cases;
      - formatter evidence for styled output;
      - verification command output summary;
      - Codex design-review outcome;
      - Codex result-review outcome.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `CSI ... m` parses and dispatches upstream-equivalent SGR attributes;
- colon metadata is preserved for SGR without changing existing non-SGR command
  behavior;
- SGR execution mutates only the active cursor style;
- styled printing writes page-owned style ids with correct row metadata and ref
  counts;
- overwriting styled cells with styled or default replacements updates style
  refs without corrupting page state;
- style-add failure cannot leave a destination cell with a stale style id or an
  incorrect ref count;
- formatter output proves styled cells are visible in VT and HTML output;
- SGR reset makes subsequent cells default styled;
- existing print, insert, wrap, scroll, cursor, erase, mode, tab, DECRQM,
  formatter, page, page-list, and ABI behavior remains intact;
- no public ABI, renderer, palette-mutation, OSC, hyperlink, alternate-screen,
  mouse, Kitty graphics, Kitty keyboard, font/shaping, non-ASCII grapheme, or
  non-macOS behavior is added;
- `cargo fmt` and the listed tests pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- SGR parsing lands but styled cell storage exposes a page/ref-count primitive
  that must be split into a prerequisite experiment;
- styled printing works only for new empty cells, but overwriting styled cells
  needs a separate safe overwrite primitive;
- separator metadata support reveals a broader CSI parser redesign that should
  happen before SGR dispatch lands.

The experiment fails if:

- SGR sequences still dispatch no actions;
- style attributes parse but do not affect printed output;
- styled output corrupts page integrity or style ref counts;
- a style-add failure can corrupt or partially overwrite an existing styled
  cell;
- non-SGR command families start accepting colon params accidentally;
- malformed SGR leaks final `m` into visible text;
- the implementation adds unrelated public ABI or renderer behavior.

## Design Review

Codex reviewed the initial design and found five real issues:
`logs/codex-review/20260601-073013-878380-last-message.md`.

The design was updated to:

- require a style-aware write primitive for both non-default styled writes and
  default-style overwrites of previously styled cells;
- require rollback-safe style ref-count mutation ordering: acquire the new style
  before mutating the cell or releasing the old style;
- map style-add failures to the current private terminal stream error surface
  and forbid partial overwrite/ref-count corruption on failure;
- pin empty, repeated, and trailing SGR separator behavior with explicit tests;
- include direct RGB underline color forms in scope;
- warn that `Page::add_style` may already create the one needed style ref, so
  implementations must not double-count with an extra `use_style`.

Codex re-reviewed the updated design and found no blocking design issues:
`logs/codex-review/20260601-073346-701630-last-message.md`.

The design is approved for implementation.
