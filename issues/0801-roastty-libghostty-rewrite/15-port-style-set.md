# Experiment 15: Port Style Hashing and Set Storage

## Description

Port Ghostty's `style.Set` layer on top of the ref-counted set foundation from
Experiment 14.

Experiment 7 ported the value-level `Style` behavior but explicitly deferred
`StyleSet`. Experiment 14 added the reusable offset-backed `RefCountedSet`.
Before `Page clone styles` can be ported, Roastty needs a real `style::Set` with
upstream-compatible style hashing, layout, add, lookup/get, ref-count, and
capacity behavior.

This experiment should not wire styles into `Page` yet. It should make
`terminal::style` own the style-specific set wrapper/context and prove it with
the upstream `Set basic usage` and `Set capacities` tests.

## Changes

1. Inspect upstream source.
   - Use `vendor/ghostty/src/terminal/style.zig` as the source of truth.
   - Re-read:
     - `Style.PackedStyle`
     - `Style.hash`
     - `style.Set = RefCountedSet(...)`
     - upstream tests:
       - `Set basic usage`
       - `Set capacities`
   - Re-check `vendor/ghostty/src/terminal/page.zig` only for the later
     `Page clone styles` dependency. Do not implement Page behavior here.
   - Do not modify `vendor/ghostty/`.

2. Add upstream-compatible style hashing.
   - Port the `PackedStyle` hashing model or an equivalent deterministic Rust
     representation that preserves upstream equality/hash semantics.
   - Do not use Rust's default `Hash`/`DefaultHasher`; it is randomized across
     processes and is not the upstream algorithm.
   - Preserve the important upstream invariant: styles that compare equal must
     produce the same hash.
   - Add tests for:
     - identical styles have identical hashes;
     - representative known-vector hashes for default, bold, italic, palette,
       and RGB styles using the chosen deterministic algorithm;
     - non-identical styles remain independently storable even though hash
       collisions are always allowed;
     - hashing is stable across repeated calls.
   - Document explicitly whether the Rust hash values are numerically identical
     to Ghostty's `std.hash.int` output or are a Roastty deterministic
     equivalent. Exact numeric parity is not required for set correctness, but
     the chosen algorithm must be stable and tested with known vectors.

3. Resolve the `Style` storage layout question explicitly.
   - Measure and test Rust `Style` size/alignment.
   - Measure and test `ref_counted_set::Item<Style>` size/alignment.
   - Compare against Ghostty/Page's current expected style-set item layout:
     - upstream `Style` value size currently represented in Page layout as `28`
       bytes;
     - upstream style set item size currently represented in Page layout as `36`
       bytes;
     - item alignment is currently represented as `4`.
   - Add explicit tests that the style set layout used for `layout(16)` matches
     the Page-compatible values:
     - `base_align = 8`;
     - `cap = 13`;
     - `table_cap = 16`;
     - `items_start = 32`;
     - `total_size = 500`.
   - If the current Rust `Style` representation does not produce compatible item
     layout, choose one focused fix:
     - make `Style`, `Color`, and `Flags` storage representations
       upstream-compatible while preserving the public helper API; or
     - introduce a dedicated storage representation used by `style::Set`, with
       explicit conversion to/from the ergonomic `Style` value.
   - Do not leave this ambiguous. Page layout and future byte-copy clone depend
     on this answer.

4. Add `style::Set`.
   - Add a style-specific wrapper or type alias over `RefCountedSet<Style>`.
   - Add the style context that hashes via `Style::hash` and compares via
     `Style` equality.
   - Expose only the operations needed by upstream style set and future Page
     style storage:
     - layout;
     - base alignment;
     - `init`;
     - `add`;
     - `get`;
     - `lookup`, if useful for tests/future Page;
     - `use` / `useMultiple`;
     - `release` / `releaseMultiple`;
     - `refCount`;
     - `count`;
     - `capacityForCount`.
   - Expose `add_with_id` now. Future `Page clone styles` depends directly on
     upstream `StyleSet.addWithId(...) orelse src_cell.style_id`; adding the
     wrapper API here is still style-set scope and avoids making the next Page
     experiment solve wrapper semantics and Page behavior at the same time.
   - Preserve Experiment 14's `add_with_id` semantics:
     - requested ID used: no alternate ID;
     - alternate ID chosen: return that alternate ID.

5. Keep Page wiring out of scope.
   - Do not add a `styles` field to `Page`.
   - Do not replace Page's existing `StyleSetLayout` helper unless the
     replacement is purely mechanical and all Page layout numeric tests remain
     unchanged.
   - Do not port:
     - `Page clone styles`;
     - `Page setStyle`;
     - `Page verifyIntegrity styles ...`;
     - `Page exactRowCapacity styles ...`;
     - `cloneFrom` / `cloneRowFrom` style handling.

6. Translate upstream tests.
   - Port `style.zig` `Set basic usage`.
   - Port `style.zig` `Set capacities`.
   - Add direct layout compatibility tests from Step 3.
   - Add `style::Set::add_with_id` tests for requested-ID and alternate-ID
     behavior using style values.
   - Keep existing style formatting tests green.

