# Experiment 2: Define Zig-to-Rust Porting Patterns

## Description

Before porting a real `libghostty` subsystem into `libroastty`, define the code
translation patterns Roastty will use when adapting Ghostty's Zig code to Rust.

This experiment is diagnostic and architectural. It should study representative
upstream Ghostty Zig code and record practical Rust translation rules. It should
not port a subsystem yet. The goal is to prevent every later experiment from
re-deciding the same questions about Zig allocators, `comptime`, tagged unions,
error unions, packed structs, pointer-heavy terminal data structures, C ABI
handles, tests, and `unsafe` Rust.

Roastty is macOS-only. Non-macOS Zig branches should be classified as omitted
unless they reveal a reusable pattern needed by the macOS implementation.

## Questions

Answer these questions in the result:

1. What recurring Zig language patterns appear in the Ghostty subsystems Roastty
   will port?
2. What Rust pattern should Roastty use for each recurring Zig pattern?
3. When is `unsafe` Rust acceptable during the initial faithful port?
4. Which upstream patterns should be preserved closely for behavior parity, and
   which should be simplified because Roastty is macOS-only?
5. How should Roastty translate upstream tests so behavior parity is proven?
6. What pattern decisions should the first real subsystem port follow?

## Changes

1. Inspect representative upstream Ghostty code.
   - Use `vendor/ghostty/` as the source of truth.
   - Inspect at least:
     - `vendor/ghostty/src/main_c.zig`
     - `vendor/ghostty/include/ghostty.h`
     - `vendor/ghostty/src/config/Config.zig`
     - `vendor/ghostty/src/Command.zig`
     - `vendor/ghostty/src/pty.zig`
     - `vendor/ghostty/src/termio/Exec.zig`
     - `vendor/ghostty/src/terminal/Tabstops.zig`
     - `vendor/ghostty/src/terminal/Screen.zig`
     - `vendor/ghostty/src/terminal/PageList.zig`
     - `vendor/ghostty/src/terminal/page.zig`
     - `vendor/ghostty/src/terminal/ref_counted_set.zig`
     - `vendor/ghostty/src/datastruct/split_tree.zig`
     - `vendor/ghostty/src/datastruct/intrusive_linked_list.zig`
     - `vendor/ghostty/src/datastruct/segmented_pool.zig`
     - `vendor/ghostty/src/App.zig`
     - `vendor/ghostty/src/Surface.zig`
     - `vendor/ghostty/src/renderer/Thread.zig`
     - `vendor/ghostty/src/termio/mailbox.zig`
     - `vendor/ghostty/src/font/backend.zig`
     - `vendor/ghostty/src/renderer/Metal.zig`
     - `vendor/ghostty/src/apprt/surface.zig`
   - Do not modify `vendor/ghostty/`.

2. Inspect current Roastty code.
   - Inspect at least:
     - `roastty/src/lib.rs`
     - `roastty/include/roastty.h`
     - `roastty/ABI_INVENTORY.md`
     - `roastty/tests/`
   - Record how current ABI/lifecycle patterns should evolve or remain stable.

3. Produce a Zig-to-Rust translation table.
   - Include at least these rows:
     - `comptime` build/config switches
     - `switch (builtin.os.tag)` platform gates
     - Zig tagged unions
     - Zig error unions and error sets
     - optional pointers and nullable values
     - allocators and arenas
     - `defer`, `errdefer`, transactional rollback, and allocation failure
     - `ArrayList`, slices, sentinel slices, and null-terminated strings
     - packed structs and bitfields
     - integer widths, casts, overflow behavior, bitsets, and packed storage
     - `extern struct` / C ABI layout
     - opaque C handles
     - callbacks/userdata
     - manual `deinit` patterns
     - pointer-heavy page/grid structures
     - intrusive/reference-counted state
     - threads, mailboxes, mutexes, atomics, and event delivery
     - tests embedded beside implementation
     - `@compileError` and unreachable platform paths

