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

# Experiment 90: Port Terminal Formatter Content

## Description

Port the content-routing portion of upstream
`terminal/formatter.zig::TerminalFormatter` into Roastty.

Experiment 89 introduced a minimal private `Screen` and ScreenFormatter content
path. Upstream's next formatter wrapper is `TerminalFormatter`: it owns a
`Terminal`, forwards its content selection to the active screen's
`ScreenFormatter`, and optionally emits terminal-level extras such as palette,
modes, scrolling region, tabstops, keyboard mode, and present working directory.

Roastty still does not have terminal runtime state, parser state, modes,
palette, tabstops on a terminal, PWD, or keyboard flags. This experiment should
therefore port only the content delegation layer: a minimal private `Terminal`
shell containing an active `Screen`, plus a private TerminalFormatter that
delegates all content output and pin maps to ScreenFormatter.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for:
     - `TerminalFormatter`;
     - `TerminalFormatter.Extra`;
     - forwarding `content` to `ScreenFormatter`;
     - terminal formatter content and pin-map tests.
   - Use current Roastty formatter code:
     - `roastty/src/terminal/screen.rs`;
     - `roastty/src/terminal/page_list.rs`;
     - Experiment 89 ScreenFormatter tests and result.
   - Do not modify `vendor/ghostty/`.

2. Add a minimal private `Terminal` module.
   - Add `roastty/src/terminal/terminal.rs`.
   - Wire it from `roastty/src/terminal/mod.rs`.
   - Add private terminal state that preserves the upstream shape enough for the
     formatter layer, for example:

     ```rust
     pub(super) struct Terminal {
         screens: TerminalScreens,
     }

     pub(super) struct TerminalScreens {
         active: Screen,
     }
     ```

   - Add an initializer that creates the active `Screen`:

     ```rust
     impl Terminal {
         fn init(cols: CellCountInt, rows: CellCountInt, max_scrollback_rows: Option<usize>)
             -> Result<Self, PageListAllocError>
     }
     ```

   - Keep the module private to `terminal`. Do not expose it through the crate
     public API or C ABI.
   - Do not add parser, PTY, app lifecycle, renderer, modes, palette, tabstops,
     PWD, keyboard state, alt screen, or multiple-screen behavior in this
     experiment.

3. Add any narrow Screen visibility required by `terminal.rs`.
   - Prefer `pub(super)` constructors/accessors or wrapper methods over broad
     field exposure.
   - Expected candidates:
     - `Screen::init(...)`;
     - `ScreenFormatter`;
     - `ScreenFormatterOptions`;
     - `ScreenFormatterContent`;
     - `ScreenFormatter::format(...)`;
     - `ScreenFormatter::format_with_pin_map(...)`.
   - If tests in `terminal.rs` need to populate active-screen content, add a
     `#[cfg(test)] pub(super)` helper on `Screen` rather than exposing PageList
     internals.
   - These visibility changes must remain internal to `terminal`.

4. Add the private TerminalFormatter content path.
   - Add private formatter types in `terminal.rs`:

     ```rust
     pub(super) struct TerminalFormatter<'a> { ... }
     pub(super) struct TerminalFormatterOptions<'a> { ... }
     pub(super) struct TerminalFormatterExtra { ... }
     ```

   - TerminalFormatter content should reuse
     `ScreenFormatterContent::Selection(Option<selection::Selection>)` and
     `ScreenFormatterContent::None` rather than inventing a separate content
     shape.
   - Default content should be `Selection(None)`, matching upstream "format the
     active screen" behavior.
   - For this experiment, terminal extras are shape-only/no-op or omitted.
     Upstream `TerminalFormatter.init` defaults extras to `.styles`, but Roastty
     cannot faithfully emit palette/screen style extras yet. This experiment
     must document that Roastty's content-only formatter intentionally emits no
     extras until the relevant terminal and screen state exists.

