# Experiment 89: Port Screen Formatter Content

## Description

Port the content-routing portion of upstream
`terminal/formatter.zig::ScreenFormatter` into Roastty.

Experiments 81-88 completed the private PageList formatter surface for plain,
VT, HTML, codepoint maps, point maps, and pin maps. Upstream's next formatter
layer is `ScreenFormatter`: it owns a `Screen`, chooses whether to emit no
content or selected/full screen content, delegates the content bytes to
`PageListFormatter`, and then optionally emits VT-only screen extras such as
cursor position, style state, hyperlink state, protection, Kitty keyboard state,
and charset state.

Roastty does not have a `Screen` type yet, so this experiment should add only
the minimal private `Screen` shell needed to host the already-ported PageList
formatter content path. It should not implement cursor state, style extras,
hyperlink extras, Kitty keyboard extras, charset extras, parser behavior, or a
Terminal wrapper.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for:
     - `ScreenFormatter`;
     - `ScreenFormatter.Content`;
     - `ScreenFormatter.Extra`;
     - content delegation to `PageListFormatter`;
     - screen formatter pin-map tests.
   - Use the completed Roastty PageList formatter code in
     `roastty/src/terminal/page_list.rs`:
     - `PageStringOptions`;
     - `PageOutputFormat`;
     - `page_string(...)`;
     - `page_string_with_pin_map(...)`;
     - selection, dump-string, point-map, and pin-map tests from Experiments
       79-88.
   - Do not modify `vendor/ghostty/`.

2. Add a minimal private `Screen` module.
   - Add `roastty/src/terminal/screen.rs`.
   - Wire it from `roastty/src/terminal/mod.rs`.
   - Add a private `pub(super) struct Screen` whose only required state for this
     experiment is:

     ```rust
     pages: PageList
     ```

   - Add an initializer that mirrors the current PageList test setup enough for
     formatter tests, for example:

     ```rust
     impl Screen {
         fn init(cols: CellCountInt, rows: CellCountInt, max_scrollback_rows: Option<usize>)
             -> Result<Self, PageListAllocError>
     }
     ```

   - Keep this type private to the terminal module. Do not expose it through the
     C ABI or crate public API.
   - Do not add cursor, parser, modes, charsets, palette, terminal, PTY, app, or
     renderer state in this experiment.

3. Add only the narrow PageList visibility needed by `screen.rs`.
   - Because `screen.rs` will be a sibling module of `page_list.rs`, it cannot
     call private PageList constructors, formatter types, or formatter methods
     unless those items are visible at the `terminal` module boundary.
   - Prefer the narrowest `pub(super)` surface or `pub(super)` wrapper methods
     needed for ScreenFormatter delegation. Expected candidates are:
     - `PageList::init(...)`;
     - `PageOutputFormat`;
     - `PageStringOptions`;
     - `PageStringWithPinMap`;
     - `PageList::page_string(...)`;
     - `PageList::page_string_with_pin_map(...)`.
   - If tests in `screen.rs` need to populate screen contents, add a
     `#[cfg(test)] pub(super)` PageList helper or a `#[cfg(test)]` Screen helper
     rather than making PageList internals broadly visible.
   - These visibility changes must remain internal to `terminal`; do not expose
     the formatter path through the crate public API or C ABI.

4. Add the private ScreenFormatter content path.
   - Add private formatter types in `screen.rs` or a new formatter module if the
     implementation would otherwise make `page_list.rs` harder to read.
   - The formatter should model upstream's content split:

     ```rust
     enum ScreenFormatterContent {
         None,
         Selection(Option<selection::Selection>),
     }
     ```

   - Default content should be `Selection(None)`, matching upstream "format the
     full active screen" behavior.
   - `None` should emit no content and produce an empty pin map.
   - `Selection(Some(...))` should emit only the selected content.
   - `Selection(None)` should emit the full screen-domain content currently
     produced by the PageList formatter.
   - For this experiment, all `ScreenFormatterExtra` flags should exist only if
     that makes the shape clearer, and every flag must be false/no-op. VT-only
     extras are deferred.