4. Define the unsafe Rust policy for Issue 801.
   - `unsafe` is allowed for the initial faithful port when it is the clearest
     way to preserve behavior, layout, or ABI semantics.
   - `unsafe` must be localized to small modules/functions.
   - Each `unsafe` block must have a short safety comment explaining the
     invariant.
   - ABI-facing and packed-memory ports must explicitly choose `repr(C)`,
     `repr(transparent)`, or ordinary Rust layout and explain why.
   - Layout-sensitive ports must include `size_of` / `align_of` assertions where
     layout parity matters.
   - Pointer-heavy ports must document ownership, lifetime, aliasing, and
     pointer provenance at the unsafe boundary.
   - Safe public APIs should not expose unsafe requirements unless an experiment
     explicitly justifies that shape.
   - Tests must cover the behavior or layout invariant that justifies the unsafe
     code.
   - Do not use `unsafe` to bypass ownership thinking when safe Rust is equally
     direct.
   - Do not start a broad unsafe cleanup effort inside Issue 801. Cleanup can be
     a later sweep once behavior parity exists.

5. Define behavior-parity rules.
   - Preserve upstream behavior unless an experiment explicitly records a
     Roastty-specific divergence.
   - Prefer local Rust idioms only when they do not alter observable behavior.
   - Keep macOS-only simplifications direct: remove non-macOS branches rather
     than preserving cross-platform abstraction layers by habit.
   - If exact behavior is uncertain, port the upstream test or create an
     equivalent test before changing the implementation shape.

6. Define test translation rules.
   - Record how to translate Zig `test` blocks into Rust unit or integration
     tests.
   - Record when upstream C examples should become Rust/C ABI integration tests.
   - Record when Swift/UI tests should be deferred to the app integration phase.
   - Require each future subsystem experiment to name the upstream tests it is
     porting or intentionally deferring.

7. Verify the diagnostic-only boundary.
   - Before recording the result, run:

     ```bash
     git status --short
     ```

   - Expected changed files are limited to Issue 801 documentation and
     gitignored review logs under `logs/`.
   - This experiment must not modify `roastty/`, `vendor/ghostty/`,
     `Cargo.toml`, `Cargo.lock`, scripts, build configuration, or source code.

8. Record the result inside this experiment file.
   - Append `## Result` and `## Conclusion` to this file.
   - Include these tables:
     - `Representative Source Patterns`
     - `Zig-to-Rust Translation Rules`
     - `Unsafe Rust Policy`
     - `Test Translation Rules`
     - `Patterns for the First Real Port`
     - `Open Pattern Questions`
   - Update the Issue 801 README experiment index status from `Designed` to
     `Pass`, `Partial`, or `Fail` after the result is recorded.

## Verification

The experiment passes if:

- the result cites concrete upstream Ghostty files for each major pattern;
- every required pattern category has a Rust translation rule;
- the unsafe Rust policy is explicit and actionable;
- behavior-parity and macOS-only simplification rules are explicit;
- test translation rules are explicit;
- the result recommends a concrete next implementation slice;
- `git status --short` confirms the diagnostic-only boundary was preserved;
- Codex reviews the completed result and approves it or all real findings are
  fixed.

The experiment is partial if:

- most patterns are classified, but one or two major patterns need a follow-up
  before the first real port can safely begin.

The experiment fails if:

- it starts porting production code instead of defining translation patterns;
- it leaves `unsafe` policy ambiguous;
- it fails to provide a concrete next implementation slice;
- it preserves non-macOS branches as live Roastty requirements without
  justification.

## Codex Review

This experiment design must be reviewed by Codex before implementation. Any real
design issues must be fixed before committing the plan or running the audit.

## Result

**Result:** Pass

The audit found enough recurring patterns to define a practical translation
policy before porting the first real subsystem. The main conclusion is that
Roastty should not try to mechanically preserve Zig's implementation mechanics
everywhere. It should preserve behavior, observable failure modes, C ABI layout,
and test coverage. Safe Rust is the default, but `unsafe` is acceptable where
layout, ABI, stable-address, OS, or pointer-heavy terminal data structures make
that the clearest faithful port.

No source code was changed.

### Representative Source Patterns

