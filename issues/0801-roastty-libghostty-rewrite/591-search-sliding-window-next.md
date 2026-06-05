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

# Experiment 591: search SlidingWindow next (the scan)

## Description

This experiment ports `SlidingWindow.next` from upstream
`terminal/search/sliding_window.zig` — the scan that completes the matcher. It
searches the window's `data` for the needle (case-insensitive ASCII), across the
two ring slices and the cross-boundary `overlap_buf`, calling `highlight`
(Experiment 590) on a hit; special-cases 1-length needles; and on a miss prunes
everything except the trailing `needle.len() - 1` overlap bytes. With this, the
`SlidingWindow` matcher is complete. It extends
`terminal::search::sliding_window`.

## Upstream behavior

```zig
pub fn next(self: *SlidingWindow) ?FlattenedHighlight {
    const data_len = self.data.len();
    if (data_len < self.needle.len) return null;
    const slices = self.data.getPtrSlice(self.data_offset, data_len - self.data_offset);

    // 1. Search the first slice.
    if (indexOfIgnoreCase(slices[0], self.needle)) |idx| return self.highlight(idx, self.needle.len);

    // 2. Search the cross-boundary overlap (only if BOTH slices are non-empty).
    if (slices[0].len > 0 and slices[1].len > 0) overlap: {
        const prefix = last min(slices[0].len, needle.len-1) bytes of slices[0];
        const suffix = first min(slices[1].len, needle.len-1) bytes of slices[1];
        copy prefix ++ suffix into overlap_buf[0..overlap_len];
        const idx = indexOfIgnoreCase(overlap_buf[0..overlap_len], needle) orelse break :overlap;
        return self.highlight(slices[0].len - prefix.len + idx, needle.len);
    }

    // 3. Search the last slice.
    if (indexOfIgnoreCase(slices[1], self.needle)) |idx| return self.highlight(slices[0].len + idx, needle.len);

    // 4. 1-length needles: clear the whole window.
    if (self.needle.len == 1) { self.clearAndRetainCapacity(); self.assertIntegrity(); return null; }

    // 5. No match: prune all but the trailing needle.len-1 overlap bytes.
    prune: {
        var saved = 0;
        // reverse-iterate metas; find the oldest meta to KEEP (the one that covers the remaining
        // `needle.len-1 - saved` overlap bytes); prune every older meta + its data.
        ... (see below) ...
    }
    self.data_offset = self.data.len() - self.needle.len + 1;
    self.assertIntegrity();
    return null;
}
```

The prune loop (step 5) reverse-iterates the metas, accumulating `saved`; for
each, `needed = needle.len - 1 - saved`; the first meta with
`cell_map.len >= needed` is the **oldest meta to keep** (every older meta is
pruned). Two details:

- The loop sets `self.data_offset = cell_map.len - needed` on the keep meta, but
  this is **immediately overwritten** by the final
  `self.data_offset = self.data.len() - self.needle.len + 1` after the prune
  block (it is read nowhere in between) — so it is **vestigial** and dropped.
- If the reverse loop never finds enough (`saved < needle.len - 1` after
  exhausting all metas), nothing is pruned. (Unreachable when
  `data_len >= needle.len`, since the metas' lengths sum to `data_len`; kept as
  a guard.)

## Rust mapping (`roastty/src/terminal/search/sliding_window.rs`)

`getPtrSlice(data_offset, ..)` becomes the two logical slices of
`data[data_offset..]` computed from `VecDeque::as_slices()`:

```rust
let (a, b) = self.data.as_slices();
let (s0, s1) = if self.data_offset <= a.len() {
    (&a[self.data_offset..], b)
} else {
    (&b[self.data_offset - a.len()..], &[][..])
};
```

`std.ascii.indexOfIgnoreCase` becomes a file-local case-insensitive ASCII
substring search. `overlap_buf` is used as the scratch (a disjoint field borrow
from `data`). `deleteOldest` becomes `VecDeque::drain`. The reverse-prune loop
becomes reverse index iteration; the vestigial inner `data_offset` is dropped.