5. Delegate to the PageList formatter instead of duplicating traversal.
   - ScreenFormatter content output must call through the completed PageList
     formatter helpers.
   - Plain, VT, and HTML output should match the equivalent PageList formatter
     output exactly for the same content selection.
   - Pin maps should be byte-indexed and should be the PageList formatter pin
     map for the delegated content.
   - Do not add ScreenFormatter point maps unless implementation proves they are
     necessary. Upstream ScreenFormatter exposes pin maps, while point maps
     remain a lower Page formatter detail.

6. Preserve scope boundaries.
   - Do not add `TerminalFormatter`, `Terminal`, parser state, cursor state,
     mode state, palette extra emission, scrolling-region emission, tabstop
     extra emission, PWD emission, keyboard extra emission, screen cursor extra
     emission, style extra emission, hyperlink extra emission, protection extra
     emission, charset extra emission, public ABI, app behavior, renderer
     behavior, PTY behavior, clipboard behavior, or UI behavior.
   - Do not rename or expose any `ghostty_*` symbols.
   - Do not move existing PageList formatter behavior unless a small private
     helper extraction is required for clean delegation.

7. Add upstream-equivalent tests.
   - Add ScreenFormatter tests for:
     - plain full-screen single-line output;
     - plain full-screen multiline output;
     - plain selected-line output;
     - `Content::None` emitting empty output and an empty pin map;
     - VT content delegation matching PageList VT output;
     - HTML content delegation matching PageList HTML output;
     - plain pin-map single-line output;
     - plain pin-map multiline output;
     - selected plain pin-map output;
     - VT and HTML pin-map output preserving byte-indexed maps;
     - invalid or garbage selection endpoints returning empty output/map via
       PageList delegation.
   - Tests may use private helpers to populate `Screen.pages`, but those helpers
     must stay in tests.
   - Keep existing PageList formatter tests unchanged and passing.

8. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty screen_formatter
     cargo test -p roastty styled_pin_map
     cargo test -p roastty pin_map
     cargo test -p roastty page_string
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

9. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real findings before proceeding.

10. Record the result.

- Append `## Result` and `## Conclusion` to this file.
- Include:
  - new file/type names and their visibility;
  - how ScreenFormatter content delegates to PageList;
  - how full-screen, selected, and no-content modes behave;
  - whether returned pin maps remain byte-indexed;
  - which upstream ScreenFormatter extras remain deferred and why;
  - verification command output summary;
  - Codex design-review outcome;
  - Codex result-review outcome.
- Update the Issue 801 README experiment index from `Designed` to `Pass`,
  `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has a private minimal `Screen` type that owns a `PageList`;
- Roastty has a private ScreenFormatter content path with upstream-shaped `None`
  and `Selection(Option<Selection>)` content modes;
- default ScreenFormatter content formats the full screen;
- selected content delegates to the PageList formatter and matches PageList
  output;
- no-content mode emits empty output and an empty pin map;
- plain, VT, and HTML content output match the equivalent PageList formatter
  output;
- pin maps are byte-indexed and match PageList pin-map delegation;
- invalid or garbage selection endpoints return empty output/map;
- existing PageList formatter behavior remains unchanged;
- no `TerminalFormatter`, `Terminal`, parser state, cursor state, mode state,
  palette extra emission, VT screen extras, public ABI, app, renderer, PTY,
  clipboard, or UI behavior is added;
- `cargo fmt`, targeted ScreenFormatter tests, formatter regression tests,
  PageList tests, and full `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- a minimal `Screen` shell proves insufficient and a small cursor or
  screen-state type must be introduced before ScreenFormatter can be faithfully
  shaped. In that case, stop with the partial result and design the next
  experiment around that missing state.

The experiment fails if:

- ScreenFormatter duplicates PageList traversal instead of delegating;
- the formatter introduces public API or ABI surface;
- VT screen extras are implemented prematurely;
- pin maps are character-indexed or shorter than output bytes;
- existing PageList formatter output or maps regress;
- tests or formatting fail.

## Design Review

