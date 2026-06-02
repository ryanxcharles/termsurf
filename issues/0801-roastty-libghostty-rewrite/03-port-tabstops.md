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

# Experiment 3: Port Terminal Tabstops

## Description

Port Ghostty's `terminal/Tabstops.zig` into Roastty as the first real
Zig-to-Rust subsystem implementation.

Experiment 2 chose `Tabstops` as the first implementation slice because it is
small, has focused upstream tests, and exercises several translation rules
without involving OS integration, renderer state, PTY IO, or pointer-heavy page
storage. This experiment should validate the porting policy on a real module
before Issue 801 moves into broader terminal internals.

The implementation must preserve upstream behavior, not merely provide a similar
tabstop helper. In particular, preserve:

- a 512-column preallocated bitset segment;
- dynamic expansion beyond the preallocated segment;
- Ghostty's exact dynamic storage sizing and `capacity()` behavior;
- 0-indexed column semantics;
- upstream `unset` XOR/toggle semantics;
- interval reset behavior;
- 80-column tabstop count behavior;
- state preservation when resize allocation fails.

## Changes

1. Add a terminal module namespace.
   - Create `roastty/src/terminal/mod.rs`.
   - Wire it from `roastty/src/lib.rs` with `mod terminal;`.
   - Keep the module internal for now unless tests require `pub(crate)`.
   - Do not expose any new C ABI.

2. Port `vendor/ghostty/src/terminal/Tabstops.zig`.
   - Create `roastty/src/terminal/tabstops.rs`.
   - Use safe Rust only unless implementation proves otherwise. `Tabstops`
     should not require `unsafe`.
   - Preserve constants equivalent to:
     - `Unit = u8`
     - `unit_bits = 8`
     - `prealloc_columns = 512`
     - `prealloc_count = 64`
   - Represent the preallocated segment as `[u8; PREALLOC_COUNT]`.
   - Represent dynamic stops as `Vec<u8>`.
   - Preserve the upstream `entry(col) = col / unit_bits` and
     `index(col) = col % unit_bits` mapping.
   - Precompute or calculate masks so they match Zig's `1 << index` values.
   - Preserve Ghostty's current dynamic sizing exactly: when resizing beyond the
     512-column preallocated segment, allocate `cols - prealloc_columns` dynamic
     `u8` entries, not the minimal number of bitset units.
   - Preserve Ghostty's `capacity()` formula:
     `(prealloc_count + dynamic_stops.len()) * unit_bits`.
   - This dynamic sizing is larger than a minimal bitset implementation would
     require, but it is upstream behavior and is observable through
     `capacity()`.

3. Preserve the public Rust behavior needed by later terminal code.
   - Provide at least:
     - `Tabstops::new(cols, interval) -> Result<Tabstops, TabstopError>`
     - `Tabstops::resize(&mut self, cols) -> Result<(), TabstopError>`
     - `Tabstops::reset(&mut self, interval)`
     - `Tabstops::set(&mut self, col)`
     - `Tabstops::unset(&mut self, col)`
     - `Tabstops::get(&self, col) -> bool`
     - `Tabstops::capacity(&self) -> usize`
     - `Tabstops::cols(&self) -> usize`
   - Use a small module-local `TabstopError` for allocation failure.
   - Keep callers responsible for only using columns within current capacity,
     matching upstream assertions for out-of-capacity access.
   - Preserve upstream `unset` semantics exactly. In Ghostty, `unset` uses XOR
     (`^=`), so a second `unset` on the same column toggles the tabstop back on.
     Do not replace this with an idempotent clear unless a later experiment
     explicitly records that divergence.

4. Preserve allocation-failure rollback.
   - `resize` must not update `cols` or replace existing dynamic storage until
     all allocation needed for the new size has succeeded.
   - Use fallible allocation APIs such as `Vec::try_reserve_exact` or a small
     helper that allows deterministic test injection.
   - Do not add a broad project allocator abstraction unless this module proves
     it is necessary.

5. Port upstream tests.
   - Port the upstream Zig tests from `Tabstops.zig`:
     - `Tabstops: basic`
     - `Tabstops: dynamic allocations`
     - `Tabstops: interval`
     - `Tabstops: count on 80`
     - `Tabstops: resize alloc failure preserves state`
   - Add a parity test for double-`unset` toggling a tabstop back on. This is
     not isolated in an upstream test, but it is required by the upstream
     implementation.
   - Preserve test names closely enough that the upstream source is obvious.
   - If Rust cannot deterministically force a real allocator failure without a
     broader allocator abstraction, add a module-local test-only failure hook or
     helper and document why it is local to `Tabstops`.