```rust
/// Search the window for the next needle occurrence (upstream `next`). Returns a flattened
/// highlight on a match (pruning consumed pages); on a miss, prunes to the trailing overlap and
/// returns `None`. The needle is assumed non-empty (the searchers guarantee it).
pub(in crate::terminal) fn next(&mut self) -> Option<Flattened> {
    // The needle must be non-empty: an empty needle would make `highlight(0, 0)` underflow
    // `end = start + len - 1` (upstream assumes this too; the searchers always pass a non-empty
    // needle). `new` accepts an empty needle, so encode the precondition with an active assert.
    assert!(!self.needle.is_empty(), "search needle must be non-empty");

    let data_len = self.data.len();
    if data_len < self.needle.len() {
        return None;
    }

    // Search the two ring slices and the cross-boundary overlap, in upstream order. Yields the
    // match's start offset (relative to `data_offset`) or `None`.
    let match_offset: Option<usize> = 'search: {
        let needle = self.needle.as_slice();
        let nlen = needle.len();
        let (a, b) = self.data.as_slices();
        let (s0, s1): (&[u8], &[u8]) = if self.data_offset <= a.len() {
            (&a[self.data_offset..], b)
        } else {
            (&b[self.data_offset - a.len()..], &[][..])
        };

        if let Some(idx) = index_of_ignore_case(s0, needle) {
            break 'search Some(idx);
        }
        if !s0.is_empty() && !s1.is_empty() {
            let nlen1 = nlen - 1;
            let plen = s0.len().min(nlen1);
            let slen = s1.len().min(nlen1);
            let overlap_len = plen + slen;
            debug_assert!(overlap_len <= self.overlap_buf.len());
            self.overlap_buf[..plen].copy_from_slice(&s0[s0.len() - plen..]);
            self.overlap_buf[plen..overlap_len].copy_from_slice(&s1[..slen]);
            if let Some(idx) = index_of_ignore_case(&self.overlap_buf[..overlap_len], needle) {
                break 'search Some(s0.len() - plen + idx);
            }
        }
        if let Some(idx) = index_of_ignore_case(s1, needle) {
            break 'search Some(s0.len() + idx);
        }
        None
    };

    if let Some(off) = match_offset {
        return Some(self.highlight(off, self.needle.len()));
    }

    // 1-length needles: clear the whole window.
    if self.needle.len() == 1 {
        self.clear_and_retain_capacity();
        self.assert_integrity();
        return None;
    }

    // No match: keep the trailing `needle.len() - 1` bytes for the future overlap; prune the rest.
    let nlen1 = self.needle.len() - 1;
    let mut saved = 0usize;
    let mut keep: Option<usize> = None;
    for i in (0..self.meta.len()).rev() {
        let cmlen = self.meta[i].cell_map.len();
        if cmlen >= nlen1 - saved {
            keep = Some(i);
            break;
        }
        saved += cmlen;
    }
    match keep {
        Some(j) if j > 0 => {
            let prune_data_len: usize = (0..j).map(|k| self.meta[k].cell_map.len()).sum();
            self.meta.drain(..j);
            self.data.drain(..prune_data_len);
        }
        Some(_) => {}                       // keep the first meta — nothing older to prune
        None => debug_assert!(saved < nlen1),
    }

    self.data_offset = self.data.len() - self.needle.len() + 1;
    self.assert_integrity();
    None
}
```

with a file-local helper:

```rust
/// Case-insensitive ASCII substring search (upstream `std.ascii.indexOfIgnoreCase`). An empty
/// needle matches at 0.
fn index_of_ignore_case(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if needle.len() > haystack.len() {
        return None;
    }
    (0..=haystack.len() - needle.len()).find(|&i| {
        haystack[i..i + needle.len()]
            .iter()
            .zip(needle)
            .all(|(h, n)| h.eq_ignore_ascii_case(n))
    })
}
```

## Scope / faithfulness notes

- **Ported**: `next` → `SlidingWindow::next`; `std.ascii.indexOfIgnoreCase` →
  the file-local `index_of_ignore_case`.
- **Faithful**: the `data_len < needle.len` early-out; the three-step search
  order (first slice → overlap-if-both-non-empty → last slice), the overlap
  prefix/suffix sizing (`min(slice.len, needle.len - 1)`) and the
  `slices[0].len - prefix.len + idx` / `slices[0].len + idx` offset mapping; the
  1-length clear; the no-match reverse prune (keep the oldest meta covering the
  trailing `needle.len - 1` bytes, prune older metas + their data) and the final
  `data_offset = data.len() - needle.len + 1`; the integrity asserts.
