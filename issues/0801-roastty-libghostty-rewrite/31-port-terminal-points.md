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

# Experiment 31: Port Terminal Points

## Description

Port upstream `terminal/point.zig` to Roastty as the coordinate vocabulary
needed by the next `PageList` work.

`PageList` uses points to distinguish four coordinate spaces: active area,
visible viewport, whole screen, and scrollback history. The type is small, but
it is a load-bearing dependency for pins, viewport conversion, selection, text
reads, scrollback, and later screen operations.

This experiment should add the point types and tests only. It should not port
`PageList`, pins, selection, screen behavior, parser behavior, public ABI, or
Swift integration.

## Changes

1. Re-read upstream source.
   - Use `vendor/ghostty/src/terminal/point.zig` as the source of truth.
   - Preserve the upstream distinction between:
     - `active`
     - `viewport`
     - `screen`
     - `history`
   - Preserve the coordinate-width model:
     - `x` uses `CellCountInt`;
     - `y` uses `u32`, because screen/history coordinates can exceed one Page's
       row count.
   - Do not modify `vendor/ghostty/`.

2. Add `roastty/src/terminal/point.rs`.
   - Define an internal `Tag` enum with the four upstream point tags.
   - Define a `Coordinate` struct with `x: CellCountInt` and `y: u32`.
   - Define a `Point` enum with one variant per tag, each carrying a
     `Coordinate`.
   - Add `Point::coord()` to return the carried coordinate, matching upstream
     `Point.coord()`.
   - Add `Point::tag()` if useful for tests or upcoming PageList code.
   - Add convenience constructors only if they keep call sites clearer; do not
     invent broader coordinate arithmetic in this experiment.

3. Decide C-layout scope explicitly.
   - Upstream `Point` also exposes a C tagged-union representation.
   - Roastty does not yet expose PageList points across the public ABI, so this
     experiment should not add public `roastty_point` ABI functions or headers.
   - If a private C-shaped representation is added for layout parity tests, keep
     it private to the Rust module and document that the public ABI can be added
     later when an exported API actually needs it.

4. Wire the module.
   - Add `mod point;` to `roastty/src/terminal/mod.rs`.
   - Keep visibility internal (`pub(super)` or private) until PageList or public
     ABI work needs a wider boundary.

5. Add tests.
   - Test each `Point` variant:
     - stores the expected coordinate;
     - returns that coordinate from `coord()`;
     - reports the expected tag if `tag()` is implemented.
   - Test `Coordinate` equality.
   - Test that `Coordinate::y` accepts values larger than `CellCountInt::MAX`.
   - Test type sizes/alignments only if the implementation introduces a C-shaped
     representation; otherwise avoid brittle layout tests for a purely
     Rust-internal enum.

6. Preserve scope.
   - Do not implement:
     - PageList nodes, pins, viewport state, scrolling, or compaction;
     - selection or text extraction;
     - screen lifecycle;
     - public C ABI additions;
     - terminal parser behavior.
   - Do not add `ghostty` names except when citing upstream paths or test
     provenance.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::point
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - APIs added;
     - C-layout decision;
     - tests added;
     - verification command output summary.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- Roastty has an internal point module matching upstream point tags and
  coordinate widths;
- each point variant preserves and returns its coordinate;
- no public ABI or PageList behavior is added prematurely;
- `cargo fmt`, targeted point tests, and full `cargo test -p roastty` pass;
- Codex reviews the experiment design and completed result and approves them, or
  all real findings are fixed.

The experiment is partial if:

- the Rust point types are implemented, but C-layout parity needs a follow-up
  because an upcoming public ABI requirement is clearer than expected;
- one layout-oriented assertion is deferred because no public representation
  exists yet.

The experiment fails if:

- `y` is narrowed to `CellCountInt`;
- active/viewport/screen/history are collapsed into one untagged coordinate;
- public ABI, PageList, screen, parser, or selection behavior is introduced in
  this slice;
- tests or formatting fail.

## Result

**Result:** Pass

Implemented `roastty/src/terminal/point.rs` and wired it into the internal
terminal module tree.

The new module adds:

- `Tag` with the upstream coordinate-space tags: active, viewport, screen, and
  history;
- `Coordinate` with `x: CellCountInt` and `y: u32`;
- `Point` variants carrying `Coordinate`;
- `Point::coord()` matching upstream `Point.coord()`;
- `Point::tag()` for tests and upcoming PageList code;
- small constructors for each variant.

No public C ABI point representation was added. Upstream has a C tagged-union
shape, but Roastty does not yet expose PageList points across the ABI, so adding
that now would be premature. The ABI-facing point shape remains deferred until a
public exported API needs it.

Added tests for:

- coordinate equality;
- `Coordinate::y` accepting values larger than `CellCountInt::MAX`;
- each point variant preserving its coordinate;
- each point variant reporting the expected tag.

Verification passed:

```bash
cargo fmt
cargo test -p roastty terminal::point
cargo test -p roastty
```

The targeted point suite reported 6 passing tests. The full `roastty` suite
reported 281 unit tests, the ABI harness, and doc tests passing.

## Conclusion

Roastty now has the small but necessary coordinate vocabulary used by upstream
`PageList`. This keeps the next PageList work grounded in Ghostty's four
coordinate spaces instead of flattening active, viewport, screen, and history
coordinates into a single ambiguous point type.