| Pattern                               | Upstream examples                                                                                                      | Notes                                                                                                                                                                                                 |
| ------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| C ABI surface                         | `vendor/ghostty/src/main_c.zig`, `vendor/ghostty/include/ghostty.h`, `roastty/src/lib.rs`, `roastty/include/roastty.h` | Ghostty exposes opaque handles, `extern struct` values, callback userdata, and string ownership rules. Roastty already mirrors this with `#[repr(C)]`, raw C handles, and `roastty_string_free`.      |
| Build-time specialization             | `Config.zig`, `pty.zig`, `font/backend.zig`, `renderer/Metal.zig`                                                      | Zig uses `comptime`, build options, and `builtin.os.tag`. Roastty is macOS-only, so non-macOS branches should be omitted unless they are needed as test fixtures or source comparison notes.          |
| Platform process and PTY work         | `Command.zig`, `pty.zig`, `termio/Exec.zig`                                                                            | The macOS path uses POSIX PTYs, `termios`, null-terminated command strings, pre/post fork callbacks, `errdefer`, and child/parent process boundaries.                                                 |
| Allocation and rollback               | `Tabstops.zig`, `PageList.zig`, `page.zig`, `split_tree.zig`, `termio/mailbox.zig`                                     | Zig relies on explicit allocators, `defer`, `errdefer`, preheated pools, page-aligned allocation, arenas, and failure tests. Rust ports need transactional constructors and failure-preserving tests. |
| Terminal bitsets and integer behavior | `Tabstops.zig`, `page.zig`, `ref_counted_set.zig`, `split_tree.zig`                                                    | The code uses fixed integer widths, explicit casts, bit masks, wrapping addition, reserved IDs, packed flags, and bounded handle types. Rust ports must avoid implicit narrowing.                     |
| Pointer-heavy terminal storage        | `PageList.zig`, `page.zig`, `ref_counted_set.zig`, `intrusive_linked_list.zig`, `segmented_pool.zig`                   | Ghostty uses intrusive links, stable pointers, offset-based page layouts, pooled memory, and serial numbers to detect reused pages. This is the highest-risk area for unsafe Rust.                    |
| Immutable/tree state                  | `split_tree.zig`                                                                                                       | Split trees are arena-backed immutable snapshots with ref-counted views and many embedded tests. Rust can use owned vectors/arenas plus explicit reference semantics.                                 |
| Thread and mailbox behavior           | `App.zig`, `renderer/Thread.zig`, `termio/mailbox.zig`, `Surface.zig`                                                  | Ghostty uses bounded queues, wakeups, app/runtime mailboxes, renderer and IO threads, mutex unlock/relock around blocking sends, and event-loop integration.                                          |
| Tagged event/message APIs             | `apprt/surface.zig`, `App.zig`, `Surface.zig`                                                                          | Zig `union(enum)` values map naturally to Rust enums; C ABI surfaces may need explicit tags and payload storage.                                                                                      |
| Tests beside implementation           | `Tabstops.zig`, `split_tree.zig`, `segmented_pool.zig`, `intrusive_linked_list.zig`                                    | Upstream Zig tests are directly usable as behavior specs. Future subsystem ports must name and port the relevant tests.                                                                               |

### Zig-to-Rust Translation Rules

