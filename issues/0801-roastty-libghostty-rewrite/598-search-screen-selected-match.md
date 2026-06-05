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

# Experiment 598: search ScreenSearch selected_match

## Description

This experiment continues `ScreenSearch` (upstream `terminal/search/screen.zig`)
with `selectedMatch` — the accessor that returns the currently-selected result
by its index. It is **self-contained** (it reads `self.selected.idx` and the two
cached result lists; no tracked-pin dereference, no recursion into the
`reloadActive` / `select` cluster), so it can be ported and tested ahead of that
large construction/selection machinery. It extends `terminal::search::screen`.

## Upstream behavior

```zig
/// Return the selected match. Does not require screen access.
pub fn selectedMatch(self: *const ScreenSearch) ?FlattenedHighlight {
    const sel = self.selected orelse return null;
    const active_len = self.active_results.items.len;
    if (sel.idx < active_len) {
        return self.active_results.items[active_len - 1 - sel.idx];
    }
    const history_len = self.history_results.items.len;
    if (sel.idx < active_len + history_len) {
        return self.history_results.items[sel.idx - active_len];
    }
    return null;
}
```

`SelectedMatch.idx` is the index from the **end** of the combined match list (0
= the most recent match — see Experiment 595's `matches`, which orders
newest-to-oldest as reversed-active then history). So:

- `idx < active_len` indexes into the active results (stored forward,
  oldest-to-newest), reversed: `active_results[active_len - 1 - idx]`.
- `active_len <= idx < active_len + history_len` indexes into the history
  results (stored newest-to-oldest): `history_results[idx - active_len]`.
- Otherwise (out of range) → `null`.

## Rust mapping (`roastty/src/terminal/search/screen.rs`)

A direct port returning an owned `Flattened` (cloned, as `selected_match`
returns a result that must outlive the borrow; roastty's `Flattened` owns its
`Vec<Chunk>`). No `screen` / pin dereference is needed, so it stays a safe `fn`.

```rust
impl ScreenSearch {
    /// Return the currently-selected match, if any (upstream `selectedMatch`). `idx` counts from
    /// the end of the combined newest-to-oldest match list: `< active_len` indexes the (forward)
    /// active results reversed, then history results follow; out of range yields `None`. Does not
    /// access the screen.
    pub(in crate::terminal) fn selected_match(&self) -> Option<Flattened> {
        let sel = self.selected.as_ref()?;
        let active_len = self.active_results.len();
        if sel.idx < active_len {
            return Some(self.active_results[active_len - 1 - sel.idx].clone());
        }
        let history_len = self.history_results.len();
        if sel.idx < active_len + history_len {
            return Some(self.history_results[sel.idx - active_len].clone());
        }
        None
    }
}
```

## Scope / faithfulness notes

- **Ported**: `selectedMatch` → `selected_match`.
- **Faithful**: the `None` when nothing is selected; the `idx < active_len`
  reversed-active index (`active_results[active_len - 1 - idx]`); the
  `idx < active_len + history_len` history index
  (`history_results[idx - active_len]`); the out-of-range `None`.