7. Verify.
   - Run:

     ```bash
     cargo fmt
     cargo test -p roastty terminal::style
     cargo test -p roastty terminal::ref_counted_set
     cargo test -p roastty terminal::page
     cargo test -p roastty
     ```

   - `cargo fmt` output must be accepted as-is.

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Include:
     - style hash implementation chosen;
     - exact Rust `Style` and set-item layout results;
     - whether `Style` storage representation changed;
     - style set API added;
     - upstream tests ported;
     - deferred Page style behavior;
     - verification command output summary.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `terminal::style` has deterministic style hashing suitable for
  `RefCountedSet`;
- style equality and style hashing are consistent;
- Rust style storage layout is explicitly tested and compatible with the Page
  style-set item layout, or a dedicated compatible storage representation is
  introduced and tested;
- `style::Set` exists and is backed by the Experiment 14 offset-backed
  `RefCountedSet`;
- `style::Set::add_with_id` exists and preserves upstream nullable return
  semantics;
- `style::Set::layout(16)` and `base_align` match the current Page-compatible
  style layout values;
- upstream `Set basic usage` and `Set capacities` are ported and pass;
- existing style formatting tests remain green;
- existing Page layout tests remain green;
- no Page style behavior is introduced;
- `cargo fmt`, targeted style/ref-counted-set/Page tests, and full
  `cargo test -p roastty` pass;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- style hashing and basic set behavior work, but layout compatibility requires a
  focused representation-change experiment before Page can safely store styles.

The experiment fails if:

- style set storage uses heap `HashMap`/`Vec` instead of the offset-backed
  `RefCountedSet`;
- hashing uses Rust's randomized default hasher;
- equal styles can hash differently;
- layout compatibility is left unmeasured;
- `style::Set` omits `add_with_id`;
- Page layout numeric tests regress;
- the experiment drifts into Page style behavior.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or implementing the
slice.

## Result

**Result:** Pass

Experiment 15 added deterministic style hashing and a real offset-backed
`style::Set` wrapper on top of Experiment 14's `RefCountedSet`.

### Hashing

The implementation added `Style::hash()` through a dedicated storage
representation:

- `StorageStyle`
- `StorageStyle::from_style`
- `StorageStyle::to_style`
- `StorageStyle::hash`

`StorageStyle` serializes the ergonomic Rust `Style` into a deterministic
28-byte page-compatible storage value and hashes those bytes with FNV-1a 64.

This is a Roastty deterministic equivalent, not a claim of numeric parity with
Ghostty's Zig `std.hash.int`. Exact numeric hash parity is not required for
`RefCountedSet` correctness because the hash only determines table placement,
but the chosen algorithm is stable and covered by known-vector tests.

Known vectors were added for:

- default style;
- bold style;
- italic style;
- palette foreground style;
- RGB foreground style.

### Layout

The experiment measured the existing ergonomic Rust `Style` layout:

- `size_of::<Style>() = 21`
- `align_of::<Style>() = 1`

That is not Page-compatible by itself, so the implementation uses `StorageStyle`
for set storage:

- `size_of::<StorageStyle>() = 28`
- `align_of::<StorageStyle>() = 4`
- `size_of::<ref_counted_set::Item<StorageStyle>>() = 36`
- `align_of::<ref_counted_set::Item<StorageStyle>>() = 4`

`style::Set::layout(16)` matches the current Page-compatible style layout:

- `base_align = 8`
- `cap = 13`
- `table_cap = 16`
- `items_start = 32`
- `total_size = 500`

Page's existing style layout helper was not replaced in this experiment. That
kept the slice limited to `style::Set`, and all existing Page layout tests
remained green.

### API Added

`terminal::style` now exposes:

- `Set`
- `Set::BASE_ALIGN`
- `Set::capacity_for_count`
- `Set::layout`
- `Set::init`
- `Set::add`
- `Set::add_with_id`
- `Set::lookup`
- `Set::get`
- `Set::use_one`
- `Set::use_multiple`
- `Set::release`
- `Set::release_multiple`
- `Set::ref_count`
- `Set::count`

`Set::add_with_id` preserves Experiment 14's nullable return semantics:

- `Ok(None)` means the requested ID was used;
- `Ok(Some(id))` means an alternate ID was chosen.

### Tests Added

The upstream tests ported are:

- `style.zig` `Set basic usage`
- `style.zig` `Set capacities`

Additional tests cover:

- storage layout compatibility;
- storage round-trip conversion;
- deterministic hash known vectors;
- requested-ID `add_with_id`;
- alternate-ID `add_with_id`.

### Deferred

This experiment intentionally did not add:

- Page `styles` field wiring;
- Page `clone styles`;
- Page `setStyle`;
- Page style integrity checks;
- Page style exact-capacity behavior;
- `cloneFrom` / `cloneRowFrom` style handling.

### Verification

The required verification passed:

```bash
cargo fmt
cargo test -p roastty terminal::style
cargo test -p roastty terminal::ref_counted_set
cargo test -p roastty terminal::page
cargo test -p roastty
```

Observed results:

- `terminal::style`: 21 passed
- `terminal::ref_counted_set`: 16 passed
- `terminal::page`: 50 passed
- full `roastty` suite: 159 Rust unit tests passed, C ABI harness passed, doc
  tests passed

## Conclusion

Roastty now has the style-specific set layer needed by Page style metadata. The
next Page experiment can wire `style::Set` into `Page` and port the first
style-backed Page test, `Page clone styles`, without also having to solve style
hashing, storage layout, or set wrapper semantics.
