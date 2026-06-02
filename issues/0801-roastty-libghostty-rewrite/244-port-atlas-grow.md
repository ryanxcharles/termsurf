+++
[implementer]
agent = "claude-code"
model = "claude-opus-4-8"
reasoning = "high"

[review.design]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"

[review.result]
agent = "codex"
model = "gpt-5.5"
reasoning = "medium"
+++

# Experiment 244: Port Atlas `grow`, `set_from_larger`, and `dump`

## Description

Complete the texture atlas port from upstream `font/Atlas.zig`. Experiment 243
ported the allocation core (`new`/`clear`, `reserve`/`fit`/`merge`, `set`); this
experiment adds the rest of the public surface: `grow` (enlarge the texture,
preserving written data), `set_from_larger` (a strided copy from a larger source
buffer), and `dump` (a PPM debug writer). It carries the remaining upstream
tests — `writing data from a larger source`, `grow`, and `grow BGR` — and closes
out `Atlas.zig`.

This is a small, coherent slice over the type already in place: one resize
mechanism plus two copy helpers, with the upstream tests as exact pass criteria.

### `grow` (lines 314–364)

```zig
pub fn grow(self: *Atlas, alloc, size_new: u32) Allocator.Error!void {
    assert(size_new >= self.size);
    if (size_new == self.size) return;
    try self.nodes.ensureUnusedCapacity(alloc, 1);
    const data_new = try alloc.alloc(u8, size_new * size_new * self.format.depth());
    const data_old = self.data;
    const size_old = self.size;
    self.data = data_new;
    self.size = size_new;
    defer alloc.free(data_old);
    @memset(self.data, 0);
    self.set(.{ .x = 0, .y = 1, .width = size_old, .height = size_old - 2 },
             data_old[size_old * self.format.depth() ..]);   // skip first border row
    self.nodes.appendAssumeCapacity(.{ .x = size_old - 1, .y = 1, .width = size_new - size_old });
    _ = self.modified.fetchAdd(1, .monotonic);
    _ = self.resized.fetchAdd(1, .monotonic);
}
```

Asserts the new size is not smaller; no-ops if equal. Allocates a new zeroed
buffer, then copies the old texture's interior rows back via `set`: the source
region is `{ x: 0, y: 1, width: size_old, height: size_old - 2 }` reading from
`data_old` offset by one full row (`size_old * depth`) — i.e. it copies the
`size_old - 2` rows between the top and bottom border rows, full-width (so no
stride is needed; `x = 0`, `width = size_old`). It then appends one new node for
the added right-hand strip
(`{ x: size_old - 1, y: 1, width: size_new - size_old }`) and bumps **both**
`modified` and `resized`. (Because it calls `set`, which also bumps `modified`,
a `grow` bumps `modified` twice and `resized` once — only the "did it change"
signal matters, so the doubled increment is harmless and upstream-faithful.)

### `set_from_larger` (lines 280–306)

Like `set`, but the source row stride is `src_width` and the copy starts at
`(src_x, src_y)` in the source, so a sub-rectangle of a larger buffer can be
copied in: per row `i`,
`src_offset = ((src_y + i) * src_width + src_x) * depth`,
`tex_offset = ((reg.y + i) * size + reg.x) * depth`, copying `reg.width * depth`
bytes; bumps `modified`. Same four border asserts as `set`.

### `dump` (lines 385–404)

Writes the atlas as a PPM to a writer, for debugging: header `P5` (grayscale) or
`P6` (BGR) followed by `{size} {size}\n255\n` then the raw bytes. BGRA (or any
other format) is unsupported and panics. (BGR is written as-is, so red/blue are
swapped versus true RGB — an upstream-documented debug-only wart.)

### Rust mapping (all in `roastty/src/font/atlas.rs`, on `impl Atlas`)