- **Faithful adaptation**: returns an owned `Flattened` (a deep clone of the
  result, as `matches` does — roastty's `Flattened` owns its `Vec<Chunk>`),
  where upstream returns the struct by value (a shallow copy). The function
  stays safe (no `screen` / pin deref).
- **Deferred**: `init` / `reloadActive` (construction), `feed`, `searchAll`, and
  `select` / `selectNext` / `selectPrev` (the large, mutually-recursive
  tracked-pin selection cluster); plus `ViewportSearch` and the search `Thread`.
- No C ABI/header/ABI-inventory change (internal Rust). Extends
  `terminal::search::screen`.

## Changes

1. `roastty/src/terminal/search/screen.rs`: add `ScreenSearch::selected_match`;
   update the module doc comment.
2. Tests (in `screen.rs`) — build a `ScreenSearch` with a `SelectedMatch`
   (`idx`, a `highlight: Tracked` with **dangling** pins — `selected_match`
   never dereferences them) and populated result lists:
   - **selects from the active area (reversed)**: active `[a, b]` (`top_x` 1,
     2), history `[h]` (`top_x` 10). `idx 0` → `b` (`top_x` 2, most recent
     active); `idx 1` → `a` (`top_x` 1).
   - **selects from history**: `idx 2` → `h` (`top_x` 10, the first history
     result).
   - **out of range**: `idx 3` → `None`.
   - **nothing selected**: `selected: None` → `None`.
   - **active empty, history non-empty**: active `[]`, history `[h]`, `idx 0` →
     `h` (the `active_len == 0` branch — Codex's design-review Optional).
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

- `selected_match` reproduces upstream (the `None` cases, the reversed-active
  index, the history index) — faithful to `terminal/search/screen.zig`;
- the tests pass (active reversed / history / out-of-range / none), and the
  existing tests still pass;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the index arithmetic or the range handling diverges
from upstream, an unrelated item changes, or any public C API/ABI changes.

## Design Review

Codex reviewed the design and **approved it**, confirming it is faithful:
returning an owned cloned `Flattened` is the right Rust adaptation (it owns its
chunk vector, so the clone preserves upstream's returned-value semantics without
aliasing cached storage); the index math matches upstream exactly (selected
index `0` → the newest active result via `active_len - 1 - idx`, then spilling
into `history_results[idx - active_len]`); and the test plan is sound (`Tracked`
is two `NonNull<Pin>` with no `Drop`, and `selected_match` never dereferences
it, so dangling pins are fine in these accessor-only tests). One Optional,
adopted:

- **Optional (adopted)**: add a case with `active_results` empty and
  `history_results` non-empty (`idx 0` → `history[0]`) to exercise the
  `active_len == 0` branch directly.

Review artifacts:

- Prompt: `logs/codex-review/20260604-d598-prompt.md`
- Result: `logs/codex-review/20260604-d598-last-message.md`

## Result

**Result:** Pass

`ScreenSearch` gained `selected_match`: it returns `None` when nothing is
selected, otherwise indexes the combined newest-to-oldest match list by
`SelectedMatch.idx` — `active_results[active_len - 1 - idx]` for
`idx < active_len` (the reversed active area),
`history_results[idx - active_len]` for the history range, and `None` past the
end — returning an owned deep clone of the `Flattened`. It is a safe fn (no
screen / pin dereference).

Gates:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty`: 3292 passed, 0 failed (five new tests; no
  regressions, up from 3287).
- `cargo build -p roastty`: no warnings.
- no-`ghostty`-name greps (font/renderer/config + terminal/search +
  lib.rs/header/abi_harness.c) clean; `git diff --check` clean.

The five new tests build a `ScreenSearch` with a `SelectedMatch` (a `Tracked`
highlight with dangling pins, never dereferenced): reversed-active indexing
(`idx 0` → the newest active result, `idx 1` → the older), the history spillover
(`idx 2` → the first history result), out-of-range (`idx 3` → `None`), no
selection (`None`), and the `active_len == 0` branch (`idx 0` → `history[0]`).

## Completion Review

Codex reviewed the completed experiment and **approved** it with **no Required
or Optional findings** (one Nit: the `## Result` / `## Conclusion` sections were
not yet saved — added here). Codex confirmed the implementation matches
upstream's indexing exactly (no selection → `None`; active results indexed in
reverse via `active_len - 1 - idx`; history via `idx - active_len`; out-of-range
→ `None`), that returning a cloned owned `Flattened` is the correct Rust
adaptation, and that the tests (active reverse indexing, history spillover,
out-of-range, no selection, the `active_len == 0` branch) are sound with the
never-dereferenced dangling `Tracked` pins.

Review artifacts:

- Prompt: `logs/codex-review/20260604-r598-prompt.md` (result)
- Result: `logs/codex-review/20260604-r598-last-message.md` (result)

## Conclusion

This experiment ports `selected_match` — the self-contained accessor that reads
the currently-selected result by its end-relative index across the reversed
active results and the history results. It advances `ScreenSearch` without
touching the large, mutually-recursive `reload_active` / `select` /
`select_next` / `select_prev` selection cluster (which manipulates tracked pins
and re-searches on screen changes) — that construction/selection cluster, plus
`init` and `feed` / `search_all`, is the remaining `ScreenSearch` work, followed
by `ViewportSearch` (`search/viewport.zig`) and the search `Thread`.