5. Delegate to ScreenFormatter.
   - TerminalFormatter content output must instantiate a ScreenFormatter over
     `terminal.screens.active`, copy the content selection, copy common
     formatting options, and delegate to `ScreenFormatter::format(...)`.
   - TerminalFormatter pin maps must delegate to
     `ScreenFormatter::format_with_pin_map(...)`.
   - Plain, VT, and HTML output must match the equivalent active-screen
     ScreenFormatter output exactly for the same content and options.
   - `codepoint_map`, `trim`, `unwrap`, `emit`, and `palette` options must be
     preserved through the TerminalFormatter -> ScreenFormatter ->
     PageListFormatter chain.
   - Do not duplicate PageList or ScreenFormatter traversal.

6. Preserve scope boundaries.
   - Do not add parser state, cursor state, mode state, palette storage, palette
     emission, scrolling-region emission, tabstop emission, PWD emission,
     keyboard emission, screen extras, terminal extras, alt-screen behavior,
     public ABI, app behavior, renderer behavior, PTY behavior, clipboard
     behavior, or UI behavior.
   - Do not expose `ghostty_*` symbols.
   - Do not change existing PageList or ScreenFormatter output semantics except
     for the narrow visibility/accessor changes needed by this wrapper.

7. Add upstream-equivalent tests.
   - Add TerminalFormatter tests for:
     - plain full active-screen single-line output;
     - plain full active-screen multiline output;
     - plain selected-line output;
     - `Content::None` emitting empty output and an empty pin map;
     - VT content delegation matching ScreenFormatter output;
     - HTML content delegation matching ScreenFormatter output;
     - codepoint-map output and pin-map delegation;
     - plain pin-map single-line output;
     - plain pin-map multiline output;
     - selected plain pin-map output;
     - VT and HTML pin-map output preserving byte-indexed maps;
     - invalid or garbage selection endpoints returning empty output/map via
       ScreenFormatter/PageList delegation.
   - Tests may use test-only helpers to populate the active screen. Those
     helpers must stay `#[cfg(test)]`.
   - Keep existing ScreenFormatter and PageList formatter tests unchanged and
     passing.

8. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal_formatter
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
      - how TerminalFormatter content delegates to ScreenFormatter;
      - how full-screen, selected, and no-content modes behave;
      - whether `codepoint_map` and pin maps remain byte-indexed through the
        delegation chain;
      - why upstream TerminalFormatter extras remain deferred;
      - verification command output summary;
      - Codex design-review outcome;
      - Codex result-review outcome.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has a private minimal `Terminal` type with an active `Screen`;
- Roastty has a private TerminalFormatter content path that reuses
  ScreenFormatter content modes;
- default TerminalFormatter content formats the full active screen;
- selected and no-content modes match upstream content routing semantics;
- plain, VT, and HTML content output match the equivalent ScreenFormatter
  output;
- `trim`, `unwrap`, `emit`, `palette`, and `codepoint_map` options are preserved
  through TerminalFormatter -> ScreenFormatter -> PageListFormatter delegation;
- pin maps are byte-indexed and match ScreenFormatter/PageList delegation;
- invalid or garbage selection endpoints return empty output/map;
- existing PageList and ScreenFormatter behavior remains unchanged;
- no parser state, cursor state, mode state, palette storage, palette emission,
  scrolling-region emission, tabstop emission, PWD emission, keyboard emission,
  screen extras, terminal extras, alt screen, public ABI, app, renderer, PTY,
  clipboard, or UI behavior is added;