6. Keep the scope narrow.
   - Do not port `Screen`, `Page`, `PageList`, parser logic, or any rendering
     code.
   - Do not modify the C header or ABI inventory.
   - Do not add dependencies unless the implementation cannot remain simple
     without them.

7. Format and test.
   - Run `cargo fmt` after Rust edits and accept its output.
   - Run:

     ```bash
     cargo test -p roastty terminal::tabstops
     cargo test -p roastty
     ```

8. Record the result.
   - Append `## Result` and `## Conclusion` to this file.
   - Update the Issue 801 README experiment index from `Designed` to `Pass`,
     `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `Tabstops` is implemented in Roastty with no C ABI changes;
- all five upstream `Tabstops.zig` behavior tests are ported or have a
  documented equivalent;
- resize allocation failure preserves the old `cols` value;
- `cargo fmt` is run and accepted;
- `cargo test -p roastty terminal::tabstops` passes;
- `cargo test -p roastty` passes;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- normal tabstop behavior is ported and tested, but deterministic allocation
  failure cannot be tested without a follow-up allocator/test-injection
  experiment.

The experiment fails if:

- it changes public ABI;
- it starts porting unrelated terminal subsystems;
- it uses unsafe Rust without a clear invariant and test;
- it ignores upstream allocation-failure behavior;
- it cannot pass the targeted Roastty tests.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or implementing the port.

## Result

**Result:** Pass

Roastty now has its first real terminal-core module ported from Ghostty:
`terminal::tabstops`.

Implemented files:

- `roastty/src/terminal/mod.rs`
- `roastty/src/terminal/tabstops.rs`
- `roastty/src/lib.rs` module wiring

The port preserves the upstream `Tabstops.zig` behavior that matters for parity:

- 512 preallocated columns backed by 64 `u8` units;
- dynamic storage above the preallocated segment;
- Ghostty's exact dynamic sizing rule: `cols - prealloc_columns` dynamic `u8`
  entries;
- Ghostty's exact `capacity()` rule:
  `(prealloc_count + dynamic_stops.len()) * unit_bits`;
- 0-indexed `entry` / `index` bit mapping;
- upstream XOR/toggle behavior for `unset`;
- interval reset behavior;
- rollback on resize allocation failure.

All upstream tests from `Tabstops.zig` were ported or matched with a direct Rust
equivalent:

- `Tabstops: basic` -> `tabstops_basic`
- `Tabstops: dynamic allocations` -> `tabstops_dynamic_allocations`
- `Tabstops: interval` -> `tabstops_interval`
- `Tabstops: count on 80` -> `tabstops_count_on_80`
- `Tabstops: resize alloc failure preserves state` ->
  `tabstops_resize_alloc_failure_preserves_state`

Two extra parity tests were added:

- `tabstops_unset_toggles`, because upstream `unset` uses XOR and therefore a
  second unset toggles the bit back on;
- `tabstops_preserves_upstream_capacity_semantics`, because Ghostty's dynamic
  storage sizing is larger than a minimal Rust bitset implementation would be
  and is observable through `capacity()`.

The allocation-failure behavior is tested with a module-local test hook. This
keeps the deterministic failure injection scoped to `Tabstops` and avoids adding
a project-wide allocator abstraction before a second module proves one is
needed.

No public C ABI or header files changed.

### Verification

Ran `cargo fmt` after Rust edits and accepted the output.

Ran:

```bash
cargo test -p roastty terminal::tabstops
cargo test -p roastty
```

Both commands passed. The targeted command ran 7 `terminal::tabstops` tests. The
full Roastty test suite ran 10 Rust unit tests, 1 C ABI harness integration
test, and doc tests.

### Completion Review

Codex reviewed the completed implementation and result and found no blocking
correctness, style, integration, or documentation issues. It confirmed the port
preserves Ghostty's capacity semantics, XOR `unset` behavior, allocation-failure
rollback, internal-only module scope, and test coverage for this experiment.

## Conclusion

Experiment 3 succeeds. The first small Ghostty subsystem has been adapted into
Roastty with behavior-parity tests, and the Experiment 2 translation rules held
up in practice.

The next experiment should continue through another small terminal-core module
before moving into pointer-heavy page storage. A good next slice is likely a
small terminal support type that depends on simple integer or enum behavior
rather than PTY, renderer, CoreText, or page-grid ownership.
