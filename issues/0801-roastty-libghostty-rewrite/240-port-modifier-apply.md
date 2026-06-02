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

# Experiment 240: Port `Modifier::apply` (u32 / i32 / f64)

## Description

Port `Modifier::apply` from upstream `font/Metrics.zig` — the numeric
application of a `Modifier` to a metric value. Upstream `apply` is a comptime
generic over any int (signed or unsigned) or float type. The `Metrics` fields it
is applied to are `u32` (most), `i32` (`overline_position`), and `f64` (the
icon/face fields), so the Rust port provides **three concrete methods** —
`apply_u32`, `apply_i32`, `apply_f64` — covering exactly those field types.
`Metrics::apply` (which dispatches per field), `ModifierSet`, and `Key` remain
deferred.

### Upstream behavior (lines 514–547)

For an **integer** `T`:

- `percent p`: `applied = round(v_as_f64 * max(0, p))`, then converted to `T`.
- `absolute abs`: `applied = v_i64 +| abs_i64` (saturating add in `i64`); if `T`
  is **unsigned**, clamp to `max(0, applied)`; then cast to `T`, saturating to
  `T`'s max on overflow.

For a **float** `T`:

- `percent p`: `v * max(0, p)`.
- `absolute abs`: `v + abs_as_f64`.

`max(0, p)` re-clamps the percent multiplier (it is already `>= 0` from `parse`,
but `apply` does not assume that).

### Rust mapping

- `apply_u32(self, v: u32) -> u32`:
  - `Percent(p)`:
    `((v as f64) * p.max(0.0)).round().clamp(0.0, u32::MAX as f64) as u32` (the
    source is `>= 0`; the clamp guards overflow that upstream's `@intFromFloat`
    would hit).
  - `Absolute(abs)`:
    `(v as i64).saturating_add(abs as i64).clamp(0, u32::MAX as i64) as u32`
    (the `clamp` lower bound `0` is the unsigned `max(0, …)`).
- `apply_i32(self, v: i32) -> i32`:
  - `Percent(p)`:
    `((v as f64) * p.max(0.0)).round().clamp(i32::MIN as f64, i32::MAX as f64) as i32`.
  - `Absolute(abs)`:
    `(v as i64).saturating_add(abs as i64).clamp(-(i32::MAX as i64), i32::MAX as i64) as i32`
    (signed — **no** lower clamp to 0). The lower bound is `-i32::MAX`
    (`-2147483647`), **not** `i32::MIN`: upstream saturates a failed cast to
    `maxInt(T) * sign`, so a negative overflow becomes `-i32::MAX`, never
    `i32::MIN`.
- `apply_f64(self, v: f64) -> f64`:
  - `Percent(p)`: `v * p.max(0.0)`.
  - `Absolute(abs)`: `v + abs as f64`.

### Faithfulness and scope notes

- `round` matches upstream `@round`; the saturating add (`+|`) maps to
  `i64::saturating_add`; the unsigned `max(0, …)` and the on-overflow saturation
  map to `clamp` to the target type's range.
- The float `percent`/`absolute` are direct (`v * p` / `v + abs`), no rounding.
- `apply_*` are `pub(crate)`, added to `impl Modifier` in
  `roastty/src/font/metrics.rs`.
- No `Metrics::apply`/`ModifierSet`/`Key`/`hash`/`formatEntry` behavior.
- No C ABI, header, or ABI inventory changes; no new dependencies.

## Changes

1. `roastty/src/font/metrics.rs`: add `apply_u32`, `apply_i32`, `apply_f64` to
   `impl Modifier`.

2. Tests in `roastty/src/font/metrics.rs`:
   - `apply_u32_percent`: `Percent(1.2).apply_u32(10) == 12`;
     `Percent(0.8).apply_u32(10) == 8`; `Percent(-1.0)` (negative re-clamped to
     `0`) → `0`.
   - `apply_u32_absolute`: `Absolute(5).apply_u32(10) == 15`;
     `Absolute(-3).apply_u32(10) == 7`; `Absolute(-20).apply_u32(10) == 0`
     (clamped to 0, not wrapped).
   - `apply_u32_saturates`:
     `Absolute(i32::MAX).apply_u32(u32::MAX) == u32::MAX`.
   - `apply_i32_signed`: `Absolute(-20).apply_i32(10) == -10` (no clamp to 0);
     `Percent(1.5).apply_i32(-4) == -6`.
   - `apply_i32_negative_overflow_saturates`:
     `Absolute(i32::MIN).apply_i32(i32::MIN) == -i32::MAX` (`-2147483647`, the
     upstream `maxInt * sign` saturation — not `i32::MIN`).
   - `apply_f64`: `Percent(1.2).apply_f64(10.0)` ≈ `12.0` (epsilon);
     `Absolute(5).apply_f64(10.0) == 15.0`;
     `Absolute(-3).apply_f64(2.5) == -0.5`;
     `Percent(-1.0).apply_f64(10.0) == 0.0` (negative re-clamped).

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

- `apply_u32`/`apply_i32`/`apply_f64` reproduce upstream exactly — percent
  rounds `v * max(0, p)`, absolute saturating-adds, the unsigned path clamps to
  `0` and the signed path does not, and overflow saturates to the target's
  range;
