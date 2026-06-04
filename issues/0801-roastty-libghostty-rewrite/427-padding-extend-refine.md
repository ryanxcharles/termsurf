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

# Experiment 427: the per-row padding-extend refinement (refine_padding_extend)

## Description

Experiment 422 ported the full-rebuild `padding_extend` reset (all edges for
`extend`/`extend-always`). Upstream then **refines** it per row, for `extend`
mode only: the top edge (`up`) is disabled on the first row, and the bottom edge
(`down`) on the last row, when that row "never extends" its background
(`neverExtendBg`). This experiment ports that refinement as a method that takes
the row's `never_extend` result as a parameter — deferring only the
`neverExtendBg` computation (which needs the renderer's terminal-core row/cell
representation, as in Experiment 423). It composes Experiment 422's reset and
the `EXTEND_*` bit constants.

## Upstream behavior

In `rebuildCells` (`renderer/generic.zig`), per row, after the reset:

```zig
switch (self.config.padding_color) {
    // These already have the correct values set above (the reset).
    .background, .@"extend-always" => {},

    // Apply heuristics for padding extension.
    .extend => if (y == 0) {
        self.uniforms.padding_extend.up = !rowNeverExtendBg(row, …);
    } else if (y == self.cells.size.rows - 1) {
        self.uniforms.padding_extend.down = !rowNeverExtendBg(row, …);
    },
}
```

So for `extend` only: the first row sets `up = !neverExtend`, the last row sets
`down = !neverExtend` (the `else if` means a single-row grid takes only the `up`
branch); middle rows are unchanged. `background` / `extend-always` are no-ops
(the reset already set them).

## Rust mapping (`roastty/src/renderer/metal/shaders.rs`)

`refine_padding_extend` takes the padding color, whether the row is first/last,
and the row's `never_extend` (upstream's `rowNeverExtendBg` result), and sets
the `up` / `down` bit accordingly:

```rust
impl MetalUniforms {
    /// Refine `padding_extend` for one row (upstream `rebuildCells`'s per-row
    /// heuristic, `extend` mode only): the first row's `up` edge and the last
    /// row's `down` edge are enabled unless the row "never extends" its
    /// background (`never_extend`, upstream's `rowNeverExtendBg`). `background` /
    /// `extend-always` are no-ops (the reset already set them).
    pub(crate) fn refine_padding_extend(
        &mut self,
        padding_color: WindowPaddingColor,
        is_first_row: bool,
        is_last_row: bool,
        never_extend: bool,
    ) {
        if padding_color != WindowPaddingColor::Extend {
            return;
        }
        if is_first_row {
            self.set_padding_extend_bit(EXTEND_UP, !never_extend);
        } else if is_last_row {
            self.set_padding_extend_bit(EXTEND_DOWN, !never_extend);
        }
    }

    fn set_padding_extend_bit(&mut self, bit: u8, on: bool) {
        if on {
            self.padding_extend |= bit;
        } else {
            self.padding_extend &= !bit;
        }
    }
}
```

The `if is_first_row … else if is_last_row` matches upstream's
`if (y == 0) … else if (y == rows - 1)` (a single-row grid takes only the `up`
branch). Setting `up`/`down` to `!never_extend` matches upstream; a small
`set_padding_extend_bit` helper does the bit set/clear.

## Scope / faithfulness notes

- **Ported (bridged)**: `MetalUniforms::refine_padding_extend` — the per-row
  `padding_extend` refinement (the `up`/`down` heuristic for `extend` mode),
  upstream's `rebuildCells` per-row `switch`.
- **Faithful**: `extend` only; the first row sets `up = !never_extend`, the last
  row (when not first) sets `down = !never_extend`; `background` /
  `extend-always` are no-ops; the bit set/clear uses the `EXTEND_UP` /
  `EXTEND_DOWN` constants (Experiment 422).
- **Faithful adaptation**: `never_extend` (upstream's
  `rowNeverExtendBg(row, …)`) is a parameter — the computation needs the
  renderer's terminal-core row/cell representation (not yet present; see
  Experiment 423's `is_perfect_fit_powerline` precursor), so the refinement
  logic is ported and tested with `never_extend` supplied, the same deferral
  pattern as the preedit-range parameters.
- **Deferred**: the full `neverExtendBg` (the semantic-prompt / per-cell checks
  composing `is_perfect_fit_powerline`), and the live `rebuildCells` call site
  that computes `never_extend` per row and runs this. (Consumed by a later
  slice; this experiment lands and tests the refinement.)
- No C ABI/header/ABI-inventory change (internal Rust).

## Changes

1. `roastty/src/renderer/metal/shaders.rs`:
   - add
     `MetalUniforms::refine_padding_extend(&mut self, padding_color, is_first_row, is_last_row, never_extend)`
     and a private `set_padding_extend_bit(&mut self, bit, on)` helper.
2. Tests (in `shaders.rs`), starting from `padding_extend == 15` (the
   `extend`-mode reset state):
   - `Extend`, first row, `never_extend = true` → `up` cleared (`11`);
     `never_extend = false` → `up` stays (`15`);
   - `Extend`, last row (not first), `never_extend = true` → `down` cleared
     (`7`);
   - `Extend`, a middle row (neither first nor last) → unchanged (`15`);
   - a single-row grid (`is_first_row` and `is_last_row` both true), `Extend`,
     `never_extend = true` → only `up` cleared (`11`), `down` untouched;
   - `Background` and `ExtendAlways` (any row) → unchanged (`15`).