Codex reviewed the first design draft and found one blocker: `screen.rs` would
be a sibling of `page_list.rs`, but the draft did not specify how it could call
PageList's currently private constructor, formatter option/result types, and
formatter methods.

The design was updated to require the narrowest possible `pub(super)` PageList
surface or `pub(super)` wrapper methods for ScreenFormatter delegation, while
keeping everything internal to the `terminal` module and out of public API/ABI.
The updated design also calls out test-only content population helpers instead
of broadly exposing PageList internals.

Codex re-reviewed the updated design and found no remaining blockers. It agreed
that the ScreenFormatter content slice is appropriately narrow: no VT extras, no
parser/cursor/Terminal state, no public ABI, and all content delegation remains
through the completed PageList formatter path.

## Result

**Result:** Pass

Implemented the minimal private ScreenFormatter content layer:

- Added `roastty/src/terminal/screen.rs`.
- Wired it from `roastty/src/terminal/mod.rs`.
- Added private `Screen`, `ScreenFormatter`, `ScreenFormatterOptions`, and
  `ScreenFormatterContent` types.
- `Screen` currently owns only a `PageList`, matching this experiment's minimal
  shell requirement.
- `ScreenFormatterContent` models upstream's content split with `None` and
  `Selection(Option<selection::Selection>)`.

ScreenFormatter content delegates directly to the completed PageList formatter
surface:

- `Selection(None)` formats the full screen-domain PageList content.
- `Selection(Some(selection))` formats the selected content.
- `None` emits empty output and an empty pin map.
- Plain, VT, and HTML outputs go through PageList delegation.
- Pin maps remain byte-indexed, and tests assert `text.len() == pin_map.len()`.
- The already-ported `codepoint_map` option is preserved through
  `ScreenFormatterOptions` and the PageList delegation wrappers.

The PageList visibility changes are narrow and remain internal to the `terminal`
module:

- `PageList::init(...)`;
- `PageList::pin(...)`;
- `PageOutputFormat`;
- `PageStringWithPinMap`;
- `CodepointMapEntry` / `CodepointReplacement`;
- `PageList::screen_format_string(...)`;
- `PageList::screen_format_string_with_pin_map(...)`;
- test-only PageList content-population helpers.

No public ABI, app behavior, renderer behavior, PTY behavior, clipboard
behavior, `Terminal`, `TerminalFormatter`, parser state, cursor state, mode
state, palette extra emission, or VT screen extras were added. Upstream
ScreenFormatter extras remain deferred because Roastty does not yet have the
screen cursor/mode/charset/hyperlink state needed to emit them faithfully.

Verification passed without warnings:

```bash
cargo fmt
cargo test -p roastty screen_formatter    # 12 unit tests passed
cargo test -p roastty styled_pin_map      # 9 unit tests passed
cargo test -p roastty pin_map             # 27 unit tests passed
cargo test -p roastty page_string         # 12 unit tests passed
cargo test -p roastty terminal::page_list # 524 unit tests passed
cargo test -p roastty                     # 829 unit tests passed; ABI harness passed
```

Codex design review found one blocker in the first draft: sibling-module
visibility was underspecified. The design was updated with narrow internal
visibility requirements and then approved.

Codex result review found one blocker in the first implementation:
`ScreenFormatterOptions` had dropped the already-ported `codepoint_map` option.
The implementation was updated to carry `codepoint_map` through both output and
pin-map delegation paths, and a direct ScreenFormatter codepoint-map test was
added. Codex re-reviewed the corrected implementation and found no remaining
blockers.

## Conclusion

Experiment 89 establishes the first private `Screen` layer and ports the
content-routing part of upstream `ScreenFormatter` without expanding into
cursor, parser, terminal, or VT extra state. The key architectural point is that
ScreenFormatter remains a wrapper over PageList formatting, not a second text
traversal.

The next experiment can continue upward through the formatter stack. The most
natural next slice is either the minimal state needed for ScreenFormatter VT
extras or the first `TerminalFormatter` content wrapper, depending on whether we
want to build screen state before terminal state.