| Zig pattern                                                  | Roastty Rust rule                                                                                                                                                                                                                                                           |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `comptime` constants and type generation                     | Use Rust `const`, associated constants, generics, and build-time generated constants only when needed. Do not recreate Zig metaprogramming if a concrete macOS type is clearer.                                                                                             |
| `switch (builtin.os.tag)` and `@compileError` platform gates | Keep only the macOS behavior. Remove Linux, Windows, GTK, Wayland, X11, OpenGL, and other non-macOS paths from live code. Record any omitted branch only when it affects behavior understanding.                                                                            |
| Zig tagged unions / `union(enum)`                            | Use Rust enums with variant payloads. Use `Box` only when variant size or recursive structure requires indirection. Use explicit C tags only at ABI boundaries.                                                                                                             |
| Error unions and error sets                                  | Use `Result<T, RoasttyError>` or a subsystem-scoped error enum. At the C ABI, map errors to the existing integer/bool/null conventions and diagnostics.                                                                                                                     |
| Optional pointers and nullable values                        | Use `Option<T>`, `Option<NonNull<T>>`, or references internally. Accept raw nullable pointers only at the C boundary and convert immediately.                                                                                                                               |
| Allocators and arenas                                        | Prefer owned Rust containers first (`Vec`, `String`, `Box`, `Arc`). Use arenas or bump allocation only when lifetime grouping or snapshot semantics are part of Ghostty behavior, as in `split_tree.zig`.                                                                   |
| `defer`, `errdefer`, and transactional rollback              | Use RAII/`Drop`, temporary builders, and commit-after-success constructors. If upstream preserves state on allocation failure, the Rust test must prove the same behavior.                                                                                                  |
| Allocation failure                                           | Do not ignore upstream failure semantics. For small ports, use fallible APIs such as `try_reserve` where the upstream test depends on allocation failure. For cases Rust cannot force deterministically, document the gap and add the closest deterministic invariant test. |
| `ArrayList`, slices, sentinel slices, and `[:0]const u8`     | Use `Vec<T>`, `&[T]`, `CString`, `CStr`, or small newtypes that state whether a value is sentinel-terminated. Never assume null termination unless the type says so.                                                                                                        |
| Packed structs, bitfields, and bitsets                       | Prefer explicit integer storage plus accessor methods or `bitflags`. Use `#[repr(C)]`/`#[repr(transparent)]` only when layout is externally observable. Add size/alignment tests where layout matters.                                                                      |
| Integer casts, widths, overflow, and wrapping                | Use explicit `try_from`, `from`, `checked_*`, `wrapping_*`, `saturating_*`, or asserted casts. Match upstream wrapping operations like `+%=` deliberately, not accidentally.                                                                                                |
| `extern struct` and ABI layout                               | Use `#[repr(C)]` and C-compatible scalar types. Add layout assertions for structs shared with Swift/C or compared against upstream headers.                                                                                                                                 |
| Opaque C handles                                             | Store Rust-owned values behind `Box::into_raw` and recover them exactly once in the matching free function. Internally wrap non-null handles in `NonNull` where possible.                                                                                                   |
| Callbacks and userdata                                       | Use `Option<unsafe extern "C" fn(...)>` plus `*mut c_void` at ABI boundaries. Safe wrappers must document callback lifetime, thread, and reentrancy expectations.                                                                                                           |
| Manual `deinit` patterns                                     | Use `Drop` for Rust-owned resources and explicit `*_free` ABI functions for C-owned handles. Avoid double ownership between safe wrappers and raw ABI handles.                                                                                                              |
| Pointer-heavy page/grid structures                           | Start with safe indices, offsets, `Vec`, slabs, or generational IDs. Introduce unsafe pointer layout only after tests prove the safe version cannot preserve required behavior, performance, or layout.                                                                     |
| Intrusive lists and ref-counted state                        | Prefer safe containers (`VecDeque`, slab indices, `Rc`, `Arc`) for first ports. If stable addresses are observable, use a small unsafe module with documented ownership and mutation invariants.                                                                            |
| Threads, mailboxes, mutexes, atomics, wakeups                | Preserve bounded capacity, backpressure, wakeup ordering, and blocking semantics. Choose Rust channels/queues first, but test behavior that upstream depends on, such as wake-after-send and full-queue behavior.                                                           |
| Embedded Zig tests                                           | Port as Rust unit tests beside the module unless the behavior crosses the C ABI, in which case use integration tests. Keep upstream test names in comments or test names for traceability.                                                                                  |

### Unsafe Rust Policy

| Allowed use                                         | Required guardrail                                                                                                                               |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| C ABI pointer conversion and opaque handles         | Convert raw pointers at the boundary, null-check first, and keep ownership single and explicit.                                                  |
| FFI callbacks/userdata                              | Keep the unsafe call in a narrow wrapper and document callback thread/lifetime assumptions.                                                      |
| `repr(C)`/`repr(transparent)` layout                | Add size/alignment assertions when the layout is consumed by C, Swift, Objective-C, or an upstream-equivalent test.                              |
| Page-aligned or offset-based terminal memory        | Put unsafe pointer arithmetic in a dedicated module with safe methods, bounds checks, and tests for layout, offsets, resize, and copy behavior.  |
| Intrusive/stable-address data structures            | Prefer safe index-based ports first. If unsafe intrusive links are used, node ownership and mutation rules must be documented at the type level. |
| OS calls and Objective-C/Metal/CoreText integration | Keep system calls behind small wrappers whose safe API states ownership and thread constraints.                                                  |