- the percent, absolute, signed, saturation, and float tests pass;
- no `Metrics::apply`/`ModifierSet`/`Key` scope is pulled in;
- no C ABI, header, or ABI inventory changes;
- `cargo fmt` accepted and `cargo test -p roastty` passes with no regressions;
- the no-`ghostty`-name gates and `git diff --check` pass;
- Codex reviews the design before implementation and the result after, with all
  real findings fixed.

The experiment is **partial** if a field type beyond `u32`/`i32`/`f64` turns out
to need applying once `Metrics::apply` is designed.

The experiment **fails** if the percent rounding, the saturating-add, or the
signed-vs-unsigned clamp diverges from upstream, if `Metrics::apply` scope leaks
in, or if any public C API/ABI changes.

## Design Review

Codex reviewed this design before implementation.

Review artifacts:

- Prompt: `logs/codex-review/20260602-084809-122583-prompt.md`
- Result: `logs/codex-review/20260602-084809-122583-last-message.md`

Codex confirmed the percent (`round(v * max(0, p))`), float (direct), and
unsigned-absolute (clamp-below-0, saturate-above-to-max) behavior is faithful,
and that the normal-path expectations are correct.

One Medium finding, fixed in the design above before this commit:

1. the signed `apply_i32` absolute overflow must saturate to `-i32::MAX`, not
   `i32::MIN`: upstream saturates a failed cast to `maxInt(T) * sign`. Changed
   the clamp lower bound to `-(i32::MAX as i64)` and added
   `apply_i32_negative_overflow_saturates`
   (`Absolute(i32::MIN).apply_i32(i32::MIN) == -i32::MAX`). The percent path
   keeps the full-range `[i32::MIN, i32::MAX]` defensive clamp, as it does not
   use the `maxInt * sign` saturation.

## Result

**Result:** Pass

Added `apply_u32`, `apply_i32`, and `apply_f64` to `impl Modifier` in
`roastty/src/font/metrics.rs`. `Percent` rounds `v * max(0, p)` (the integer
variants clamp to the target range, defensively for the percent path);
`Absolute` saturating-adds in `i64` — `apply_u32` clamps to `[0, u32::MAX]` (the
unsigned clamp-below-0 and saturate-above), `apply_i32` clamps to
`[-i32::MAX, i32::MAX]` (the `maxInt * sign` saturation, so negative overflow is
`-i32::MAX`, not `i32::MIN`); `apply_f64` is direct (`v * max(0, p)` /
`v + abs`).

Tests added (6): `apply_u32_percent`, `apply_u32_absolute` (including the
clamp-to-0 case), `apply_u32_saturates`, `apply_i32_signed`,
`apply_i32_negative_overflow_saturates`
(`Absolute(i32::MIN).apply_i32(i32::MIN) == -i32::MAX`), and `apply_f64`.

### Verification

```bash
cargo fmt -p roastty
cargo test -p roastty font
cargo test -p roastty
```

Observed:

- `font`: 33 passed (27 prior + 6 new).
- Full `roastty`: 2309 unit tests passed (2303 prior + 6 new), plus the C ABI
  harness passed.
- `cargo fmt -p roastty -- --check`: clean.
- `cargo build -p roastty`: no warnings.
- No-`ghostty`-name gates passed for `roastty/src/font` and for
  `roastty/src/lib.rs`, `roastty/include/roastty.h`,
  `roastty/tests/abi_harness.c`.
- `git diff --check`: clean.

No C ABI, header, or ABI inventory changes; no `Metrics::apply`/`ModifierSet`/
`Key` scope pulled in.

### Completion Review

Codex reviewed the completed implementation and found **no issues** ("nothing
needs to change before the result commit").

Review artifacts:

- Prompt: `logs/codex-review/20260602-085047-917514-prompt.md`
- Result: `logs/codex-review/20260602-085047-917514-last-message.md`

Codex confirmed all three methods match upstream (integer percent
`round(v * max(0, p))`, float `v * max(0, p)`, absolute `i64::saturating_add`
with the unsigned `[0, u32::MAX]` clamp and the signed `[-i32::MAX, i32::MAX]`
saturation matching `maxInt * sign`), that the six tests cover the normal,
clamp, and both overflow-saturation paths, and that there are no
precision/visibility/scope concerns.

## Conclusion

Experiment 240 succeeds. `Modifier` now applies to the `u32`/`i32`/`f64` metric
field types with the exact upstream rounding and saturation. Both Codex gates
passed (one design finding fixed — the `-i32::MAX` signed-overflow saturation;
zero result findings).

With `Modifier`, `parse`, and `apply` all in place, the remaining `Metrics.zig`
modifier work is the `Key` enum (one per `Metrics` field), the `ModifierSet` map
(`Key → Modifier`), and `Metrics::apply` — which iterates a `ModifierSet`,
dispatches each `Modifier` to the right `apply_*` for the keyed field, and
carries the special cell-height-adjustment logic (re-centering the baseline) and
the icon-height pairing. `Metrics::apply` holds the actual upstream
`Metrics.zig` tests and is the next slice, completing that file's behavior.