- `pub(crate) fn grow(&mut self, size_new: u32)`: **infallible** (no allocator
  to thread; `Vec` allocation aborts on OOM). `assert!(size_new >= self.size)` —
  an always-on assert (not `debug_assert!`) placed **before** any state change,
  matching upstream's `assert` and ensuring a shrink misuse panics with the
  atlas intact rather than after `mem::replace` has already swapped the buffer.
  Early-return on `size_new == self.size`. Then:
  - `let depth = self.format.depth() as usize;`
  - `let size_old = self.size;`
  - `let data_old = std::mem::replace(&mut self.data, vec![0u8; size_new as usize * size_new as usize * depth]);`
    (the new buffer is already zeroed by `vec!`, so no separate memset);
  - `self.size = size_new;`
  - `self.set(Region { x: 0, y: 1, width: size_old, height: size_old - 2 }, &data_old[size_old as usize * depth..]);`
    (`data_old` is a separate local, so the `&mut self` / `&data_old` borrows do
    not alias);
  - `self.nodes.push(Node { x: size_old - 1, y: 1, width: size_new - size_old });`
    (`Vec::push`, the analog of `appendAssumeCapacity` after the upstream
    `ensureUnusedCapacity` — Rust grows the `Vec` as needed);
  - `self.modified.fetch_add(1, Ordering::Relaxed); self.resized.fetch_add(1, Ordering::Relaxed);`.
- `pub(crate) fn set_from_larger(&mut self, reg: Region, src: &[u8], src_width: u32, src_x: u32, src_y: u32)`:
  the four `debug_assert!` border checks, then the per-row `copy_from_slice`
  with `src_offset = ((src_y + i) * src_width + src_x) * depth` and
  `tex_offset = ((reg.y + i) * size + reg.x) * depth` (`usize` math), then bump
  `modified`.