- `cargo fmt`, targeted TerminalFormatter tests, ScreenFormatter regression
  tests, PageList formatter tests, PageList tests, and full
  `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- TerminalFormatter content delegation exposes a missing active-screen ownership
  or lifetime shape that requires a separate terminal state experiment before
  formatting can be ported cleanly.

The experiment fails if:

- TerminalFormatter duplicates ScreenFormatter or PageList traversal instead of
  delegating;
- terminal or screen extras are implemented prematurely;
- the formatter introduces public API or ABI surface;
- `codepoint_map` is dropped in the delegation chain;
- pin maps are character-indexed or shorter than output bytes;
- existing PageList or ScreenFormatter output/maps regress;
- tests or formatting fail.

## Design Review

Codex reviewed the design and found no blockers. It agreed that the minimal
Terminal-level content router is the right next slice after Experiment 89
because it adds active-screen ownership and delegates to `ScreenFormatter`
instead of walking PageList content again.

Codex specifically approved the requirement that `trim`, `unwrap`, `emit`,
`palette`, and `codepoint_map` survive the full TerminalFormatter ->
ScreenFormatter -> PageListFormatter chain. It also accepted the explicit scope
boundary around upstream's default `.styles` extras: Roastty's TerminalFormatter
content slice intentionally emits no terminal or screen extras until the
relevant terminal state exists.

Codex noted one non-blocking implementation detail: `TerminalFormatterExtra` is
only a sketch, so implementation should treat extras as optional/no-op and
should not add state just to satisfy the sketch.

## Result

**Result:** Pass

Roastty now has a private minimal terminal formatter content path in
`roastty/src/terminal/terminal.rs`.

The new `Terminal` and `TerminalScreens` types are private to the `terminal`
module. They hold only an active `Screen`, initialized from the same dimensions
and scrollback limit used by the Screen/PageList formatter path. No public API,
C ABI surface, parser state, cursor state, mode state, palette storage, tabstop
state, PWD state, keyboard state, alt-screen behavior, renderer behavior, PTY
behavior, clipboard behavior, or UI behavior was added.

`TerminalFormatter` is also private. Its default content is
`ScreenFormatterContent::Selection(None)`, so it formats the full active screen.
Selected content passes through the same `ScreenFormatterContent::Selection`
shape, and `ScreenFormatterContent::None` emits empty output and an empty pin
map. The terminal formatter does not traverse PageList itself: both `format()`
and `format_with_pin_map()` instantiate `ScreenFormatter` over
`terminal.screens.active`, copy the selected content mode, and delegate to the
screen formatter.

`TerminalFormatterOptions` wraps `ScreenFormatterOptions`, preserving `emit`,
`trim`, `unwrap`, `palette`, and `codepoint_map` through the TerminalFormatter
-> ScreenFormatter -> PageListFormatter chain. Plain, VT, and HTML terminal
formatter output now matches the equivalent ScreenFormatter output for the
active screen. Pin maps remain byte-indexed by delegation, including the VT/HTML
styled formatter paths and codepoint replacement path.

The implementation added narrow test-only helpers on `Screen` and `PageList` to
populate active-screen content and styled cells. Those helpers are
`#[cfg(test)]` and remain internal to the `terminal` module.

Upstream TerminalFormatter extras remain deferred. Ghostty can emit terminal
extras such as palette, modes, scrolling region, tabstops, keyboard mode, PWD,
and screen style data. Roastty does not yet have the corresponding terminal
state, so emitting those extras here would either be fake or premature.

Verification passed:

```text
cargo fmt
cargo test -p roastty terminal_formatter      # 13 passed
cargo test -p roastty screen_formatter        # 16 passed
cargo test -p roastty styled_pin_map          # 9 passed
cargo test -p roastty pin_map                 # 33 passed
cargo test -p roastty page_string             # 12 passed
cargo test -p roastty terminal::page_list     # 524 passed
cargo test -p roastty                         # 842 unit + 1 ABI passed
```

Codex result review found no blockers. It confirmed the implementation matches
the plan's scope boundary, keeps the new terminal types private, delegates
formatter content to `ScreenFormatter`, and preserves byte-indexed pin maps by
delegation. Codex noted one non-blocking gap: `unwrap` preservation is not
isolated by a dedicated TerminalFormatter test. The option is structurally
forwarded through `TerminalFormatterOptions.screen`, and the lower formatter
suites passed, so no code change was required.

## Conclusion

Experiment 90 completes the content-only TerminalFormatter layer. Roastty now
has the same formatter stack shape as the upstream terminal-level content path:
Terminal -> active Screen -> PageList. The next experiment can move to the next
upstream terminal formatter slice without reworking content routing.