`unsafe` is not allowed as a shortcut around ownership design. It is allowed as
a temporary faithfulness tool when behavior parity is otherwise likely to drift.
Unsafe cleanup should be a later issue once Roastty has working parity.

### Test Translation Rules

| Upstream test source               | Roastty rule                                                                                                                                        |
| ---------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| Zig `test` blocks in small modules | Port directly into Rust `#[cfg(test)]` module tests. Keep the upstream test intent and edge cases.                                                  |
| Allocation-failure tests           | Preserve when observable behavior depends on rollback. Use fallible Rust APIs or a deterministic test allocator/helper where practical.             |
| C ABI examples and header behavior | Convert to Rust integration tests and, where needed, small C compile/link smoke tests against `roastty/include/roastty.h`.                          |
| Swift/macOS app behavior           | Defer until the Swift frontend integration phase unless the library can expose the behavior through deterministic C/Rust tests first.               |
| Large subsystem integration tests  | Port after the relevant module slice exists. Each experiment must list tests ported, tests deferred, and why.                                       |
| Non-macOS tests                    | Omit unless they validate platform-independent terminal behavior. Do not keep Linux/Windows behavior alive just to satisfy upstream platform tests. |

### Patterns for the First Real Port

The first implementation experiment should port
`vendor/ghostty/src/terminal/Tabstops.zig` into a new Rust module such as
`roastty/src/terminal/tabstops.rs`.

This is the right first slice because it is small, behavior-rich, and tests the
translation policy without involving OS integrations or the full terminal page
model. It exercises bitsets, fixed integer operations, dynamic allocation,
allocation-failure rollback, and upstream test parity.

The port should:

1. Preserve the preallocated 512-column bitset behavior.
2. Preserve the dynamic expansion behavior above the preallocated segment.
3. Port the upstream tests for basic set/get/unset, dynamic allocation, interval
   reset, 80-column tabstop count, and resize allocation failure/state
   preservation.
4. Use safe Rust first. `unsafe` should not be needed for `Tabstops`.
5. Use fallible resize behavior where needed to preserve upstream rollback
   semantics.
6. Add only the minimal module wiring needed to run the tests.

### Open Pattern Questions

| Question                                                                                    | Status                                                                                                                                      |
| ------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| Should page/grid storage use exact unsafe contiguous layouts or safe Rust containers first? | Defer until the terminal page slice. The policy is safe-first, but page layout is the most likely justified unsafe area.                    |
| Which allocator/failure-testing helper should Roastty standardize on?                       | The `Tabstops` port should answer this narrowly. If it needs a reusable fallible allocation test helper, design it there.                   |
| How closely should `split_tree.zig` preserve arena-backed immutability?                     | Defer until split tree port. Behavior suggests preserving snapshot semantics, but Rust can likely do this safely.                           |
| What queue implementation should replace Ghostty's bounded blocking queues?                 | Defer until app/runtime threading experiments. The required behavior is bounded send, wakeup, and backpressure, not a specific queue crate. |
| How much of Metal/CoreText should be direct FFI versus higher-level crates?                 | Defer until rendering/font slices. Unsafe wrappers are expected, but the crate choices should be evaluated when those modules are active.   |

### Boundary Check

`git status --short` was clean before recording this result. This experiment
modified only Issue 801 documentation. It did not modify `roastty/`,
`vendor/ghostty/`, workspace files, scripts, build configuration, or source
code.

### Completion Review

Codex reviewed the completed experiment result and found no blocking issues. It
confirmed that the result satisfies the verification criteria, that the unsafe
Rust policy is actionable, and that `Tabstops` is an appropriate first real
port.

## Conclusion

Experiment 2 succeeds. We have a concrete porting policy and enough examples to
begin real implementation without re-litigating every Zig-to-Rust decision.

The next experiment should port `terminal/Tabstops.zig` into Roastty with its
tests. That slice is intentionally small: it validates the behavior-parity,
fallible-allocation, integer, and test-porting rules before the issue moves into
harder terminal structures like pages, scrollback, intrusive storage, PTY IO,
and renderer threading.