- `pub(crate) fn dump<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()>`:
  match `format` to the magic **character** (`Grayscale → '5'`, `Bgr → '6'`,
  `Bgra → panic!` with a clear message — the macOS port has no live `Bgra` dump
  path and upstream panics identically), then
  `write!(w, "P{}\n{} {}\n255\n", magic, self.size, self.size)`, then
  `w.write_all(&self.data)`. The magic must be a `char` (or written as a byte) —
  formatting a `u8` with `{}` would print its decimal value (`b'5'` → `53`,
  yielding `P53`); upstream uses character formatting. (`std::io::Write` is the
  faithful analog of upstream's `std.Io.Writer`.)

### Faithfulness and scope notes

- **`grow OOM`/`grow error` are not ported** (the same infallible-allocation
  reason as `init error`/`reserve error` in Exp 243): they inject a failing
  allocator to assert `OutOfMemory` and that `modified`/`resized` stay
  unchanged. Rust's default allocation aborts on OOM, so there is no error to
  return and no partial state to verify. Documented per the issue's Test Parity
  rule; the not-ported set is now
  `init error`/`reserve error`/`grow error`/`grow OOM`.
- `grow` reuses `set` exactly as upstream does, so the interior-row copy and the
  border handling stay identical to the core slice.
- After this experiment `Atlas.zig` is fully ported except the WASM bindings
  (out of scope, macOS-only).
- No C ABI, header, or ABI inventory changes; no new dependencies (std only).

## Changes

1. `roastty/src/font/atlas.rs`: add `grow`, `set_from_larger`, and `dump` to
   `impl Atlas`; update the module doc comment to note the atlas is complete
   (minus WASM).

2. Tests in `roastty/src/font/atlas.rs` — port the remaining upstream tests:
   - `writing_data_from_larger_source`: `new(32, Grayscale)`, `reserve(2, 2)`,
     `set_from_larger(reg, &[..5×4 grid..], 5, 2, 1)`; bumps `modified`;
     `data[33]==1, data[34]==2, data[65]==3, data[66]==4`, and **no** `8` (the
     out-of-region source filler) appears anywhere in `data`.
   - `grow_preserves_data`: `new(4, Grayscale)`, `reserve(2, 2)`, then
     `reserve(1, 1)` is `AtlasFull`; `set(reg, &[1,2,3,4])` → `data[5..]==1,2`
     and `data[9..]==3,4`; `grow(size + 1)` bumps `modified` and `resized`;
     `reserve(1, 1)` now succeeds; data preserved at the new offsets
     (`data[size+1]==1, data[size+2]==2, data[size*2+1]==3, data[size*2+2]==4`).
   - `grow_bgr`: `new(4, Bgr)`, `reserve(2, 2)`, `reserve(1, 1)` is `AtlasFull`;
     `set` the 4×3-byte block; verify the top-left/next-row BGR offsets and the
     border zero before and after `grow(size + 1)`; then `reserve(1, 3)` and
     `reserve(2, 1)` succeed and a final `reserve(1, 1)` is `AtlasFull`.
   - `dump_grayscale_header` (a Rust-side equivalent, since upstream has no
     `dump` test): `new(4, Grayscale)`, `dump` into a `Vec<u8>`, assert it
     begins with `b"P5\n4 4\n255\n"` and that the remaining bytes equal
     `self.data` (length `4*4*1`).

3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo test -p roastty font
cargo test -p roastty
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `grow` enlarges the texture, preserves the interior rows at the new offsets
  via `set`, appends the right-hand node, and bumps `modified` and `resized`; a
  `size_new == size` call is a no-op and a smaller size is rejected;
- `set_from_larger` copies the correct sub-rectangle with the source stride and
  bumps `modified`;
- `dump` writes the correct PPM header and the raw data for grayscale/BGR;
- the four ported tests (`writing_data_from_larger_source`,
  `grow_preserves_data`, `grow_bgr`, `dump_grayscale_header`) pass with the
  exact expected values;
- the `grow OOM`/`grow error` tests are documented as not ported;
- no C ABI, header, or ABI inventory changes;
- `cargo fmt` accepted and `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if `grow` reveals the node list or offset typing
needs a representation change that should be its own slice.

The experiment **fails** if `grow` loses or misplaces data, bumps the wrong
counters, or mishandles the no-op/shrink cases, if `set_from_larger` reads the
wrong source offsets, if `dump` writes the wrong header, or if any public C
API/ABI changes.

## Design Review

Codex reviewed this design before implementation.

Review artifacts:

- Prompt: `logs/codex-review/20260602-092230-473173-prompt.md`
- Result: `logs/codex-review/20260602-092230-473173-last-message.md`

Codex traced the `grow` copy math and confirmed it is correct (old row 1 at
`size_old * depth`, copying `size_old - 2` full-width rows into new `y = 1`
preserves grayscale offsets `5/6/9/10` before grow and
`size+1/size+2/size*2+1/size*2+2` after), that `set_from_larger`'s offsets match
upstream, that the doubled `modified` bump (from `set` plus `grow`) is faithful
and harmless, that the `mem::replace` ownership is borrow-safe, and that
omitting the OOM-injection tests is acceptable under the infallible-allocation
model.

Two findings, both fixed in the design above before this commit:

1. **High — `dump` magic byte must be character-formatted.** The mapping used
   `b'5'`/`b'6'` with `write!("P{}…", magic)`; in Rust `{}` on a `u8` prints its
   decimal value (`53`), so the header would read `P53`/`P54`. Fixed to use a
   `char` magic (`'5'`/`'6'`) so the header is `P5`/`P6`, matching upstream's
   character formatting.
2. **Medium — `grow` shrink precondition must be always-on `assert!`.** The
   design used `debug_assert!(size_new >= self.size)`; in a release build a
   shrink would then proceed into `mem::replace` (mutating
   `self.data`/`self.size`) before panicking on a later slice. Upstream asserts
   up-front. Fixed to `assert!` placed before any state change, so a misuse
   panics with the atlas intact.

## Result

**Result:** Pass

Added `set_from_larger`, `grow`, and `dump` to `impl Atlas` in
`roastty/src/font/atlas.rs`, and updated the module doc to note the atlas's full
public surface is ported (minus WASM). `grow` asserts up-front
(`assert!(size_new >= self.size)`), no-ops on equal size, swaps in a new zeroed
`Vec` via `std::mem::replace`, copies the interior rows back through `set`
(`Region { x: 0, y: 1, width: size_old, height: size_old - 2 }` from
`&data_old[size_old * depth..]`), appends the right-hand node, and bumps
`modified` and `resized`. `set_from_larger` mirrors `set` with the
`src_width`/`src_x`/`src_y` source stride. `dump` writes `P5`/`P6` (via a `char`
magic, so the header is `P5` not `P53`), `{size} {size}\n255\n`, then the raw
data; `Bgra` panics.

Tests added (4): `writing_data_from_larger_source`, `grow_preserves_data`,
`grow_bgr` (the three ported upstream tests), and `dump_grayscale_header` (a
Rust-side equivalent, since upstream has no `dump` test). The
`grow OOM`/`grow error` upstream tests are not ported (infallible `Vec`
allocation, per the Faithfulness notes).

### Verification

```bash
cargo fmt -p roastty
cargo test -p roastty atlas
cargo test -p roastty
```

Observed:

- `atlas`: 10 passed (6 prior + 4 new).
- Full `roastty`: 2328 unit tests passed (2324 prior + 4 new), plus the C ABI
  harness passed.
- `cargo fmt -p roastty -- --check`: clean.
- `cargo build -p roastty`: no warnings.
- No-`ghostty`-name gates passed for `roastty/src/font` and for
  `roastty/src/lib.rs`, `roastty/include/roastty.h`,
  `roastty/tests/abi_harness.c`.
- `git diff --check`: clean.

The `grow` data-copy offsets reproduced exactly (grayscale `5/6/9/10` before
grow and `size+1/size+2/size*2+1/size*2+2` after; BGR top-left/next-row offsets
and the border zero, before and after grow). No C ABI, header, or ABI inventory
changes. `Atlas.zig` is now fully ported except the WASM bindings.

### Completion Review

Codex reviewed the completed implementation and found **no issues** ("nothing
needs to change before the result commit").

Review artifacts:

- Prompt: `logs/codex-review/20260602-092617-775920-prompt.md`
- Result: `logs/codex-review/20260602-092617-775920-last-message.md`

Codex confirmed the design-gate fixes are correctly implemented (`grow`'s
up-front `assert!` before mutation, `dump`'s `char` magic so the header is
`P5`/`P6`), that `grow` matches upstream (no-op on equal size, non-aliasing
`mem::replace`, interior-row copy through `set`, right-hand node appended, both
`modified` and `resized` bumped), that `set_from_larger` uses the correct
source/texture offsets and bumps `modified`, and that the four tests cover the
expected upstream values and the Rust `dump` equivalent.

## Conclusion

Experiment 244 succeeds, completing the texture `Atlas` port. `grow`,
`set_from_larger`, and `dump` are in place atop the Exp 243 core, so `Atlas.zig`
is fully reproduced (minus the out-of-scope WASM bindings), with all of its
non-OOM tests ported. Both Codex gates passed (two design findings fixed — the
`dump` character-format bug and the `grow` always-on assert; zero result
findings).

With the atlas done, the font layer's remaining work is the face/rasterization
path: the CoreText `Face` that produces a glyph bitmap and the `FaceMetrics`
feeding `Metrics::calc`, the `Glyph`/atlas write that rasterization performs,
and above them the `Collection`/`SharedGrid` glyph cache. The next experiment
will port the smallest coherent next type there — likely the `FaceMetrics`
source side of the CoreText face (the metrics extraction that the already-ported
`Metrics::calc` consumes) or the glyph-rasterization result type — keeping the
same one-surface, predictable-tests sizing.