3. Format and test (`cargo fmt`, accept output).

## Verification

```bash
cargo fmt
cargo fmt -- --check
cargo test -p roastty refine_padding_extend
cargo test -p roastty
cargo build -p roastty            # no warnings
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/font roastty/src/renderer roastty/src/config && exit 1 || true
rg -n 'ghostty|Ghostty|GHOSTTY' roastty/src/lib.rs roastty/include/roastty.h roastty/tests/abi_harness.c && exit 1 || true
git diff --check
```

The experiment **passes** if:

- `refine_padding_extend` sets the first row's `up` and the last row's `down` to
  `!never_extend` for `extend` mode (with the `else if` single-row behavior),
  and is a no-op for `background` / `extend-always` — faithful to upstream's
  per-row `switch`;
- the tests pass (the up/down clear and keep; the middle-row and single-row
  cases; the non-`extend` no-ops), and the existing tests still pass;
- the full `neverExtendBg` and the live call site stay deferred;
- `cargo fmt` accepted, `cargo build -p roastty` has no warnings, and
  `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment **fails** if the wrong edge/row is refined, the single-row
`else if` behavior differs from upstream, a non-`extend` mode is touched, or any
public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation and **approved** it with **no
findings**. It confirmed the design matches upstream's per-row refinement:
`WindowPaddingColor::Extend` is the only active mode, `Background` and
`ExtendAlways` are no-ops, the first row updates `EXTEND_UP`, the last row
updates `EXTEND_DOWN`, and the `else if` correctly preserves upstream's
single-row behavior (taking only the first-row/up branch). It confirmed the bit
handling is sound (`|= bit` and `&= !bit` set or clear only the targeted `u8`
flag, with `EXTEND_UP = 4` / `EXTEND_DOWN = 8` the right shader constants), and
that passing `never_extend` as a parameter is a reasonable bounded slice because
the full `rowNeverExtendBg` still depends on terminal-core row/cell data not
present in this layer. It judged the planned tests to cover the important
behavior, including the single-row edge case and the non-`Extend` no-ops.

Review artifacts:

- Prompt: `logs/codex-review/20260604-091451-d427-prompt.md` (design)
- Result: `logs/codex-review/20260604-091451-d427-last-message.md` (design)

## Result

**Result:** Pass

The per-row padding-extend refinement is now live.

- `roastty/src/renderer/metal/shaders.rs`:
  `MetalUniforms::refine_padding_extend(&mut self, padding_color, is_first_row, is_last_row, never_extend)`
  — `Extend` mode only; the first row sets `up` (`EXTEND_UP`) and the last row
  (`else if`) sets `down` (`EXTEND_DOWN`) to `!never_extend`; `Background` /
  `ExtendAlways` are no-ops. A private
  `set_padding_extend_bit(&mut self, bit, on)` helper sets/clears a single bit.

Test (in `shaders.rs`): `refine_padding_extend_applies_extend_heuristics` — from
`padding_extend == 15`: `Extend` first `never_extend=true` → `11` (up cleared);
`Extend` first `never_extend=false` → `15` (up set); `Extend` last (not first)
`never_extend=true` → `7` (down cleared); `Extend` middle row → `15`; a
single-row grid (first AND last) `Extend` `never_extend=true` → `11` (only `up`,
via the `else if`); `Background` / `ExtendAlways` → `15` (no-op).

Gate results:

- `cargo fmt -p roastty` accepted; `--check` clean.
- `cargo test -p roastty` → 2908 passed, 0 failed (+1, no regressions).
- `cargo build -p roastty` → no warnings.
- No-`ghostty`-name gates (font + renderer + config +
  `lib.rs`/header/`abi_harness.c`) clean; `git diff --check` clean.

## Conclusion

The `padding_extend` logic is now fully shaped: the full-rebuild reset
(Experiment 422) and the per-row `Extend`-mode refinement (this experiment),
with only the `never_extend` computation (`neverExtendBg`) deferred — it needs
the renderer's terminal-core row/cell representation (the
`is_perfect_fit_powerline` predicate from Experiment 423 is its codepoint half).
The remaining renderer-bridge work: the full `neverExtendBg` (awaiting that
row/cell representation), the live per-frame call sites (the renderer `init` and
`updateFrame`/`drawFrame`, which depend on the live render `State`), and the
custom-shader uniforms; beyond the renderer, the other subsystems.

## Completion Review

Codex reviewed the completed implementation and result and **approved** with
**no findings**. It confirmed `refine_padding_extend` matches upstream's branch
(active only for `WindowPaddingColor::Extend`; the first row maps to
`EXTEND_UP = !never_extend`, the last row to `EXTEND_DOWN = !never_extend`, and
the `else if` preserves the single-row behavior where only the up edge is
refined; `Background` and `ExtendAlways` are no-ops). It confirmed the bit
helper is correct for `u8` (`|= bit` sets only the target flag, `&= !bit` clears
only that flag), and that the test covers the up/down clear paths, the
set-back-to-on path, the middle-row no-op, the single-row `else if`, and the
non-`Extend` no-ops. No public C ABI/header impact; nothing needed to change
before the result commit.

Review artifacts:

- Prompt: `logs/codex-review/20260604-091638-r427-prompt.md` (result)
- Result: `logs/codex-review/20260604-091638-r427-last-message.md` (result)
