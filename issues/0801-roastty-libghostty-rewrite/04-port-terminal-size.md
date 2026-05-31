# Experiment 4: Port Terminal Size Offsets

## Description

Port Ghostty's `terminal/size.zig` into Roastty as the next small terminal-core
support module.

`size.zig` is the foundation for later page/grid work. It defines the integer
widths and offset-addressing helpers used by `page.zig`, `PageList.zig`,
`ref_counted_set.zig`, and other pointer-heavy terminal storage. Porting it now
lets Roastty establish the unsafe boundary for offset-to-pointer conversion
before the larger page storage port depends on it.

This experiment is still narrow. It should port only the `size.zig` support
types and tests, not `Page`, `PageList`, `Screen`, parser logic, rendering, or
PTY code.

## Changes

1. Add the module.
   - Create `roastty/src/terminal/size.rs`.
   - Wire it from `roastty/src/terminal/mod.rs`.
   - Keep it internal for now. Do not expose any new C ABI.

2. Port the integer constants and type aliases.
   - Preserve:
     - `max_page_size = u32::MAX`
     - `OffsetInt = u32`
     - `CellCountInt = u16`
     - `StyleCountInt = CellCountInt`
     - `HyperlinkCountInt = CellCountInt`
     - `GraphemeBytesInt = u32`
     - `StringBytesInt = u32`
   - Use Rust names that fit the existing module style, but keep enough upstream
     naming in tests/comments to make provenance obvious.

3. Port `Offset(T)`.
   - Represent it as a Rust generic `Offset<T>` containing a `u32` byte offset
     and `PhantomData<T>`.
   - Use `#[repr(transparent)]` so the storage layout is exactly the `u32`
     offset. `PhantomData<T>` is zero-sized and must not affect layout.
   - Add tests/assertions for `size_of::<Offset<u8>>() == size_of::<u32>()` and
     `align_of::<Offset<u8>>() == align_of::<u32>()`.
   - Preserve the zero default.
   - Provide pointer conversion methods equivalent to Zig's `ptr`.
   - Pointer conversion is an allowed unsafe boundary for this issue. Keep it
     narrow and documented.
   - The safe public method may return raw pointers rather than references.
     Creating Rust references from arbitrary offset-derived addresses must stay
     unsafe.
   - Check alignment before returning a typed pointer, matching the upstream
     assertion.

4. Port `Offset<T>::Slice`.
   - Zig exposes this as a nested type. Rust cannot nest a type definition in a
     generic struct, so use a sibling generic type such as `OffsetSlice<T>`.
   - Store `offset: Offset<T>` and `len: usize`.
   - Provide an unsafe slice conversion method that mirrors upstream behavior.

5. Port `OffsetBuf`.
   - Represent the base pointer and byte offset.
   - Provide equivalents for:
     - `init`
     - `initOffset`
     - `start`
     - `member`
     - `add`
     - `rebase`
   - Keep raw pointer manipulation contained inside this module.
   - Document the invariant: `base` must point at the beginning of the true
     allocation, and callers must ensure any derived typed offset is in-bounds
     and properly aligned before dereferencing.
   - Every path that converts a `usize` offset into `OffsetInt` / `u32` must use
     checked conversion or assertion, never silent truncation. This applies to
     `OffsetBuf::member`, `getOffset`, and any direct constructor helper.

6. Port `getOffset`.
   - Provide a helper that computes a typed `Offset<T>` from a base pointer and
     a typed pointer.
   - Reject or assert negative offsets rather than wrapping.
   - Reject or assert offsets larger than `u32::MAX` rather than truncating.
   - Keep the return type `Offset<T>`.

7. Port upstream tests.
   - Port the tests from `vendor/ghostty/src/terminal/size.zig`:
     - `Offset`
     - `Offset ptr u8`
     - `Offset ptr structural`
     - `getOffset bytes`
     - `getOffset structs`
   - Add Rust-specific tests for:
     - `Offset<T>` size/alignment parity with `u32`
     - `OffsetSlice<T>` address/range behavior
     - `OffsetBuf::member`
     - `OffsetBuf::add`
     - `OffsetBuf::rebase`
     - alignment assertion/panic behavior for misaligned typed offsets, if this
       can be tested deterministically.

