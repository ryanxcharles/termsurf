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

# Experiment 83: Port Formatter Codepoint Map

## Description

Port upstream `terminal/formatter.zig` codepoint replacement maps into Roastty's
private PageList formatter.

Experiment 82 added private VT/HTML styled formatting and intentionally deferred
`codepoint_map`. Upstream's replacement map is a small formatter option that
transforms emitted codepoints before the final plain, VT, or HTML encoding step.
It is independent from `Screen`, `Terminal`, pin maps, hyperlinks, and terminal
extras, making it the next narrow formatter slice.

## Changes

1. Re-read the upstream source of truth.
   - Use `vendor/ghostty/src/terminal/formatter.zig` for:
     - `CodepointMap`;
     - `CodepointMap.Replacement`;
     - `Options.codepoint_map`;
     - `PageFormatter.writeCodepointWithReplacement`;
     - tests named `Page codepoint_map ...`.
   - Use existing Roastty code:
     - `roastty/src/terminal/page_list.rs`;
     - Experiment 81 plain formatter helpers;
     - Experiment 82 VT/HTML formatter helpers.
   - Do not modify `vendor/ghostty/`.

2. Add private codepoint-map value types.
   - Preferred shape:

     ```rust
     #[derive(Debug, Clone, PartialEq, Eq)]
     enum CodepointReplacement {
         Codepoint(char),
         String(String),
     }

     #[derive(Debug, Clone, PartialEq, Eq)]
     struct CodepointMapEntry {
         range: std::ops::RangeInclusive<u32>,
         replacement: CodepointReplacement,
     }
     ```

   - Keep these types private to `page_list.rs`.
   - Add a constructor or validator that rejects invalid ranges where
     `start > end`.
   - Ranges must only contain valid Unicode scalar values. Either store range
     endpoints as `char`, or validate `u32` endpoints with `char::from_u32(...)`
     and reject surrogate/out-of-range values before construction.
   - Do not add public API or ABI exposure.

3. Add codepoint-map support to formatter options.
   - Add an optional borrowed map to the private formatter options:

     ```rust
     codepoint_map: Option<&'a [CodepointMapEntry]>
     ```

   - Thread it through:
     - `PageStringOptions`;
     - `PlainPageFormat` or a shared plain/styled codepoint writer;
     - `StyledPageFormat`.
   - Existing `selection_string()` and `dump_string()` behavior must remain
     unchanged by passing no codepoint map.

4. Match upstream replacement semantics.
   - For each emitted base codepoint and attached grapheme codepoint:
     - scan the map from last entry to first;
     - use the first range that contains the codepoint;
     - if no entry matches, emit the original codepoint;
     - if replacement is `Codepoint`, emit that one codepoint through the
       existing plain/VT/HTML writer;
     - if replacement is `String`, iterate the string's Unicode scalar values
       and emit each through the existing plain/VT/HTML writer.
   - Replacements are not recursive. A replacement codepoint or string is passed
     directly to the final plain/VT/HTML writer and must not be looked up in the
     codepoint map again.
   - Replacement must happen before HTML escaping and non-ASCII numeric entity
     conversion.
   - Replacement must apply to plain, VT, and HTML output.

5. Add upstream-equivalent tests.
   - Add tests for:
     - single codepoint replacement;
     - conflicting replacement entries preferring the last matching entry;
     - replacement with a string;
     - range replacement;
     - multiple ranges;
     - Unicode string replacement;
     - empty map preserving output;
     - replacement of attached grapheme codepoints;
     - non-recursive replacement, e.g. `a -> "b"` and `b -> "c"` emits `b`;
     - replacement in VT output;
     - replacement in HTML output, proving replacement happens before escaping
       and numeric entity conversion.
     - generated alignment blanks are not replaced. Mapping `' '` should not
       affect spaces emitted from accumulated blank cells, while explicit space
       cells under the chosen trim mode should match upstream behavior.
   - Add guard tests:
     - reversed ranges are rejected or cannot be constructed;
     - invalid Unicode scalar ranges are rejected or cannot be constructed;
     - existing `selection_string`, `dump_string`, and `page_string` behavior is
       unchanged when no map is supplied.

6. Keep scope narrow.
   - Do not add pin maps or point maps in this experiment, even though upstream
     tests also check replacement-to-coordinate mapping.
   - Do not add `Screen`, `Terminal`, parser state, cursor state, terminal
     extras, hyperlinks, writer abstraction, public ABI, app, renderer,
     clipboard, PTY, or UI behavior.
   - Do not expose codepoint maps outside the terminal module.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty codepoint_map
     cargo test -p roastty page_string
     cargo test -p roastty dump_string
     cargo test -p roastty selection_string
     cargo test -p roastty terminal::page_list
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Independent review.
   - Before implementation, get Codex review of this experiment design.
   - Record the design-review outcome in this experiment file before
     implementation.
   - After implementation and verification, get Codex review of the completed
     result.
   - Fix all real findings before proceeding.

9. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - helper/type names and location;
     - how invalid ranges or invalid Unicode scalar replacements are handled;
     - which upstream codepoint-map behaviors were ported;
     - which upstream pin-map behaviors remain deferred;
     - verification command output summary;
     - Codex design-review outcome;
     - Codex result-review outcome.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has private formatter codepoint-map types equivalent to the scoped
  upstream behavior;
- replacements scan from last entry to first and prefer the last matching range;
- replacements are not recursively remapped;
- replacement applies to base codepoints and grapheme codepoints;
- `Codepoint` and `String` replacements work in plain, VT, and HTML output;
- HTML replacement happens before escaping and non-ASCII numeric entity
  conversion;
- generated alignment blanks are not replaced as if they were source cells;
- reversed or invalid Unicode scalar ranges cannot enter the formatter;
- existing no-map `selection_string`, `dump_string`, and `page_string` behavior
  remains unchanged;
- no pin maps, point maps, `Screen`, `Terminal`, parser state, cursor state,
  terminal extras, hyperlinks, writer abstraction, public ABI, app, renderer,
  clipboard, PTY, or UI behavior is added;
- `cargo fmt`, targeted codepoint-map tests, formatter regression tests,
  PageList tests, and full `cargo test -p roastty` pass;
- Codex design and result reviews approve the experiment, or all real findings
  are fixed before proceeding.

The experiment is partial if:

- codepoint replacements work for plain output, but a specific VT/HTML escaping
  interaction exposes a missing lower-level formatter primitive that should be
  split into the next experiment.

The experiment fails if:

- codepoint-map behavior cannot be implemented without adding pin maps,
  `Screen`, `Terminal`, parser state, public API, app, renderer, PTY, clipboard,
  or UI behavior;
- replacement order differs from upstream;
- replacement recursively remaps replacement output;
- replacement bypasses HTML escaping/numeric entity behavior;
- generated alignment blanks are remapped;
- no-map formatter behavior regresses;
- tests or formatting fail.

## Design Review

Codex reviewed the initial design and found no blockers. It agreed that
`codepoint_map` is the right narrow next formatter slice after Experiment 82
because it sits directly in the PageFormatter codepoint emission path and does
not require pin maps, `Screen`, `Terminal`, hyperlinks, or public API.

Codex identified three high-value improvements before implementation:

- specify and test that replacements are not recursive. Upstream applies the map
  once and then emits replacement output directly through the final writer;
- tighten scalar validation. Replacement codepoints and range endpoints must be
  valid Unicode scalar values, not arbitrary `u32`s;
- test generated alignment blanks. Spaces emitted from accumulated blank cells
  are formatter-generated alignment, not source codepoints, and should not be
  remapped by a `' '` replacement.

The design now requires those behaviors and tests.

## Result

**Result:** Pass

Implemented private formatter codepoint-map support in
`roastty/src/terminal/page_list.rs`. The new private types are
`CodepointReplacement` and `CodepointMapEntry`; `PageStringOptions`,
`PlainPageFormat`, and `StyledPageFormat` now carry an optional borrowed
`codepoint_map` used only by the private PageList formatter path.

`CodepointMapEntry::new(...)` rejects reversed ranges, out-of-range Unicode
values, and ranges that intersect surrogate codepoints. Replacements are stored
as either a valid Rust `char` or a valid Rust `String`, so invalid scalar
replacement output cannot enter the formatter through these private types.

The ported behavior matches the scoped upstream formatter semantics:

- map entries are scanned from last to first, so the last matching range wins;
- replacements are one-shot and are not recursively remapped;
- replacements apply to base cell codepoints and attached grapheme codepoints;
- `Codepoint` and `String` replacements work in plain, VT, and HTML output;
- HTML output applies replacement before escaping and numeric entity conversion;
- generated alignment blanks are not treated as source spaces and are not
  remapped;
- no-map `selection_string`, `dump_string`, and `page_string` behavior remains
  unchanged.

The upstream pin-map and point-map behavior remains deferred. This experiment
did not add pin maps, point maps, `Screen`, `Terminal`, parser state, cursor
state, terminal extras, hyperlinks, writer abstraction, public ABI, app,
renderer, clipboard, PTY, or UI behavior.

Verification passed:

```text
cargo fmt
cargo test -p roastty codepoint_map        # 15 passed
cargo test -p roastty page_string          # 12 passed
cargo test -p roastty dump_string          # 13 passed
cargo test -p roastty selection_string     # 22 passed
cargo test -p roastty terminal::page_list  # 466 passed
cargo test -p roastty                      # 759 unit tests + ABI harness + doctests passed
```

Codex design review found no blockers and requested explicit non-recursive,
Unicode-scalar, and generated-blank coverage; those were added to the design
before implementation.

Codex result review first found one useful test gap: replacement while a VT/HTML
style wrapper is active. Added
`codepoint_map_styled_output_keeps_replacement_inside_style`, reran the full
verification set, and re-ran Codex review. The second review found no blockers
and said the implementation is acceptable to record.

## Conclusion

Experiment 83 completes the private `codepoint_map` slice of Ghostty's formatter
behavior for Roastty's PageList formatter. The remaining formatter work is still
outside this slice: pin/point maps and any future public formatter API can be
ported in later experiments when their surrounding infrastructure exists.