- **Faithful adaptation**: `getPtrSlice` → the two `as_slices()`-derived slices
  of `data[data_offset..]`; `indexOfIgnoreCase` → `index_of_ignore_case`
  (`eq_ignore_ascii_case` per byte); `overlap_buf` stays the scratch (written
  via a disjoint field borrow from `data`); `MetaBuf` reverse iteration + `.idx`
  arithmetic → reverse index iteration computing the keep index `j` directly
  (`prune_count = j`, matching "delete all metas up to but not including the
  keep meta"); `deleteOldest` → `drain`; the `unreachable`/`runtime_safety`
  guards → `debug_assert*`.
- **Vestigial (dropped)**: the inner `data_offset = cell_map.len - needed` on
  the keep meta — it is overwritten by the final
  `data_offset = data.len() - needle.len + 1` with no intervening read.
- **Precondition (active assert)**: the needle must be non-empty — an empty
  needle would make `highlight(0, 0)` underflow `end = start + len - 1`
  (upstream assumes this too). Since `new` accepts an empty needle and `next` is
  safe, this is encoded as an active `assert!` at the top of `next` rather than
  only documented.
- **Deferred**: the higher-level searchers (`active` / `pagelist` / `screen` /
  `viewport`) and the search `Thread`.
- No C ABI/header/ABI-inventory change (internal Rust). Extends
  `terminal::search::sliding_window`.

## Changes

1. `roastty/src/terminal/search/sliding_window.rs`: add `SlidingWindow::next`
   and the file-local `index_of_ignore_case`; update the module doc comment to
   note the matcher (`next`) is complete.
2. Tests (in `sliding_window.rs`):
   - **`index_of_ignore_case` unit**: `"hello world"` / `"world"` → `Some(6)`;
     case-insensitive `"WORLD"` → `Some(6)`; `"abHELlo"` / `"hell"` → `Some(2)`;
     no match → `None`; needle longer than haystack → `None`; empty needle →
     `Some(0)`.
   - **forward match**: append `"hello world"`, `next()` with needle `"world"` →
     `Some` with one chunk, `top_x == 6`, `bot_x == 10`.
   - **case-insensitive match**: needle `"WORLD"` → same result.
   - **no match prunes to overlap tail**: append `"hello world"` (data 12 bytes,
     one meta), needle `"zzzz"` → `next()` is `None`, `meta.len() == 1` (the
     lone meta is the keep meta), `data_offset == 12 - 4 + 1 == 9`.
   - **1-length needle clears**: append `"hello"`, needle `"z"` → `next()` is
     `None` and the window is cleared (`data` and `meta` empty).
   - **empty needle panics**: `next()` on a window built with an empty needle
     panics (the non-empty precondition), via `#[should_panic]`.
   - **sequential matches**: append `"ababab"`, needle `"ab"` → three successive
     `next()` calls each return `Some` (the three occurrences), the fourth
     `None`.
   - **overlap (cross-ring-boundary)**: construct a `data` `VecDeque` whose
     `as_slices()` returns two non-empty slices splitting a needle across the
     boundary (push_back/pop_front to force the wrap), with a single
     dangling-node meta and a **forward** window. The match is positioned to
     span the boundary but stay **within the single meta**, so `highlight` takes
     the within-one-meta path and never dereferences the (dangling) node; the
     test also does not dereference any returned chunk node. `next()` finds the
     overlap match at the mapped offset `s0.len() - prefix.len() + idx`.
     (Codex's review confirmed this is sound under those constraints; a
     real-node wrapped deque would need disproportionate setup.)
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty terminal::search
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config roastty/src/terminal/search && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `next` reproduces upstream's scan (the three-step search order, the overlap
  buffer assembly and offset mapping, the 1-length clear, and the no-match prune
  - final `data_offset`) and `index_of_ignore_case` matches
    `std.ascii.indexOfIgnoreCase` — faithful to
    `terminal/search/sliding_window.zig`;
- the tests pass (helper unit / forward / case / no-match / 1-length /
  sequential / overlap), and the existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the search order, the overlap assembly/offset, the
1-length clear, the prune, or the final `data_offset` diverges from upstream, an
unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed the design and **confirmed the hard parts sound**: (Q1) the inner
`data_offset = cell_map.len - needed` is overwritten on every path that reaches
the final prune tail with no intervening read, so dropping it is faithful; (Q2)
`prune_count = j` is correct (metas `0..j` are older than the keep meta at index
`j`), and `j > 0` matches upstream's `prune_count == 0` no-op; (Q3)
`needle.len - 1 - saved` never underflows for a non-empty needle (`saved` only
advances when the current meta is too small, preserving
`saved < needle.len - 1`); (Q4) the disjoint field borrows (`self.data` /
`self.needle` / `self.overlap_buf`) are accepted when written directly inside
the search block; (Q5/Q6) the three-step search order and the ASCII-insensitive
byte matching match upstream. It found one Required and two Optionals:

- **Required (adopted)**: add an **active** non-empty-needle assert at the top
  of `next` (`assert!(!self.needle.is_empty(), ...)`). Documenting the
  precondition is not enough — `new` accepts an empty needle and `next` is safe,
  so an empty needle would reach `highlight(0, 0)` and underflow. An active
  assert faithfully encodes upstream's assumption without inventing behavior for
  invalid input.
- **Optional (adopted)**: add a `next_empty_needle_panics` (`#[should_panic]`)
  test making the searcher-guarantee contract explicit.
- **Optional (considered, kept as designed)**: the manually-wrapped `VecDeque` +
  dangling-node overlap test is sound as long as the match stays within one meta
  and no returned chunk node is dereferenced (both hold here). A real-node
  wrapped-deque test would be marginally more robust but needs disproportionate
  setup; the test is kept with the constraints documented.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d591-prompt.md`
- Result: `logs/codex-review/20260604-d591-last-message.md`

## Result

**Result:** Pass

`terminal::search::sliding_window` gained `SlidingWindow::next` and the
file-local `index_of_ignore_case`. `next` asserts a non-empty needle, returns
`None` when the data is shorter than the needle, searches `data[data_offset..]`
across the two `as_slices()`-derived ring slices and the cross-boundary
`overlap_buf` (in the upstream s0 → overlap → s1 order, the overlap assembled
via a disjoint field borrow), returns `highlight` on a hit, clears the window
for a 1-length needle, and on a miss reverse-prunes everything before the oldest
meta covering the trailing `needle.len() - 1` bytes and sets
`data_offset = data.len() - needle.len() + 1`. The vestigial inner
`data_offset = cell_map.len - needed` was dropped (overwritten with no
intervening read). `index_of_ignore_case` is a case-insensitive ASCII substring
search (empty needle → `Some(0)`).

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3265 passed, 0 failed (eight new tests; no
  regressions, up from 3257).
- `cargo build -p roastty`: no warnings (the disjoint `overlap_buf` / `data`
  field borrow compiled cleanly).
- no-`ghostty`-name greps (font/renderer/config + terminal/search +
  lib.rs/header/abi_harness.c) clean; `git diff --check` clean.

The eight new tests: the `index_of_ignore_case` unit (case-insensitive,
no-match, needle-longer, empty); a forward match (`"world"` → `top_x 6` /
`bot_x 10`); a case-insensitive match (`"WORLD"`); a no-match prune (`"zzzz"` →
`None`, `meta.len() == 1`, `data_offset == 9`); the 1-length clear (`"z"` →
window emptied); successive matches (`"ababab"` / `"ab"` →
`Some, Some, Some, None`); the empty-needle panic; and the cross-ring-boundary
overlap (a manually wrapped `VecDeque` splitting `"hello"` as
`("abhel", "locd")`, a single dangling-node meta, the match found within one
meta at `top_x 2` / `bot_x 6`).

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no Required
or Optional findings** (one Nit: the `## Result` / `## Conclusion` sections were
not yet saved — added here). Codex confirmed the implementation is faithful: it
preserves the s0 → overlap → s1 order, maps overlap offsets correctly, clears
the window for one-byte needles, and uses the reverse-meta prune with the final
`data_offset = data.len() - needle.len() + 1`; dropping the inner `data_offset`
write is correctly documented and safe (overwritten with no intervening read);
the active empty-needle assert is the right precondition encoding;
`index_of_ignore_case` matches the ASCII-only behavior; and the
wrapped-`VecDeque` overlap test is sound under the documented constraints.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r591-prompt.md` (result)
- Result: `logs/codex-review/20260604-r591-last-message.md` (result)

## Conclusion

This experiment ports `next`, completing the `SlidingWindow` matcher: search the
window's data (the two ring slices + the cross-page `overlap_buf`) for the
needle (case-insensitive ASCII), return a `Flattened` highlight on a hit
(pruning consumed pages), special-case 1-length needles, and on a miss prune to
the trailing `needle.len() - 1` overlap bytes. The `VecDeque` adaptation maps
`getPtrSlice` to `as_slices()` arithmetic, `indexOfIgnoreCase` to a file-local
helper, and `deleteOldest` to `drain`; the inner `data_offset` write is dropped
as vestigial. With the skeleton (587), encoder (588), `append` (589),
`highlight` (590), and `next` (591), the `SlidingWindow` is a complete port of
`terminal/search/sliding_window.zig`. The remaining search work is the
higher-level searchers that drive the window — `ActiveSearch`
(`search/active.zig`, the smallest, ~175 lines: search only the mutable active
area), then `PageListSearch` / `ScreenSearch` / `ViewportSearch`, and finally
the search `Thread`.