8. Scope guard.
   - Do not port `Page`, `PageList`, `ref_counted_set`, or any allocator logic.
   - Do not add dependencies.
   - Do not expose public ABI.
   - Do not hide unsafe pointer conversion behind safe references.

9. Format and test.
   - Run `cargo fmt` after Rust edits and accept its output.
   - Run:

     ```bash
     cargo test -p roastty terminal::size
     cargo test -p roastty
     ```

10. Record the result.
    - Append `## Result` and `## Conclusion` to this file.
    - Update the Issue 801 README experiment index from `Designed` to `Pass`,
      `Partial`, or `Fail`.

## Verification

The experiment passes if:

- `terminal::size` is implemented in Roastty with no C ABI changes;
- all five upstream `size.zig` tests are ported or have documented equivalents;
- the unsafe pointer conversion boundary is narrow and documented;
- alignment and offset behavior match upstream tests;
- `cargo fmt` is run and accepted;
- `cargo test -p roastty terminal::size` passes;
- `cargo test -p roastty` passes;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- the integer constants and offset wrappers are ported, but one of the unsafe
  pointer helpers needs redesign before `Page` can depend on it.

The experiment fails if:

- it starts porting page/grid storage;
- it exposes new C ABI;
- it uses unsafe pointer conversion without documenting the invariant;
- it creates safe references from arbitrary derived pointers without an unsafe
  call boundary;
- it cannot pass the targeted Roastty tests.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or implementing the port.

## Result

**Result:** Pass

Roastty now has a `terminal::size` module ported from Ghostty's
`terminal/size.zig`.

Implemented files:

- `roastty/src/terminal/size.rs`
- `roastty/src/terminal/mod.rs` module wiring

The port preserves the upstream support model needed by later page/grid work:

- terminal page size and integer-width aliases;
- `Offset<T>` as transparent `u32` offset storage with `PhantomData<T>`;
- `OffsetSlice<T>` as the Rust equivalent of Zig's nested `Offset(T).Slice`;
- `OffsetBuf` with `init`, `init_offset`, `start`, `member`, `add`, and
  `rebase`;
- `get_offset` from a base pointer and typed pointer;
- checked `usize -> u32` conversion for offset creation;
- alignment assertions before typed pointer conversion.

The unsafe boundary is intentionally narrow. The module returns raw typed
pointers from offset conversion and requires an unsafe call only when turning an
`OffsetSlice<T>` into a Rust slice reference. The safety invariant is documented
at that conversion point: callers must ensure the derived pointer is valid for
the requested length and lifetime.

All upstream tests from `size.zig` were ported or matched with direct Rust
equivalents:

- `Offset` -> `offset`
- `Offset ptr u8` -> `offset_ptr_u8`
- `Offset ptr structural` -> `offset_ptr_structural`
- `getOffset bytes` -> `get_offset_bytes`
- `getOffset structs` -> `get_offset_structs`

Additional Rust-specific tests were added for the boundary that Rust makes
explicit:

- `Offset<T>` size/alignment parity with `u32`;
- `OffsetSlice<T>` address/range behavior;
- `OffsetBuf::member`;
- `OffsetBuf::add`;
- `OffsetBuf::rebase`;
- misaligned typed pointer panic behavior;
- pointer-before-base rejection;
- offset-too-large rejection.

No public C ABI or header files changed.

### Verification

Ran `cargo fmt` after Rust edits and accepted the output.

Ran:

```bash
cargo test -p roastty terminal::size
cargo test -p roastty
```

Both commands passed. The targeted command ran 12 `terminal::size` tests. The
full Roastty test suite ran 22 Rust unit tests, 1 C ABI harness integration
test, and doc tests.

### Completion Review

Codex reviewed the completed implementation and result and found no blocking
correctness, style, integration, or documentation issues. It confirmed the port
preserves the upstream `size.zig` behavior, keeps the unsafe boundary narrow,
covers layout/range/offset-buffer/slice behavior with tests, and accurately
records the result.

## Conclusion

Experiment 4 succeeds. Roastty now has the offset and size primitives needed by
future page/grid storage work, with the first deliberate unsafe terminal-core
boundary kept small and tested.

The next experiment can either port another small terminal support module, or
begin designing the first page-storage slice that uses `terminal::size`. Because
page storage is pointer-heavy and substantially riskier, the next step should
probably be a design-only experiment that decomposes `page.zig` into smaller
safe/unsafe implementation slices before porting it.
